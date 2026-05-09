use super::*;

// ---------------------------------------------------------------------------
// GPUIView — custom UIView subclass for touch input, Metal layer, and
// lifecycle callbacks (resize, appearance change).
// ---------------------------------------------------------------------------

pub(crate) static mut GPUI_VIEW_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_gpui_view_class() {
    unsafe {
        let superclass = class!(UIView);
        let mut decl =
            ClassDecl::new("GPUIView", superclass).expect("failed to declare GPUIView class");

        // Ivar to hold a raw pointer to Rc<Mutex<IosWindowState>>
        decl.add_ivar::<*mut c_void>(WINDOW_STATE_IVAR);

        // Touch input
        decl.add_method(
            sel!(touchesBegan:withEvent:),
            handle_touches_began as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(touchesMoved:withEvent:),
            handle_touches_moved as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(touchesEnded:withEvent:),
            handle_touches_ended as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(touchesCancelled:withEvent:),
            handle_touches_cancelled as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );

        // Layout (resize, rotation, split view)
        decl.add_method(
            sel!(layoutSubviews),
            handle_layout_subviews as extern "C" fn(&Object, Sel),
        );

        // Safe area insets change
        decl.add_method(
            sel!(safeAreaInsetsDidChange),
            handle_safe_area_insets_change as extern "C" fn(&Object, Sel),
        );

        // Appearance (dark/light mode change)
        decl.add_method(
            sel!(traitCollectionDidChange:),
            handle_trait_collection_change as extern "C" fn(&Object, Sel, *mut Object),
        );
        decl.add_method(
            sel!(canBecomeFirstResponder),
            can_become_first_responder as extern "C" fn(&Object, Sel) -> BOOL,
        );

        // Two-finger scroll pan gesture
        decl.add_method(
            sel!(handleScrollPan:),
            handle_scroll_pan as extern "C" fn(&Object, Sel, *mut Object),
        );

        // Single-finger scroll pan gesture
        decl.add_method(
            sel!(handleSingleFingerPan:),
            handle_single_finger_pan as extern "C" fn(&Object, Sel, *mut Object),
        );

        // Pinch gesture
        decl.add_method(
            sel!(handlePinch:),
            handle_pinch as extern "C" fn(&Object, Sel, *mut Object),
        );

        // Rotation gesture
        decl.add_method(
            sel!(handleRotation:),
            handle_rotation as extern "C" fn(&Object, Sel, *mut Object),
        );

        // iPadOS hover gesture (pointer support)
        decl.add_method(
            sel!(handleHover:),
            handle_hover as extern "C" fn(&Object, Sel, *mut Object),
        );

        // Long-press gesture (simulates right-click for context menus)
        decl.add_method(
            sel!(handleLongPress:),
            handle_long_press as extern "C" fn(&Object, Sel, *mut Object),
        );
        decl.add_method(
            sel!(pressesBegan:withEvent:),
            handle_presses_began as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(pressesEnded:withEvent:),
            handle_presses_ended as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(pressesCancelled:withEvent:),
            handle_presses_cancelled as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );

        // Make CAMetalLayer the view's own backing layer
        decl.add_class_method(
            sel!(layerClass),
            gpui_view_layer_class as extern "C" fn(&Class, Sel) -> *const Class,
        );

        GPUI_VIEW_CLASS = decl.register();
    }
}

extern "C" fn gpui_view_layer_class(_self: &Class, _sel: Sel) -> *const Class {
    class!(CAMetalLayer)
}

/// Recover the `Rc<Mutex<IosWindowState>>` from the view's ivar without
/// consuming the Rc (the ivar still holds its reference).
unsafe fn get_window_state(view: &Object) -> Option<Rc<Mutex<IosWindowState>>> {
    unsafe {
        let ptr: *mut c_void = *view.get_ivar(WINDOW_STATE_IVAR);
        if ptr.is_null() {
            return None;
        }
        let rc = Rc::from_raw(ptr as *const Mutex<IosWindowState>);
        let clone = rc.clone();
        std::mem::forget(rc); // Don't drop — ivar still holds it
        Some(clone)
    }
}

