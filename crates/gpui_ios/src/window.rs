use super::*;

pub(crate) struct IosWindowState {
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) display: Rc<dyn PlatformDisplay>,
    pub(crate) scale_factor: f32,
    pub(crate) ui_window: *mut Object,
    pub(crate) ui_view_controller: *mut Object,
    pub(crate) ui_view: *mut Object,
    // Drag-and-drop integration for external file drops.
    pub(crate) drop_interaction: *mut Object,
    pub(crate) drop_delegate: *mut Object,
    pub(crate) renderer: MetalRenderer,
    // CADisplayLink driving the frame loop
    pub(crate) display_link: *mut Object,
    pub(crate) display_link_target: *mut Object,
    pub(crate) display_link_callback_ptr: *mut c_void,
    // Touch tracking — primary finger only
    pub(crate) tracked_touch: Option<*mut Object>,
    pub(crate) last_touch_position: Option<Point<Pixels>>,
    // Live modifier state from hardware keyboard presses
    pub(crate) current_modifiers: Modifiers,
    // Scroll momentum after single-finger pan ends
    pub(crate) scroll_momentum: Option<ScrollMomentum>,
    // Safe area insets
    pub(crate) safe_area_insets: UIEdgeInsets,
    // Background appearance
    pub(crate) background_appearance: WindowBackgroundAppearance,
    pub(crate) blur_view: *mut Object,
    // Gesture delegate for simultaneous recognition
    pub(crate) gesture_delegate: *mut Object,
    // Scene lifecycle
    pub(crate) is_active: bool,
    pub(crate) scene_observer: *mut Object,
    // Callbacks
    pub(crate) should_close: Option<Box<dyn FnMut() -> bool>>,
    pub(crate) request_frame: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    pub(crate) on_input: Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>,
    pub(crate) on_active_change: Option<Box<dyn FnMut(bool)>>,
    pub(crate) on_hover_change: Option<Box<dyn FnMut(bool)>>,
    pub(crate) on_resize: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    pub(crate) on_moved: Option<Box<dyn FnMut()>>,
    pub(crate) on_close: Option<Box<dyn FnOnce()>>,
    pub(crate) on_hit_test_window_control: Option<Box<dyn FnMut() -> Option<WindowControlArea>>>,
    pub(crate) on_appearance_change: Option<Box<dyn FnMut()>>,
    pub(crate) title: String,
    pub(crate) input_session: Option<IosTextInputSession>,
}

pub(crate) struct IosWindow(Rc<Mutex<IosWindowState>>);

impl IosWindow {
    pub(crate) fn root_view_controller(&self) -> *mut Object {
        self.0.lock().ui_view_controller
    }

