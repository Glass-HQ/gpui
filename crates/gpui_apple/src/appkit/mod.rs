use cocoa::{
    base::{id, nil},
    foundation::{NSAutoreleasePool, NSString},
};

pub mod native_controls;
mod platform_native_controls;

pub use platform_native_controls::MacNativeControls;

#[allow(clippy::disallowed_methods)]
pub(crate) unsafe fn ns_string(string: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(string).autorelease() }
}