/// Extract the primary touch position from a UITouch set relative to the view.
/// Returns `(position, tap_count)`.
unsafe fn primary_touch_info(
    touches: *mut Object,
    view: &Object,
    state: &Mutex<IosWindowState>,
) -> Option<(Point<Pixels>, usize)> {
    let all_objects: *mut Object = msg_send![touches, allObjects];
    let count: usize = msg_send![all_objects, count];
    if count == 0 {
        return None;
    }

    let mut lock = state.lock();

    // Find the tracked touch, or pick the first one if we're not tracking yet
    let touch = if let Some(tracked) = lock.tracked_touch {
        let mut found: *mut Object = std::ptr::null_mut();
        for i in 0..count {
            let t: *mut Object = msg_send![all_objects, objectAtIndex: i];
            if t == tracked {
                found = t;
                break;
            }
        }
        if found.is_null() {
            return None;
        }
        found
    } else {
        let touch: *mut Object = msg_send![all_objects, objectAtIndex: 0usize];
        lock.tracked_touch = Some(touch);
        touch
    };

    let location: CGPoint = msg_send![touch, locationInView: view as *const Object as *mut Object];
    let tap_count: usize = msg_send![touch, tapCount];
    let position = point(px(location.x as f32), px(location.y as f32));

    // Update last known mouse position
    lock.last_touch_position = Some(position);

    Some((position, tap_count))
}

pub(crate) fn dispatch_input(
    state: &Mutex<IosWindowState>,
    input: PlatformInput,
) -> crate::DispatchEventResult {
    let mut lock = state.lock();
    if let Some(mut callback) = lock.on_input.take() {
        drop(lock);
        let result = callback(input);
        state.lock().on_input = Some(callback);
        result
    } else {
        crate::DispatchEventResult {
            propagate: true,
            default_prevented: false,
        }
    }
}

extern "C" fn handle_touches_began(
    this: &Object,
    _sel: Sel,
    touches: *mut Object,
    _event: *mut Object,
) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };

    // Cancel any active scroll momentum when a new touch begins
    state.lock().scroll_momentum = None;

    let Some((position, click_count)) = (unsafe { primary_touch_info(touches, this, &state) })
    else {
        return;
    };

    let modifiers = state.lock().current_modifiers;
    log::trace!(
        "[touch] touchesBegan at ({}, {}), click_count={}",
        position.x.to_f64(),
        position.y.to_f64(),
        click_count
    );
    let result = dispatch_input(
        &state,
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers,
            click_count,
            first_mouse: false,
        }),
    );
    log::trace!(
        "[touch] dispatch_input result: propagate={}, default_prevented={}",
        result.propagate,
        result.default_prevented
    );

    let _ = result;
}

extern "C" fn handle_touches_moved(
    this: &Object,
    _sel: Sel,
    touches: *mut Object,
    _event: *mut Object,
) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    let Some((position, _)) = (unsafe { primary_touch_info(touches, this, &state) }) else {
        return;
    };

    let modifiers = state.lock().current_modifiers;
    dispatch_input(
        &state,
        PlatformInput::MouseMove(MouseMoveEvent {
            position,
            pressed_button: Some(MouseButton::Left),
            modifiers,
        }),
    );
}

extern "C" fn handle_touches_ended(
    this: &Object,
    _sel: Sel,
    touches: *mut Object,
    _event: *mut Object,
) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    let Some((position, click_count)) = (unsafe { primary_touch_info(touches, this, &state) })
    else {
        return;
    };

    // Clear tracked touch and grab modifiers
    let modifiers = {
        let mut lock = state.lock();
        lock.tracked_touch = None;
        lock.current_modifiers
    };

    dispatch_input(
        &state,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers,
            click_count,
        }),
    );
}

extern "C" fn handle_touches_cancelled(
    this: &Object,
    _sel: Sel,
    _touches: *mut Object,
    _event: *mut Object,
) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };

    // Use last known position or zero, clear tracked touch, grab modifiers
    let (position, modifiers) = {
        let mut lock = state.lock();
        let pos = lock.last_touch_position.unwrap_or_else(Point::default);
        lock.tracked_touch = None;
        (pos, lock.current_modifiers)
    };

    dispatch_input(
        &state,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers,
            click_count: 1,
        }),
    );
}

