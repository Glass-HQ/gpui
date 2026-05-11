//! Shared attachment helpers for Objective-C hosted views.

use objc::{msg_send, runtime::Object, sel, sel_impl};

#[allow(non_camel_case_types)]
pub type id = *mut Object;

/// Attaches a child view to a parent view.
///
/// # Safety
///
/// Both arguments must be valid Objective-C view instances for the current
/// backend.
pub unsafe fn attach_subview(parent: id, child: id) {
    unsafe {
        let _: () = msg_send![parent, addSubview: child];
    }
}

/// Detaches a child view from its current parent.
///
/// # Safety
///
/// `child` must be a valid Objective-C view instance.
pub unsafe fn detach_from_parent(child: id) {
    unsafe {
        let _: () = msg_send![child, removeFromSuperview];
    }
}
