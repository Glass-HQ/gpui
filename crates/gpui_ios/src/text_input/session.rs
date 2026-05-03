use super::{backed_view::IosBackedTextInput, events::IosTextInputEvent};
use gpui::{
    Bounds, Pixels, PlatformInputHandler, TextInputConfig, TextInputSubmitBehavior, UTF16Selection,
    point, px,
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{BOOL, Class, NO, Object, Sel, YES},
    sel, sel_impl,
};
use std::{ffi::c_void, ops::Range, ptr};

type Id = *mut Object;

const DELEGATE_IVAR: &str = "sessionPtr";
static mut TEXT_INPUT_DELEGATE_CLASS: *const Class = ptr::null();

#[ctor::ctor]
fn register_text_input_delegate_class() {
    unsafe {
        let mut decl = ClassDecl::new("GPUIiOSTextInputDelegate", class!(NSObject))
            .expect("failed to declare GPUIiOSTextInputDelegate");
        decl.add_ivar::<*mut c_void>(DELEGATE_IVAR);
        decl.add_method(
            sel!(editingChanged:),
            editing_changed as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textFieldDidBeginEditing:),
            text_field_did_begin_editing as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textFieldDidEndEditing:),
            text_field_did_end_editing as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textFieldDidChangeSelection:),
            text_field_did_change_selection as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(backedTextFieldDidPressDeleteOnEmpty:),
            backed_text_field_did_press_delete_on_empty as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textFieldShouldReturn:),
            text_field_should_return as extern "C" fn(&Object, Sel, Id) -> BOOL,
        );
        decl.add_method(
            sel!(textViewDidBeginEditing:),
            text_view_did_begin_editing as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textViewDidEndEditing:),
            text_view_did_end_editing as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textViewDidChange:),
            text_view_did_change as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textViewDidChangeSelection:),
            text_view_did_change_selection as extern "C" fn(&Object, Sel, Id),
        );
        decl.add_method(
            sel!(textView:shouldChangeTextInRange:replacementText:),
            text_view_should_change_text as extern "C" fn(&Object, Sel, Id, Id, Id) -> BOOL,
        );
        TEXT_INPUT_DELEGATE_CLASS = decl.register();
    }
}

extern "C" fn editing_changed(this: &Object, _: Sel, text_input: Id) {
    with_session(this, |session| session.handle_text_change(text_input));
}

extern "C" fn text_field_did_begin_editing(this: &Object, _: Sel, _text_input: Id) {
    with_session(this, |session| {
        session.handle_event(IosTextInputEvent::Focused)
    });
}

extern "C" fn text_field_did_end_editing(this: &Object, _: Sel, text_input: Id) {
    with_session(this, |session| {
        session.flush_text_state(text_input);
        session.handle_event(IosTextInputEvent::Blurred);
    });
}

extern "C" fn text_field_did_change_selection(this: &Object, _: Sel, text_input: Id) {
    with_session(this, |session| session.handle_selection_change(text_input));
}

extern "C" fn backed_text_field_did_press_delete_on_empty(this: &Object, _: Sel, _text_input: Id) {
    with_session(this, |session| session.handle_empty_backspace());
}

extern "C" fn text_field_should_return(this: &Object, _: Sel, _text_input: Id) -> BOOL {
    with_session(this, |session| session.handle_return_key());
    NO
}

extern "C" fn text_view_did_begin_editing(this: &Object, _: Sel, _text_input: Id) {
    with_session(this, |session| {
        session.handle_event(IosTextInputEvent::Focused)
    });
}

extern "C" fn text_view_did_end_editing(this: &Object, _: Sel, text_input: Id) {
    with_session(this, |session| {
        session.flush_text_state(text_input);
        session.handle_event(IosTextInputEvent::Blurred);
    });
}

extern "C" fn text_view_did_change(this: &Object, _: Sel, text_input: Id) {
    with_session(this, |session| session.handle_text_change(text_input));
}

extern "C" fn text_view_did_change_selection(this: &Object, _: Sel, text_input: Id) {
    with_session(this, |session| session.handle_selection_change(text_input));
}

extern "C" fn text_view_should_change_text(
    this: &Object,
    _: Sel,
    _text_view: Id,
    _range: Id,
    replacement_text: Id,
) -> BOOL {
    let replacement_text = unsafe { get_ns_string(replacement_text) };
    with_session(this, |session| {
        if replacement_text == "\n"
            && session.last_config.submit_behavior != TextInputSubmitBehavior::InsertNewline
        {
            session.handle_return_key();
            return;
        }
        session.allow_text_change = true;
    });
    YES
}

