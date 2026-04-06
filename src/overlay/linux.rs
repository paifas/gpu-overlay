use raw_window_handle::RawWindowHandle;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as XprotoExt;
use x11rb::wrapper::ConnectionExt as WrapperExt;

pub fn setup_overlay(raw: &RawWindowHandle) {
    let window_id = match raw {
        RawWindowHandle::Xlib(h) => h.window as u32,
        RawWindowHandle::Xcb(h) => h.window.get(),
        _ => return,
    };

    if let Err(e) = setup_x11_overlay(window_id) {
        eprintln!("gpu-overlay: failed to setup X11 overlay: {}", e);
    }
}

fn setup_x11_overlay(window_id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, _) = x11rb::connect(None)?;

    // Intern the EWMH atoms we need
    let net_wm_window_type = conn.intern_atom(false, b"_NET_WM_WINDOW_TYPE")?.reply()?.atom;
    let net_wm_window_type_dock = conn.intern_atom(false, b"_NET_WM_WINDOW_TYPE_DOCK")?.reply()?.atom;
    let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let net_wm_state_above = conn.intern_atom(false, b"_NET_WM_STATE_ABOVE")?.reply()?.atom;
    let net_wm_state_sticky = conn.intern_atom(false, b"_NET_WM_STATE_STICKY")?.reply()?.atom;
    let net_wm_window_opacity = conn.intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?.reply()?.atom;

    // Set window type to DOCK (taskbar-like, stays on top)
    conn.change_property32(
        x11rb::protocol::xproto::PropMode::REPLACE,
        window_id,
        net_wm_window_type,
        x11rb::protocol::xproto::AtomEnum::ATOM,
        &[net_wm_window_type_dock],
    )?;

    // Set state: ABOVE + STICKY (always on top, all desktops)
    conn.change_property32(
        x11rb::protocol::xproto::PropMode::REPLACE,
        window_id,
        net_wm_state,
        x11rb::protocol::xproto::AtomEnum::ATOM,
        &[net_wm_state_above, net_wm_state_sticky],
    )?;

    // Set opacity to fully opaque (0xffffffff = 100%)
    // The window transparency is handled by OpenGL, not by the compositor
    conn.change_property32(
        x11rb::protocol::xproto::PropMode::REPLACE,
        window_id,
        net_wm_window_opacity,
        x11rb::protocol::xproto::AtomEnum::CARDINAL,
        &[0xffffffff],
    )?;

    // Skip taskbar and pager
    let net_wm_state_skip_taskbar = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR")?.reply()?.atom;
    let net_wm_state_skip_pager = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER")?.reply()?.atom;
    conn.change_property32(
        x11rb::protocol::xproto::PropMode::REPLACE,
        window_id,
        net_wm_state,
        x11rb::protocol::xproto::AtomEnum::ATOM,
        &[net_wm_state_above, net_wm_state_sticky, net_wm_state_skip_taskbar, net_wm_state_skip_pager],
    )?;

    conn.flush()?;
    Ok(())
}
