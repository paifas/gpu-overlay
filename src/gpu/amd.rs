use super::{GpuMetrics, GpuMonitor};
use std::fs;
use std::path::Path;

pub struct AmdMonitor {
    hwmon_path: Option<String>,
    card_name: Option<String>,
}

impl AmdMonitor {
    pub fn is_available() -> bool {
        find_amd_hwmon().is_some()
    }

    pub fn new() -> Self {
        let hwmon = find_amd_hwmon();
        let card_name = hwmon.as_ref().and_then(|p| {
            let name_path = format!("{}/device/product_number", p);
            fs::read_to_string(&name_path)
                .ok()
                .map(|s| s.trim().to_string())
        });
        Self {
            hwmon_path: hwmon,
            card_name,
        }
    }
}

impl GpuMonitor for AmdMonitor {
    fn metrics(&mut self) -> Vec<GpuMetrics> {
        let hwmon = match &self.hwmon_path {
            Some(p) => p,
            None => return Vec::new(),
        };

        let read_file = |name: &str| -> Option<String> {
            fs::read_to_string(format!("{}/{}", hwmon, name))
                .ok()
                .map(|s| s.trim().to_string())
        };

        let core_temp = read_file("temp1_input")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| v / 1000.0);

        let memory_temp = read_file("temp2_input")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| v / 1000.0);

        let core_utilization = read_file("gpu_busy_percent")
            .and_then(|v| v.parse::<f32>().ok());

        let memory_utilization = read_file("mem_busy_percent")
            .and_then(|v| v.parse::<f32>().ok());

        let vram_used_mb = read_file("mem_info_vram_used")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| v / (1024.0 * 1024.0));

        let vram_total_mb = read_file("mem_info_vram_total")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| v / (1024.0 * 1024.0));

        let core_clock_mhz = read_file("freq1_input")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| v / 1_000_000.0);

        let memory_clock_mhz = read_file("freq2_input")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| v / 1_000_000.0);

        let name = self
            .card_name
            .clone()
            .unwrap_or_else(|| "AMD GPU".to_string());

        vec![GpuMetrics {
            name,
            vendor: None,
            core_temp,
            memory_temp,
            core_utilization,
            memory_utilization,
            vram_used_mb,
            vram_total_mb,
            core_clock_mhz,
            memory_clock_mhz,
        }]
    }
}

fn find_amd_hwmon() -> Option<String> {
    let base = Path::new("/sys/class/drm");
    let entries = fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let hwmon_dir = entry.path().join("device/hwmon");
        if let Ok(hwmon_entries) = fs::read_dir(&hwmon_dir) {
            for he in hwmon_entries.flatten() {
                let hp = he.path();
                if hp.join("temp1_input").exists() || hp.join("gpu_busy_percent").exists() {
                    return Some(hp.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}
