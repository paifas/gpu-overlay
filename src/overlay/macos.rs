use objc2::runtime::AnyObject;
use objc2::msg_send;
use objc2_app_kit::{
    NSColor, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use raw_window_handle::RawWindowHandle;

pub fn setup_overlay(raw: &RawWindowHandle) {
    let ns_view = match raw {
        RawWindowHandle::AppKit(handle) => handle.ns_view.as_ptr(),
        _ => return,
    };
    if ns_view.is_null() {
        return;
    }

    unsafe {
        let view = ns_view as *const AnyObject;
        let window: Option<&NSWindow> = msg_send![&*view, window];
        let Some(window) = window else {
            return;
        };

        // Click-through
        let _: () = msg_send![window, setIgnoresMouseEvents: true];

        // Borderless
        window.setStyleMask(NSWindowStyleMask::Borderless);

        // Transparent background
        window.setOpaque(false);
        let bg_color = NSColor::clearColor();
        window.setBackgroundColor(Some(&bg_color));

        // Always on top, above menu bar (NSMainMenuWindowLevel = 24)
        window.setLevel(25);

        // Visible on all Spaces and in fullscreen
        let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary;
        window.setCollectionBehavior(behavior);

        // No shadow
        window.setHasShadow(false);
    }
}