    pub(crate) fn new(
        _handle: AnyWindowHandle,
        options: WindowParams,
        display: Rc<dyn PlatformDisplay>,
    ) -> Self {
        log::debug!("creating iOS window");
        let (
            ui_window,
            ui_view_controller,
            ui_view,
            drop_interaction,
            drop_delegate,
            gesture_delegate,
            bounds,
            scale_factor,
        ) = unsafe {
            let screen: *mut Object = msg_send![class!(UIScreen), mainScreen];
            let screen_bounds: CGRect = msg_send![screen, bounds];
            let scale: f64 = msg_send![screen, scale];

            // On iOS 13+, UIWindow must be associated with a UIWindowScene.
            let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
            let scenes: *mut Object = msg_send![app, connectedScenes];
            let all_scenes: *mut Object = msg_send![scenes, allObjects];
            let scene_count: usize = msg_send![all_scenes, count];
            let ui_window: *mut Object = if scene_count > 0 {
                let scene: *mut Object = msg_send![all_scenes, objectAtIndex: 0usize];
                log::info!("creating UIWindow with UIWindowScene");
                let w: *mut Object = msg_send![class!(UIWindow), alloc];
                msg_send![w, initWithWindowScene: scene]
            } else {
                log::warn!("no UIWindowScene found, falling back to initWithFrame:");
                let w: *mut Object = msg_send![class!(UIWindow), alloc];
                msg_send![w, initWithFrame: screen_bounds]
            };

            let ui_view_controller: *mut Object = msg_send![class!(UIViewController), new];

            let ui_view: *mut Object = msg_send![GPUI_VIEW_CLASS, alloc];
            let ui_view: *mut Object = msg_send![ui_view, initWithFrame: screen_bounds];

            // Enable multi-touch for all gesture recognizers
            let _: () = msg_send![ui_view, setMultipleTouchEnabled: YES];

            // Create gesture delegate for simultaneous recognition
            let gesture_delegate: *mut Object = msg_send![GPUI_GESTURE_DELEGATE_CLASS, new];

            // Two-finger pan gesture for scroll
            let pan: *mut Object = msg_send![class!(UIPanGestureRecognizer), alloc];
            let pan: *mut Object =
                msg_send![pan, initWithTarget: ui_view action: sel!(handleScrollPan:)];
            let _: () = msg_send![pan, setMinimumNumberOfTouches: 2usize];
            let _: () = msg_send![pan, setMaximumNumberOfTouches: 2usize];
            let _: () = msg_send![pan, setDelegate: gesture_delegate];
            let _: () = msg_send![ui_view, addGestureRecognizer: pan];

            // Single-finger pan gesture for scroll
            let single_pan: *mut Object = msg_send![class!(UIPanGestureRecognizer), alloc];
            let single_pan: *mut Object = msg_send![single_pan,
                    initWithTarget: ui_view action: sel!(handleSingleFingerPan:)];
            let _: () = msg_send![single_pan, setMinimumNumberOfTouches: 1usize];
            let _: () = msg_send![single_pan, setMaximumNumberOfTouches: 1usize];
            let _: () = msg_send![ui_view, addGestureRecognizer: single_pan];

            // Pinch gesture
            let pinch: *mut Object = msg_send![class!(UIPinchGestureRecognizer), alloc];
            let pinch: *mut Object =
                msg_send![pinch, initWithTarget: ui_view action: sel!(handlePinch:)];
            let _: () = msg_send![pinch, setDelegate: gesture_delegate];
            let _: () = msg_send![ui_view, addGestureRecognizer: pinch];

            // Rotation gesture
            let rotation: *mut Object = msg_send![class!(UIRotationGestureRecognizer), alloc];
            let rotation: *mut Object =
                msg_send![rotation, initWithTarget: ui_view action: sel!(handleRotation:)];
            let _: () = msg_send![rotation, setDelegate: gesture_delegate];
            let _: () = msg_send![ui_view, addGestureRecognizer: rotation];

            // iPadOS hover gesture (pointer/trackpad/Apple Pencil hover)
            let hover: *mut Object = msg_send![class!(UIHoverGestureRecognizer), alloc];
            let hover: *mut Object =
                msg_send![hover, initWithTarget: ui_view action: sel!(handleHover:)];
            let _: () = msg_send![ui_view, addGestureRecognizer: hover];

            // Long-press gesture for simulated right-click (context menu)
            let long_press: *mut Object = msg_send![class!(UILongPressGestureRecognizer), alloc];
            let long_press: *mut Object =
                msg_send![long_press, initWithTarget: ui_view action: sel!(handleLongPress:)];
            let _: () = msg_send![long_press, setDelegate: gesture_delegate];
            let _: () = msg_send![ui_view, addGestureRecognizer: long_press];

            // Native file drag/drop for external files (UIDropInteraction).
            let drop_delegate: *mut Object = msg_send![GPUI_DROP_DELEGATE_CLASS, new];
            let drop_interaction: *mut Object = msg_send![class!(UIDropInteraction), alloc];
            let drop_interaction: *mut Object =
                msg_send![drop_interaction, initWithDelegate: drop_delegate];
            let _: () = msg_send![ui_view, addInteraction: drop_interaction];

            let _: () = msg_send![ui_view_controller, setView: ui_view];
            let _: () = msg_send![ui_window, setRootViewController: ui_view_controller];
            let _: () = msg_send![ui_window, makeKeyAndVisible];
            let _: BOOL = msg_send![ui_view, becomeFirstResponder];

            let bounds = Bounds::new(
                point(px(0.0), px(0.0)),
                size(
                    px(screen_bounds.size.width as f32),
                    px(screen_bounds.size.height as f32),
                ),
            );
            (
                ui_window,
                ui_view_controller,
                ui_view,
                drop_interaction,
                drop_delegate,
                gesture_delegate,
                bounds,
                scale as f32,
            )
        };

        // Create the Metal renderer. The view's own layer is already a CAMetalLayer
        // (via the layerClass override), so we attach the renderer to it directly.
        let instance_buffer_pool = Arc::new(Mutex::new(InstanceBufferPool::default()));
        let mut renderer = MetalRenderer::new(instance_buffer_pool, false);

        unsafe {
            // The view's layer IS the CAMetalLayer (via layerClass override).
            // Replace the renderer's internal layer with the view's own layer
            // so drawing goes directly to it — no sublayer needed.
            let view_layer: *mut Object = msg_send![ui_view, layer];
            let view_metal_layer = MetalLayer::from_ptr(view_layer as *mut CAMetalLayer);
            // from_ptr creates an owning wrapper; retain so the view keeps its layer alive
            let _: () = msg_send![view_layer, retain];
            renderer.replace_layer(view_metal_layer);
            let _: () = msg_send![view_layer, setContentsScale: scale_factor as f64];
        }

        let drawable_size = bounds.size.to_device_pixels(scale_factor);
        renderer.update_drawable_size(drawable_size);

        log::info!(
            "iOS window created ({}x{} @{}x)",
            bounds.size.width.to_f64(),
            bounds.size.height.to_f64(),
            scale_factor,
        );

        let window = Self(Rc::new(Mutex::new(IosWindowState {
            bounds: if options.bounds.size.width > Pixels::ZERO
                && options.bounds.size.height > Pixels::ZERO
            {
                options.bounds
            } else {
                bounds
            },
            display,
            scale_factor,
            ui_window,
            ui_view_controller,
            ui_view,
            drop_interaction,
            drop_delegate,
            renderer,
            display_link: std::ptr::null_mut(),
            display_link_target: std::ptr::null_mut(),
            display_link_callback_ptr: std::ptr::null_mut(),
            tracked_touch: None,
            last_touch_position: None,
            current_modifiers: Modifiers::default(),
            scroll_momentum: None,
            safe_area_insets: UIEdgeInsets::default(),
            background_appearance: WindowBackgroundAppearance::Opaque,
            blur_view: std::ptr::null_mut(),
            gesture_delegate,
            is_active: true,
            scene_observer: std::ptr::null_mut(),
            should_close: None,
            request_frame: None,
            on_input: None,
            on_active_change: None,
            on_hover_change: None,
            on_resize: None,
            on_moved: None,
            on_close: None,
            on_hit_test_window_control: None,
            on_appearance_change: None,
            title: String::new(),
            input_session: None,
        })));

        // Set the window state ivar on the GPUIView so touch handlers can
        // access it.
        unsafe {
            let view_state_ptr = Rc::into_raw(window.0.clone()) as *mut c_void;
            (*ui_view).set_ivar::<*mut c_void>(WINDOW_STATE_IVAR, view_state_ptr);
            let drop_state_ptr = Rc::into_raw(window.0.clone()) as *mut c_void;
            (*drop_delegate).set_ivar::<*mut c_void>(WINDOW_STATE_IVAR, drop_state_ptr);
        }

        // Register for UIScene lifecycle notifications
        window.register_scene_notifications();

        window
    }

