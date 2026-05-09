//! Minimal native macOS sidebar.
//!
//! Demonstrates the exact API surface needed to produce:
//! - A sidebar that extends into the titlebar (traffic lights in the sidebar zone)
//! - The toggle button at the trailing (right) edge of the sidebar header
//!
//! Both behaviors come from `native_sidebar`'s two defaults:
//!
//!   manage_window_chrome: true  — installs NSSplitViewController as
//!     window.contentViewController, sets NSFullSizeContentViewWindowMask,
//!     hides the window title, and makes the titlebar transparent.
//!
//!   manage_toolbar: true  — creates an NSToolbar whose items are:
//!     [NSFlexibleSpaceItem, NSToolbarToggleSidebarItem, NSToolbarSidebarTrackingSeparator]
//!     The FlexibleSpace fills the sidebar zone, pushing the toggle to its
//!     trailing edge. The TrackingSeparator marks the sidebar/content boundary.
//!
//! Neither flag needs to be set explicitly — they are both true by default.
//!
//! Collapse state is intentionally not tracked in GPUI. AppKit owns it via the
//! native toggle button (toggleSidebar: through the responder chain). Mirroring
//! it in GPUI state and also handling ToggleSidebar causes a double-toggle:
//! AppKit collapses, then GPUI re-expands on the very next render.

use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, native_sidebar, prelude::*, px,
    size,
};

struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            // native_sidebar wraps NSSplitViewController. The id must be stable
            // across renders so GPUI can diff against the previous state and
            // avoid redundant AppKit calls during paint.
            native_sidebar("example.sidebar", &["Home", "Settings", "Help"])
                // Initial sidebar width in logical pixels. AppKit enforces the
                // min/max range when the user drags the divider.
                .sidebar_width(240.0)
                .min_sidebar_width(160.0)
                .max_sidebar_width(400.0)
                // size_full() so the element fills the GPUI content view, which
                // is the detail (right) pane of the NSSplitViewController.
                .size_full(),
        )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_| Root),
        )
        .unwrap();

        cx.activate(true);
    });
}
