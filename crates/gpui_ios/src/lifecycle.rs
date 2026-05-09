use super::*;

pub(crate) static mut GPUI_GESTURE_DELEGATE_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_gesture_delegate_class() {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("GPUIGestureDelegate", superclass)
            .expect("failed to declare GPUIGestureDelegate class");

        decl.add_method(
            sel!(gestureRecognizer:shouldRecognizeSimultaneouslyWithGestureRecognizer:),
            gesture_should_recognize_simultaneously
                as extern "C" fn(&Object, Sel, *mut Object, *mut Object) -> BOOL,
        );

        GPUI_GESTURE_DELEGATE_CLASS = decl.register();
    }
}

extern "C" fn gesture_should_recognize_simultaneously(
    _this: &Object,
    _sel: Sel,
    _gesture1: *mut Object,
    _gesture2: *mut Object,
) -> BOOL {
    YES
}

// ---------------------------------------------------------------------------
// GPUIThermalObserver — observes thermal state change notifications
// ---------------------------------------------------------------------------

pub(crate) static mut GPUI_THERMAL_OBSERVER_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_thermal_observer_class() {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("GPUIThermalObserver", superclass)
            .expect("failed to declare GPUIThermalObserver class");

        decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);

        decl.add_method(
            sel!(thermalStateChanged:),
            handle_thermal_state_changed as extern "C" fn(&Object, Sel, *mut Object),
        );

        GPUI_THERMAL_OBSERVER_CLASS = decl.register();
    }
}

extern "C" fn handle_thermal_state_changed(this: &Object, _sel: Sel, _notification: *mut Object) {
    unsafe {
        let callback_ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
        if !callback_ptr.is_null() {
            let callback = &mut *(callback_ptr as *mut Box<dyn FnMut()>);
            callback();
        }
    }
}

// ---------------------------------------------------------------------------
// GPUIInputModeObserver — observes keyboard layout / input mode changes
// ---------------------------------------------------------------------------

pub(crate) static mut GPUI_INPUT_MODE_OBSERVER_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_input_mode_observer_class() {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("GPUIInputModeObserver", superclass)
            .expect("failed to declare GPUIInputModeObserver class");

        decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);

        decl.add_method(
            sel!(inputModeChanged:),
            handle_input_mode_changed as extern "C" fn(&Object, Sel, *mut Object),
        );

        GPUI_INPUT_MODE_OBSERVER_CLASS = decl.register();
    }
}

extern "C" fn handle_input_mode_changed(this: &Object, _sel: Sel, _notification: *mut Object) {
    // Defer to next run loop iteration to avoid re-entrant RefCell borrows.
    // This notification can fire synchronously during window setup (e.g. when
    // the keyboard proxy becomes first responder), at which point the App
    // RefCell is already borrowed by open_window.
    unsafe {
        let callback_ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
        if !callback_ptr.is_null() {
            // Prevent the callback from being freed or moved — the observer
            // object (and its ivar) outlives this dispatch. We pass the raw
            // pointer through dispatch_async_f; the trampoline dereferences it.
            dispatch_async_f(
                dispatch_get_main_queue_ptr(),
                callback_ptr,
                Some(input_mode_changed_trampoline),
            );
        }
    }
}

unsafe extern "C" fn input_mode_changed_trampoline(context: *mut c_void) {
    unsafe {
        let callback = &mut *(context as *mut Box<dyn FnMut()>);
        callback();
    }
}

// ---------------------------------------------------------------------------
// GPUISceneObserver — receives UIScene lifecycle notifications and forwards
// them to the window state callbacks.
// ---------------------------------------------------------------------------

pub(crate) static mut GPUI_SCENE_OBSERVER_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_scene_observer_class() {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("GPUISceneObserver", superclass)
            .expect("failed to declare GPUISceneObserver class");

        decl.add_ivar::<*mut c_void>(WINDOW_STATE_IVAR);

        decl.add_method(
            sel!(sceneDidActivate:),
            handle_scene_did_activate as extern "C" fn(&Object, Sel, *mut Object),
        );
        decl.add_method(
            sel!(sceneWillDeactivate:),
            handle_scene_will_deactivate as extern "C" fn(&Object, Sel, *mut Object),
        );
        decl.add_method(
            sel!(sceneDidEnterBackground:),
            handle_scene_did_enter_background as extern "C" fn(&Object, Sel, *mut Object),
        );
        decl.add_method(
            sel!(sceneWillEnterForeground:),
            handle_scene_will_enter_foreground as extern "C" fn(&Object, Sel, *mut Object),
        );

        GPUI_SCENE_OBSERVER_CLASS = decl.register();
    }
}

extern "C" fn handle_scene_did_activate(this: &Object, _sel: Sel, _notification: *mut Object) {
    let Some(state) = (unsafe { get_scene_observer_state(this) }) else {
        return;
    };
    log::debug!("scene did activate");
    let mut lock = state.lock();
    lock.is_active = true;
    if let Some(mut callback) = lock.on_active_change.take() {
        drop(lock);
        callback(true);
        state.lock().on_active_change = Some(callback);
    }
}

extern "C" fn handle_scene_will_deactivate(this: &Object, _sel: Sel, _notification: *mut Object) {
    let Some(state) = (unsafe { get_scene_observer_state(this) }) else {
        return;
    };
    log::debug!("scene will deactivate");
    let mut lock = state.lock();
    lock.is_active = false;
    if let Some(mut callback) = lock.on_active_change.take() {
        drop(lock);
        callback(false);
        state.lock().on_active_change = Some(callback);
    }
}

extern "C" fn handle_scene_did_enter_background(
    this: &Object,
    _sel: Sel,
    _notification: *mut Object,
) {
    let Some(state) = (unsafe { get_scene_observer_state(this) }) else {
        return;
    };
    log::debug!("scene entered background — pausing display link");
    let lock = state.lock();
    if !lock.display_link.is_null() {
        unsafe {
            let _: () = msg_send![lock.display_link, setPaused: YES];
        }
    }
}

extern "C" fn handle_scene_will_enter_foreground(
    this: &Object,
    _sel: Sel,
    _notification: *mut Object,
) {
    let Some(state) = (unsafe { get_scene_observer_state(this) }) else {
        return;
    };
    log::debug!("scene will enter foreground — resuming display link");
    let lock = state.lock();
    if !lock.display_link.is_null() {
        unsafe {
            let _: () = msg_send![lock.display_link, setPaused: NO];
        }
    }
}

unsafe fn get_scene_observer_state(observer: &Object) -> Option<Rc<Mutex<IosWindowState>>> {
    unsafe {
        let ptr: *mut c_void = *observer.get_ivar(WINDOW_STATE_IVAR);
        if ptr.is_null() {
            return None;
        }
        let rc = Rc::from_raw(ptr as *const Mutex<IosWindowState>);
        let clone = rc.clone();
        std::mem::forget(rc);
        Some(clone)
    }
}
