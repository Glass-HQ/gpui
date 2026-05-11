#![cfg(target_os = "ios")]

mod app_bridge;
mod dispatcher;
mod display;
mod display_link;
mod drag_drop;
mod events;
mod gestures;
mod keyboard;
mod lifecycle;
mod platform;
mod text_input;
mod view;
mod window;

pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use display_link::*;
pub(crate) use drag_drop::*;
pub(crate) use events::*;
pub(crate) use gestures::*;
pub(crate) use keyboard::*;
pub(crate) use lifecycle::*;
pub(crate) use platform::*;
pub(crate) use view::*;
pub(crate) use window::*;

#[cfg(feature = "font-kit")]
mod open_type;
#[cfg(feature = "font-kit")]
mod text_system;

use anyhow::{Result, anyhow};
use block::ConcreteBlock;
use collections::HashMap;
use core_foundation::{
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::{CFDictionary, CFMutableDictionary},
    string::CFString,
};
use ctor::ctor;
use foreign_types::ForeignType as _;
use futures::channel::oneshot;
#[cfg(not(feature = "font-kit"))]
use gpui::NoopTextSystem;
use gpui::hash;
use gpui::{
    Action, AnyWindowHandle, BackgroundExecutor, Bounds, Capslock, ClipboardEntry, ClipboardItem,
    CursorStyle, DevicePixels, DispatchEventResult, DisplayId, Edges, ExternalPaths, FileDropEvent,
    ForegroundExecutor, GLOBAL_THREAD_TIMINGS, GpuSpecs, Image, ImageFormat, KeyDownEvent,
    KeyUpEvent, KeybindingKeystroke, Keymap, Keystroke, Menu, MenuItem, Modifiers,
    ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    OwnedMenu, PathPromptOptions, PinchEvent, Pixels, Platform, PlatformAtlas, PlatformDispatcher,
    PlatformDisplay, PlatformInput, PlatformInputHandler,
    PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem, PlatformWindow, Point,
    Priority, PromptButton, PromptLevel, RequestFrameOptions, RotationEvent, RunnableVariant,
    Scene, ScrollDelta, ScrollWheelEvent, Size, THREAD_TIMINGS, Task, TaskTiming, ThermalState,
    ThreadTaskTimings, TouchPhase, WindowAppearance, WindowBackgroundAppearance, WindowBounds,
    WindowControlArea, WindowParams, point, px, size,
};
use gpui_metal::{InstanceBufferPool, MetalRenderer};
use metal::{CAMetalLayer, MetalLayer};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{BOOL, Class, NO, Object, Sel, YES},
    sel, sel_impl,
};
use parking_lot::Mutex;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, UiKitWindowHandle, WindowHandle,
};
use std::{
    cell::Cell,
    ffi::c_void,
    path::{Path, PathBuf},
    ptr::{self, NonNull, addr_of},
    rc::Rc,
    sync::{Arc, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};
use text_input::IosTextInputSession;
#[cfg(feature = "font-kit")]
use text_system::IosTextSystem;

type DispatchQueue = *mut c_void;
type DispatchTime = u64;

const DISPATCH_TIME_NOW: DispatchTime = 0;
const DISPATCH_QUEUE_PRIORITY_HIGH: isize = 2;
const DISPATCH_QUEUE_PRIORITY_DEFAULT: isize = 0;
const DISPATCH_QUEUE_PRIORITY_LOW: isize = -2;

const CALLBACK_IVAR: &str = "gpui_callback";
const WINDOW_STATE_IVAR: &str = "gpui_window_state";

const UISCENE_DID_ACTIVATE: &[u8] = b"UISceneDidActivateNotification\0";
const UISCENE_WILL_DEACTIVATE: &[u8] = b"UISceneWillDeactivateNotification\0";
const UISCENE_DID_ENTER_BACKGROUND: &[u8] = b"UISceneDidEnterBackgroundNotification\0";
const UISCENE_WILL_ENTER_FOREGROUND: &[u8] = b"UISceneWillEnterForegroundNotification\0";

unsafe extern "C" {
    static _dispatch_main_q: c_void;
    static NSRunLoopCommonModes: *mut Object;
    fn dispatch_get_global_queue(identifier: isize, flags: usize) -> DispatchQueue;
    fn dispatch_async_f(
        queue: DispatchQueue,
        context: *mut c_void,
        work: Option<unsafe extern "C" fn(*mut c_void)>,
    );
    fn dispatch_after_f(
        when: DispatchTime,
        queue: DispatchQueue,
        context: *mut c_void,
        work: Option<unsafe extern "C" fn(*mut c_void)>,
    );
    fn dispatch_time(when: DispatchTime, delta: i64) -> DispatchTime;
}

// ---------------------------------------------------------------------------
// Platform types
// ---------------------------------------------------------------------------

/// Scroll momentum state for simulating iOS-like deceleration after a finger lift.
struct ScrollMomentum {
    velocity: Point<f32>,
    position: Point<Pixels>,
    last_time: Instant,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct UIEdgeInsets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

unsafe impl objc::Encode for UIEdgeInsets {
    fn encode() -> objc::Encoding {
        let encoding = format!(
            "{{UIEdgeInsets={}{}{}{}}}",
            f64::encode().as_str(),
            f64::encode().as_str(),
            f64::encode().as_str(),
            f64::encode().as_str()
        );
        unsafe { objc::Encoding::from_str(&encoding) }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGPoint {
    x: f64,
    y: f64,
}

unsafe impl objc::Encode for CGPoint {
    fn encode() -> objc::Encoding {
        let encoding = format!(
            "{{CGPoint={}{}}}",
            f64::encode().as_str(),
            f64::encode().as_str()
        );
        unsafe { objc::Encoding::from_str(&encoding) }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGSize {
    width: f64,
    height: f64,
}

unsafe impl objc::Encode for CGSize {
    fn encode() -> objc::Encoding {
        let encoding = format!(
            "{{CGSize={}{}}}",
            f64::encode().as_str(),
            f64::encode().as_str()
        );
        unsafe { objc::Encoding::from_str(&encoding) }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

unsafe impl objc::Encode for CGRect {
    fn encode() -> objc::Encoding {
        let encoding = format!(
            "{{CGRect={}{}}}",
            CGPoint::encode().as_str(),
            CGSize::encode().as_str()
        );
        unsafe { objc::Encoding::from_str(&encoding) }
    }
}

unsafe fn ns_string(value: &str) -> *mut Object {
    let cstring = std::ffi::CString::new(value).unwrap_or_default();
    msg_send![class!(NSString), stringWithUTF8String: cstring.as_ptr()]
}

/// Query UIKit for the current system appearance (Light/Dark mode).
fn detect_system_appearance() -> WindowAppearance {
    unsafe {
        let screen: *mut Object = msg_send![class!(UIScreen), mainScreen];
        let traits: *mut Object = msg_send![screen, traitCollection];
        let style: isize = msg_send![traits, userInterfaceStyle];
        // UIUserInterfaceStyle: 0 = Unspecified, 1 = Light, 2 = Dark
        match style {
            2 => WindowAppearance::Dark,
            _ => WindowAppearance::Light,
        }
    }
}

// ---------------------------------------------------------------------------
// Security framework bindings for Keychain access (identical API on iOS/macOS)
// ---------------------------------------------------------------------------

mod ios_security {
    #![allow(non_upper_case_globals)]

    use core_foundation::{
        base::{CFTypeRef, OSStatus},
        dictionary::CFDictionaryRef,
        string::CFStringRef,
    };

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        pub static kSecClass: CFStringRef;
        pub static kSecClassInternetPassword: CFStringRef;
        pub static kSecAttrServer: CFStringRef;
        pub static kSecAttrAccount: CFStringRef;
        pub static kSecValueData: CFStringRef;
        pub static kSecReturnAttributes: CFStringRef;
        pub static kSecReturnData: CFStringRef;

        pub fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
        pub fn SecItemUpdate(query: CFDictionaryRef, attributes: CFDictionaryRef) -> OSStatus;
        pub fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
        pub fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    }

    pub const errSecSuccess: OSStatus = 0;
    pub const errSecUserCanceled: OSStatus = -128;
    pub const errSecItemNotFound: OSStatus = -25300;
}

// ---------------------------------------------------------------------------
// Public entry point for URL scheme handling from Swift
// ---------------------------------------------------------------------------

/// Called from Swift's `application(_:open:options:)` to forward URL opens
/// into the GPUI platform callback system.
///
/// # Safety
/// `url_ptr` must be a valid null-terminated C string pointer.
/// Must be called on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_handle_open_url(url_ptr: *const std::os::raw::c_char) {
    if url_ptr.is_null() {
        return;
    }
    let url = unsafe { std::ffi::CStr::from_ptr(url_ptr) }
        .to_string_lossy()
        .into_owned();

    unsafe {
        let ptr = IOS_PLATFORM_STATE_PTR.load(std::sync::atomic::Ordering::Acquire);
        if !ptr.is_null() {
            let state = &*(ptr as *const Mutex<IosPlatformState>);
            let mut lock = state.lock();
            if let Some(ref mut callback) = lock.open_urls {
                callback(vec![url]);
            }
        }
    }
}

/// Global pointer to the IosPlatformState, set during IosPlatform::new.
/// Used by gpui_ios_handle_open_url to fire the callback from Swift.
static IOS_PLATFORM_STATE_PTR: std::sync::atomic::AtomicPtr<c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
