use super::*;

// ---------------------------------------------------------------------------
// GPUIDropDelegate — handles external file drag/drop with UIDropInteraction.
// ---------------------------------------------------------------------------

pub(crate) static mut GPUI_DROP_DELEGATE_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_drop_delegate_class() {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("GPUIDropDelegate", superclass)
            .expect("failed to declare GPUIDropDelegate");

        decl.add_ivar::<*mut c_void>(WINDOW_STATE_IVAR);

        decl.add_method(
            sel!(dropInteraction:canHandleSession:),
            drop_can_handle_session
                as extern "C" fn(&Object, Sel, *mut Object, *mut Object) -> BOOL,
        );
        decl.add_method(
            sel!(dropInteraction:sessionDidEnter:),
            drop_session_did_enter as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(dropInteraction:sessionDidUpdate:),
            drop_session_did_update
                as extern "C" fn(&Object, Sel, *mut Object, *mut Object) -> *mut Object,
        );
        decl.add_method(
            sel!(dropInteraction:sessionDidExit:),
            drop_session_did_exit as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(dropInteraction:performDrop:),
            drop_perform_drop as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );

        GPUI_DROP_DELEGATE_CLASS = decl.register();
    }
}

unsafe fn drop_delegate_state(delegate: &Object) -> Option<Rc<Mutex<IosWindowState>>> {
    unsafe {
        let ptr: *mut c_void = *delegate.get_ivar(WINDOW_STATE_IVAR);
        if ptr.is_null() {
            return None;
        }
        let rc = Rc::from_raw(ptr as *const Mutex<IosWindowState>);
        let clone = rc.clone();
        std::mem::forget(rc);
        Some(clone)
    }
}

fn drop_location(state: &Mutex<IosWindowState>, session: *mut Object) -> Point<Pixels> {
    unsafe {
        let ui_view = state.lock().ui_view;
        let location: CGPoint = msg_send![session, locationInView: ui_view];
        point(px(location.x as f32), px(location.y as f32))
    }
}

extern "C" fn drop_can_handle_session(
    _this: &Object,
    _sel: Sel,
    _interaction: *mut Object,
    session: *mut Object,
) -> BOOL {
    unsafe {
        let types: *mut Object = msg_send![class!(NSMutableArray), array];
        let _: () = msg_send![types, addObject: ns_string("public.file-url")];
        let _: () = msg_send![types, addObject: ns_string("public.url")];
        let has_types: BOOL = msg_send![session, hasItemsConformingToTypeIdentifiers: types];
        let can_load_urls: BOOL = msg_send![session, canLoadObjectsOfClass: class!(NSURL)];
        if has_types == YES || can_load_urls == YES {
            YES
        } else {
            NO
        }
    }
}

extern "C" fn drop_session_did_enter(
    this: &Object,
    _sel: Sel,
    _interaction: *mut Object,
    session: *mut Object,
) {
    let Some(state) = (unsafe { drop_delegate_state(this) }) else {
        return;
    };
    let position = drop_location(&state, session);
    dispatch_input(
        &state,
        PlatformInput::FileDrop(FileDropEvent::Pending { position }),
    );
}

extern "C" fn drop_session_did_update(
    this: &Object,
    _sel: Sel,
    _interaction: *mut Object,
    session: *mut Object,
) -> *mut Object {
    let Some(state) = (unsafe { drop_delegate_state(this) }) else {
        return std::ptr::null_mut();
    };
    let position = drop_location(&state, session);
    dispatch_input(
        &state,
        PlatformInput::FileDrop(FileDropEvent::Pending { position }),
    );

    unsafe {
        let proposal: *mut Object = msg_send![class!(UIDropProposal), alloc];
        msg_send![proposal, initWithDropOperation: 2usize] // UIDropOperationCopy
    }
}

extern "C" fn drop_session_did_exit(
    this: &Object,
    _sel: Sel,
    _interaction: *mut Object,
    _session: *mut Object,
) {
    let Some(state) = (unsafe { drop_delegate_state(this) }) else {
        return;
    };
    dispatch_input(&state, PlatformInput::FileDrop(FileDropEvent::Exited));
}

extern "C" fn drop_perform_drop(
    this: &Object,
    _sel: Sel,
    _interaction: *mut Object,
    session: *mut Object,
) {
    let Some(state) = (unsafe { drop_delegate_state(this) }) else {
        return;
    };
    let position = drop_location(&state, session);

    // Use a Mutex<Option<…>> wrapper so the Rc is always reclaimed when the
    // block is dropped, even if UIKit never invokes the completion handler.
    let state_holder = Arc::new(std::sync::Mutex::new(Some(state.clone())));

    let block = ConcreteBlock::new(move |objects: *mut Object| {
        let Some(state) = state_holder.lock().unwrap().take() else {
            return;
        };

        let mut paths = Vec::<PathBuf>::new();

        unsafe {
            if !objects.is_null() {
                let count: usize = msg_send![objects, count];
                for index in 0..count {
                    let url: *mut Object = msg_send![objects, objectAtIndex: index];
                    if url.is_null() {
                        continue;
                    }

                    let is_file: BOOL = msg_send![url, isFileURL];
                    if is_file != YES {
                        continue;
                    }

                    let started: BOOL = msg_send![url, startAccessingSecurityScopedResource];
                    let path_string: *mut Object = msg_send![url, path];
                    if !path_string.is_null() {
                        let utf8: *const std::os::raw::c_char = msg_send![path_string, UTF8String];
                        if !utf8.is_null() {
                            let path = std::ffi::CStr::from_ptr(utf8)
                                .to_string_lossy()
                                .into_owned();
                            if !path.is_empty() {
                                paths.push(PathBuf::from(path));
                            }
                        }
                    }
                    if started == YES {
                        let _: () = msg_send![url, stopAccessingSecurityScopedResource];
                    }
                }
            }

            if !paths.is_empty() {
                let external_paths = ExternalPaths(paths.into_iter().collect());
                dispatch_input(
                    &state,
                    PlatformInput::FileDrop(FileDropEvent::Entered {
                        position,
                        paths: external_paths,
                    }),
                );
                dispatch_input(
                    &state,
                    PlatformInput::FileDrop(FileDropEvent::Submit { position }),
                );
            }
            dispatch_input(&state, PlatformInput::FileDrop(FileDropEvent::Exited));
        }
    });
    let block = block.copy();

    unsafe {
        let _: *mut Object =
            msg_send![session, loadObjectsOfClass: class!(NSURL) completion: block];
    }
}