fn with_session(this: &Object, f: impl FnOnce(&mut SessionState)) {
    unsafe {
        let session_ptr: *mut c_void = *this.get_ivar(DELEGATE_IVAR);
        if session_ptr.is_null() {
            return;
        }
        f(&mut *(session_ptr as *mut SessionState));
    }
}

pub(crate) struct IosTextInputSession {
    state: Box<SessionState>,
}

struct SessionState {
    parent_view: Id,
    delegate: Id,
    backed_input: IosBackedTextInput,
    input_handler: Option<PlatformInputHandler>,
    is_attached_this_frame: bool,
    is_syncing_from_gpui: bool,
    allow_text_change: bool,
    last_known_text: String,
    last_known_selection: Option<UTF16Selection>,
    last_known_marked_range: Option<Range<usize>>,
    last_config: TextInputConfig,
}

impl IosTextInputSession {
    pub(crate) unsafe fn new(parent_view: Id) -> Self {
        let delegate: Id = unsafe { msg_send![TEXT_INPUT_DELEGATE_CLASS, alloc] };
        let delegate: Id = unsafe { msg_send![delegate, init] };
        let last_config = TextInputConfig::default();
        let backed_input = unsafe { IosBackedTextInput::new(parent_view, delegate, &last_config) };

        let mut state = Box::new(SessionState {
            parent_view,
            delegate,
            backed_input,
            input_handler: None,
            is_attached_this_frame: false,
            is_syncing_from_gpui: false,
            allow_text_change: true,
            last_known_text: String::new(),
            last_known_selection: Some(UTF16Selection {
                range: 0..0,
                reversed: false,
            }),
            last_known_marked_range: None,
            last_config,
        });

        let session_ptr = (&mut *state) as *mut SessionState as *mut c_void;
        unsafe {
            (*delegate).set_ivar::<*mut c_void>(DELEGATE_IVAR, session_ptr);
        }
        Self { state }
    }

    pub(crate) fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.state.is_attached_this_frame = true;
        self.state.input_handler = Some(input_handler);
        self.state.sync_from_gpui();
        if !self.state.backed_input.is_first_responder() {
            self.state.backed_input.become_first_responder();
        }
    }

    pub(crate) fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.state.is_attached_this_frame = false;
        self.state.input_handler.take()
    }

    pub(crate) fn completed_frame(&mut self) {
        if self.state.is_attached_this_frame {
            self.state.allow_text_change = true;
            return;
        }

        if self.state.backed_input.is_first_responder() {
            self.state.backed_input.resign_first_responder();
        }
        unsafe {
            let _: BOOL = msg_send![self.state.parent_view, becomeFirstResponder];
        }
    }
}

impl Drop for IosTextInputSession {
    fn drop(&mut self) {
        unsafe {
            self.state.backed_input.teardown();
            let _: () = msg_send![self.state.delegate, release];
        }
    }
}

impl SessionState {
    fn handle_text_change(&mut self, text_input: Id) {
        if !self.allow_text_change {
            return;
        }
        self.allow_text_change = true;
        let text = unsafe { get_control_text(text_input) };
        let (selection_utf16, marked_range_utf16) = sanitize_text_state(
            &text,
            unsafe { selected_text_range_utf16(text_input) },
            unsafe { marked_text_range_utf16(text_input) },
        );
        self.handle_event(IosTextInputEvent::TextChanged {
            text,
            selection_utf16,
            marked_range_utf16,
        });
    }

    fn handle_selection_change(&mut self, text_input: Id) {
        let text = unsafe { get_control_text(text_input) };
        let (selection_utf16, marked_range_utf16) = sanitize_text_state(
            &text,
            unsafe { selected_text_range_utf16(text_input) },
            unsafe { marked_text_range_utf16(text_input) },
        );
        self.handle_event(IosTextInputEvent::SelectionChanged {
            selection_utf16,
            marked_range_utf16,
        });
    }

    fn handle_empty_backspace(&mut self) {
        if let Some(input_handler) = self.input_handler.as_mut() {
            input_handler.delete_backward();
        }
    }