    pub(crate) fn register_scene_notifications(&self) {
        unsafe {
            let observer: *mut Object = msg_send![GPUI_SCENE_OBSERVER_CLASS, new];
            let state_ptr = Rc::into_raw(self.0.clone()) as *mut c_void;
            (*observer).set_ivar::<*mut c_void>(WINDOW_STATE_IVAR, state_ptr);

            let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];

            let did_activate: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: UISCENE_DID_ACTIVATE.as_ptr()];
            let _: () = msg_send![center, addObserver: observer
                selector: sel!(sceneDidActivate:)
                name: did_activate
                object: std::ptr::null::<Object>()];

            let will_deactivate: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: UISCENE_WILL_DEACTIVATE.as_ptr()];
            let _: () = msg_send![center, addObserver: observer
                selector: sel!(sceneWillDeactivate:)
                name: will_deactivate
                object: std::ptr::null::<Object>()];

            let did_enter_bg: *mut Object = msg_send![class!(NSString), stringWithUTF8String: UISCENE_DID_ENTER_BACKGROUND.as_ptr()];
            let _: () = msg_send![center, addObserver: observer
                selector: sel!(sceneDidEnterBackground:)
                name: did_enter_bg
                object: std::ptr::null::<Object>()];

            let will_enter_fg: *mut Object = msg_send![class!(NSString), stringWithUTF8String: UISCENE_WILL_ENTER_FOREGROUND.as_ptr()];
            let _: () = msg_send![center, addObserver: observer
                selector: sel!(sceneWillEnterForeground:)
                name: will_enter_fg
                object: std::ptr::null::<Object>()];

            self.0.lock().scene_observer = observer;
        }
    }
}

