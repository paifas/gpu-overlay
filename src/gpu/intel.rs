use super::{GpuMetrics, GpuMonitor};
use std::fs;
use std::path::{Path, PathBuf};

pub struct IntelMonitor {
    card_path: Option<PathBuf>,
    hwmon_path: Option<PathBuf>,
    card_name: Option<String>,
}

impl IntelMonitor {
    pub fn is_available() -> bool {
        !find_intel_cards().is_empty()
    }

    pub fn new() -> Self {
        let cards = find_intel_cards();
        let card_path = cards.into_iter().next();
        let hwmon_path = card_path.as_ref().and_then(|p| find_hwmon(p));
        let card_name = card_path.as_ref().and_then(|p| {
            fs::read_to_string(p.join("device/driver/module"))
                .ok()
                .map(|_| "Intel GPU".to_string())
                .or_else(|| {
                    p.file_name()
                        .map(|n| format!("Intel ({})", n.to_string_lossy()))
                })
        });
        Self {
            card_path,
            hwmon_path,
            card_name,
        }
    }
}

impl GpuMonitor for IntelMonitor {
    fn metrics(&mut self) -> Vec<GpuMetrics> {
        let card = match &self.card_path {
            Some(p) => p,
            None => return Vec::new(),
        };

        let hwmon = self.hwmon_path.as_ref();

        let core_temp = hwmon.and_then(|h| {
            fs::read_to_string(h.join("temp1_input"))
                .ok()
                .and_then(|v| v.trim().parse::<f32>().ok())
                .map(|v| v / 1000.0)
        });

        let core_clock_mhz = fs::read_to_string(card.join("device/gt_act_freq_mhz"))
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok());

        let max_clock_mhz = fs::read_to_string(card.join("device/gt_max_freq_mhz"))
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok());

        let core_utilization = match (core_clock_mhz, max_clock_mhz) {
            (Some(act), Some(max)) if max > 0.0 => Some(act / max * 100.0),
            _ => None,
        };

        let name = self
            .card_name
            .clone()
            .unwrap_or_else(|| "Intel GPU".to_string());

        vec![GpuMetrics {
            name,
            core_temp,
            memory_temp: None,
            core_utilization,
            memory_utilization: None,
            vram_used_mb: None,
            vram_total_mb: None,
            core_clock_mhz,
            memory_clock_mhz: None,
        }]
    }
}

fn find_intel_cards() -> Vec<PathBuf> {
    let base = Path::new("/sys/class/drm");
    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut cards = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let driver_link = path.join("device/driver");
        let target = match std::fs::read_link(&driver_link) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if target
            .file_name()
            .map(|n| n.to_string_lossy().contains("i915"))
            .unwrap_or(false)
        {
            cards.push(path);
        }
    }
    cards
}

fn find_hwmon(card_path: &Path) -> Option<PathBuf> {
    let hwmon_dir = card_path.join("device/hwmon");
    let entries = fs::read_dir(&hwmon_dir).ok()?;
    let mut first: Option<PathBuf> = None;
    for entry in entries.flatten() {
        let p = entry.path();
        if first.is_none() {
            first = Some(p.clone());
        }
        if p.join("temp1_input").exists() {
            return Some(p);
        }
    }
    first
}