extern "C" fn handle_layout_subviews(this: &Object, _sel: Sel) {
    unsafe {
        // Call [super layoutSubviews]
        let superclass = class!(UIView);
        let _: () = msg_send![super(this, superclass), layoutSubviews];

        let Some(state) = get_window_state(this) else {
            return;
        };

        let bounds: CGRect = msg_send![this, bounds];
        let scale: f64 = msg_send![this, contentScaleFactor];

        // The view's layer IS the Metal layer (via layerClass override)
        let metal_layer: *mut Object = msg_send![this, layer];
        let _: () = msg_send![metal_layer, setContentsScale: scale];

        let new_size = Size {
            width: px(bounds.size.width as f32),
            height: px(bounds.size.height as f32),
        };
        let scale_factor = scale as f32;
        let device_width = f32::from(new_size.width) * scale_factor;
        let device_height = f32::from(new_size.height) * scale_factor;

        // Read safe area insets while we have the view reference — they may change on rotation
        let insets: UIEdgeInsets = msg_send![this, safeAreaInsets];

        let mut lock = state.lock();
        let size_changed = lock.bounds.size != new_size || lock.scale_factor != scale_factor;
        if !size_changed {
            // Still update insets in case they changed without a size change
            lock.safe_area_insets = insets;
            return;
        }

        lock.bounds.size = new_size;
        lock.scale_factor = scale_factor;
        lock.safe_area_insets = insets;

        // The view's layer IS the Metal layer (via replace_layer), so UIKit
        // auto-sizes it. Just update the drawable size for rendering.
        lock.renderer.update_drawable_size(size(
            DevicePixels(device_width as i32),
            DevicePixels(device_height as i32),
        ));

        if let Some(mut callback) = lock.on_resize.take() {
            drop(lock);
            callback(new_size, scale_factor);
            state.lock().on_resize = Some(callback);
        }
    }
}

extern "C" fn handle_trait_collection_change(
    this: &Object,
    _sel: Sel,
    _previous_trait_collection: *mut Object,
) {
    unsafe {
        let superclass = class!(UIView);
        let _: () = msg_send![super(this, superclass), traitCollectionDidChange: _previous_trait_collection];

        let Some(state) = get_window_state(this) else {
            return;
        };

        // Check if the user interface style actually changed
        let current_traits: *mut Object = msg_send![this, traitCollection];
        let current_style: isize = msg_send![current_traits, userInterfaceStyle];

        if !_previous_trait_collection.is_null() {
            let previous_style: isize = msg_send![_previous_trait_collection, userInterfaceStyle];
            if current_style == previous_style {
                return;
            }
        }

        log::info!(
            "appearance changed to {}",
            if current_style == 2 { "dark" } else { "light" }
        );

        let mut lock = state.lock();
        if let Some(mut callback) = lock.on_appearance_change.take() {
            drop(lock);
            callback();
            state.lock().on_appearance_change = Some(callback);
        }
    }
}

extern "C" fn can_become_first_responder(_this: &Object, _sel: Sel) -> BOOL {
    YES
}

fn keycode_to_key_name(keycode: isize) -> Option<&'static str> {
    match keycode {
        0x04 => Some("a"),
        0x05 => Some("b"),
        0x06 => Some("c"),
        0x07 => Some("d"),
        0x08 => Some("e"),
        0x09 => Some("f"),
        0x0A => Some("g"),
        0x0B => Some("h"),
        0x0C => Some("i"),
        0x0D => Some("j"),
        0x0E => Some("k"),
        0x0F => Some("l"),
        0x10 => Some("m"),
        0x11 => Some("n"),
        0x12 => Some("o"),
        0x13 => Some("p"),
        0x14 => Some("q"),
        0x15 => Some("r"),
        0x16 => Some("s"),
        0x17 => Some("t"),
        0x18 => Some("u"),
        0x19 => Some("v"),
        0x1A => Some("w"),
        0x1B => Some("x"),
        0x1C => Some("y"),
        0x1D => Some("z"),
        0x1E => Some("1"),
        0x1F => Some("2"),
        0x20 => Some("3"),
        0x21 => Some("4"),
        0x22 => Some("5"),
        0x23 => Some("6"),
        0x24 => Some("7"),
        0x25 => Some("8"),
        0x26 => Some("9"),
        0x27 => Some("0"),
        0x28 => Some("enter"),
        0x29 => Some("escape"),
        0x2A => Some("backspace"),
        0x2B => Some("tab"),
        0x2C => Some("space"),
        0x2D => Some("-"),
        0x2E => Some("="),
        0x2F => Some("["),
        0x30 => Some("]"),
        0x31 => Some("\\"),
        0x33 => Some(";"),
        0x34 => Some("'"),
        0x35 => Some("`"),
        0x36 => Some(","),
        0x37 => Some("."),
        0x38 => Some("/"),
        0x4F => Some("right"),
        0x50 => Some("left"),
        0x51 => Some("down"),
        0x52 => Some("up"),
        0xE0 => Some("leftcontrol"),
        0xE1 => Some("leftshift"),
        0xE2 => Some("leftalt"),
        0xE3 => Some("leftmeta"),
        0xE4 => Some("rightcontrol"),
        0xE5 => Some("rightshift"),
        0xE6 => Some("rightalt"),
        0xE7 => Some("rightmeta"),
        _ => None,
    }
}

