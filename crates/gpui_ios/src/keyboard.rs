use super::*;

#[derive(Clone)]
pub(crate) struct IosKeyboardLayout {
    id: String,
    name: String,
}

impl IosKeyboardLayout {
    pub(crate) fn current() -> Self {
        // Try to query the current text input mode for a language tag
        let (id, name) = unsafe {
            let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
            if app.is_null() {
                return Self {
                    id: "ios".into(),
                    name: "iOS".into(),
                };
            }
            // Get active text input mode
            let input_modes: *mut Object = msg_send![class!(UITextInputMode), activeInputModes];
            if !input_modes.is_null() {
                let count: usize = msg_send![input_modes, count];
                if count > 0 {
                    let mode: *mut Object = msg_send![input_modes, objectAtIndex: 0usize];
                    let lang: *mut Object = msg_send![mode, primaryLanguage];
                    if !lang.is_null() {
                        let utf8: *const std::os::raw::c_char = msg_send![lang, UTF8String];
                        if !utf8.is_null() {
                            let s = std::ffi::CStr::from_ptr(utf8)
                                .to_string_lossy()
                                .into_owned();
                            return Self {
                                id: s.clone(),
                                name: s,
                            };
                        }
                    }
                }
            }
            ("ios".into(), "iOS".into())
        };
        Self { id, name }
    }
}

impl PlatformKeyboardLayout for IosKeyboardLayout {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub(crate) struct IosKeyboardMapper {
    key_equivalents: Option<HashMap<char, char>>,
}

impl IosKeyboardMapper {
    pub(crate) fn new(layout_id: &str) -> Self {
        // Map non-QWERTY physical key positions to their QWERTY equivalents
        // so that keyboard shortcuts (Cmd+Z, Cmd+C, etc.) work on non-QWERTY
        // hardware keyboard layouts attached to iPad/iPhone.
        let mappings: Option<&[(char, char)]> = if layout_id.starts_with("fr") {
            // AZERTY (France)
            Some(&[('a', 'q'), ('q', 'a'), ('z', 'w'), ('w', 'z'), ('m', ';')])
        } else if layout_id.starts_with("de") || layout_id.starts_with("at") {
            // QWERTZ (German/Austrian)
            Some(&[('y', 'z'), ('z', 'y')])
        } else if layout_id.starts_with("cs")
            || layout_id.starts_with("sk")
            || layout_id.starts_with("hu")
        {
            // QWERTZ (Czech/Slovak/Hungarian)
            Some(&[('y', 'z'), ('z', 'y')])
        } else if layout_id.starts_with("be") {
            // AZERTY (Belgium)
            Some(&[('a', 'q'), ('q', 'a'), ('z', 'w'), ('w', 'z'), ('m', ';')])
        } else if layout_id.starts_with("tr") {
            // Turkish F-layout
            Some(&[('f', 'a'), ('g', 's'), ('j', 'h'), ('k', 'j')])
        } else {
            // QWERTY (English, Spanish, Portuguese, Italian, etc.)
            None
        };

        let key_equivalents = mappings.map(|pairs| pairs.iter().copied().collect());
        Self { key_equivalents }
    }
}

impl PlatformKeyboardMapper for IosKeyboardMapper {
    fn map_key_equivalent(
        &self,
        mut keystroke: Keystroke,
        use_key_equivalents: bool,
    ) -> KeybindingKeystroke {
        if use_key_equivalents
            && let Some(map) = &self.key_equivalents
            && keystroke.key.chars().count() == 1
            && let Some(mapped) = map.get(&keystroke.key.chars().next().unwrap())
        {
            keystroke.key = mapped.to_string();
        }
        KeybindingKeystroke::from_keystroke(keystroke)
    }

    fn get_key_equivalents(&self) -> Option<&HashMap<char, char>> {
        self.key_equivalents.as_ref()
    }
}
