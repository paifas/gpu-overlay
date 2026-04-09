use super::{GpuMetrics, GpuMonitor};

pub struct AppleMonitor {
    gpu_name: String,
    util_state: Option<UtilState>,
}

struct UtilState {
    last_busy: u64,
    last_total: u64,
}

impl AppleMonitor {
    pub fn is_available() -> bool {
        if !cfg!(target_os = "macos") {
            return false;
        }
        std::process::Command::new("ioreg")
            .args(["-r", "-c", "AGXAccelerator"])
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn new() -> Self {
        let gpu_name = query_gpu_name();
        Self {
            gpu_name,
            util_state: None,
        }
    }
}

impl GpuMonitor for AppleMonitor {
    fn metrics(&mut self) -> Vec<GpuMetrics> {
        let core_temp = get_gpu_temp();
        let core_util = get_gpu_util(&mut self.util_state);
        let (vram_used, vram_total) = get_vram();
        let core_clock = get_clock_speed();

        vec![GpuMetrics {
            name: self.gpu_name.clone(),
            core_temp,
            memory_temp: None,
            core_utilization: core_util,
            memory_utilization: None,
            vram_used_mb: vram_used,
            vram_total_mb: vram_total,
            core_clock_mhz: core_clock,
            memory_clock_mhz: None,
        }]
    }
}

fn query_gpu_name() -> String {
    let output = std::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output();
    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Chipset Model:") {
                if let Some(idx) = trimmed.find(':') {
                    return trimmed[idx + 1..].trim().to_string();
                }
            }
        }
    }
    "Apple GPU".to_string()
}

fn get_gpu_temp() -> Option<f32> {
    let output = std::process::Command::new("ioreg")
        .args(["-l", "-w0"])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout);
    for line in s.lines() {
        if line.contains("temp") && line.contains("GPU") {
            if let Some(val) = line.split("\"temp\"").nth(1) {
                let num: String = val
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(t) = num.parse::<f32>() {
                    return Some(t);
                }
            }
        }
    }
    None
}

fn get_gpu_util(state: &mut Option<UtilState>) -> Option<f32> {
    let output = std::process::Command::new("ioreg")
        .args(["-l", "-w0"])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout);

    let mut busy: u64 = 0;
    let mut total: u64 = 0;

    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.contains("\"AccumulatedBusyTime\"") {
            if let Some(val) = extract_hex_u64(trimmed) {
                busy = val;
            }
        }
        if trimmed.contains("\"TotalRunningTime\"") {
            if let Some(val) = extract_hex_u64(trimmed) {
                total = val;
            }
        }
    }

    if total == 0 {
        return None;
    }

    let util = match state {
        Some(prev) => {
            let db = busy.saturating_sub(prev.last_busy) as f64;
            let dt = total.saturating_sub(prev.last_total) as f64;
            if dt > 0.0 {
                (db / dt * 100.0) as f32
            } else {
                0.0
            }
        }
        None => 0.0,
    };

    *state = Some(UtilState {
        last_busy: busy,
        last_total: total,
    });
    Some(util)
}

fn get_vram() -> (Option<f32>, Option<f32>) {
    let output = match std::process::Command::new("ioreg")
        .args(["-r", "-c", "AGXAccelerator"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return (None, None),
    };
    let s = String::from_utf8_lossy(&output.stdout);

    let mut used: Option<f32> = None;
    let mut total: Option<f32> = None;

    for line in s.lines() {
        let trimmed = line.trim();
        if let Some(val) = extract_perf_stat(trimmed, "Used VRAM") {
            used = Some(val / (1024.0 * 1024.0));
        }
        if let Some(val) = extract_perf_stat(trimmed, "Total VRAM") {
            total = Some(val / (1024.0 * 1024.0));
        }
    }

    (used, total)
}

fn get_clock_speed() -> Option<f32> {
    let output = std::process::Command::new("ioreg")
        .args(["-r", "-c", "AGXAccelerator"])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout);

    for line in s.lines() {
        let trimmed = line.trim();
        if let Some(val) = extract_perf_stat(trimmed, "Frequency") {
            return Some(val);
        }
        if let Some(val) = extract_perf_stat(trimmed, "GPU Domain") {
            return Some(val);
        }
    }
    None
}

fn extract_perf_stat(line: &str, key: &str) -> Option<f32> {
    let search = format!("\"{}\"", key);
    if !line.contains(&search) {
        return None;
    }
    if let Some(idx) = line.find('=') {
        let rest = &line[idx + 1..].trim();
        let num: String = rest
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        num.parse::<f32>().ok()
    } else {
        None
    }
}

fn extract_hex_u64(s: &str) -> Option<u64> {
    let part = s.split('=').nth(1)?.trim();
    let digits = part.trim_start_matches("0x").trim_start_matches("0X");
    u64::from_str_radix(digits, 16).ok()
}
