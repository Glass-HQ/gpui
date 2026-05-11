//! NSString helpers shared by hosted Apple backends.

use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};

/// Creates an autoreleased NSString from a Rust string.
///
/// On macOS this uses Cocoa's NSString convenience. On iOS it falls back to the
/// Objective-C runtime directly.
#[allow(clippy::disallowed_methods)]
pub unsafe fn ns_string(string: &str) -> id {
    #[cfg(target_os = "macos")]
    unsafe {
        NSString::alloc(nil).init_str(string).autorelease()
    }

    #[cfg(target_os = "ios")]
    unsafe {
        use objc::{class, msg_send, runtime::Object};
        let cls = class!(NSString);
        let ns: *mut Object = msg_send![cls, alloc];
        let ns: *mut Object = msg_send![ns,
            initWithBytes: string.as_ptr()
            length: string.len()
            encoding: 4u64
        ];
        msg_send![ns, autorelease]
    }
}