fn modifiers_from_flags(flags: isize) -> Modifiers {
    Modifiers {
        control: flags & 0x040000 != 0,
        alt: flags & 0x080000 != 0,
        shift: flags & 0x020000 != 0,
        platform: flags & 0x100000 != 0,
        function: flags & 0x800000 != 0,
    }
}

fn is_modifier_key(keycode: isize) -> bool {
    (224..=231).contains(&keycode)
}

extern "C" fn handle_presses_began(
    this: &Object,
    _sel: Sel,
    presses: *mut Object,
    _event: *mut Object,
) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let all: *mut Object = msg_send![presses, allObjects];
        let count: usize = msg_send![all, count];
        for i in 0..count {
            let press: *mut Object = msg_send![all, objectAtIndex: i];
            let key: *mut Object = msg_send![press, key];
            if key.is_null() {
                continue;
            }
            let keycode: isize = msg_send![key, keyCode];
            let modifier_flags: isize = msg_send![key, modifierFlags];
            let modifiers = modifiers_from_flags(modifier_flags);
            state.lock().current_modifiers = modifiers;

            if is_modifier_key(keycode) {
                dispatch_input(
                    &state,
                    PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                        modifiers,
                        capslock: Capslock::default(),
                    }),
                );
                continue;
            }

            let key_name = keycode_to_key_name(keycode)
                .map(ToString::to_string)
                .or_else(|| {
                    let chars: *mut Object = msg_send![key, charactersIgnoringModifiers];
                    if chars.is_null() {
                        return None;
                    }
                    let utf8: *const std::os::raw::c_char = msg_send![chars, UTF8String];
                    if utf8.is_null() {
                        return None;
                    }
                    Some(
                        std::ffi::CStr::from_ptr(utf8)
                            .to_string_lossy()
                            .to_lowercase(),
                    )
                });
            let Some(key_name) = key_name else {
                continue;
            };

            let key_char = {
                let chars: *mut Object = msg_send![key, characters];
                if chars.is_null() {
                    None
                } else {
                    let utf8: *const std::os::raw::c_char = msg_send![chars, UTF8String];
                    if utf8.is_null() {
                        None
                    } else {
                        Some(
                            std::ffi::CStr::from_ptr(utf8)
                                .to_string_lossy()
                                .into_owned(),
                        )
                    }
                }
            };

            dispatch_input(
                &state,
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke: Keystroke {
                        modifiers,
                        key: key_name,
                        key_char,
                        native_key_code: Some(keycode as u16),
                    },
                    is_held: false,
                    prefer_character_input: false,
                }),
            );
        }
    }
}

extern "C" fn handle_presses_ended(
    this: &Object,
    _sel: Sel,
    presses: *mut Object,
    _event: *mut Object,
) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let all: *mut Object = msg_send![presses, allObjects];
        let count: usize = msg_send![all, count];
        for i in 0..count {
            let press: *mut Object = msg_send![all, objectAtIndex: i];
            let key: *mut Object = msg_send![press, key];
            if key.is_null() {
                continue;
            }
            let keycode: isize = msg_send![key, keyCode];
            let modifier_flags: isize = msg_send![key, modifierFlags];
            let modifiers = modifiers_from_flags(modifier_flags);
            state.lock().current_modifiers = modifiers;

            if is_modifier_key(keycode) {
                dispatch_input(
                    &state,
                    PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                        modifiers,
                        capslock: Capslock::default(),
                    }),
                );
                continue;
            }

            let Some(key_name) = keycode_to_key_name(keycode).map(ToString::to_string) else {
                continue;
            };

            dispatch_input(
                &state,
                PlatformInput::KeyUp(KeyUpEvent {
                    keystroke: Keystroke {
                        modifiers,
                        key: key_name,
                        key_char: None,
                        native_key_code: Some(keycode as u16),
                    },
                }),
            );
        }
    }
}

