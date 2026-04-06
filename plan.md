# GPU Overlay Rebuild Plan

## Context

The current project is a Rust GPU temperature monitoring overlay (`gpu-overlay`). It uses raw OpenGL for rendering, hand-coded bitmap fonts (~286 lines of pixel-by-pixel character definitions), and the `cocoa`/`objc` crates (both deprecated) for macOS window overlay properties.

**Why rebuild:**
- 28 deprecation warnings from deprecated `cocoa`/`objc` crates
- Linux support is incomplete (overlay and hotkey modules are stubs)
- Custom bitmap font renderer is massive, incomplete (only 0-9, A-Z, a-z, and a few symbols), and unmaintainable
- Raw OpenGL text rendering is overkill when modern UI toolkits handle this trivially
- Window position is hardcoded to `(1580, 0)` — only works on 1920-wide displays

**What's worth keeping from the current code:**
- The `GpuMonitor` trait abstraction — clean, vendor-agnostic interface
- Apple Silicon `ioreg` parsing for temperature and utilization
- NVIDIA `nvidia-smi` CSV parsing
- AMD sysfs/hwmon reading
- The overall concept: transparent, always-on-top, click-through overlay

---

## Requirements

### GPU Vendors
| Vendor | Platform | Data Source | Priority |
|---|---|---|---|
| Apple Silicon (M-series) | macOS | `ioreg` (IORegistry) | High |
| NVIDIA | macOS + Linux | `nvidia-smi` subprocess | High |
| AMD | Linux | sysfs/hwmon (`/sys/class/drm/`) | Medium (Linux later) |
| Intel (Arc/integrated) | Linux | sysfs i915 (`/sys/class/drm/`) | Medium (Linux later) |

### Metrics to Display
| Metric | Unit | Apple | NVIDIA | AMD | Intel |
|---|---|---|---|---|---|
| Core temperature | C | ioreg `temp` + `GPU` | nvidia-smi `temperature.gpu` | hwmon `temp1_input` | hwmon `temp1_input` |
| Memory temperature | C | N/A (unified memory) | nvidia-smi `temperature.memory` | hwmon `temp2_input` | N/A |
| Core utilization | % | ioreg delta `AccumulatedBusyTime`/`TotalRunningTime` | nvidia-smi `utilization.gpu` | sysfs `gpu_busy_percent` | approx from `gt_act_freq_mhz`/`gt_max_freq_mhz` |
| Memory utilization | % | N/A | nvidia-smi `utilization.memory` | sysfs `mem_busy_percent` | N/A |
| VRAM used | MiB | ioreg `PerformanceStatistics` > `Used VRAM` | nvidia-smi `memory.used` | sysfs `mem_info_vram_used` | N/A (shared RAM) |
| VRAM total | MiB | ioreg `PerformanceStatistics` > `Total VRAM` | nvidia-smi `memory.total` | sysfs `mem_info_vram_total` | N/A (shared RAM) |
| Core clock | MHz | ioreg `PerformanceStatistics` (best effort) | nvidia-smi `clocks.current.sm` | hwmon `freq1_input` / 1e6 | sysfs `gt_act_freq_mhz` |
| Memory clock | MHz | N/A | nvidia-smi `clocks.current.mem` | hwmon `freq2_input` / 1e6 | N/A |

All fields use `Option<f32>` — vendors report different subsets. The overlay skips lines where a metric is `None`.

### Overlay Behavior
- **Transparent** background with dark semi-transparent panel (rgba ~0.08, 0.08, 0.12, 0.85)
- **Always on top** — above menu bar level (`NSMainMenuWindowLevel + 1` on macOS)
- **Click-through** — mouse events pass through to windows below
- **All Spaces + fullscreen** — visible on every macOS Space and in fullscreen apps
- **No shadow** — clean floating panel look
- **No decorations** — borderless window
- **Always visible** — no hotkey toggle needed (simplifies code, avoids macOS Accessibility permission prompts)
- **Fixed position** — top-right corner, calculated dynamically from monitor resolution via winit
- **No decorations, not resizable, not active** — the window should never steal focus
- **Multi-GPU** — display all detected GPUs simultaneously, each in its own block
- **Compact layout** — small monospace text, MSI Afterburner-style minimal panel

