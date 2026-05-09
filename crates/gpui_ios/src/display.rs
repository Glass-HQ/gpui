use super::*;

#[derive(Debug)]
pub(crate) struct IosDisplay {
    id: DisplayId,
}

impl IosDisplay {
    pub(crate) fn primary() -> Self {
        Self {
            id: DisplayId::new(1),
        }
    }
}

impl PlatformDisplay for IosDisplay {
    fn id(&self) -> DisplayId {
        self.id
    }

    fn uuid(&self) -> Result<uuid::Uuid> {
        // Generate a stable UUID from the device's identifierForVendor
        let bytes = unsafe {
            let device: *mut Object = msg_send![class!(UIDevice), currentDevice];
            let vendor_id: *mut Object = msg_send![device, identifierForVendor];
            if !vendor_id.is_null() {
                let uuid_string: *mut Object = msg_send![vendor_id, UUIDString];
                if !uuid_string.is_null() {
                    let utf8: *const std::os::raw::c_char = msg_send![uuid_string, UTF8String];
                    if !utf8.is_null() {
                        let s = std::ffi::CStr::from_ptr(utf8).to_string_lossy();
                        if let Ok(parsed) = uuid::Uuid::parse_str(&s) {
                            return Ok(parsed);
                        }
                    }
                }
            }
            [0x01u8; 16]
        };
        Ok(uuid::Uuid::from_bytes(bytes))
    }

    fn bounds(&self) -> Bounds<Pixels> {
        // Query current screen bounds dynamically (handles rotation)
        unsafe {
            let screen: *mut Object = msg_send![class!(UIScreen), mainScreen];
            let bounds: CGRect = msg_send![screen, bounds];
            Bounds::new(
                point(px(0.0), px(0.0)),
                size(px(bounds.size.width as f32), px(bounds.size.height as f32)),
            )
        }
    }
}