extern "C" fn handle_presses_cancelled(
    this: &Object,
    _sel: Sel,
    presses: *mut Object,
    event: *mut Object,
) {
    handle_presses_ended(this, _sel, presses, event);
}

// ---------------------------------------------------------------------------
// iPadOS hover gesture — fires MouseMove without a pressed button when the
// user hovers a pointer (trackpad, mouse, Apple Pencil hover) over the view.
// ---------------------------------------------------------------------------

extern "C" fn handle_hover(this: &Object, _sel: Sel, gesture: *mut Object) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let gesture_state: isize = msg_send![gesture, state];
        // UIGestureRecognizerState: 1=Began, 2=Changed, 3=Ended, 4=Cancelled
        match gesture_state {
            1 | 2 => {
                let location: CGPoint =
                    msg_send![gesture, locationInView: this as *const Object as *mut Object];
                let position = point(px(location.x as f32), px(location.y as f32));
                state.lock().last_touch_position = Some(position);

                let modifiers = state.lock().current_modifiers;
                dispatch_input(
                    &state,
                    PlatformInput::MouseMove(MouseMoveEvent {
                        position,
                        pressed_button: None,
                        modifiers,
                    }),
                );
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Long-press gesture — simulates right-click (context menu) on iOS.
// ---------------------------------------------------------------------------

extern "C" fn handle_long_press(this: &Object, _sel: Sel, gesture: *mut Object) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let gesture_state: isize = msg_send![gesture, state];
        let location: CGPoint =
            msg_send![gesture, locationInView: this as *const Object as *mut Object];
        let position = point(px(location.x as f32), px(location.y as f32));

        let modifiers = state.lock().current_modifiers;
        // UIGestureRecognizerState: 1=Began, 3=Ended, 4=Cancelled
        match gesture_state {
            1 => {
                dispatch_input(
                    &state,
                    PlatformInput::MouseDown(MouseDownEvent {
                        button: MouseButton::Right,
                        position,
                        modifiers,
                        click_count: 1,
                        first_mouse: false,
                    }),
                );
            }
            3 | 4 => {
                dispatch_input(
                    &state,
                    PlatformInput::MouseUp(MouseUpEvent {
                        button: MouseButton::Right,
                        position,
                        modifiers,
                        click_count: 1,
                    }),
                );
            }
            _ => {}
        }
    }
}

extern "C" fn handle_scroll_pan(this: &Object, _sel: Sel, gesture: *mut Object) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let gesture_state: isize = msg_send![gesture, state];
        // UIGestureRecognizerState: 1=Began, 2=Changed, 3=Ended, 4=Cancelled
        let touch_phase = match gesture_state {
            1 => TouchPhase::Started,
            2 => TouchPhase::Moved,
            3 | 4 => TouchPhase::Ended,
            _ => return,
        };

        // Get translation (cumulative) and reset to zero for incremental deltas
        let translation: CGPoint =
            msg_send![gesture, translationInView: this as *const Object as *mut Object];
        let zero = CGPoint { x: 0.0, y: 0.0 };
        let _: () =
            msg_send![gesture, setTranslation: zero inView: this as *const Object as *mut Object];

        // Get position of the gesture centroid
        let location: CGPoint =
            msg_send![gesture, locationInView: this as *const Object as *mut Object];
        let position = point(px(location.x as f32), px(location.y as f32));

        let delta = ScrollDelta::Pixels(point(px(translation.x as f32), px(translation.y as f32)));

        let modifiers = state.lock().current_modifiers;
        dispatch_input(
            &state,
            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position,
                delta,
                modifiers,
                touch_phase,
            }),
        );
    }
}

// ---------------------------------------------------------------------------
// Single-finger scroll pan gesture
// ---------------------------------------------------------------------------

