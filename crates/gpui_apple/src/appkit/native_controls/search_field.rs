use cocoa::{
    base::id,
    foundation::{NSPoint, NSRect, NSSize},
};
use objc::{class, msg_send, sel, sel_impl};

/// Creates a new NSSearchField with placeholder text.
pub unsafe fn create_native_search_field(placeholder: &str) -> id {
    unsafe {
        use super::super::ns_string;
        let field: id = msg_send![class!(NSSearchField), alloc];
        let field: id = msg_send![field, initWithFrame: NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(220.0, 24.0),
        )];
        let _: () = msg_send![field, setPlaceholderString: ns_string(placeholder)];
        let _: () = msg_send![field, setEditable: 1i8];
        let _: () = msg_send![field, setSelectable: 1i8];
        let _: () = msg_send![field, setBezeled: 1i8];
        let _: () = msg_send![field, setDrawsBackground: 1i8];
        let _: () = msg_send![field, setSendsSearchStringImmediately: 1i8];
        let _: () = msg_send![field, setSendsWholeSearchString: 0i8];
        let _: () = msg_send![field, setAutoresizingMask: 0u64];
        field
    }
}

/// Creates a toolbar-native search item backed by NSSearchToolbarItem.
pub unsafe fn create_native_search_toolbar_item(identifier: id) -> id {
    unsafe {
        let item: id = msg_send![class!(NSSearchToolbarItem), alloc];
        msg_send![item, initWithItemIdentifier: identifier]
    }
}

/// Returns the NSSearchField owned by an NSSearchToolbarItem.
pub unsafe fn get_native_search_toolbar_item_search_field(item: id) -> id {
    unsafe { msg_send![item, searchField] }
}

/// Sets the preferred expanded width AppKit uses while the toolbar search item is focused.
pub unsafe fn set_native_search_toolbar_item_preferred_width(item: id, width: f64) {
    unsafe {
        let _: () = msg_send![item, setPreferredWidthForSearchField: width];
    }
}

/// Controls whether clicking cancel clears the field and resigns focus.
pub unsafe fn set_native_search_toolbar_item_resigns_first_responder_with_cancel(
    item: id,
    resigns: bool,
) {
    unsafe {
        let _: () = msg_send![item, setResignsFirstResponderWithCancel: resigns as i8];
    }
}

/// Expands and focuses a toolbar search item using the native AppKit interaction.
pub unsafe fn begin_native_search_toolbar_item_interaction(item: id) {
    unsafe {
        let _: () = msg_send![item, beginSearchInteraction];
    }
}

/// Sets the current search text.
pub unsafe fn set_native_search_field_string_value(field: id, value: &str) {
    unsafe {
        use super::super::ns_string;
        let _: () = msg_send![field, setStringValue: ns_string(value)];
    }
}

/// Sets placeholder text.
pub unsafe fn set_native_search_field_placeholder(field: id, placeholder: &str) {
    unsafe {
        use super::super::ns_string;
        let _: () = msg_send![field, setPlaceholderString: ns_string(placeholder)];
    }
}

/// Sets a stable identifier used for programmatic focus lookups.
pub unsafe fn set_native_search_field_identifier(field: id, identifier: &str) {
    unsafe {
        use super::super::ns_string;
        let _: () = msg_send![field, setIdentifier: ns_string(identifier)];
    }
}

/// Controls whether the field sends each partial query as the user types.
pub unsafe fn set_native_search_field_sends_immediately(field: id, sends: bool) {
    unsafe {
        let _: () = msg_send![field, setSendsSearchStringImmediately: sends as i8];
    }
}

/// Controls whether the field only sends the complete search string.
pub unsafe fn set_native_search_field_sends_whole_string(field: id, sends: bool) {
    unsafe {
        let _: () = msg_send![field, setSendsWholeSearchString: sends as i8];
    }
}

/// Releases an NSSearchField created by GPUI.
pub unsafe fn release_native_search_field(field: id) {
    unsafe {
        let _: () = msg_send![field, release];
    }
}
