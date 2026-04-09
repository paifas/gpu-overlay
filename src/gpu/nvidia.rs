use super::{GpuMetrics, GpuMonitor};
use std::path::Path;
use std::collections::HashMap;

pub struct NvidiaMonitor {
    smi_path: String,
    vendor_cache: HashMap<String, String>,
}

impl NvidiaMonitor {
    pub fn is_available() -> bool {
        which_nvidia_smi().is_some()
    }

    pub fn new() -> Self {
        Self {
            smi_path: which_nvidia_smi().unwrap_or_else(|| "nvidia-smi".to_string()),
            vendor_cache: HashMap::new(),
        }
    }

    fn resolve_vendor(&mut self, bus_id: &str) -> Option<String> {
        if let Some(v) = self.vendor_cache.get(bus_id) {
            return Some(v.clone());
        }
        // nvidia-smi gives "00000000:0A:00.0", sysfs uses "0000:0a:00.0"
        let lower = bus_id.to_lowercase();
        let parts: Vec<&str> = lower.split(':').collect();
        let sysfs_name = if parts.len() >= 3 {
            // Truncate domain to 4 hex digits
            let domain = parts[0].trim_start_matches('0');
            format!("{:0>4}:{}:{}", domain, parts[1], parts[2])
        } else {
            lower.clone()
        };
        let path = format!("/sys/bus/pci/devices/{}/subsystem_vendor", sysfs_name);
        let raw = std::fs::read_to_string(&path).ok()?;
        let hex = raw.trim().trim_start_matches("0x").trim_start_matches('0');
        let vendor = match hex {
            "1043" => "ASUS",
            "1462" => "MSI",
            "10de" => "NVIDIA",
            "19da" => "ZOTAC",
            "3842" => "EVGA",
            "7394" => "PNY",
            "1b4c" => "GIGABYTE",
            "1569" => "Colorful",
            _ => hex,
        }.to_string();
        self.vendor_cache.insert(bus_id.to_string(), vendor.clone());
        Some(vendor)
    }
}

impl GpuMonitor for NvidiaMonitor {
    fn metrics(&mut self) -> Vec<GpuMetrics> {
        let output = match std::process::Command::new(&self.smi_path)
            .args([
                "--query-gpu=name,gpu_bus_id,temperature.gpu,temperature.memory,utilization.gpu,utilization.memory,memory.used,memory.total,clocks.current.sm,clocks.current.memory",
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
            if parts.len() < 10 {
                continue;
            }

            let parse_f = |s: &str| -> Option<f32> {
                let s = s.trim();
                if s == "[N/A]" || s.is_empty() {
                    return None;
                }
                s.parse().ok()
            };

            let vendor = self.resolve_vendor(parts[1]);

            results.push(GpuMetrics {
                name: parts[0].to_string(),
                vendor,
                core_temp: parse_f(parts[2]),
                memory_temp: parse_f(parts[3]),
                core_utilization: parse_f(parts[4]),
                memory_utilization: parse_f(parts[5]),
                vram_used_mb: parse_f(parts[6]),
                vram_total_mb: parse_f(parts[7]),
                core_clock_mhz: parse_f(parts[8]),
                memory_clock_mhz: parse_f(parts[9]),
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
