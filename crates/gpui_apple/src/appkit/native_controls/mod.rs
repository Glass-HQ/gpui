mod alert;
mod button;
mod checkbox;
mod collection;
mod combo_box;
mod glass_effect_view;
mod image_view;
mod menu;
mod outline;
mod panel;
mod popover;
mod popup;
mod progress;
mod search_field;
mod segmented;
mod sidebar;
mod slider;
mod stack_view;
mod stepper;
mod switch;
mod tab_view;
mod table;
mod text_field;
mod tracking_area;
mod visual_effect_view;

pub use alert::*;
pub use button::*;
pub use checkbox::*;
pub use collection::*;
pub use combo_box::*;
pub use glass_effect_view::*;
pub use image_view::*;
pub use menu::*;
pub use outline::*;
pub use panel::*;
pub use popover::*;
pub use popup::*;
pub use progress::*;
pub use search_field::*;
pub use segmented::*;
pub use sidebar::*;
pub use slider::*;
pub use stack_view::*;
pub use stepper::*;
pub use switch::*;
pub use tab_view::*;
pub use table::*;
pub use text_field::*;
pub use tracking_area::*;
pub use visual_effect_view::*;

use cocoa::{
    base::id,
    foundation::{NSPoint, NSRect, NSSize},
};
use gpui::{Bounds, Pixels};
use objc::{msg_send, sel, sel_impl};

pub(super) const CALLBACK_IVAR: &str = "callbackPtr";

// =============================================================================
// Shared helpers
// =============================================================================

pub fn attach_and_position(
    parent: *mut std::ffi::c_void,
    view: id,
    bounds: Bounds<Pixels>,
    _scale: f32,
) {
    unsafe {
        let parent_view = parent as id;
        let x = f32::from(bounds.origin.x) as f64;
        let y = f32::from(bounds.origin.y) as f64;
        let w = f32::from(bounds.size.width) as f64;
        let h = f32::from(bounds.size.height) as f64;

        let is_flipped: bool = msg_send![parent_view, isFlipped];
        let final_y = if is_flipped {
            y
        } else {
            let parent_frame: NSRect = msg_send![parent_view, frame];
            parent_frame.size.height - y - h
        };

        let frame = NSRect::new(NSPoint::new(x, final_y), NSSize::new(w, h));
        let _: () = msg_send![view, setFrame: frame];

        let superview: id = msg_send![view, superview];
        if superview != parent_view {
            let _: () = msg_send![parent_view, addSubview: view];
        }
    }
}

pub fn remove_from_parent(view: id) {
    unsafe {
        let _: () = msg_send![view, removeFromSuperview];
    }
}

pub unsafe fn set_native_control_enabled(control: id, enabled: bool) {
    unsafe {
        let _: () = msg_send![control, setEnabled: enabled as i8];
    }
}

pub unsafe fn set_native_view_tooltip(view: id, tooltip: &str) {
    unsafe {
        use super::ns_string;
        let _: () = msg_send![view, setToolTip: ns_string(tooltip)];
    }
}
