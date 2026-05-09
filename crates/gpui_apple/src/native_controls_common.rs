use std::{ffi::c_void, ptr};

use gpui::native_controls::{ComboBoxCallbacks, NativeControlState, TextFieldCallbacks};

macro_rules! define_cleanup_with_target {
    ($name:ident, $native_controls:ident, $id_ty:ty, $remove_fn:ident, $release_target:ident, $release_view:ident) => {
        unsafe fn $name(view: *mut c_void, target: *mut c_void) {
            unsafe {
                if !target.is_null() {
                    $native_controls::$release_target(target);
                }
                if !view.is_null() {
                    $native_controls::$remove_fn(view as $id_ty);
                    $native_controls::$release_view(view as $id_ty);
                }
            }
        }
    };
}

macro_rules! define_cleanup_view_only {
    ($name:ident, $native_controls:ident, $id_ty:ty, $remove_fn:ident, $release_view:ident) => {
        unsafe fn $name(view: *mut c_void, _target: *mut c_void) {
            unsafe {
                if !view.is_null() {
                    $native_controls::$remove_fn(view as $id_ty);
                    $native_controls::$release_view(view as $id_ty);
                }
            }
        }
    };
}

pub(crate) use define_cleanup_view_only;
pub(crate) use define_cleanup_with_target;

pub(crate) fn has_text_field_callbacks(callbacks: &TextFieldCallbacks) -> bool {
    callbacks.on_change.is_some()
        || callbacks.on_begin_editing.is_some()
        || callbacks.on_end_editing.is_some()
        || callbacks.on_submit.is_some()
        || callbacks.on_move_up.is_some()
        || callbacks.on_move_down.is_some()
        || callbacks.on_cancel.is_some()
}

pub(crate) fn has_combo_box_callbacks(callbacks: &ComboBoxCallbacks) -> bool {
    callbacks.on_select.is_some() || callbacks.on_change.is_some() || callbacks.on_submit.is_some()
}

#[cfg(target_os = "ios")]
pub(crate) fn normalized_progress(value: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        0.0
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }
}

pub(crate) fn optional_target<T>(
    callback: Option<T>,
    create_target: impl FnOnce(T) -> *mut c_void,
) -> *mut c_void {
    callback.map(create_target).unwrap_or(ptr::null_mut())
}

pub(crate) unsafe fn replace_target_if_present<T>(
    state: &mut NativeControlState,
    callback: Option<T>,
    release_target: unsafe fn(*mut c_void),
    create_target: impl FnOnce(T) -> *mut c_void,
) {
    if let Some(callback) = callback {
        unsafe {
            release_target(state.target());
        }
        state.set_target(create_target(callback));
    }
}

#[cfg(target_os = "ios")]
pub(crate) unsafe fn reset_target<T>(
    state: &mut NativeControlState,
    callback: Option<T>,
    release_target: unsafe fn(*mut c_void),
    create_target: impl FnOnce(T) -> *mut c_void,
) {
    if !state.target().is_null() {
        unsafe {
            release_target(state.target());
        }
        state.set_target(ptr::null_mut());
    }
    if let Some(callback) = callback {
        state.set_target(create_target(callback));
    }
}