    fn handle_return_key(&mut self) {
        match self.last_config.submit_behavior {
            TextInputSubmitBehavior::InsertNewline => {
                if self.last_config.multiline {
                    if let Some(input_handler) = self.input_handler.as_mut() {
                        input_handler.replace_text_in_range(None, "\n");
                    }
                }
            }
            TextInputSubmitBehavior::Submit => {
                if let Some(input_handler) = self.input_handler.as_mut() {
                    input_handler.submit_text_input();
                }
                self.handle_event(IosTextInputEvent::Submit);
            }
            TextInputSubmitBehavior::SubmitAndBlur => {
                if let Some(input_handler) = self.input_handler.as_mut() {
                    input_handler.submit_text_input();
                }
                self.handle_event(IosTextInputEvent::Submit);
                self.backed_input.resign_first_responder();
            }
        }
    }

    fn handle_event(&mut self, event: IosTextInputEvent) {
        if self.is_syncing_from_gpui {
            return;
        }

        match event {
            IosTextInputEvent::Focused => {
                log::debug!("[ios-input] backed input became first responder");
            }
            IosTextInputEvent::Blurred => {
                log::debug!("[ios-input] backed input resigned first responder");
            }
            IosTextInputEvent::Submit => {
                log::debug!("[ios-input] text input submitted");
            }
            IosTextInputEvent::SelectionChanged {
                selection_utf16,
                marked_range_utf16,
            } => {
                self.last_known_marked_range = marked_range_utf16;
                let selection = selection_utf16
                    .map(|range| selection_from_range(range, self.last_known_selection.as_ref()));
                self.last_known_selection = selection.clone();
                if let Some(input_handler) = self.input_handler.as_mut() {
                    input_handler.set_selected_text_range(selection);
                }
            }
            IosTextInputEvent::TextChanged {
                text,
                selection_utf16,
                marked_range_utf16,
            } => {
                let selection = selection_utf16
                    .clone()
                    .map(|range| selection_from_range(range, self.last_known_selection.as_ref()));
                self.propagate_text_change(&text, selection_utf16, marked_range_utf16.clone());
                self.last_known_text = text;
                self.last_known_selection = selection;
                self.last_known_marked_range = marked_range_utf16;
            }
        }
    }

    fn propagate_text_change(
        &mut self,
        new_text: &str,
        selection_utf16: Option<Range<usize>>,
        marked_range_utf16: Option<Range<usize>>,
    ) {
        let diff = text_diff(&self.last_known_text, new_text);
        let selected_range = if let Some(marked_range_utf16) = marked_range_utf16.clone() {
            selection_utf16
                .clone()
                .map(|selection| clamp_relative_selection(selection, marked_range_utf16))
        } else {
            None
        };
        let propagated_selection = selection_utf16
            .clone()
            .map(|range| selection_from_range(range, self.last_known_selection.as_ref()));

        let Some(input_handler) = self.input_handler.as_mut() else {
            return;
        };

        if marked_range_utf16.is_some() {
            input_handler.replace_and_mark_text_in_range(
                Some(diff.old_range_utf16.clone()),
                &diff.new_text,
                selected_range,
            );
        } else {
            input_handler.replace_text_in_range(Some(diff.old_range_utf16.clone()), &diff.new_text);
            if self.last_known_marked_range.is_some() {
                input_handler.unmark_text();
            }
        }

        input_handler.set_selected_text_range(propagated_selection);
    }

    fn flush_text_state(&mut self, text_input: Id) {
        let text = unsafe { get_control_text(text_input) };
        let (selection_utf16, marked_range_utf16) = sanitize_text_state(
            &text,
            unsafe { selected_text_range_utf16(text_input) },
            unsafe { marked_text_range_utf16(text_input) },
        );
        if text != self.last_known_text
            || selection_utf16.as_ref() != self.last_known_selection.as_ref().map(|s| &s.range)
            || marked_range_utf16 != self.last_known_marked_range
        {
            self.handle_event(IosTextInputEvent::TextChanged {
                text,
                selection_utf16,
                marked_range_utf16,
            });
        }
    }

    fn sync_from_gpui(&mut self) {
        let Some(input_handler) = self.input_handler.as_mut() else {
            return;
        };

        let snapshot = snapshot_from_gpui(input_handler);
        let should_rebuild = self.last_config.multiline != snapshot.config.multiline;
        if should_rebuild {
            unsafe {
                self.backed_input
                    .rebuild(self.parent_view, self.delegate, &snapshot.config);
            }
        }

        self.is_syncing_from_gpui = true;
        self.last_config = snapshot.config.clone();
        self.backed_input.apply_config(&snapshot.config);
        self.backed_input.set_frame(snapshot.bounds);

        if snapshot.text != self.last_known_text {
            self.backed_input.set_text(&snapshot.text);
            self.last_known_text = snapshot.text.clone();
        }

        if let Some(selection) = snapshot.selection
            && self.last_known_selection.as_ref() != Some(&selection)
        {
            self.backed_input
                .set_selected_range_utf16(selection.range.clone());
            self.last_known_selection = Some(selection);
        }

        self.last_known_marked_range = snapshot.marked_range;
        self.is_syncing_from_gpui = false;
    }
}

