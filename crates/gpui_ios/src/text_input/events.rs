use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IosTextInputEvent {
    Focused,
    Blurred,
    TextChanged {
        text: String,
        selection_utf16: Option<Range<usize>>,
        marked_range_utf16: Option<Range<usize>>,
    },
    SelectionChanged {
        selection_utf16: Option<Range<usize>>,
        marked_range_utf16: Option<Range<usize>>,
    },
    Submit,
}
