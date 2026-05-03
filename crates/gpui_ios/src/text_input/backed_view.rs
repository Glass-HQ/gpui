use super::traits::{Id, apply_text_input_traits, should_use_text_view};
use gpui::{Bounds, Pixels, Point, TextInputConfig, TextInputSoftKeyboardPolicy};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{BOOL, Class, NO, Object, Sel, YES},
    sel, sel_impl,
};
use std::{ops::Range, ptr};

const DELETE_DELEGATE_IVAR: &str = "deleteDelegate";
static mut GPUI_BACKED_TEXT_FIELD_CLASS: *const Class = ptr::null();

#[ctor::ctor]
fn register_backed_text_field_class() {
    unsafe {
        let mut decl = ClassDecl::new("GPUIBackedTextField", class!(UITextField))
            .expect("failed to declare GPUIBackedTextField");
        decl.add_ivar::<Id>(DELETE_DELEGATE_IVAR);
        decl.add_method(
            sel!(keyboardInputShouldDelete:),
            keyboard_input_should_delete as extern "C" fn(&Object, Sel, Id) -> BOOL,
        );
        GPUI_BACKED_TEXT_FIELD_CLASS = decl.register();
    }
}

extern "C" fn keyboard_input_should_delete(this: &Object, _: Sel, text_field: Id) -> BOOL {
    unsafe {
        let text: Id = msg_send![text_field, text];
        let length: usize = if text.is_null() {
            0
        } else {
            msg_send![text, length]
        };
        if length == 0 {
            let delegate: Id = *this.get_ivar(DELETE_DELEGATE_IVAR);
            if !delegate.is_null() {
                let _: () = msg_send![delegate, backedTextFieldDidPressDeleteOnEmpty: text_field];
            }
        }
    }
    YES
}

pub(super) struct IosBackedTextInput {
    view: Id,
    kind: BackedTextInputKind,
    hidden_input_view: Option<Id>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BackedTextInputKind {
    TextField,
    TextView,
}

impl IosBackedTextInput {
    pub(super) unsafe fn new(parent_view: Id, delegate: Id, config: &TextInputConfig) -> Self {
        let kind = if should_use_text_view(config) {
            BackedTextInputKind::TextView
        } else {
            BackedTextInputKind::TextField
        };
        let view = unsafe { create_view(kind, config) };
        let mut backing = Self {
            view,
            kind,
            hidden_input_view: None,
        };
        unsafe {
            let _: () = msg_send![parent_view, addSubview: view];
        }
        backing.set_delegate(delegate);
        backing.apply_config(config);
        backing
    }

    #[allow(dead_code)]
    pub(super) fn kind(&self) -> BackedTextInputKind {
        self.kind
    }

    #[allow(dead_code)]
    pub(super) fn view(&self) -> Id {
        self.view
    }

    pub(super) unsafe fn rebuild(
        &mut self,
        parent_view: Id,
        delegate: Id,
        config: &TextInputConfig,
    ) {
        let text = self.text();
        let selection = self.selected_range_utf16();
        unsafe {
            self.teardown();
        }

        let kind = if should_use_text_view(config) {
            BackedTextInputKind::TextView
        } else {
            BackedTextInputKind::TextField
        };
        let view = unsafe { create_view(kind, config) };
        let _: () = msg_send![parent_view, addSubview: view];
        self.view = view;
        self.kind = kind;
        self.hidden_input_view = None;
        self.set_delegate(delegate);
        self.apply_config(config);
        self.set_text(&text);
        if let Some(selection) = selection {
            self.set_selected_range_utf16(selection);
        }
    }

    pub(super) fn set_delegate(&self, delegate: Id) {
        unsafe {
            match self.kind {
                BackedTextInputKind::TextField => {
                    (*self.view).set_ivar::<Id>(DELETE_DELEGATE_IVAR, delegate);
                    let _: () = msg_send![self.view, setDelegate: delegate];
                    let _: () = msg_send![
                        self.view,
                        addTarget: delegate
                        action: sel!(editingChanged:)
                        forControlEvents: 1u64 << 17
                    ];
                }
                BackedTextInputKind::TextView => {
                    let _: () = msg_send![self.view, setDelegate: delegate];
                }
            }
        }
    }

    pub(super) fn apply_config(&mut self, config: &TextInputConfig) {
        unsafe {
            apply_text_input_traits(self.view, config);
            match config.soft_keyboard {
                TextInputSoftKeyboardPolicy::Automatic => {
                    let _: () = msg_send![self.view, setInputView: ptr::null_mut::<Object>()];
                }
                TextInputSoftKeyboardPolicy::Hidden => {
                    let hidden_input_view = self.hidden_input_view.get_or_insert_with(|| {
                        let view: Id = msg_send![class!(UIView), alloc];
                        let view: Id = msg_send![view, init];
                        view
                    });
                    let _: () = msg_send![self.view, setInputView: *hidden_input_view];
                }
            }

            match self.kind {
                BackedTextInputKind::TextField => {
                    let _: () = msg_send![self.view, setBorderStyle: 0i64];
                }
                BackedTextInputKind::TextView => {
                    let _: () = msg_send![self.view, setScrollEnabled: YES];
                    let _: () =
                        msg_send![self.view, setTextContainerInset: UIEdgeInsets::default()];
                }
            }
        }
    }