// ---------------------------------------------------------------------------
// GPUIDocumentPickerDelegate — bridges UIDocumentPicker callbacks to Rust
// oneshot channels used by prompt_for_paths/prompt_for_new_path.
// ---------------------------------------------------------------------------

pub(crate) enum PickerResultSender {
    Multiple(oneshot::Sender<Result<Option<Vec<PathBuf>>>>),
    Single(oneshot::Sender<Result<Option<PathBuf>>>),
}

pub(crate) struct DocumentPickerCallbackContext {
    pub(crate) sender: PickerResultSender,
    pub(crate) temp_paths: Vec<PathBuf>,
}

pub(crate) static mut GPUI_DOCUMENT_PICKER_DELEGATE_CLASS: *const Class = std::ptr::null();

#[ctor]
fn register_document_picker_delegate_class() {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("GPUIDocumentPickerDelegate", superclass)
            .expect("failed to declare GPUIDocumentPickerDelegate");

        decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);
        decl.add_method(
            sel!(documentPicker:didPickDocumentsAtURLs:),
            document_picker_did_pick as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
        );
        decl.add_method(
            sel!(documentPickerWasCancelled:),
            document_picker_was_cancelled as extern "C" fn(&Object, Sel, *mut Object),
        );

        GPUI_DOCUMENT_PICKER_DELEGATE_CLASS = decl.register();
    }
}

unsafe fn take_document_picker_context(
    delegate: *mut Object,
) -> Option<Box<DocumentPickerCallbackContext>> {
    unsafe {
        let ptr: *mut c_void = *(*delegate).get_ivar(CALLBACK_IVAR);
        if ptr.is_null() {
            return None;
        }
        (*delegate).set_ivar::<*mut c_void>(CALLBACK_IVAR, std::ptr::null_mut());
        Some(Box::from_raw(ptr as *mut DocumentPickerCallbackContext))
    }
}

unsafe fn release_document_picker_delegate(delegate: *mut Object) {
    unsafe {
        let platform_ptr = IOS_PLATFORM_STATE_PTR.load(Ordering::Acquire);
        if !platform_ptr.is_null() {
            let platform_state = &*(platform_ptr as *const Mutex<IosPlatformState>);
            let mut lock = platform_state.lock();
            if let Some(index) = lock
                .document_picker_delegates
                .iter()
                .position(|candidate| *candidate == delegate)
            {
                lock.document_picker_delegates.swap_remove(index);
            }
        }
        let _: () = msg_send![delegate, release];
    }
}

fn urls_to_paths(urls: *mut Object) -> Vec<PathBuf> {
    let mut result = Vec::new();
    unsafe {
        if urls.is_null() {
            return result;
        }
        let count: usize = msg_send![urls, count];
        for index in 0..count {
            let url: *mut Object = msg_send![urls, objectAtIndex: index];
            if url.is_null() {
                continue;
            }

            let is_file: BOOL = msg_send![url, isFileURL];
            if is_file != YES {
                continue;
            }

            let started: BOOL = msg_send![url, startAccessingSecurityScopedResource];
            let path_obj: *mut Object = msg_send![url, path];
            if !path_obj.is_null() {
                let utf8: *const std::os::raw::c_char = msg_send![path_obj, UTF8String];
                if !utf8.is_null() {
                    let path = std::ffi::CStr::from_ptr(utf8)
                        .to_string_lossy()
                        .into_owned();
                    if !path.is_empty() {
                        result.push(PathBuf::from(path));
                    }
                }
            }
            if started == YES {
                let _: () = msg_send![url, stopAccessingSecurityScopedResource];
            }
        }
    }
    result
}

fn finish_document_picker(delegate: &Object, result: Result<Option<Vec<PathBuf>>>) {
    unsafe {
        let delegate_ptr = delegate as *const Object as *mut Object;
        let Some(mut context) = take_document_picker_context(delegate_ptr) else {
            release_document_picker_delegate(delegate_ptr);
            return;
        };

        for temp_path in context.temp_paths.drain(..) {
            let _ = std::fs::remove_file(temp_path);
        }

        match context.sender {
            PickerResultSender::Multiple(sender) => {
                let _ = sender.send(result);
            }
            PickerResultSender::Single(sender) => {
                let mapped = result.map(|paths| paths.and_then(|p| p.into_iter().next()));
                let _ = sender.send(mapped);
            }
        }

        release_document_picker_delegate(delegate_ptr);
    }
}

extern "C" fn document_picker_did_pick(
    this: &Object,
    _sel: Sel,
    _controller: *mut Object,
    urls: *mut Object,
) {
    let paths = urls_to_paths(urls);
    finish_document_picker(this, Ok(Some(paths)));
}

extern "C" fn document_picker_was_cancelled(this: &Object, _sel: Sel, _controller: *mut Object) {
    finish_document_picker(this, Ok(None));
}