### Configuration
- **Zero-config** — no config files, no CLI flags needed. Sensible defaults only.

### Refresh Rate
- **1 second** — poll GPU metrics every 1 second via background thread

### Platform Priority
- **macOS first** — get Apple Silicon + NVIDIA working on macOS
- **Linux later** — AMD + Intel + NVIDIA on Linux in a follow-up pass

---

## Stack

### Rendering: egui + egui_glow
- **egui** — immediate-mode GUI. Perfect for a static overlay that just displays text labels every frame. No interactive widgets, no state management overhead.
- **egui_glow** — egui's OpenGL backend using the `glow` crate (safe Rust GL wrapper). Handles all font rendering, text layout, and drawing internally.
- **Built-in font rendering** via `ab_glyph` — replaces the entire 286-line hand-coded bitmap font. Proper Unicode support, antialiasing, variable-width fonts for free.
- Alternative considered: **iced** (retained-mode) — more structured but adds Elm-architecture overhead for a static overlay with no user interaction. Not worth the complexity.
- Alternative considered: **raw OpenGL + fontdue** — minimal dependencies but still requires manual text layout, vertex buffers, shaders. egui handles all of this.

### Window: winit + glutin
- **winit 0.30** — cross-platform window creation, event loop, monitor info for dynamic positioning
- **glutin 0.32** + **glutin-winit 0.5** — OpenGL context creation with alpha support (for transparent window)
- Window attributes: `with_transparent(true)`, `with_decorations(false)`, `with_resizable(false)`, `with_active(false)`

### macOS Overlay: objc2-app-kit
- **objc2-app-kit** — actively maintained replacement for deprecated `cocoa`/`objc`
- After winit creates the window, use `raw-window-handle` to get the `NSView` pointer, then set overlay properties via `objc2-app-kit`:
  - `setIgnoresMouseEvents(true)` — click-through
  - `setLevel(NSMainMenuWindowLevel + 1)` — always on top, above menu bar
  - `setCollectionBehavior(CanJoinAllSpaces | FullScreenAuxiliary)` — visible everywhere
  - `setHasShadow(false)` — no drop shadow

### Future Linux: x11rb
- **x11rb** — X11 client library for setting overlay window properties on Linux
- Will set `_NET_WM_WINDOW_TYPE_DOCK`, `_NET_WM_STATE_ABOVE`, `_NET_WM_WINDOW_OPACITY`
- Wayland support via layer-shell protocol (complex, deferred)

---

## Architecture

```
src/
  main.rs              -- Entry point, winit event loop, app state, threading
  gpu/
    mod.rs             -- GpuMetrics struct, GpuMonitor trait, detect_monitors()
    apple.rs           -- Apple Silicon GPU (ioreg-based)
    nvidia.rs          -- NVIDIA GPU (nvidia-smi-based)
    amd.rs             -- AMD GPU (sysfs/hwmon, Linux only, cfg-gated)
    intel.rs           -- Intel GPU (sysfs/i915, Linux only, cfg-gated)
  overlay/
    mod.rs             -- Platform dispatch (cfg target_os)
    macos.rs           -- objc2-app-kit overlay window setup
    linux.rs           -- x11rb overlay setup (placeholder for now)
  ui.rs                -- egui panel drawing logic
```

### Data Flow

```
Background Thread                    Main Thread (winit event loop)
  loop {                               |
    poll all monitors                  |
    update Arc<Mutex<Vec<GpuMetrics>>> |
    sleep 1s                           |
  }                                    |
                                       about_to_wait: if 1s elapsed, request_redraw()
                                       RedrawRequested:
                                         try_lock() metrics (non-blocking)
                                         egui: draw dark panel + metric lines
                                         swap buffers
```

- GPU polling runs on a background thread to avoid blocking the UI (especially important for `nvidia-smi` subprocess calls which take 50-200ms)
- Data shared via `Arc<Mutex<Vec<GpuMetrics>>>`
- UI thread does `try_lock()` on each redraw — if contended, draws last frame's data (no blocking)
- Redraw timer driven by winit's `about_to_wait` callback (same pattern as current code)

---

## Key Types

