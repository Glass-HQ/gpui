//! Objective-C ivar access helpers.

use objc::{Encode, runtime::Object};

/// Reads an ivar from an Objective-C object.
///
/// # Safety
///
/// The caller must ensure `obj` is valid and the ivar exists with type `T`.
pub unsafe fn get_ivar<T: Copy + Encode>(obj: *mut Object, name: &str) -> T {
    unsafe { *(*obj).get_ivar(name) }
}

/// Writes an ivar on an Objective-C object.
///
/// # Safety
///
/// The caller must ensure `obj` is valid and the ivar exists with type `T`.
pub unsafe fn set_ivar<T: Encode>(obj: *mut Object, name: &str, value: T) {
    unsafe {
        (*obj).set_ivar(name, value);
    }
}
