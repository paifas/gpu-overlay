#[cfg(target_os = "macos")]
pub mod apple;
pub mod nvidia;
#[cfg(target_os = "linux")]
pub mod amd;
#[cfg(target_os = "linux")]
pub mod intel;

#[derive(Debug, Clone)]
pub struct GpuMetrics {
    pub name: String,
    pub core_temp: Option<f32>,
    pub memory_temp: Option<f32>,
    pub core_utilization: Option<f32>,
    pub memory_utilization: Option<f32>,
    pub vram_used_mb: Option<f32>,
    pub vram_total_mb: Option<f32>,
    pub core_clock_mhz: Option<f32>,
    pub memory_clock_mhz: Option<f32>,
}

pub trait GpuMonitor: Send {
    fn metrics(&mut self) -> Vec<GpuMetrics>;
}

pub fn detect_monitors() -> Vec<Box<dyn GpuMonitor>> {
    let mut monitors: Vec<Box<dyn GpuMonitor>> = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if apple::AppleMonitor::is_available() {
            monitors.push(Box::new(apple::AppleMonitor::new()));
        }
    }

    if nvidia::NvidiaMonitor::is_available() {
        monitors.push(Box::new(nvidia::NvidiaMonitor::new()));
    }

    #[cfg(target_os = "linux")]
    {
        if amd::AmdMonitor::is_available() {
            monitors.push(Box::new(amd::AmdMonitor::new()));
        }
        if intel::IntelMonitor::is_available() {
            monitors.push(Box::new(intel::IntelMonitor::new()));
        }
    }

    monitors
}