extern "C" fn handle_single_finger_pan(this: &Object, _sel: Sel, gesture: *mut Object) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let gesture_state: isize = msg_send![gesture, state];
        let touch_phase = match gesture_state {
            1 => TouchPhase::Started,
            2 => TouchPhase::Moved,
            3 | 4 => TouchPhase::Ended,
            _ => return,
        };

        let translation: CGPoint =
            msg_send![gesture, translationInView: this as *const Object as *mut Object];
        let zero = CGPoint { x: 0.0, y: 0.0 };
        let _: () =
            msg_send![gesture, setTranslation: zero inView: this as *const Object as *mut Object];

        let location: CGPoint =
            msg_send![gesture, locationInView: this as *const Object as *mut Object];
        let position = point(px(location.x as f32), px(location.y as f32));

        let delta = ScrollDelta::Pixels(point(px(translation.x as f32), px(translation.y as f32)));

        let modifiers = state.lock().current_modifiers;
        dispatch_input(
            &state,
            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position,
                delta,
                modifiers,
                touch_phase,
            }),
        );

        // Capture velocity for momentum on end
        if matches!(touch_phase, TouchPhase::Ended) {
            let velocity: CGPoint =
                msg_send![gesture, velocityInView: this as *const Object as *mut Object];
            let vx = velocity.x as f32;
            let vy = velocity.y as f32;
            if vx.abs() > 0.5 || vy.abs() > 0.5 {
                state.lock().scroll_momentum = Some(ScrollMomentum {
                    velocity: point(vx, vy),
                    position,
                    last_time: Instant::now(),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pinch gesture
// ---------------------------------------------------------------------------

extern "C" fn handle_pinch(this: &Object, _sel: Sel, gesture: *mut Object) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let gesture_state: isize = msg_send![gesture, state];
        let touch_phase = match gesture_state {
            1 => TouchPhase::Started,
            2 => TouchPhase::Moved,
            3 | 4 => TouchPhase::Ended,
            _ => return,
        };

        let scale: f64 = msg_send![gesture, scale];
        // Reset to 1.0 so next callback gives incremental scale
        let _: () = msg_send![gesture, setScale: 1.0f64];

        let location: CGPoint =
            msg_send![gesture, locationInView: this as *const Object as *mut Object];
        let center = point(px(location.x as f32), px(location.y as f32));

        let modifiers = state.lock().current_modifiers;
        dispatch_input(
            &state,
            PlatformInput::Pinch(PinchEvent {
                position: center,
                delta: scale as f32 - 1.0,
                modifiers,
                phase: touch_phase,
            }),
        );
    }
}

// ---------------------------------------------------------------------------
// Rotation gesture
// ---------------------------------------------------------------------------

extern "C" fn handle_rotation(this: &Object, _sel: Sel, gesture: *mut Object) {
    let Some(state) = (unsafe { get_window_state(this) }) else {
        return;
    };
    unsafe {
        let gesture_state: isize = msg_send![gesture, state];
        let touch_phase = match gesture_state {
            1 => TouchPhase::Started,
            2 => TouchPhase::Moved,
            3 | 4 => TouchPhase::Ended,
            _ => return,
        };

        let rotation: f64 = msg_send![gesture, rotation];
        // Reset to 0 so next callback gives incremental rotation
        let _: () = msg_send![gesture, setRotation: 0.0f64];

        let location: CGPoint =
            msg_send![gesture, locationInView: this as *const Object as *mut Object];
        let center = point(px(location.x as f32), px(location.y as f32));

        let modifiers = state.lock().current_modifiers;
        dispatch_input(
            &state,
            PlatformInput::Rotation(RotationEvent {
                center,
                rotation: rotation as f32,
                modifiers,
                touch_phase,
            }),
        );
    }
}

// ---------------------------------------------------------------------------
// Safe area insets change
// ---------------------------------------------------------------------------

extern "C" fn handle_safe_area_insets_change(this: &Object, _sel: Sel) {
    unsafe {
        let superclass = class!(UIView);
        let _: () = msg_send![super(this, superclass), safeAreaInsetsDidChange];

        let Some(state) = get_window_state(this) else {
            return;
        };

        let insets: UIEdgeInsets = msg_send![this, safeAreaInsets];

        let mut lock = state.lock();
        lock.safe_area_insets = insets;

        // Fire on_resize so the window re-evaluates safe_area_insets() and redraws.
        // We pass the same size — the viewport size hasn't changed, but the
        // inset-aware layout must be recomputed.
        let current_size = lock.bounds.size;
        let scale_factor = lock.scale_factor;
        if let Some(mut callback) = lock.on_resize.take() {
            drop(lock);
            callback(current_size, scale_factor);
            state.lock().on_resize = Some(callback);
        }
    }
}

// ---------------------------------------------------------------------------
// GPUIGestureDelegate — allows simultaneous gesture recognition
// ---------------------------------------------------------------------------
