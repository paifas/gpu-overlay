mod gpu;
mod overlay;
mod ui;

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glutin::context::NotCurrentGlContext;
use glutin::display::GetGlDisplay;
use glutin::prelude::{GlDisplay, GlSurface};
use glutin::surface::SurfaceAttributesBuilder;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::raw_window_handle::HasWindowHandle;

struct App {
    gl_window: Option<GlutinWindowContext>,
    gl: Option<Arc<glow::Context>>,
    egui_glow: Option<egui_glow::EguiGlow>,
    metrics: Arc<Mutex<Vec<gpu::GpuMetrics>>>,
    last_poll: Instant,
    sized: bool,
}

struct GlutinWindowContext {
    window: winit::window::Window,
    gl_context: glutin::context::PossiblyCurrentContext,
    gl_display: glutin::display::Display,
    gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlutinWindowContext {
    unsafe fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        let window_attrs = winit::window::WindowAttributes::default()
            .with_title("gpu-overlay")
            .with_inner_size(winit::dpi::LogicalSize::new(300u32, 200u32))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_active(false)
            .with_visible(false);

        let config_template = glutin::config::ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);

        let (mut window, gl_config) = glutin_winit::DisplayBuilder::new()
            .with_window_attributes(Some(window_attrs.clone()))
            .build(
                event_loop,
                config_template,
                |mut iter| iter.next().expect("no suitable GL config found"),
            )
            .expect("failed to build GL window");

        let gl_display = gl_config.display();

        let raw_handle = window.as_ref().map(|w| {
            w.window_handle()
                .expect("failed to get window handle")
                .as_raw()
        });

        let context_attrs = glutin::context::ContextAttributesBuilder::new()
            .build(raw_handle);
        let fallback_attrs = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(raw_handle);

        let not_current = unsafe {
            gl_display
                .create_context(&gl_config, &context_attrs)
                .unwrap_or_else(|_| {
                    gl_display
                        .create_context(&gl_config, &fallback_attrs)
                        .expect("failed to create GL context")
                })
        };

        let window = window.take().unwrap_or_else(|| {
            glutin_winit::finalize_window(event_loop, window_attrs, &gl_config)
                .expect("failed to finalize window")
        });

        // Position at top-right corner
        if let Some(monitor) = window.primary_monitor().or_else(|| window.available_monitors().into_iter().next()) {
            let screen_size = monitor.size();
            let scale = monitor.scale_factor();
            let win_width = 300.0 * scale;
            let x = screen_size.width as f64 - win_width;
            let _ = window.set_outer_position(winit::dpi::PhysicalPosition::new(
                x as i32,
                0,
            ));
        }

        // macOS overlay setup
        #[cfg(target_os = "macos")]
        {
            let handle = window.window_handle().expect("failed to get window handle");
            overlay::macos::setup_overlay(&handle.as_raw());
        }

        // Linux X11 overlay setup
        #[cfg(target_os = "linux")]
        {
            let handle = window.window_handle().expect("failed to get window handle");
            overlay::linux::setup_overlay(&handle.as_raw());
        }

