use super::*;

pub struct IosPlatform {
    state: Mutex<IosPlatformState>,
}

pub(crate) struct IosPlatformState {
    pub(crate) background_executor: BackgroundExecutor,
    pub(crate) foreground_executor: ForegroundExecutor,
    pub(crate) text_system: Arc<dyn PlatformTextSystem>,
    pub(crate) display: Rc<IosDisplay>,
    pub(crate) active_window: Option<AnyWindowHandle>,
    pub(crate) active_view_controller: *mut Object,
    pub(crate) open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    pub(crate) on_quit: Option<Box<dyn FnMut()>>,
    pub(crate) on_reopen: Option<Box<dyn FnMut()>>,
    pub(crate) on_thermal_state_change: Option<Box<dyn FnMut()>>,
    pub(crate) thermal_observer: *mut Object,
    pub(crate) input_mode_observer: *mut Object,
    pub(crate) app_menu_action: Option<Box<dyn FnMut(&dyn Action)>>,
    pub(crate) will_open_menu: Option<Box<dyn FnMut()>>,
    pub(crate) validate_app_menu: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    pub(crate) document_picker_delegates: Vec<*mut Object>,
}

impl IosPlatform {
    pub fn new(_headless: bool) -> Self {
        log::info!("iOS platform initialized");
        let dispatcher = Arc::new(IosDispatcher::new());
        let background_executor = BackgroundExecutor::new(dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(dispatcher);
        let platform = Self {
            state: Mutex::new(IosPlatformState {
                background_executor,
                foreground_executor,
                text_system: {
                    #[cfg(feature = "font-kit")]
                    {
                        Arc::new(IosTextSystem::new())
                    }
                    #[cfg(not(feature = "font-kit"))]
                    {
                        Arc::new(NoopTextSystem::new())
                    }
                },
                display: Rc::new(IosDisplay::primary()),
                active_window: None,
                active_view_controller: std::ptr::null_mut(),
                open_urls: None,
                on_quit: None,
                on_reopen: None,
                on_thermal_state_change: None,
                thermal_observer: std::ptr::null_mut(),
                input_mode_observer: std::ptr::null_mut(),
                app_menu_action: None,
                will_open_menu: None,
                validate_app_menu: None,
                document_picker_delegates: Vec::new(),
            }),
        };

        // Store a global pointer to the platform state for gpui_ios_handle_open_url
        let state_ptr = &platform.state as *const Mutex<IosPlatformState> as *mut c_void;
        IOS_PLATFORM_STATE_PTR.store(state_ptr, std::sync::atomic::Ordering::Release);

        platform
    }

    fn active_presenting_controller(&self) -> Option<*mut Object> {
        let controller = self.state.lock().active_view_controller;
        if !controller.is_null() {
            return Some(controller);
        }

        // Use the modern UIWindowScene API (iOS 13+) instead of the
        // deprecated UIApplication.keyWindow property.
        unsafe {
            let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
            if app.is_null() {
                return None;
            }
            let scenes: *mut Object = msg_send![app, connectedScenes];
            if scenes.is_null() {
                return None;
            }
            let count: usize = msg_send![scenes, count];
            for i in 0..count {
                let scene: *mut Object = msg_send![scenes, objectAtIndex: i];
                if scene.is_null() {
                    continue;
                }
                // Check if this is a UIWindowScene (responds to `windows`)
                let has_windows: BOOL = msg_send![scene, respondsToSelector: sel!(windows)];
                if has_windows == NO {
                    continue;
                }
                let windows: *mut Object = msg_send![scene, windows];
                if windows.is_null() {
                    continue;
                }
                let win_count: usize = msg_send![windows, count];
                for j in 0..win_count {
                    let window: *mut Object = msg_send![windows, objectAtIndex: j];
                    if window.is_null() {
                        continue;
                    }
                    let is_key: BOOL = msg_send![window, isKeyWindow];
                    if is_key == YES {
                        let root: *mut Object = msg_send![window, rootViewController];
                        if !root.is_null() {
                            return Some(root);
                        }
                    }
                }
            }
            None
        }
    }

    fn create_document_picker_delegate(
        &self,
        sender: PickerResultSender,
        temp_paths: Vec<PathBuf>,
    ) -> *mut Object {
        unsafe {
            let delegate: *mut Object = msg_send![GPUI_DOCUMENT_PICKER_DELEGATE_CLASS, new];
            let context = Box::new(DocumentPickerCallbackContext { sender, temp_paths });
            (*delegate)
                .set_ivar::<*mut c_void>(CALLBACK_IVAR, Box::into_raw(context) as *mut c_void);
            self.state.lock().document_picker_delegates.push(delegate);
            delegate
        }
    }
}

impl Platform for IosPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.state.lock().background_executor.clone()
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.state.lock().foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.state.lock().text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        on_finish_launching();
    }

    fn quit(&self) {
        if let Some(mut callback) = self.state.lock().on_quit.take() {
            callback();
        }
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {}

    fn activate(&self, _ignoring_other_apps: bool) {}

    fn hide(&self) {}

    fn hide_other_apps(&self) {}

    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![self.state.lock().display.clone()]
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.state.lock().display.clone())
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        self.state.lock().active_window
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> Result<Box<dyn PlatformWindow>> {
        let display = self.state.lock().display.clone();
        let window = IosWindow::new(handle, options, display);
        let mut platform_state = self.state.lock();
        platform_state.active_window = Some(handle);
        platform_state.active_view_controller = window.root_view_controller();
        Ok(Box::new(window))
    }

    fn window_appearance(&self) -> WindowAppearance {
        detect_system_appearance()
    }

    fn open_url(&self, url: &str) {
        unsafe {
            let ns_url_string: *mut Object = msg_send![class!(NSString),
                stringWithUTF8String: std::ffi::CString::new(url).unwrap_or_default().as_ptr()
            ];
            let ns_url: *mut Object = msg_send![class!(NSURL), URLWithString: ns_url_string];
            if ns_url.is_null() {
                log::error!("failed to create NSURL from: {}", url);
                return;
            }
            let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
            let options: *mut Object = msg_send![class!(NSDictionary), dictionary];
            let _: () = msg_send![app, openURL: ns_url
                options: options
                completionHandler: std::ptr::null::<c_void>()];
        }
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.state.lock().open_urls = Some(callback);
    }

    fn register_url_scheme(&self, url: &str) -> Task<Result<()>> {
        let scheme = url
            .trim()
            .trim_end_matches("://")
            .split(':')
            .next()
            .unwrap_or(url)
            .to_string();
        Task::ready({
            unsafe {
                let bundle: *mut Object = msg_send![class!(NSBundle), mainBundle];
                if bundle.is_null() {
                    Err(anyhow!(
                        "main bundle unavailable; cannot validate URL scheme"
                    ))
                } else {
                    let info: *mut Object = msg_send![bundle, infoDictionary];
                    if info.is_null() {
                        Err(anyhow!("Info.plist missing; cannot validate URL scheme"))
                    } else {
                        let key = ns_string("CFBundleURLTypes");
                        let url_types: *mut Object = msg_send![info, objectForKey: key];
                        if url_types.is_null() {
                            Err(anyhow!(
                                "URL scheme '{}' is not declared in CFBundleURLTypes",
                                scheme
                            ))
                        } else {
                            let mut found = false;
                            let type_count: usize = msg_send![url_types, count];
                            for i in 0..type_count {
                                let url_type: *mut Object = msg_send![url_types, objectAtIndex: i];
                                if url_type.is_null() {
                                    continue;
                                }
                                let schemes_key = ns_string("CFBundleURLSchemes");
                                let schemes: *mut Object =
                                    msg_send![url_type, objectForKey: schemes_key];
                                if schemes.is_null() {
                                    continue;
                                }
                                let scheme_count: usize = msg_send![schemes, count];
                                for j in 0..scheme_count {
                                    let item: *mut Object = msg_send![schemes, objectAtIndex: j];
                                    if item.is_null() {
                                        continue;
                                    }
                                    let utf8: *const std::os::raw::c_char =
                                        msg_send![item, UTF8String];
                                    if utf8.is_null() {
                                        continue;
                                    }
                                    let declared = std::ffi::CStr::from_ptr(utf8)
                                        .to_string_lossy()
                                        .into_owned();
                                    if declared.eq_ignore_ascii_case(&scheme) {
                                        found = true;
                                        break;
                                    }
                                }
                                if found {
                                    break;
                                }
                            }

                            if found {
                                Ok(())
                            } else {
                                Err(anyhow!(
                                    "URL scheme '{}' is not declared in CFBundleURLTypes",
                                    scheme
                                ))
                            }
                        }
                    }
                }
            }
        })
    }

    fn prompt_for_paths(
        &self,
        options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (tx, rx) = oneshot::channel();

        let Some(presenter) = self.active_presenting_controller() else {
            let _ = tx.send(Err(anyhow!(
                "no active view controller to present document picker"
            )));
            return rx;
        };

        if !options.files && !options.directories {
            let _ = tx.send(Err(anyhow!(
                "invalid path prompt options: at least one of files/directories must be true"
            )));
            return rx;
        }

        // Use the modern UTType-based API (iOS 14+) instead of the deprecated
        // initWithDocumentTypes:inMode: initializer.
        unsafe {
            let content_types: *mut Object = msg_send![class!(NSMutableArray), array];
            if options.files {
                let data_type: *mut Object =
                    msg_send![class!(UTType), typeWithIdentifier: ns_string("public.data")];
                if !data_type.is_null() {
                    let _: () = msg_send![content_types, addObject: data_type];
                }
            }
            if options.directories {
                let folder_type: *mut Object =
                    msg_send![class!(UTType), typeWithIdentifier: ns_string("public.folder")];
                if !folder_type.is_null() {
                    let _: () = msg_send![content_types, addObject: folder_type];
                }
            }

            let picker: *mut Object = msg_send![class!(UIDocumentPickerViewController), alloc];
            let picker: *mut Object = msg_send![picker, initForOpeningContentTypes: content_types];
            if picker.is_null() {
                let _ = tx.send(Err(anyhow!(
                    "failed to create UIDocumentPickerViewController"
                )));
                return rx;
            }

            let _: () = msg_send![picker, setAllowsMultipleSelection: if options.multiple { YES } else { NO }];
            let delegate =
                self.create_document_picker_delegate(PickerResultSender::Multiple(tx), Vec::new());
            let _: () = msg_send![picker, setDelegate: delegate];
            let _: () = msg_send![presenter,
                presentViewController: picker
                animated: YES
                completion: std::ptr::null::<c_void>()
            ];
        }
        rx
    }

    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (tx, rx) = oneshot::channel();

        let Some(presenter) = self.active_presenting_controller() else {
            let _ = tx.send(Err(anyhow!(
                "no active view controller to present document picker"
            )));
            return rx;
        };

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let suggested = suggested_name.unwrap_or("untitled.txt");
        let source_path = std::env::temp_dir().join(format!("gpui-export-{now_ms}-{suggested}"));

        if let Err(error) = std::fs::write(&source_path, []) {
            let _ = tx.send(Err(anyhow!(
                "failed to create temporary export file '{}': {error}",
                source_path.display()
            )));
            return rx;
        }

        unsafe {
            let path_string = ns_string(source_path.to_string_lossy().as_ref());
            let source_url: *mut Object =
                msg_send![class!(NSURL), fileURLWithPath: path_string isDirectory: NO];
            if source_url.is_null() {
                let _ = std::fs::remove_file(&source_path);
                let _ = tx.send(Err(anyhow!(
                    "failed to create file URL for '{}'",
                    source_path.display()
                )));
                return rx;
            }

            // Use the modern initForExportingURLs: API (iOS 14+) instead of
            // the deprecated initWithURL:inMode: initializer.
            let urls: *mut Object = msg_send![class!(NSArray), arrayWithObject: source_url];
            let picker: *mut Object = msg_send![class!(UIDocumentPickerViewController), alloc];
            let picker: *mut Object = msg_send![picker, initForExportingURLs: urls];
            if picker.is_null() {
                let _ = std::fs::remove_file(&source_path);
                let _ = tx.send(Err(anyhow!(
                    "failed to create UIDocumentPickerViewController for export"
                )));
                return rx;
            }

            let delegate = self
                .create_document_picker_delegate(PickerResultSender::Single(tx), vec![source_path]);
            let _: () = msg_send![picker, setDelegate: delegate];
            let _: () = msg_send![presenter,
                presentViewController: picker
                animated: YES
                completion: std::ptr::null::<c_void>()
            ];
        }
        rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        true
    }

    fn reveal_path(&self, _path: &Path) {}

    fn open_with_system(&self, _path: &Path) {}

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.state.lock().on_quit = Some(callback);
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.state.lock().on_reopen = Some(callback);
    }

    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}

    fn get_menus(&self) -> Option<Vec<OwnedMenu>> {
        None
    }

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.state.lock().app_menu_action = Some(callback);
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.state.lock().will_open_menu = Some(callback);
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.state.lock().validate_app_menu = Some(callback);
    }

    fn thermal_state(&self) -> ThermalState {
        unsafe {
            let process_info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
            let state: isize = msg_send![process_info, thermalState];
            // NSProcessInfoThermalState: 0=Nominal, 1=Fair, 2=Serious, 3=Critical
            match state {
                1 => ThermalState::Fair,
                2 => ThermalState::Serious,
                3 => ThermalState::Critical,
                _ => ThermalState::Nominal,
            }
        }
    }

    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        let mut platform_state = self.state.lock();

        // Remove previous observer if any
        unsafe {
            if !platform_state.thermal_observer.is_null() {
                let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];
                let _: () = msg_send![center, removeObserver: platform_state.thermal_observer];
                // Free the old callback stored in the ivar
                let old_ptr: *mut c_void =
                    *(*platform_state.thermal_observer).get_ivar(CALLBACK_IVAR);
                if !old_ptr.is_null() {
                    let _ = Box::from_raw(old_ptr as *mut Box<dyn FnMut()>);
                }
                let _: () = msg_send![platform_state.thermal_observer, release];
                platform_state.thermal_observer = std::ptr::null_mut();
            }
        }

        platform_state.on_thermal_state_change = Some(callback);

        // Heap-allocate the callback pointer so it has a stable address
        // independent of the mutex lock. This avoids a dangling pointer
        // when the lock is dropped.
        let callback_box: Box<Box<dyn FnMut()>> =
            Box::new(platform_state.on_thermal_state_change.take().unwrap());
        let callback_ptr = Box::into_raw(callback_box) as *mut c_void;

        // Store a reference back so we can call it from GPUI too
        // (the Box is now owned by the ivar, we reconstruct a reference)
        unsafe {
            let observer: *mut Object = msg_send![GPUI_THERMAL_OBSERVER_CLASS, new];
            (*observer).set_ivar::<*mut c_void>(CALLBACK_IVAR, callback_ptr);

            let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];
            let name_bytes = b"NSProcessInfoThermalStateDidChangeNotification\0";
            let name: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: name_bytes.as_ptr()];
            let process_info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
            let _: () = msg_send![center,
                addObserver: observer
                selector: sel!(thermalStateChanged:)
                name: name
                object: process_info
            ];

            platform_state.thermal_observer = observer;
        }
    }

    fn app_path(&self) -> Result<PathBuf> {
        std::env::current_exe().map_err(Into::into)
    }

    fn path_for_auxiliary_executable(&self, _name: &str) -> Result<PathBuf> {
        Err(anyhow!(
            "auxiliary executable lookup is not implemented on iOS"
        ))
    }

    fn set_cursor_style(&self, _style: CursorStyle) {}

    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        unsafe {
            let pasteboard: *mut Object = msg_send![class!(UIPasteboard), generalPasteboard];
            let metadata_type = ns_string("dev.gpui.clipboard-metadata");

            let has_strings: BOOL = msg_send![pasteboard, hasStrings];
            if has_strings == YES {
                let ns_string: *mut Object = msg_send![pasteboard, string];
                if !ns_string.is_null() {
                    let utf8: *const std::os::raw::c_char = msg_send![ns_string, UTF8String];
                    if !utf8.is_null() {
                        let text = std::ffi::CStr::from_ptr(utf8)
                            .to_string_lossy()
                            .into_owned();

                        let meta_data: *mut Object =
                            msg_send![pasteboard, dataForPasteboardType: metadata_type];
                        if !meta_data.is_null() {
                            let meta_string: *mut Object = msg_send![class!(NSString), alloc];
                            let encoding: usize = 4; // NSUTF8StringEncoding
                            let meta_string: *mut Object = msg_send![meta_string,
                                initWithData: meta_data
                                encoding: encoding
                            ];
                            if !meta_string.is_null() {
                                let meta_utf8: *const std::os::raw::c_char =
                                    msg_send![meta_string, UTF8String];
                                if !meta_utf8.is_null() {
                                    let metadata = std::ffi::CStr::from_ptr(meta_utf8)
                                        .to_string_lossy()
                                        .into_owned();
                                    return Some(ClipboardItem::new_string_with_metadata(
                                        text, metadata,
                                    ));
                                }
                            }
                        }
                        return Some(ClipboardItem::new_string(text));
                    }
                }
            }

            let has_images: BOOL = msg_send![pasteboard, hasImages];
            if has_images == YES {
                let image_obj: *mut Object = msg_send![pasteboard, image];
                if !image_obj.is_null() {
                    let image_data: *mut Object = msg_send![image_obj, pngData];
                    if !image_data.is_null() {
                        let length: usize = msg_send![image_data, length];
                        let bytes: *const u8 = msg_send![image_data, bytes];
                        if !bytes.is_null() && length > 0 {
                            let bytes = std::slice::from_raw_parts(bytes, length).to_vec();
                            let image = Image {
                                format: ImageFormat::Png,
                                id: hash(&bytes),
                                bytes,
                            };
                            return Some(ClipboardItem::new_image(&image));
                        }
                    }
                }
            }

            None
        }
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        unsafe {
            let pasteboard: *mut Object = msg_send![class!(UIPasteboard), generalPasteboard];
            if let [ClipboardEntry::Image(image)] = item.entries() {
                let ns_data: *mut Object = msg_send![class!(NSData),
                    dataWithBytes: image.bytes().as_ptr()
                    length: image.bytes().len() as u64
                ];
                if !ns_data.is_null() {
                    let ui_image: *mut Object = msg_send![class!(UIImage), imageWithData: ns_data];
                    if !ui_image.is_null() {
                        let _: () = msg_send![pasteboard, setImage: ui_image];
                        return;
                    }
                }
            }

            if let Some(text) = item.text() {
                let ns_text = ns_string(&text);
                let _: () = msg_send![pasteboard, setString: ns_text];

                if let Some(metadata) = item.metadata() {
                    let metadata_type = ns_string("dev.gpui.clipboard-metadata");
                    let metadata_ns = ns_string(metadata.as_str());
                    let encoding: usize = 4; // NSUTF8StringEncoding
                    let metadata_data: *mut Object =
                        msg_send![metadata_ns, dataUsingEncoding: encoding];
                    if !metadata_data.is_null() {
                        let _: () = msg_send![pasteboard,
                            setData: metadata_data
                            forPasteboardType: metadata_type
                        ];
                    }
                }
            }
        }
    }

    fn write_credentials(&self, url: &str, username: &str, password: &[u8]) -> Task<Result<()>> {
        let url = url.to_string();
        let username = username.to_string();
        let password = password.to_vec();
        self.state.lock().background_executor.spawn(async move {
            unsafe {
                use ios_security::*;

                let url = CFString::from(url.as_str());
                let username = CFString::from(username.as_str());
                let password = CFData::from_buffer(&password);

                let mut query_attrs = CFMutableDictionary::with_capacity(2);
                query_attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                query_attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());

                let mut attrs = CFMutableDictionary::with_capacity(4);
                attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());
                attrs.set(kSecAttrAccount as *const _, username.as_CFTypeRef());
                attrs.set(kSecValueData as *const _, password.as_CFTypeRef());

                let mut verb = "updating";
                let mut status = SecItemUpdate(
                    query_attrs.as_concrete_TypeRef(),
                    attrs.as_concrete_TypeRef(),
                );

                if status == errSecItemNotFound {
                    verb = "creating";
                    status = SecItemAdd(attrs.as_concrete_TypeRef(), ptr::null_mut());
                }
                anyhow::ensure!(status == errSecSuccess, "{verb} password failed: {status}");
            }
            Ok(())
        })
    }

    fn read_credentials(&self, url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        let url = url.to_string();
        self.state.lock().background_executor.spawn(async move {
            let url = CFString::from(url.as_str());
            let cf_true = CFBoolean::true_value().as_CFTypeRef();

            unsafe {
                use ios_security::*;

                let mut attrs = CFMutableDictionary::with_capacity(4);
                attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());
                attrs.set(kSecReturnAttributes as *const _, cf_true);
                attrs.set(kSecReturnData as *const _, cf_true);

                let mut result = CFTypeRef::from(ptr::null());
                let status = SecItemCopyMatching(attrs.as_concrete_TypeRef(), &mut result);
                match status {
                    ios_security::errSecSuccess => {}
                    ios_security::errSecItemNotFound | ios_security::errSecUserCanceled => {
                        return Ok(None);
                    }
                    _ => anyhow::bail!("reading password failed: {status}"),
                }

                let result = CFType::wrap_under_create_rule(result)
                    .downcast::<CFDictionary>()
                    .ok_or_else(|| anyhow!("keychain item was not a dictionary"))?;
                let username = result
                    .find(kSecAttrAccount as *const _)
                    .ok_or_else(|| anyhow!("account was missing from keychain item"))?;
                let username = CFType::wrap_under_get_rule(*username)
                    .downcast::<CFString>()
                    .ok_or_else(|| anyhow!("account was not a string"))?;
                let password = result
                    .find(kSecValueData as *const _)
                    .ok_or_else(|| anyhow!("password was missing from keychain item"))?;
                let password = CFType::wrap_under_get_rule(*password)
                    .downcast::<CFData>()
                    .ok_or_else(|| anyhow!("password was not data"))?;

                Ok(Some((username.to_string(), password.bytes().to_vec())))
            }
        })
    }

    fn delete_credentials(&self, url: &str) -> Task<Result<()>> {
        let url = url.to_string();
        self.state.lock().background_executor.spawn(async move {
            unsafe {
                use ios_security::*;

                let url = CFString::from(url.as_str());
                let mut query_attrs = CFMutableDictionary::with_capacity(2);
                query_attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
                query_attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());

                let status = SecItemDelete(query_attrs.as_concrete_TypeRef());
                anyhow::ensure!(status == errSecSuccess, "delete password failed: {status}");
            }
            Ok(())
        })
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(IosKeyboardLayout::current())
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        let layout = IosKeyboardLayout::current();
        Rc::new(IosKeyboardMapper::new(layout.id()))
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        let mut platform_state = self.state.lock();

        // Remove previous observer if any
        unsafe {
            if !platform_state.input_mode_observer.is_null() {
                let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];
                let _: () = msg_send![center, removeObserver: platform_state.input_mode_observer];
                let old_ptr: *mut c_void =
                    *(*platform_state.input_mode_observer).get_ivar(CALLBACK_IVAR);
                if !old_ptr.is_null() {
                    let _ = Box::from_raw(old_ptr as *mut Box<dyn FnMut()>);
                }
                let _: () = msg_send![platform_state.input_mode_observer, release];
                platform_state.input_mode_observer = std::ptr::null_mut();
            }
        }

        // Heap-allocate callback so the pointer is stable
        let callback_box: Box<Box<dyn FnMut()>> = Box::new(callback);
        let callback_ptr = Box::into_raw(callback_box) as *mut c_void;

        // Register for UITextInputCurrentInputModeDidChangeNotification
        unsafe {
            let observer: *mut Object = msg_send![GPUI_INPUT_MODE_OBSERVER_CLASS, new];
            (*observer).set_ivar::<*mut c_void>(CALLBACK_IVAR, callback_ptr);

            let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];
            let name_bytes = b"UITextInputCurrentInputModeDidChangeNotification\0";
            let name: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: name_bytes.as_ptr()];
            let _: () = msg_send![center,
                addObserver: observer
                selector: sel!(inputModeChanged:)
                name: name
                object: std::ptr::null::<Object>()
            ];

            platform_state.input_mode_observer = observer;
        }
    }
}