### GpuMetrics
```rust
#[derive(Debug, Clone)]
pub struct GpuMetrics {
    pub name: String,
    pub core_temp: Option<f32>,        // degrees C
    pub memory_temp: Option<f32>,      // degrees C
    pub core_utilization: Option<f32>, // 0-100%
    pub memory_utilization: Option<f32>, // 0-100%
    pub vram_used_mb: Option<f32>,     // MiB
    pub vram_total_mb: Option<f32>,    // MiB
    pub core_clock_mhz: Option<f32>,   // MHz
    pub memory_clock_mhz: Option<f32>, // MHz
}
```

### GpuMonitor trait
```rust
pub trait GpuMonitor: Send {
    fn metrics(&mut self) -> Vec<GpuMetrics>;
}
```

Each vendor implementation:
- Has an `is_available() -> bool` static method for detection
- Has a `new() -> Self` constructor
- Implements `GpuMonitor::metrics()` to poll its data source
- Returns a `Vec<GpuMetrics>` (one entry per GPU — e.g., multi-GPU NVIDIA setups)

### detect_monitors()
```rust
pub fn detect_monitors() -> Vec<Box<dyn GpuMonitor>> {
    // Check each vendor's is_available(), construct monitors for detected GPUs
}
```

---

## Dependencies (Cargo.toml)

```toml
[package]
name = "gpu-overlay"
version = "0.1.0"
edition = "2021"

[dependencies]
egui = "0.31"
egui-winit = "0.31"
egui_glow = "0.31"
glow = "0.16"
winit = "0.30"
glutin = "0.32"
glutin-winit = "0.5"
raw-window-handle = "0.6"

[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = ["NSWindow", "NSView", "NSColor"] }
objc2-foundation = { version = "0.3", features = ["NSGeometry"] }

[target.'cfg(target_os = "linux")'.dependencies]
x11rb = "0.13"
```

Note: versions should be verified against crates.io before implementation. egui 0.31, winit 0.30, glutin 0.32 are the latest as of early 2026.

---

## Implementation Order

### Phase 1: Data Layer (testable via stdout)
Build and test all GPU monitors before touching any UI code.

1. **`src/gpu/mod.rs`** — Define `GpuMetrics` struct, `GpuMonitor` trait, `detect_monitors()` function
2. **`src/gpu/apple.rs`** — Port from current `monitor/apple.rs`. Expand with:
   - VRAM from ioreg `PerformanceStatistics` dictionary (keys: `Used VRAM`, `Total VRAM` — need to verify exact key names by running `ioreg -r -c AGXAccelerator`)
   - Clock speed from ioreg `PerformanceStatistics` (best effort — may not be available on all macOS versions)
   - Keep existing temperature parsing (ioreg `temp` + `GPU`)
   - Keep existing utilization (delta of `AccumulatedBusyTime` / `TotalRunningTime`)
3. **`src/gpu/nvidia.rs`** — Port from current `monitor/nvidia.rs`. Expand nvidia-smi query to:
   - `--query-gpu=name,temperature.gpu,temperature.memory,utilization.gpu,utilization.memory,memory.used,memory.total,clocks.current.sm,clocks.current.mem`
   - Parse all 9 CSV fields
4. **`src/gpu/amd.rs`** — Port from current `monitor/linux.rs`. Expand with:
   - VRAM: `mem_info_vram_used`, `mem_info_vram_total` (in bytes, convert to MiB)
   - Clock: `freq1_input` (core, in Hz, convert to MHz), `freq2_input` (memory)
   - Keep existing: `temp1_input`, `temp2_input`, `gpu_busy_percent`, `mem_busy_percent`
5. **`src/gpu/intel.rs`** — New implementation:
   - Detect by checking `/sys/class/drm/card*/device/driver` symlink points to `i915`
   - Temperature: `hwmon/temp1_input` (millidegrees C, divide by 1000)
   - Core clock: `gt_act_freq_mhz` (actual frequency in MHz)
   - Utilization: approximated from `gt_act_freq_mhz / gt_max_freq_mhz * 100` (rough but functional)
6. **Test** — Write a temporary main that loops every second, calls `detect_monitors()`, prints `GpuMetrics` to stdout

### Phase 2: Overlay Window + Rendering
Build the visual overlay on macOS.

