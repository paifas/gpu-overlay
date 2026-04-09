use super::{GpuMetrics, GpuMonitor};
use std::path::Path;

pub struct NvidiaMonitor {
    smi_path: String,
}

impl NvidiaMonitor {
    pub fn is_available() -> bool {
        which_nvidia_smi().is_some()
    }

    pub fn new() -> Self {
        Self {
            smi_path: which_nvidia_smi().unwrap_or_else(|| "nvidia-smi".to_string()),
        }
    }
}

impl GpuMonitor for NvidiaMonitor {
    fn metrics(&mut self) -> Vec<GpuMetrics> {
        let output = match std::process::Command::new(&self.smi_path)
            .args([
                "--query-gpu=name,temperature.gpu,temperature.memory,utilization.gpu,utilization.memory,memory.used,memory.total,clocks.current.sm,clocks.current.memory",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 9 {
                continue;
            }

            let parse_f = |s: &str| -> Option<f32> {
                let s = s.trim();
                if s == "[N/A]" || s.is_empty() {
                    return None;
                }
                s.parse().ok()
            };

            results.push(GpuMetrics {
                name: parts[0].to_string(),
                core_temp: parse_f(parts[1]),
                memory_temp: parse_f(parts[2]),
                core_utilization: parse_f(parts[3]),
                memory_utilization: parse_f(parts[4]),
                vram_used_mb: parse_f(parts[5]),
                vram_total_mb: parse_f(parts[6]),
                core_clock_mhz: parse_f(parts[7]),
                memory_clock_mhz: parse_f(parts[8]),
            });
        }

        results
    }
}

fn which_nvidia_smi() -> Option<String> {
    let candidates = ["/usr/bin/nvidia-smi", "/usr/local/bin/nvidia-smi"];
    for p in &candidates {
        if Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    // Try PATH lookup
    if let Some(var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&var) {
            let full = dir.join("nvidia-smi");
            if full.is_file() {
                return Some(full.to_string_lossy().to_string());
            }
        }
    }
    None
}