    pub(super) fn set_frame(&self, bounds: Bounds<Pixels>) {
        let frame = CGRect {
            origin: CGPoint {
                x: bounds.origin.x.to_f64(),
                y: bounds.origin.y.to_f64(),
            },
            size: CGSize {
                width: bounds.size.width.to_f64().max(1.0),
                height: bounds.size.height.to_f64().max(1.0),
            },
        };
        unsafe {
            let _: () = msg_send![self.view, setFrame: frame];
        }
    }

    pub(super) fn text(&self) -> String {
        unsafe { get_view_text(self.view) }
    }

    pub(super) fn set_text(&self, text: &str) {
        unsafe {
            let _: () = msg_send![self.view, setText: ns_string(text)];
        }
    }

    #[allow(dead_code)]
    pub(super) fn marked_range_utf16(&self) -> Option<Range<usize>> {
        unsafe { range_for_text_range(self.view, msg_send![self.view, markedTextRange]) }
    }

    pub(super) fn selected_range_utf16(&self) -> Option<Range<usize>> {
        unsafe { range_for_text_range(self.view, msg_send![self.view, selectedTextRange]) }
    }

    pub(super) fn set_selected_range_utf16(&self, range_utf16: Range<usize>) {
        unsafe {
            if let Some(range) = make_text_range(self.view, range_utf16) {
                let _: () = msg_send![self.view, setSelectedTextRange: range];
            }
        }
    }

    #[allow(dead_code)]
    pub(super) fn first_rect_for_range(&self, range_utf16: Range<usize>) -> Option<Bounds<Pixels>> {
        unsafe {
            let range = make_text_range(self.view, range_utf16)?;
            let rect: CGRect = msg_send![self.view, firstRectForRange: range];
            let window: Id = msg_send![self.view, window];
            if window.is_null() {
                return None;
            }
            let screen_rect: CGRect = msg_send![window, convertRectToScreen: rect];
            Some(bounds_from_rect(screen_rect))
        }
    }

    #[allow(dead_code)]
    pub(super) fn caret_rect_for_position(&self, position_utf16: usize) -> Option<Bounds<Pixels>> {
        unsafe {
            let beginning: Id = msg_send![self.view, beginningOfDocument];
            let position: Id = msg_send![self.view, positionFromPosition: beginning offset: position_utf16 as isize];
            if position.is_null() {
                return None;
            }
            let rect: CGRect = msg_send![self.view, caretRectForPosition: position];
            let window: Id = msg_send![self.view, window];
            if window.is_null() {
                return None;
            }
            let screen_rect: CGRect = msg_send![window, convertRectToScreen: rect];
            Some(bounds_from_rect(screen_rect))
        }
    }

    #[allow(dead_code)]
    pub(super) fn character_index_for_point(&self, point: Point<Pixels>) -> Option<usize> {
        unsafe {
            let local_point = CGPoint {
                x: point.x.to_f64(),
                y: point.y.to_f64(),
            };
            let position: Id = msg_send![self.view, closestPositionToPoint: local_point];
            if position.is_null() {
                return None;
            }
            let beginning: Id = msg_send![self.view, beginningOfDocument];
            let offset: isize =
                msg_send![self.view, offsetFromPosition: beginning toPosition: position];
            Some(offset.max(0) as usize)
        }
    }

    pub(super) fn is_first_responder(&self) -> bool {
        unsafe {
            let is_first_responder: BOOL = msg_send![self.view, isFirstResponder];
            is_first_responder == YES
        }
    }

    pub(super) fn become_first_responder(&self) {
        unsafe {
            let _: BOOL = msg_send![self.view, becomeFirstResponder];
        }
    }

    pub(super) fn resign_first_responder(&self) {
        unsafe {
            let _: BOOL = msg_send![self.view, resignFirstResponder];
        }
    }

