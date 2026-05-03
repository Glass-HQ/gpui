use gpui::{
    TextInputAutocapitalization, TextInputAutocorrection, TextInputConfig, TextInputContentType,
    TextInputKeyboardAppearance, TextInputKeyboardType, TextInputReturnKeyType,
    TextInputSpellChecking,
};
use objc::{class, msg_send, runtime::Object, sel, sel_impl};

pub(super) type Id = *mut Object;

pub(super) fn should_use_text_view(config: &TextInputConfig) -> bool {
    config.multiline
}

pub(super) unsafe fn apply_text_input_traits(view: Id, config: &TextInputConfig) {
    let _: () = msg_send![view, setKeyboardType: keyboard_type(config.keyboard_type)];
    let _: () = msg_send![view, setReturnKeyType: return_key_type(config.return_key_type)];
    let _: () = msg_send![view, setSecureTextEntry: config.secure_entry];
    let _: () = msg_send![view, setAutocorrectionType: autocorrection_type(config.autocorrection)];
    let _: () = msg_send![view, setSpellCheckingType: spell_checking_type(config.spell_checking)];
    let _: () = msg_send![
        view,
        setAutocapitalizationType: autocapitalization_type(config.autocapitalization)
    ];
    let _: () = msg_send![
        view,
        setKeyboardAppearance: keyboard_appearance(config.keyboard_appearance)
    ];
    let _: () = msg_send![
        view,
        setEnablesReturnKeyAutomatically: config.enables_return_key_automatically
    ];

    if let Some(enabled) = config.smart_insert_delete {
        let smart_insert_delete_type = if enabled { 1isize } else { 2isize };
        let _: () = msg_send![view, setSmartInsertDeleteType: smart_insert_delete_type];
    }

    let content_type = config
        .text_content_type
        .and_then(|content_type| unsafe { content_type_name(content_type) });
    match content_type {
        Some(content_type) => {
            let _: () = msg_send![view, setTextContentType: content_type];
        }
        None => {
            let _: () = msg_send![view, setTextContentType: std::ptr::null_mut::<Object>()];
        }
    }

    let _: () = msg_send![view, reloadInputViews];
}

fn keyboard_type(keyboard_type: TextInputKeyboardType) -> isize {
    match keyboard_type {
        TextInputKeyboardType::Default => 0,
        TextInputKeyboardType::AsciiCapable => 1,
        TextInputKeyboardType::NumbersAndPunctuation => 2,
        TextInputKeyboardType::Url => 3,
        TextInputKeyboardType::NumberPad => 4,
        TextInputKeyboardType::PhonePad => 5,
        TextInputKeyboardType::NamePhonePad => 6,
        TextInputKeyboardType::EmailAddress => 7,
        TextInputKeyboardType::DecimalPad => 8,
        TextInputKeyboardType::Twitter => 9,
        TextInputKeyboardType::WebSearch => 10,
        TextInputKeyboardType::AsciiCapableNumberPad => 11,
    }
}

fn return_key_type(return_key_type: TextInputReturnKeyType) -> isize {
    match return_key_type {
        TextInputReturnKeyType::Default => 0,
        TextInputReturnKeyType::Go => 1,
        TextInputReturnKeyType::Google => 2,
        TextInputReturnKeyType::Join => 3,
        TextInputReturnKeyType::Next => 4,
        TextInputReturnKeyType::Route => 5,
        TextInputReturnKeyType::Search => 6,
        TextInputReturnKeyType::Send => 7,
        TextInputReturnKeyType::Yahoo => 8,
        TextInputReturnKeyType::Done => 9,
        TextInputReturnKeyType::EmergencyCall => 10,
        TextInputReturnKeyType::Continue => 11,
    }
}

fn autocorrection_type(autocorrection: TextInputAutocorrection) -> isize {
    match autocorrection {
        TextInputAutocorrection::Default => 0,
        TextInputAutocorrection::No => 1,
        TextInputAutocorrection::Yes => 2,
    }
}

fn spell_checking_type(spell_checking: TextInputSpellChecking) -> isize {
    match spell_checking {
        TextInputSpellChecking::Default => 0,
        TextInputSpellChecking::No => 1,
        TextInputSpellChecking::Yes => 2,
    }
}

fn autocapitalization_type(autocapitalization: TextInputAutocapitalization) -> isize {
    match autocapitalization {
        TextInputAutocapitalization::None => 0,
        TextInputAutocapitalization::Words => 1,
        TextInputAutocapitalization::Sentences => 2,
        TextInputAutocapitalization::AllCharacters => 3,
    }
}

fn keyboard_appearance(appearance: TextInputKeyboardAppearance) -> isize {
    match appearance {
        TextInputKeyboardAppearance::Default => 0,
        TextInputKeyboardAppearance::Dark => 1,
        TextInputKeyboardAppearance::Light => 2,
    }
}

unsafe fn content_type_name(content_type: TextInputContentType) -> Option<Id> {
    let name = match content_type {
        TextInputContentType::Name => "name",
        TextInputContentType::NamePrefix => "namePrefix",
        TextInputContentType::GivenName => "givenName",
        TextInputContentType::MiddleName => "middleName",
        TextInputContentType::FamilyName => "familyName",
        TextInputContentType::NameSuffix => "nameSuffix",
        TextInputContentType::Nickname => "nickname",
        TextInputContentType::JobTitle => "jobTitle",
        TextInputContentType::OrganizationName => "organizationName",
        TextInputContentType::Location => "location",
        TextInputContentType::FullStreetAddress => "fullStreetAddress",
        TextInputContentType::StreetAddressLine1 => "streetAddressLine1",
        TextInputContentType::StreetAddressLine2 => "streetAddressLine2",
        TextInputContentType::AddressCity => "addressCity",
        TextInputContentType::AddressState => "addressState",
        TextInputContentType::PostalCode => "postalCode",
        TextInputContentType::CountryName => "countryName",
        TextInputContentType::TelephoneNumber => "telephoneNumber",
        TextInputContentType::EmailAddress => "emailAddress",
        TextInputContentType::Url => "URL",
        TextInputContentType::CreditCardNumber => "creditCardNumber",
        TextInputContentType::Username => "username",
        TextInputContentType::Password => "password",
        TextInputContentType::NewPassword => "newPassword",
        TextInputContentType::OneTimeCode => "oneTimeCode",
    };

    let c_name = std::ffi::CString::new(name).ok()?;
    let ns_string: Id = msg_send![class!(NSString), stringWithUTF8String: c_name.as_ptr()];
    if ns_string.is_null() {
        None
    } else {
        Some(ns_string)
    }
}