struct GpuiSnapshot {
    bounds: Bounds<Pixels>,
    config: TextInputConfig,
    text: String,
    selection: Option<UTF16Selection>,
    marked_range: Option<Range<usize>>,
}

fn snapshot_from_gpui(input_handler: &mut PlatformInputHandler) -> GpuiSnapshot {
    let selection = input_handler.selected_text_range(false);
    let caret_range = selection
        .as_ref()
        .map(|selection| {
            if selection.reversed {
                selection.range.start..selection.range.start
            } else {
                selection.range.end..selection.range.end
            }
        })
        .unwrap_or(0..0);
    let bounds = input_handler
        .bounds_for_range(caret_range)
        .unwrap_or_else(default_bounds);

    let mut adjusted = None;
    let text = input_handler
        .text_for_range(0..usize::MAX / 2, &mut adjusted)
        .unwrap_or_default();

    GpuiSnapshot {
        bounds,
        config: input_handler.text_input_config(),
        text,
        selection,
        marked_range: input_handler.marked_text_range(),
    }
}

fn default_bounds() -> Bounds<Pixels> {
    Bounds::from_corners(point(px(0.0), px(0.0)), point(px(1.0), px(1.0)))
}

fn selection_from_range(range: Range<usize>, previous: Option<&UTF16Selection>) -> UTF16Selection {
    let reversed = previous.is_some_and(|selection| selection.range == range && selection.reversed);
    UTF16Selection { range, reversed }
}

fn sanitize_text_state(
    text: &str,
    selection_utf16: Option<Range<usize>>,
    marked_range_utf16: Option<Range<usize>>,
) -> (Option<Range<usize>>, Option<Range<usize>>) {
    let text_len_utf16 = utf16_len(text);
    (
        selection_utf16.and_then(|range| sanitize_selection_range(range, text_len_utf16)),
        marked_range_utf16.and_then(|range| sanitize_marked_range(range, text_len_utf16)),
    )
}

fn sanitize_selection_range(range: Range<usize>, text_len_utf16: usize) -> Option<Range<usize>> {
    if range.start > range.end {
        log::warn!(
            "[ios-input] dropping reversed selection range {}..{} for text len {}",
            range.start,
            range.end,
            text_len_utf16
        );
        return None;
    }

    Some(range.start.min(text_len_utf16)..range.end.min(text_len_utf16))
}

fn sanitize_marked_range(range: Range<usize>, text_len_utf16: usize) -> Option<Range<usize>> {
    if range.start > range.end {
        log::warn!(
            "[ios-input] dropping reversed marked range {}..{} for text len {}",
            range.start,
            range.end,
            text_len_utf16
        );
        return None;
    }

    if range.end > text_len_utf16 {
        log::warn!(
            "[ios-input] dropping stale marked range {}..{} for text len {}",
            range.start,
            range.end,
            text_len_utf16
        );
        return None;
    }

    Some(range)
}

fn clamp_relative_selection(selection: Range<usize>, marked_range: Range<usize>) -> Range<usize> {
    let start = selection.start.saturating_sub(marked_range.start);
    let end = selection.end.saturating_sub(marked_range.start);
    let marked_len = marked_range.end.saturating_sub(marked_range.start);
    start.min(marked_len)..end.min(marked_len)
}

struct TextDiff {
    old_range_utf16: Range<usize>,
    new_text: String,
}

