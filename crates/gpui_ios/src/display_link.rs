use super::*;

// CADisplayLink target — an ObjC class whose `step:` method drives the frame
// loop on iOS, equivalent to CVDisplayLink on macOS.
// ---------------------------------------------------------------------------

pub(crate) static mut DISPLAY_LINK_TARGET_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_display_link_target_class() {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("GPUIDisplayLinkTarget", superclass)
            .expect("failed to declare GPUIDisplayLinkTarget class");
        decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);
        decl.add_method(
            sel!(step:),
            display_link_step as extern "C" fn(&Object, Sel, *mut Object),
        );
        DISPLAY_LINK_TARGET_CLASS = decl.register();
    }
}

extern "C" fn display_link_step(this: &Object, _sel: Sel, _display_link: *mut Object) {
    unsafe {
        let callback_ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
        if !callback_ptr.is_null() {
            let callback = &*(callback_ptr as *const Box<dyn Fn()>);
            callback();
        }
    }
}