impl Drop for IosWindow {
    fn drop(&mut self) {
        log::info!("iOS window destroyed");
        unsafe {
            let mut state = self.0.lock();

            // Remove scene notification observer
            if !state.scene_observer.is_null() {
                let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];
                let _: () = msg_send![center, removeObserver: state.scene_observer];

                // Release the Rc held by the observer's ivar
                let ptr: *mut c_void = *(*state.scene_observer).get_ivar(WINDOW_STATE_IVAR);
                if !ptr.is_null() {
                    let _ = Rc::from_raw(ptr as *const Mutex<IosWindowState>);
                }
                let _: () = msg_send![state.scene_observer, release];
                state.scene_observer = std::ptr::null_mut();
            }

            // Release the Rc held by the GPUIView's ivar
            if !state.ui_view.is_null() {
                let ptr: *mut c_void = *(*state.ui_view).get_ivar(WINDOW_STATE_IVAR);
                if !ptr.is_null() {
                    let _ = Rc::from_raw(ptr as *const Mutex<IosWindowState>);
                    (*state.ui_view)
                        .set_ivar::<*mut c_void>(WINDOW_STATE_IVAR, std::ptr::null_mut());
                }
            }
            if !state.drop_delegate.is_null() {
                let ptr: *mut c_void = *(*state.drop_delegate).get_ivar(WINDOW_STATE_IVAR);
                if !ptr.is_null() {
                    let _ = Rc::from_raw(ptr as *const Mutex<IosWindowState>);
                    (*state.drop_delegate)
                        .set_ivar::<*mut c_void>(WINDOW_STATE_IVAR, std::ptr::null_mut());
                }
            }

            // Invalidate the CADisplayLink (removes it from the run loop).
            if !state.display_link.is_null() {
                let _: () = msg_send![state.display_link, invalidate];
                state.display_link = std::ptr::null_mut();
            }
            if !state.display_link_target.is_null() {
                let _: () = msg_send![state.display_link_target, release];
                state.display_link_target = std::ptr::null_mut();
            }
            // Free the leaked callback closure.
            if !state.display_link_callback_ptr.is_null() {
                let _ = Box::from_raw(state.display_link_callback_ptr as *mut Box<dyn Fn()>);
                state.display_link_callback_ptr = std::ptr::null_mut();
            }

            // Release blur view if present
            if !state.blur_view.is_null() {
                let _: () = msg_send![state.blur_view, removeFromSuperview];
                let _: () = msg_send![state.blur_view, release];
                state.blur_view = std::ptr::null_mut();
            }

            state.input_session.take();

            if !state.drop_interaction.is_null() {
                let _: () = msg_send![state.drop_interaction, release];
                state.drop_interaction = std::ptr::null_mut();
            }
            if !state.drop_delegate.is_null() {
                let _: () = msg_send![state.drop_delegate, release];
                state.drop_delegate = std::ptr::null_mut();
            }

            // Release gesture delegate
            if !state.gesture_delegate.is_null() {
                let _: () = msg_send![state.gesture_delegate, release];
                state.gesture_delegate = std::ptr::null_mut();
            }

            if !state.ui_view.is_null() {
                let _: () = msg_send![state.ui_view, release];
                state.ui_view = std::ptr::null_mut();
            }
            if !state.ui_view_controller.is_null() {
                let _: () = msg_send![state.ui_view_controller, release];
                state.ui_view_controller = std::ptr::null_mut();
            }
            if !state.ui_window.is_null() {
                let _: () = msg_send![state.ui_window, release];
                state.ui_window = std::ptr::null_mut();
            }
            if let Some(callback) = state.on_close.take() {
                callback();
            }
        }
    }
}

impl HasWindowHandle for IosWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let state = self.0.lock();
        let ui_view =
            NonNull::new(state.ui_view.cast::<c_void>()).ok_or(HandleError::Unavailable)?;
        let mut handle = UiKitWindowHandle::new(ui_view);
        handle.ui_view_controller = NonNull::new(state.ui_view_controller.cast::<c_void>());
        unsafe { Ok(WindowHandle::borrow_raw(handle.into())) }
    }
}

