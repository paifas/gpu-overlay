use egui::{Color32, RichText};

use crate::gpu::GpuMetrics;

pub fn draw_panel(egui_ctx: &egui::Context, metrics: &[GpuMetrics]) -> (f32, f32) {
    let mut size = (0.0f32, 0.0f32);
    egui::Area::new(egui::Id::new("gpu-overlay"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(egui_ctx, |ui| {
            draw_gpu_panel(ui, metrics);
            let rect = ui.min_rect();
            size = (rect.width(), rect.height());
        });
    size
}

fn draw_gpu_panel(ui: &mut egui::Ui, metrics: &[GpuMetrics]) {
    let panel_frame = egui::Frame::default()
        .fill(Color32::from_rgba_unmultiplied(20, 20, 30, 140))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(8));

    ui.vertical(|ui| {
        for (i, gpu) in metrics.iter().enumerate() {
            if i > 0 {
                ui.add_space(2.0);
                let sep_rect = ui.available_rect_before_wrap();
                let painter = ui.painter();
                painter.line_segment(
                    [
                        egui::pos2(sep_rect.left(), sep_rect.top()),
                        egui::pos2(sep_rect.right(), sep_rect.top()),
                    ],
                    egui::Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 40)),
                );
                ui.add_space(2.0);
            }

            panel_frame.show(ui, |ui| {
                    // GPU name (discrete header)
                    ui.label(
                        RichText::new(&gpu.name)
                            .color(Color32::from_rgba_unmultiplied(180, 190, 200, 160))
                            .size(9.0),
                    );

                    // Vendor (bold header) + Temperature on same line
                    if gpu.vendor.is_some() || gpu.core_temp.is_some() || gpu.memory_temp.is_some() {
                        ui.horizontal(|ui| {
                            if let Some(ref vendor) = gpu.vendor {
                                ui.label(
                                    RichText::new(vendor.as_str())
                                        .color(Color32::from_rgb(230, 240, 230))
                                        .size(11.0)
                                        .strong(),
                                );
                            }
                            if gpu.core_temp.is_some() || gpu.memory_temp.is_some() {
                                let mut parts = Vec::new();
                                if let Some(t) = gpu.core_temp {
                                    parts.push(format!("Core:{:.0}C", t));
                                }
                                if let Some(t) = gpu.memory_temp {
                                    parts.push(format!("Mem:{:.0}C", t));
                                }
                                ui.label(
                                    RichText::new(parts.join("  "))
                                        .color(temp_color(gpu.core_temp.or(gpu.memory_temp).unwrap_or(50.0)))
                                        .size(10.0)
                                        .monospace(),
                                );
                            }
                        });
                    }

                    // Utilization line
                    if gpu.core_utilization.is_some() || gpu.memory_utilization.is_some() {
                        let mut parts = Vec::new();
                        if let Some(u) = gpu.core_utilization {
                            parts.push(format!("Util:{:.0}%", u));
                        }
                        if let Some(u) = gpu.memory_utilization {
                            parts.push(format!("MU:{:.0}%", u));
                        }
                        ui.label(
                            RichText::new(parts.join("  "))
                                .color(Color32::from_rgb(200, 210, 200))
                                .size(10.0)
                                .monospace(),
                        );
                    }

                    // VRAM line
                    if gpu.vram_used_mb.is_some() || gpu.vram_total_mb.is_some() {
                        let used = gpu.vram_used_mb.unwrap_or(0.0);
                        let total = gpu.vram_total_mb.unwrap_or(0.0);
                        let label = if total > 0.0 {
                            format!("VRAM: {:.1}/{:.1} GB", used / 1024.0, total / 1024.0)
                        } else {
                            format!("VRAM: {:.1} GB", used / 1024.0)
                        };
                        ui.label(
                            RichText::new(label)
                                .color(Color32::from_rgb(200, 210, 200))
                                .size(10.0)
                                .monospace(),
                        );
                    }

                    // Clock speed line
                    if gpu.core_clock_mhz.is_some() || gpu.memory_clock_mhz.is_some() {
                        let mut parts = Vec::new();
                        if let Some(c) = gpu.core_clock_mhz {
                            parts.push(format!("Clk:{:.0}MHz", c));
                        }
                        if let Some(c) = gpu.memory_clock_mhz {
                            parts.push(format!("MemClk:{:.0}MHz", c));
                        }
                        ui.label(
                            RichText::new(parts.join("  "))
                                .color(Color32::from_rgb(180, 190, 180))
                                .size(10.0)
                                .monospace(),
                        );
                    }
                });
            }
        });
}

fn temp_color(temp: f32) -> Color32 {
    if temp < 50.0 {
        Color32::from_rgb(100, 220, 100)
    } else if temp < 70.0 {
        Color32::from_rgb(230, 200, 80)
    } else if temp < 85.0 {
        Color32::from_rgb(240, 140, 60)
    } else {
        Color32::from_rgb(240, 70, 70)
    }
}