fn text_diff(old: &str, new: &str) -> TextDiff {
    let mut prefix_old_bytes = 0;
    let mut prefix_new_bytes = 0;
    let mut prefix_utf16 = 0;

    let mut old_chars = old.chars();
    let mut new_chars = new.chars();
    loop {
        match (old_chars.next(), new_chars.next()) {
            (Some(old_char), Some(new_char)) if old_char == new_char => {
                prefix_old_bytes += old_char.len_utf8();
                prefix_new_bytes += new_char.len_utf8();
                prefix_utf16 += old_char.len_utf16();
            }
            _ => break,
        }
    }

    let old_remaining = &old[prefix_old_bytes..];
    let new_remaining = &new[prefix_new_bytes..];
    let mut suffix_old_bytes = 0;
    let mut suffix_new_bytes = 0;
    let mut suffix_utf16 = 0;
    let mut old_rev = old_remaining.chars().rev();
    let mut new_rev = new_remaining.chars().rev();
    loop {
        match (old_rev.next(), new_rev.next()) {
            (Some(old_char), Some(new_char))
                if old_char == new_char
                    && prefix_old_bytes + suffix_old_bytes + old_char.len_utf8() <= old.len()
                    && prefix_new_bytes + suffix_new_bytes + new_char.len_utf8() <= new.len() =>
            {
                suffix_old_bytes += old_char.len_utf8();
                suffix_new_bytes += new_char.len_utf8();
                suffix_utf16 += old_char.len_utf16();
            }
            _ => break,
        }
    }

    let old_range_utf16 = prefix_utf16..utf16_len(old).saturating_sub(suffix_utf16);
    let new_text = new[prefix_new_bytes..new.len().saturating_sub(suffix_new_bytes)].to_string();
    TextDiff {
        old_range_utf16,
        new_text,
    }
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

unsafe fn get_control_text(text_input: Id) -> String {
    unsafe {
        get_ns_string({
            let text: Id = msg_send![text_input, text];
            text
        })
    }
}

unsafe fn get_ns_string(string: Id) -> String {
    if string.is_null() {
        return String::new();
    }
    let utf8: *const std::os::raw::c_char = msg_send![string, UTF8String];
    if utf8.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(utf8) }
            .to_string_lossy()
            .into_owned()
    }
}

unsafe fn marked_text_range_utf16(text_input: Id) -> Option<Range<usize>> {
    let marked_range: Id = msg_send![text_input, markedTextRange];
    if marked_range.is_null() {
        return None;
    }
    let beginning: Id = msg_send![text_input, beginningOfDocument];
    let start: Id = msg_send![marked_range, start];
    let end: Id = msg_send![marked_range, end];
    let start_offset: isize =
        msg_send![text_input, offsetFromPosition: beginning toPosition: start];
    let end_offset: isize = msg_send![text_input, offsetFromPosition: beginning toPosition: end];
    Some(start_offset.max(0) as usize..end_offset.max(0) as usize)
}

unsafe fn selected_text_range_utf16(text_input: Id) -> Option<Range<usize>> {
    let selected_range: Id = msg_send![text_input, selectedTextRange];
    if selected_range.is_null() {
        return None;
    }
    let beginning: Id = msg_send![text_input, beginningOfDocument];
    let start: Id = msg_send![selected_range, start];
    let end: Id = msg_send![selected_range, end];
    let start_offset: isize =
        msg_send![text_input, offsetFromPosition: beginning toPosition: start];
    let end_offset: isize = msg_send![text_input, offsetFromPosition: beginning toPosition: end];
    Some(start_offset.max(0) as usize..end_offset.max(0) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_diff_handles_middle_replacement() {
        let diff = text_diff("abcXYZdef", "abc123def");
        assert_eq!(diff.old_range_utf16, 3..6);
        assert_eq!(diff.new_text, "123");
    }

    #[test]
    fn text_diff_handles_emoji_utf16() {
        let diff = text_diff("a😀b", "a😎b");
        assert_eq!(diff.old_range_utf16, 1..3);
        assert_eq!(diff.new_text, "😎");
    }

    #[test]
    fn sanitize_selection_clamps_stale_offsets() {
        assert_eq!(sanitize_selection_range(2..9, 4), Some(2..4));
        assert_eq!(sanitize_selection_range(8..9, 4), Some(4..4));
    }

    #[test]
    fn sanitize_selection_rejects_reversed_offsets() {
        assert_eq!(sanitize_selection_range(4..2, 8), None);
    }

    #[test]
    fn sanitize_marked_range_rejects_stale_offsets() {
        assert_eq!(sanitize_marked_range(1..6, 4), None);
    }

    #[test]
    fn sanitize_text_state_clamps_selection_and_drops_marked_range() {
        assert_eq!(
            sanitize_text_state("abcd", Some(1..9), Some(2..8)),
            (Some(1..4), None)
        );
    }

    #[test]
    fn selection_from_range_preserves_existing_direction_for_same_range() {
        let previous = UTF16Selection {
            range: 2..5,
            reversed: true,
        };
        assert_eq!(
            selection_from_range(2..5, Some(&previous)),
            UTF16Selection {
                range: 2..5,
                reversed: true,
            }
        );
        assert_eq!(
            selection_from_range(2..6, Some(&previous)),
            UTF16Selection {
                range: 2..6,
                reversed: false,
            }
        );
    }
}