7. **`src/main.rs`** — Application skeleton:
   - winit event loop with `ApplicationHandler`
   - Create transparent window via winit attributes
   - Create glutin GL context with alpha support
   - Create `glow::Context` from glutin display
   - Position window at top-right using winit monitor API (`primary_monitor()` -> `size()` -> calculate position)
   - Integrate egui: create `egui::Context`, `egui_glow::Painter`, wire up `egui_winit`
8. **`src/overlay/macos.rs`** — macOS overlay window setup using `objc2-app-kit`:
   - Get `NSView` from `raw-window-handle`
   - Get parent `NSWindow` from the view
   - Set: `ignoresMouseEvents = true`, `level = NSMainMenuWindowLevel + 1`, `collectionBehavior = CanJoinAllSpaces | FullScreenAuxiliary`, `hasShadow = false`
9. **`src/overlay/linux.rs`** — Placeholder stub (returns immediately)
10. **`src/ui.rs`** — egui panel rendering:
    - Dark semi-transparent `Frame` (rgba ~0.08, 0.08, 0.12, 0.85)
    - Small monospace font (egui's built-in `TextStyle::Monospace` at ~10-12px)
    - For each GPU: name header, then metric lines (skip `None` fields)
    - Separator line between GPUs

### Phase 3: Threading + Polish
Make it production-quality.

11. **Background polling thread** — Spawn a thread that:
    - Calls all monitors every 1 second
    - Updates `Arc<Mutex<Vec<GpuMetrics>>>` with latest data
    - Uses `std::thread::sleep(Duration::from_secs(1))` between polls
12. **Dynamic window sizing** — After first data collection, calculate required height based on GPU count and available metrics, resize window accordingly
13. **Error handling** — If a monitor fails, skip it for that cycle, print warning to stderr, don't crash. Handle `nvidia-smi` installed but driver not loaded.
14. **Cleanup** — Remove any temporary test code, verify no warnings on `cargo build`

---

## Files to Delete
Everything in current `src/` — full rebuild:
- `src/main.rs`, `src/monitor/`, `src/overlay/`, `src/renderer/`, `src/hotkey/`

Also delete `Cargo.lock` to get fresh dependency resolution.

---

## Known Challenges

1. **Apple Silicon VRAM and clock speeds** — The `ioreg` keys for `PerformanceStatistics` are not well-documented and may vary by macOS version. The `AGXAccelerator` class exposes this data but the exact key names need verification by running `ioreg -r -c AGXAccelerator` on the target machine. Some keys may not exist on all macOS versions — the implementation must be defensive and return `None` gracefully.

2. **macOS OpenGL deprecation** — Apple deprecated OpenGL in macOS 10.14 Mojave (2018). It still works on all current macOS versions including Sequoia. If Apple ever removes it, the migration path is to swap `egui_glow` for `egui_wgpu` (which uses Metal on macOS). This is a backend swap, not a rewrite.

3. **nvidia-smi latency** — Subprocess invocation takes 50-200ms. The background thread prevents this from blocking the UI. Future optimization: use `nvml-wrapper` crate (NVML C library bindings) to avoid subprocess overhead.

4. **Intel GPU temperature on macOS (Intel Macs)** — Older Intel Macs with Intel integrated GPUs may expose GPU temperature through IORegistry, but the exact paths vary by model. Lower priority since Intel Macs are being phased out.

5. **Window resizing with GPU hotplug** — If an eGPU is connected/disconnected, the overlay should adapt. Practical approach: size the window for maximum expected content, with unused space remaining transparent.

---

## Visual Layout Example

```
Apple M2 Pro
Core: 45C  Util: 23%
VRAM: 4.2/16.0 GB
Clk: 1197 MHz
─────────────────────
NVIDIA RTX 4090
Core: 72C  Mem: 68C
Util: 87%   Mem: 45%
VRAM: 18.4/24.0 GB
Clk: 2580/10501 MHz
```

Lines where a metric is `None` are omitted entirely, keeping the overlay compact.

---

## Verification

1. `cargo build` — no errors, minimal warnings
2. Run on macOS — transparent overlay appears top-right with Apple GPU stats
3. Verify overlay is click-through (clicks go to windows below)
4. Verify overlay is visible on all Spaces
5. Verify overlay is visible in fullscreen apps
6. If NVIDIA GPU present: verify dual GPU display
7. Verify metrics refresh every ~1 second
8. Verify no focus stealing — overlay window should never become the active window