        let (w, h): (u32, u32) = window.inner_size().into();
        let surface_attrs = SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
            .build(
                window.window_handle().expect("handle").as_raw(),
                NonZeroU32::new(w).unwrap_or(NonZeroU32::MIN),
                NonZeroU32::new(h).unwrap_or(NonZeroU32::MIN),
            );

        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &surface_attrs)
                .expect("failed to create surface")
        };

        let gl_context = not_current.make_current(&gl_surface).expect("make_current");

        gl_surface
            .set_swap_interval(
                &gl_context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::MIN),
            )
            .ok();

        window.set_visible(true);

        Self {
            window,
            gl_context,
            gl_display,
            gl_surface,
        }
    }

    fn window(&self) -> &winit::window::Window {
        &self.window
    }

    fn resize(&self, size: winit::dpi::PhysicalSize<u32>) {
        self.gl_surface.resize(
            &self.gl_context,
            size.width.try_into().unwrap(),
            size.height.try_into().unwrap(),
        );
    }

    fn swap_buffers(&self) {
        self.gl_surface.swap_buffers(&self.gl_context).ok();
    }

    fn get_proc_address(&self, addr: &std::ffi::CStr) -> *const std::ffi::c_void {
        self.gl_display.get_proc_address(addr)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.gl_window.is_some() {
            return;
        }

        let gl_window = unsafe { GlutinWindowContext::new(event_loop) };
        let gl = unsafe {
            Arc::new(glow::Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s).expect("cstring");
                gl_window.get_proc_address(&s)
            }))
        };

        let egui_glow = egui_glow::EguiGlow::new(event_loop, gl.clone(), None, None, true);

        self.gl_window = Some(gl_window);
        self.gl = Some(gl);
        self.egui_glow = Some(egui_glow);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
            event_loop.exit();
            return;
        }

        if let WindowEvent::Resized(size) = &event {
            if let Some(gl_window) = &self.gl_window {
                gl_window.resize(*size);
            }
        }

        let gl_window = match self.gl_window.as_mut() {
            Some(w) => w,
            None => return,
        };
        let egui_glow: &mut egui_glow::EguiGlow = match self.egui_glow.as_mut() {
            Some(e) => e,
            None => return,
        };

        if matches!(event, WindowEvent::RedrawRequested) {
            let metrics = self.metrics.lock().unwrap().clone();

            // Resize window to fit content on first data
            if !self.sized && !metrics.is_empty() {
                self.sized = true;
                let scale = gl_window.window().scale_factor();
                let width = 280.0;
                // Estimate: ~13px per metric line, ~5 lines per GPU, padding, separators
                let gpu_count = metrics.len() as f32;
                let lines_per_gpu: Vec<f32> = metrics.iter().map(|m| {
                    let mut lines = 1.0; // name
                    if m.core_temp.is_some() || m.memory_temp.is_some() { lines += 1.0; }
                    if m.core_utilization.is_some() || m.memory_utilization.is_some() { lines += 1.0; }
                    if m.vram_used_mb.is_some() || m.vram_total_mb.is_some() { lines += 1.0; }
                    if m.core_clock_mhz.is_some() || m.memory_clock_mhz.is_some() { lines += 1.0; }
                    lines
                }).collect();
                let total_lines: f32 = lines_per_gpu.iter().sum();
                let height = 16.0 + total_lines * 15.0 + (gpu_count - 1.0).max(0.0) * 12.0;
                let size = winit::dpi::LogicalSize::new(width, height);
                let _ = gl_window.window().request_inner_size(size);

                // Reposition top-right
                if let Some(monitor) = gl_window.window().primary_monitor() {
                    let screen = monitor.size();
                    let phys: winit::dpi::PhysicalSize<f64> = size.to_physical(scale);
                    let x = screen.width as i32 - phys.width as i32;
                    let _ = gl_window.window().set_outer_position(
                        winit::dpi::PhysicalPosition::new(x, 0)
                    );
                }
            }

            // First: clear to transparent
            unsafe {
                use glow::HasContext as _;
                let gl = self.gl.as_ref().unwrap();
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }

            // Then: run egui layout + paint
            egui_glow.run(gl_window.window(), |egui_ctx| {
                ui::draw_panel(egui_ctx, &metrics);
            });
            egui_glow.paint(gl_window.window());

            gl_window.swap_buffers();
            return;
        }

        let event_response = egui_glow.on_window_event(gl_window.window(), &event);
        if event_response.repaint {
            gl_window.window().request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.last_poll = Instant::now();
            if let Some(gl_window) = &self.gl_window {
                gl_window.window().request_redraw();
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");

    let metrics = Arc::new(Mutex::new(Vec::new()));
    let metrics_clone = metrics.clone();

    // Background polling thread
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    std::thread::Builder::new()
        .name("gpu-poll".into())
        .spawn(move || {
            let mut monitors = gpu::detect_monitors();
            while running_clone.load(Ordering::SeqCst) {
                let mut all_metrics = Vec::new();
                for mon in &mut monitors {
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| mon.metrics())) {
                        Ok(m) => all_metrics.extend(m),
                        Err(e) => {
                            if let Some(s) = e.downcast_ref::<&str>() {
                                eprintln!("gpu-overlay: monitor panicked: {}", s);
                            }
                        }
                    }
                }
                if let Ok(mut guard) = metrics_clone.lock() {
                    *guard = all_metrics;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        })
        .expect("failed to spawn gpu-poll thread");

    let mut app = App {
        gl_window: None,
        gl: None,
        egui_glow: None,
        metrics,
        last_poll: Instant::now(),
        sized: false,
    };

    event_loop.run_app(&mut app).unwrap();
    running.store(false, Ordering::SeqCst);
}

