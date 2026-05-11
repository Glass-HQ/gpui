//! Objective-C memory-management helpers.

use objc::{msg_send, runtime::Object, sel, sel_impl};

#[allow(non_camel_case_types)]
pub type id = *mut Object;

/// Sends `retain` if the object is non-null.
///
/// # Safety
///
/// The caller must ensure `obj` is a valid Objective-C object.
pub unsafe fn retain(obj: id) {
    if !obj.is_null() {
        unsafe {
            let _: id = msg_send![obj, retain];
        }
    }
}

/// Sends `release` if the object is non-null.
///
/// # Safety
///
/// The caller must ensure `obj` is a valid Objective-C object.
pub unsafe fn release(obj: id) {
    if !obj.is_null() {
        unsafe {
            let _: () = msg_send![obj, release];
        }
    }
}