    pub(super) unsafe fn teardown(&mut self) {
        let _: () = msg_send![self.view, removeFromSuperview];
        let _: () = msg_send![self.view, setDelegate: ptr::null_mut::<Object>()];
        let _: () = msg_send![self.view, release];
        if let Some(hidden_input_view) = self.hidden_input_view.take() {
            let _: () = msg_send![hidden_input_view, release];
        }
    }
}

unsafe fn create_view(kind: BackedTextInputKind, config: &TextInputConfig) -> Id {
    let view: Id = match kind {
        BackedTextInputKind::TextField => {
            let text_field_class = unsafe { GPUI_BACKED_TEXT_FIELD_CLASS };
            let view: Id = msg_send![text_field_class, alloc];
            msg_send![view, init]
        }
        BackedTextInputKind::TextView => {
            let view: Id = msg_send![class!(UITextView), alloc];
            msg_send![view, init]
        }
    };

    let clear: Id = msg_send![class!(UIColor), clearColor];
    let _: () = msg_send![view, setBackgroundColor: clear];
    let _: () = msg_send![view, setTextColor: clear];
    let _: () = msg_send![view, setTintColor: clear];
    let _: () = msg_send![view, setOpaque: NO];
    let _: () = msg_send![view, setAlpha: 0.02f32];
    let _: () = msg_send![view, setAccessibilityElementsHidden: YES];
    if config.multiline {
        let _: () = msg_send![view, setTextContainerInset: UIEdgeInsets::default()];
    }
    view
}

unsafe fn get_view_text(view: Id) -> String {
    let text: Id = msg_send![view, text];
    if text.is_null() {
        return String::new();
    }

    let utf8: *const std::os::raw::c_char = msg_send![text, UTF8String];
    if utf8.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(utf8) }
            .to_string_lossy()
            .into_owned()
    }
}

unsafe fn range_for_text_range(view: Id, text_range: Id) -> Option<Range<usize>> {
    if text_range.is_null() {
        return None;
    }
    let beginning: Id = msg_send![view, beginningOfDocument];
    let start: Id = msg_send![text_range, start];
    let end: Id = msg_send![text_range, end];
    let start_offset: isize = msg_send![view, offsetFromPosition: beginning toPosition: start];
    let end_offset: isize = msg_send![view, offsetFromPosition: beginning toPosition: end];
    Some(start_offset.max(0) as usize..end_offset.max(0) as usize)
}

unsafe fn make_text_range(view: Id, range_utf16: Range<usize>) -> Option<Id> {
    let beginning: Id = msg_send![view, beginningOfDocument];
    let start: Id =
        msg_send![view, positionFromPosition: beginning offset: range_utf16.start as isize];
    let end: Id = msg_send![view, positionFromPosition: beginning offset: range_utf16.end as isize];
    if start.is_null() || end.is_null() {
        return None;
    }
    let range: Id = msg_send![view, textRangeFromPosition: start toPosition: end];
    if range.is_null() { None } else { Some(range) }
}

#[allow(dead_code)]
fn bounds_from_rect(rect: CGRect) -> Bounds<Pixels> {
    Bounds::from_corners(
        gpui::point(
            gpui::px(rect.origin.x as f32),
            gpui::px(rect.origin.y as f32),
        ),
        gpui::point(
            gpui::px((rect.origin.x + rect.size.width) as f32),
            gpui::px((rect.origin.y + rect.size.height) as f32),
        ),
    )
}

unsafe fn ns_string(value: &str) -> Id {
    let string: Id = msg_send![class!(NSString), alloc];
    let string: Id = msg_send![
        string,
        initWithBytes: value.as_ptr()
        length: value.len()
        encoding: 4u64
    ];
    let string: Id = msg_send![string, autorelease];
    string
}

#[repr(C)]
pub(super) struct CGPoint {
    pub(super) x: f64,
    pub(super) y: f64,
}

unsafe impl objc::Encode for CGPoint {
    fn encode() -> objc::Encoding {
        unsafe {
            objc::Encoding::from_str(&format!(
                "{{CGPoint={}{}}}",
                f64::encode().as_str(),
                f64::encode().as_str()
            ))
        }
    }
}

#[repr(C)]
pub(super) struct CGSize {
    pub(super) width: f64,
    pub(super) height: f64,
}

unsafe impl objc::Encode for CGSize {
    fn encode() -> objc::Encoding {
        unsafe {
            objc::Encoding::from_str(&format!(
                "{{CGSize={}{}}}",
                f64::encode().as_str(),
                f64::encode().as_str()
            ))
        }
    }
}

#[repr(C)]
pub(super) struct CGRect {
    pub(super) origin: CGPoint,
    pub(super) size: CGSize,
}

unsafe impl objc::Encode for CGRect {
    fn encode() -> objc::Encoding {
        unsafe {
            objc::Encoding::from_str(&format!(
                "{{CGRect={}{}}}",
                CGPoint::encode().as_str(),
                CGSize::encode().as_str()
            ))
        }
    }
}

#[repr(C)]
pub(super) struct UIEdgeInsets {
    pub(super) top: f64,
    pub(super) left: f64,
    pub(super) bottom: f64,
    pub(super) right: f64,
}

impl Default for UIEdgeInsets {
    fn default() -> Self {
        Self {
            top: 0.0,
            left: 0.0,
            bottom: 0.0,
            right: 0.0,
        }
    }
}

unsafe impl objc::Encode for UIEdgeInsets {
    fn encode() -> objc::Encoding {
        unsafe {
            objc::Encoding::from_str(&format!(
                "{{UIEdgeInsets={}{}{}{}}}",
                f64::encode().as_str(),
                f64::encode().as_str(),
                f64::encode().as_str(),
                f64::encode().as_str()
            ))
        }
    }
}