impl HasDisplayHandle for IosWindow {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::uikit())
    }
}

impl PlatformWindow for IosWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.lock().bounds
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Windowed(self.bounds())
    }

    fn content_size(&self) -> Size<Pixels> {
        self.bounds().size
    }

    fn safe_area_insets(&self) -> Edges<Pixels> {
        let insets = self.0.lock().safe_area_insets;
        Edges {
            top: px(insets.top as f32),
            right: px(insets.right as f32),
            bottom: px(insets.bottom as f32),
            left: px(insets.left as f32),
        }
    }

    fn resize(&mut self, size: Size<Pixels>) {
        // iOS manages view layout via UIKit; this just updates cached state
        // as a fallback for callers that set size programmatically.
        log::debug!("resize({:?}) — iOS manages layout via UIKit", size);
        self.0.lock().bounds.size = size;
    }

    fn scale_factor(&self) -> f32 {
        self.0.lock().scale_factor
    }

    fn appearance(&self) -> WindowAppearance {
        detect_system_appearance()
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.0.lock().display.clone())
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.0
            .lock()
            .last_touch_position
            .unwrap_or_else(Point::default)
    }

    fn modifiers(&self) -> Modifiers {
        self.0.lock().current_modifiers
    }

    fn capslock(&self) -> Capslock {
        Capslock::default()
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        let mut state = self.0.lock();
        unsafe {
            let ui_view = state.ui_view;
            let session = state
                .input_session
                .get_or_insert_with(|| IosTextInputSession::new(ui_view));
            session.set_input_handler(input_handler);
        }
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.0
            .lock()
            .input_session
            .as_mut()
            .and_then(IosTextInputSession::take_input_handler)
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<oneshot::Receiver<usize>> {
        None
    }

    fn activate(&self) {
        unsafe {
            let ui_window = self.0.lock().ui_window;
            let _: () = msg_send![ui_window, makeKeyAndVisible];
        }
    }

    fn is_active(&self) -> bool {
        self.0.lock().is_active
    }

    fn is_hovered(&self) -> bool {
        false
    }

    fn completed_frame(&self) {
        if let Some(session) = self.0.lock().input_session.as_mut() {
            session.completed_frame();
        }
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.0.lock().background_appearance
    }

    fn set_title(&mut self, title: &str) {
        self.0.lock().title = title.to_string();
    }

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut lock = self.0.lock();
        lock.background_appearance = background_appearance;

        unsafe {
            // Remove existing blur view if present
            if !lock.blur_view.is_null() {
                let _: () = msg_send![lock.blur_view, removeFromSuperview];
                let _: () = msg_send![lock.blur_view, release];
                lock.blur_view = std::ptr::null_mut();
            }

            match background_appearance {
                WindowBackgroundAppearance::Opaque => {
                    // Metal layer opaque
                    let layer: *mut Object = msg_send![lock.ui_view, layer];
                    let _: () = msg_send![layer, setOpaque: YES];
                }
                WindowBackgroundAppearance::Transparent => {
                    let layer: *mut Object = msg_send![lock.ui_view, layer];
                    let _: () = msg_send![layer, setOpaque: NO];
                }
                WindowBackgroundAppearance::Blurred => {
                    let layer: *mut Object = msg_send![lock.ui_view, layer];
                    let _: () = msg_send![layer, setOpaque: NO];

                    // Create UIVisualEffectView with system material blur
                    let effect: *mut Object = msg_send![class!(UIBlurEffect),
                        effectWithStyle: 6isize]; // UIBlurEffectStyleSystemMaterial
                    let blur_view: *mut Object = msg_send![class!(UIVisualEffectView), alloc];
                    let blur_view: *mut Object = msg_send![blur_view, initWithEffect: effect];

                    let bounds: CGRect = msg_send![lock.ui_view, bounds];
                    let _: () = msg_send![blur_view, setFrame: bounds];
                    // Auto-resize with parent
                    let autoresizing: usize = 0x3F; // FlexibleWidth | FlexibleHeight | all margins
                    let _: () = msg_send![blur_view, setAutoresizingMask: autoresizing];

                    // Insert behind the Metal content
                    let _: () = msg_send![lock.ui_view, insertSubview: blur_view atIndex: 0isize];

                    lock.blur_view = blur_view;
                }
                // Windows-only variants — treat as opaque on iOS
                _ => {
                    let layer: *mut Object = msg_send![lock.ui_view, layer];
                    let _: () = msg_send![layer, setOpaque: YES];
                }
            }
        }
    }

    fn minimize(&self) {}

    fn zoom(&self) {}

    fn toggle_fullscreen(&self) {}

    fn is_fullscreen(&self) -> bool {
        false
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.lock().request_frame = Some(callback);

        log::info!("CADisplayLink started");

        let window_state = self.0.clone();
        let first_frame_done = Rc::new(Cell::new(false));
        let first_frame_clone = first_frame_done.clone();

        let step_fn: Box<dyn Fn()> = Box::new(move || {
            // Process scroll momentum
            {
                let mut lock = window_state.lock();
                if let Some(ref mut momentum) = lock.scroll_momentum {
                    let now = Instant::now();
                    let dt_ms = now.duration_since(momentum.last_time).as_millis() as f32;
                    momentum.last_time = now;

                    // Exponential decay: v *= 0.998^dt_ms
                    let decay = 0.998f32.powf(dt_ms);
                    momentum.velocity.x *= decay;
                    momentum.velocity.y *= decay;

                    let vx = momentum.velocity.x;
                    let vy = momentum.velocity.y;
                    let position = momentum.position;
                    let dt_sec = dt_ms / 1000.0;

                    let modifiers = lock.current_modifiers;
                    if vx.abs() < 0.5 && vy.abs() < 0.5 {
                        // Momentum exhausted — send final event
                        lock.scroll_momentum = None;
                        if let Some(mut input_cb) = lock.on_input.take() {
                            drop(lock);
                            input_cb(PlatformInput::ScrollWheel(ScrollWheelEvent {
                                position,
                                delta: ScrollDelta::Pixels(point(px(0.0), px(0.0))),
                                modifiers,
                                touch_phase: TouchPhase::Ended,
                            }));
                            window_state.lock().on_input = Some(input_cb);
                        }
                    } else {
                        let dx = vx * dt_sec;
                        let dy = vy * dt_sec;
                        if let Some(mut input_cb) = lock.on_input.take() {
                            drop(lock);
                            input_cb(PlatformInput::ScrollWheel(ScrollWheelEvent {
                                position,
                                delta: ScrollDelta::Pixels(point(px(dx), px(dy))),
                                modifiers,
                                touch_phase: TouchPhase::Moved,
                            }));
                            window_state.lock().on_input = Some(input_cb);
                        }
                    }
                }
            }

            let mut cb = match window_state.lock().request_frame.take() {
                Some(cb) => cb,
                None => return,
            };

            let mut opts = RequestFrameOptions::default();
            if !first_frame_clone.get() {
                first_frame_clone.set(true);
                log::info!("first frame rendered");
                opts.force_render = true;
            }

            cb(opts);
            window_state.lock().request_frame = Some(cb);
        });

        let boxed_fn = Box::new(step_fn);
        let fn_ptr = Box::into_raw(boxed_fn) as *mut c_void;

        unsafe {
            let target: *mut Object = msg_send![DISPLAY_LINK_TARGET_CLASS, new];
            (*target).set_ivar::<*mut c_void>(CALLBACK_IVAR, fn_ptr);

            let display_link: *mut Object = msg_send![
                class!(CADisplayLink),
                displayLinkWithTarget: target
                selector: sel!(step:)
            ];

            let run_loop: *mut Object = msg_send![class!(NSRunLoop), mainRunLoop];
            let _: () =
                msg_send![display_link, addToRunLoop: run_loop forMode: NSRunLoopCommonModes];

            let mut state = self.0.lock();
            state.display_link = display_link;
            state.display_link_target = target;
            state.display_link_callback_ptr = fn_ptr;
        }
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        self.0.lock().on_input = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.lock().on_active_change = Some(callback);
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.lock().on_hover_change = Some(callback);
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.lock().on_resize = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().on_moved = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.lock().should_close = Some(callback);
    }

    fn on_hit_test_window_control(&self, callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
        self.0.lock().on_hit_test_window_control = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.lock().on_close = Some(callback);
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().on_appearance_change = Some(callback);
    }

    fn draw(&self, scene: &Scene) {
        self.0.lock().renderer.draw(scene);
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.0.lock().renderer.sprite_atlas().clone()
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        false
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        None
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {}

    fn raw_native_view_ptr(&self) -> *mut c_void {
        self.0.lock().ui_view.cast::<c_void>()
    }

}
