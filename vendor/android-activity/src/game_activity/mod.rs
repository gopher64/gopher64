#![cfg(feature = "game-activity")]

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::unix::prelude::*;
use std::panic::catch_unwind;
use std::ptr::NonNull;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{ptr, thread};

use libc::c_void;
use log::{error, trace, Level};

use jni_sys::*;

use ndk_sys::ALooper_wake;
use ndk_sys::{ALooper, ALooper_pollAll};

use ndk::asset::AssetManager;
use ndk::configuration::Configuration;
use ndk::native_window::NativeWindow;

use crate::util::{abort_on_panic, android_log, log_panic};
use crate::{
    util, AndroidApp, ConfigurationRef, InputStatus, MainEvent, PollEvent, Rect, WindowManagerFlags,
};

mod ffi;

pub mod input;
use input::{Axis, InputEvent, KeyEvent, MotionEvent};

// The only time it's safe to update the android_app->savedState pointer is
// while handling a SaveState event, so this API is only exposed for those
// events...
#[derive(Debug)]
pub struct StateSaver<'a> {
    app: &'a AndroidAppInner,
}

impl<'a> StateSaver<'a> {
    pub fn store(&self, state: &'a [u8]) {
        // android_native_app_glue specifically expects savedState to have been allocated
        // via libc::malloc since it will automatically handle freeing the data once it
        // has been handed over to the Java Activity / main thread.
        unsafe {
            let app_ptr = self.app.native_app.as_ptr();

            // In case the application calls store() multiple times for some reason we
            // make sure to free any pre-existing state...
            if !(*app_ptr).savedState.is_null() {
                libc::free((*app_ptr).savedState);
                (*app_ptr).savedState = ptr::null_mut();
                (*app_ptr).savedStateSize = 0;
            }

            let buf = libc::malloc(state.len());
            if buf.is_null() {
                panic!("Failed to allocate save_state buffer");
            }

            // Since it's a byte array there's no special alignment requirement here.
            //
            // Since we re-define `buf` we ensure it's not possible to access the buffer
            // via its original pointer for the lifetime of the slice.
            {
                let buf: &mut [u8] = std::slice::from_raw_parts_mut(buf.cast(), state.len());
                buf.copy_from_slice(state);
            }

            (*app_ptr).savedState = buf;
            (*app_ptr).savedStateSize = state.len() as _;
        }
    }
}

#[derive(Debug)]
pub struct StateLoader<'a> {
    app: &'a AndroidAppInner,
}
impl<'a> StateLoader<'a> {
    pub fn load(&self) -> Option<Vec<u8>> {
        unsafe {
            let app_ptr = self.app.native_app.as_ptr();
            if !(*app_ptr).savedState.is_null() && (*app_ptr).savedStateSize > 0 {
                let buf: &mut [u8] = std::slice::from_raw_parts_mut(
                    (*app_ptr).savedState.cast(),
                    (*app_ptr).savedStateSize as usize,
                );
                let state = buf.to_vec();
                Some(state)
            } else {
                None
            }
        }
    }
}

#[derive(Clone)]
pub struct AndroidAppWaker {
    // The looper pointer is owned by the android_app and effectively
    // has a 'static lifetime, and the ALooper_wake C API is thread
    // safe, so this can be cloned safely and is send + sync safe
    looper: NonNull<ALooper>,
}
unsafe impl Send for AndroidAppWaker {}
unsafe impl Sync for AndroidAppWaker {}

impl AndroidAppWaker {
    pub fn wake(&self) {
        unsafe {
            ALooper_wake(self.looper.as_ptr());
        }
    }
}

impl AndroidApp {
    pub(crate) unsafe fn from_ptr(ptr: NonNull<ffi::android_app>) -> Self {
        // Note: we don't use from_ptr since we don't own the android_app.config
        // and need to keep in mind that the Drop handler is going to call
        // AConfiguration_delete()
        let config = Configuration::clone_from_ptr(NonNull::new_unchecked((*ptr.as_ptr()).config));

        Self {
            inner: Arc::new(RwLock::new(AndroidAppInner {
                native_app: NativeAppGlue { ptr },
                config: ConfigurationRef::new(config),
                native_window: Default::default(),
            })),
        }
    }
}

#[derive(Debug)]
struct NativeAppGlue {
    ptr: NonNull<ffi::android_app>,
}
impl Deref for NativeAppGlue {
    type Target = NonNull<ffi::android_app>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}
unsafe impl Send for NativeAppGlue {}
unsafe impl Sync for NativeAppGlue {}

#[derive(Debug)]
pub struct AndroidAppInner {
    native_app: NativeAppGlue,
    config: ConfigurationRef,
    native_window: RwLock<Option<NativeWindow>>,
}

impl AndroidAppInner {
    pub fn vm_as_ptr(&self) -> *mut c_void {
        let app_ptr = self.native_app.as_ptr();
        unsafe { (*(*app_ptr).activity).vm as _ }
    }

    pub fn activity_as_ptr(&self) -> *mut c_void {
        let app_ptr = self.native_app.as_ptr();
        unsafe { (*(*app_ptr).activity).javaGameActivity as _ }
    }

    pub fn native_window(&self) -> Option<NativeWindow> {
        self.native_window.read().unwrap().clone()
    }

    pub fn poll_events<F>(&self, timeout: Option<Duration>, mut callback: F)
    where
        F: FnMut(PollEvent),
    {
        trace!("poll_events");

        unsafe {
            let native_app = &self.native_app;

            let mut fd: i32 = 0;
            let mut events: i32 = 0;
            let mut source: *mut core::ffi::c_void = ptr::null_mut();

            let timeout_milliseconds = if let Some(timeout) = timeout {
                timeout.as_millis() as i32
            } else {
                -1
            };
            trace!("Calling ALooper_pollAll, timeout = {timeout_milliseconds}");
            let id = ALooper_pollAll(
                timeout_milliseconds,
                &mut fd,
                &mut events,
                &mut source as *mut *mut core::ffi::c_void,
            );
            match id {
                ffi::ALOOPER_POLL_WAKE => {
                    trace!("ALooper_pollAll returned POLL_WAKE");

                    if ffi::android_app_input_available_wake_up(native_app.as_ptr()) {
                        log::debug!("Notifying Input Available");
                        callback(PollEvent::Main(MainEvent::InputAvailable));
                    }

                    callback(PollEvent::Wake);
                }
                ffi::ALOOPER_POLL_CALLBACK => {
                    // ALooper_pollAll is documented to handle all callback sources internally so it should
                    // never return a _CALLBACK source id...
                    error!("Spurious ALOOPER_POLL_CALLBACK from ALopper_pollAll() (ignored)");
                }
                ffi::ALOOPER_POLL_TIMEOUT => {
                    trace!("ALooper_pollAll returned POLL_TIMEOUT");
                    callback(PollEvent::Timeout);
                }
                ffi::ALOOPER_POLL_ERROR => {
                    // If we have an IO error with our pipe to the main Java thread that's surely
                    // not something we can recover from
                    panic!("ALooper_pollAll returned POLL_ERROR");
                }
                id if id >= 0 => {
                    match id as u32 {
                        ffi::NativeAppGlueLooperId_LOOPER_ID_MAIN => {
                            trace!("ALooper_pollAll returned ID_MAIN");
                            let source: *mut ffi::android_poll_source = source.cast();
                            if !source.is_null() {
                                let cmd_i = ffi::android_app_read_cmd(native_app.as_ptr());

                                let cmd = match cmd_i as u32 {
                                    //NativeAppGlueAppCmd_UNUSED_APP_CMD_INPUT_CHANGED => AndroidAppMainEvent::InputChanged,
                                    ffi::NativeAppGlueAppCmd_APP_CMD_INIT_WINDOW => {
                                        MainEvent::InitWindow {}
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_TERM_WINDOW => {
                                        MainEvent::TerminateWindow {}
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_WINDOW_RESIZED => {
                                        MainEvent::WindowResized {}
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_WINDOW_REDRAW_NEEDED => {
                                        MainEvent::RedrawNeeded {}
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_CONTENT_RECT_CHANGED => {
                                        MainEvent::ContentRectChanged {}
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_GAINED_FOCUS => {
                                        MainEvent::GainedFocus
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_LOST_FOCUS => {
                                        MainEvent::LostFocus
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_CONFIG_CHANGED => {
                                        MainEvent::ConfigChanged {}
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_LOW_MEMORY => {
                                        MainEvent::LowMemory
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_START => MainEvent::Start,
                                    ffi::NativeAppGlueAppCmd_APP_CMD_RESUME => MainEvent::Resume {
                                        loader: StateLoader { app: self },
                                    },
                                    ffi::NativeAppGlueAppCmd_APP_CMD_SAVE_STATE => {
                                        MainEvent::SaveState {
                                            saver: StateSaver { app: self },
                                        }
                                    }
                                    ffi::NativeAppGlueAppCmd_APP_CMD_PAUSE => MainEvent::Pause,
                                    ffi::NativeAppGlueAppCmd_APP_CMD_STOP => MainEvent::Stop,
                                    ffi::NativeAppGlueAppCmd_APP_CMD_DESTROY => MainEvent::Destroy,
                                    ffi::NativeAppGlueAppCmd_APP_CMD_WINDOW_INSETS_CHANGED => {
                                        MainEvent::InsetsChanged {}
                                    }
                                    _ => unreachable!(),
                                };

                                trace!("Read ID_MAIN command {cmd_i} = {cmd:?}");

                                trace!("Calling android_app_pre_exec_cmd({cmd_i})");
                                ffi::android_app_pre_exec_cmd(native_app.as_ptr(), cmd_i);
                                match cmd {
                                    MainEvent::ConfigChanged { .. } => {
                                        self.config.replace(Configuration::clone_from_ptr(
                                            NonNull::new_unchecked((*native_app.as_ptr()).config),
                                        ));
                                    }
                                    MainEvent::InitWindow { .. } => {
                                        let win_ptr = (*native_app.as_ptr()).window;
                                        // It's important that we use ::clone_from_ptr() here
                                        // because NativeWindow has a Drop implementation that
                                        // will unconditionally _release() the native window
                                        *self.native_window.write().unwrap() =
                                            Some(NativeWindow::clone_from_ptr(
                                                NonNull::new(win_ptr).unwrap(),
                                            ));
                                    }
                                    MainEvent::TerminateWindow { .. } => {
                                        *self.native_window.write().unwrap() = None;
                                    }
                                    _ => {}
                                }

                                trace!("Invoking callback for ID_MAIN command = {:?}", cmd);
                                callback(PollEvent::Main(cmd));

                                trace!("Calling android_app_post_exec_cmd({cmd_i})");
                                ffi::android_app_post_exec_cmd(native_app.as_ptr(), cmd_i);
                            } else {
                                panic!("ALooper_pollAll returned ID_MAIN event with NULL android_poll_source!");
                            }
                        }
                        _ => {
                            error!("Ignoring spurious ALooper event source: id = {id}, fd = {fd}, events = {events:?}, data = {source:?}");
                        }
                    }
                }
                _ => {
                    error!("Spurious ALooper_pollAll return value {id} (ignored)");
                }
            }
        }
    }

    pub fn set_window_flags(
        &self,
        add_flags: WindowManagerFlags,
        remove_flags: WindowManagerFlags,
    ) {
        unsafe {
            let activity = (*self.native_app.as_ptr()).activity;
            ffi::GameActivity_setWindowFlags(activity, add_flags.bits(), remove_flags.bits())
        }
    }

    // TODO: move into a trait
    pub fn show_soft_input(&self, show_implicit: bool) {
        unsafe {
            let activity = (*self.native_app.as_ptr()).activity;
            let flags = if show_implicit {
                ffi::ShowImeFlags_SHOW_IMPLICIT
            } else {
                0
            };
            ffi::GameActivity_showSoftInput(activity, flags);
        }
    }

    // TODO: move into a trait
    pub fn hide_soft_input(&self, hide_implicit_only: bool) {
        unsafe {
            let activity = (*self.native_app.as_ptr()).activity;
            let flags = if hide_implicit_only {
                ffi::HideImeFlags_HIDE_IMPLICIT_ONLY
            } else {
                0
            };
            ffi::GameActivity_hideSoftInput(activity, flags);
        }
    }

    pub fn enable_motion_axis(&mut self, axis: Axis) {
        unsafe { ffi::GameActivityPointerAxes_enableAxis(axis as i32) }
    }

    pub fn disable_motion_axis(&mut self, axis: Axis) {
        unsafe { ffi::GameActivityPointerAxes_disableAxis(axis as i32) }
    }

    pub fn create_waker(&self) -> AndroidAppWaker {
        unsafe {
            // From the application's pov we assume the app_ptr and looper pointer
            // have static lifetimes and we can safely assume they are never NULL.
            let app_ptr = self.native_app.as_ptr();
            AndroidAppWaker {
                looper: NonNull::new_unchecked((*app_ptr).looper),
            }
        }
    }

    pub fn config(&self) -> ConfigurationRef {
        self.config.clone()
    }

    pub fn content_rect(&self) -> Rect {
        unsafe {
            let app_ptr = self.native_app.as_ptr();
            Rect {
                left: (*app_ptr).contentRect.left,
                right: (*app_ptr).contentRect.right,
                top: (*app_ptr).contentRect.top,
                bottom: (*app_ptr).contentRect.bottom,
            }
        }
    }

    pub fn asset_manager(&self) -> AssetManager {
        unsafe {
            let app_ptr = self.native_app.as_ptr();
            let am_ptr = NonNull::new_unchecked((*(*app_ptr).activity).assetManager);
            AssetManager::from_ptr(am_ptr)
        }
    }

    pub fn input_events<F>(&self, mut callback: F)
    where
        F: FnMut(&InputEvent) -> InputStatus,
    {
        let buf = unsafe {
            let app_ptr = self.native_app.as_ptr();
            let input_buffer = ffi::android_app_swap_input_buffers(app_ptr);
            if input_buffer.is_null() {
                return;
            }
            InputBuffer::from_ptr(NonNull::new_unchecked(input_buffer))
        };

        let mut keys_iter = KeyEventsLendingIterator::new(&buf);
        while let Some(key_event) = keys_iter.next() {
            callback(&InputEvent::KeyEvent(key_event));
        }
        let mut motion_iter = MotionEventsLendingIterator::new(&buf);
        while let Some(motion_event) = motion_iter.next() {
            callback(&InputEvent::MotionEvent(motion_event));
        }
    }

    pub fn internal_data_path(&self) -> Option<std::path::PathBuf> {
        unsafe {
            let app_ptr = self.native_app.as_ptr();
            util::try_get_path_from_ptr((*(*app_ptr).activity).internalDataPath)
        }
    }

    pub fn external_data_path(&self) -> Option<std::path::PathBuf> {
        unsafe {
            let app_ptr = self.native_app.as_ptr();
            util::try_get_path_from_ptr((*(*app_ptr).activity).externalDataPath)
        }
    }

    pub fn obb_path(&self) -> Option<std::path::PathBuf> {
        unsafe {
            let app_ptr = self.native_app.as_ptr();
            util::try_get_path_from_ptr((*(*app_ptr).activity).obbPath)
        }
    }
}

struct MotionEventsLendingIterator<'a> {
    pos: usize,
    count: usize,
    buffer: &'a InputBuffer<'a>,
}

// A kind of lending iterator but since our MSRV is 1.60 we can't handle this
// via a generic trait. The iteration of motion events is entirely private
// though so this is ok for now.
impl<'a> MotionEventsLendingIterator<'a> {
    fn new(buffer: &'a InputBuffer<'a>) -> Self {
        Self {
            pos: 0,
            count: buffer.motion_events_count(),
            buffer,
        }
    }
    fn next(&mut self) -> Option<MotionEvent<'a>> {
        if self.pos < self.count {
            let ga_event = unsafe { &(*self.buffer.ptr.as_ptr()).motionEvents[self.pos] };
            let event = MotionEvent::new(ga_event);
            self.pos += 1;
            Some(event)
        } else {
            None
        }
    }
}

struct KeyEventsLendingIterator<'a> {
    pos: usize,
    count: usize,
    buffer: &'a InputBuffer<'a>,
}

// A kind of lending iterator but since our MSRV is 1.60 we can't handle this
// via a generic trait. The iteration of key events is entirely private
// though so this is ok for now.
impl<'a> KeyEventsLendingIterator<'a> {
    fn new(buffer: &'a InputBuffer<'a>) -> Self {
        Self {
            pos: 0,
            count: buffer.key_events_count(),
            buffer,
        }
    }
    fn next(&mut self) -> Option<KeyEvent<'a>> {
        if self.pos < self.count {
            let ga_event = unsafe { &(*self.buffer.ptr.as_ptr()).keyEvents[self.pos] };
            let event = KeyEvent::new(ga_event);
            self.pos += 1;
            Some(event)
        } else {
            None
        }
    }
}

struct InputBuffer<'a> {
    ptr: NonNull<ffi::android_input_buffer>,
    _lifetime: PhantomData<&'a ffi::android_input_buffer>,
}

impl<'a> InputBuffer<'a> {
    pub(crate) fn from_ptr(ptr: NonNull<ffi::android_input_buffer>) -> InputBuffer<'a> {
        Self {
            ptr,
            _lifetime: PhantomData::default(),
        }
    }

    pub fn motion_events_count(&self) -> usize {
        unsafe { (*self.ptr.as_ptr()).motionEventsCount as usize }
    }

    pub fn key_events_count(&self) -> usize {
        unsafe { (*self.ptr.as_ptr()).keyEventsCount as usize }
    }
}

impl<'a> Drop for InputBuffer<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::android_app_clear_motion_events(self.ptr.as_ptr());
            ffi::android_app_clear_key_events(self.ptr.as_ptr());
        }
    }
}

// Rust doesn't give us a clean way to directly export symbols from C/C++
// so we rename the C/C++ symbols and re-export these JNI entrypoints from
// Rust...
//
// https://github.com/rust-lang/rfcs/issues/2771
extern "C" {
    pub fn Java_com_google_androidgamesdk_GameActivity_loadNativeCode_C(
        env: *mut JNIEnv,
        javaGameActivity: jobject,
        path: jstring,
        funcName: jstring,
        internalDataDir: jstring,
        obbDir: jstring,
        externalDataDir: jstring,
        jAssetMgr: jobject,
        savedState: jbyteArray,
    ) -> jlong;

    pub fn GameActivity_onCreate_C(
        activity: *mut ffi::GameActivity,
        savedState: *mut ::std::os::raw::c_void,
        savedStateSize: libc::size_t,
    );
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_google_androidgamesdk_GameActivity_loadNativeCode(
    env: *mut JNIEnv,
    java_game_activity: jobject,
    path: jstring,
    func_name: jstring,
    internal_data_dir: jstring,
    obb_dir: jstring,
    external_data_dir: jstring,
    jasset_mgr: jobject,
    saved_state: jbyteArray,
) -> jni_sys::jlong {
    Java_com_google_androidgamesdk_GameActivity_loadNativeCode_C(
        env,
        java_game_activity,
        path,
        func_name,
        internal_data_dir,
        obb_dir,
        external_data_dir,
        jasset_mgr,
        saved_state,
    )
}

#[no_mangle]
pub unsafe extern "C" fn GameActivity_onCreate(
    activity: *mut ffi::GameActivity,
    saved_state: *mut ::std::os::raw::c_void,
    saved_state_size: libc::size_t,
) {
    GameActivity_onCreate_C(activity, saved_state, saved_state_size);
}

extern "Rust" {
    pub fn android_main(app: AndroidApp);
}

// This is a spring board between android_native_app_glue and the user's
// `app_main` function. This is run on a dedicated thread spawned
// by android_native_app_glue.
#[no_mangle]
#[allow(unused_unsafe)] // Otherwise rust 1.64 moans about using unsafe{} in unsafe functions
pub unsafe extern "C" fn _rust_glue_entry(native_app: *mut ffi::android_app) {
    abort_on_panic(|| {
        // Maybe make this stdout/stderr redirection an optional / opt-in feature?...
        let mut logpipe: [RawFd; 2] = Default::default();
        libc::pipe(logpipe.as_mut_ptr());
        libc::dup2(logpipe[1], libc::STDOUT_FILENO);
        libc::dup2(logpipe[1], libc::STDERR_FILENO);
        thread::spawn(move || {
            let tag = CStr::from_bytes_with_nul(b"RustStdoutStderr\0").unwrap();
            let file = File::from_raw_fd(logpipe[0]);
            let mut reader = BufReader::new(file);
            let mut buffer = String::new();
            loop {
                buffer.clear();
                if let Ok(len) = reader.read_line(&mut buffer) {
                    if len == 0 {
                        break;
                    } else if let Ok(msg) = CString::new(buffer.clone()) {
                        android_log(Level::Info, tag, &msg);
                    }
                }
            }
        });

        let jvm = unsafe {
            let jvm = (*(*native_app).activity).vm;
            let activity: jobject = (*(*native_app).activity).javaGameActivity;
            ndk_context::initialize_android_context(jvm.cast(), activity.cast());

            // Since this is a newly spawned thread then the JVM hasn't been attached
            // to the thread yet. Attach before calling the applications main function
            // so they can safely make JNI calls
            let mut jenv_out: *mut core::ffi::c_void = std::ptr::null_mut();
            if let Some(attach_current_thread) = (*(*jvm)).AttachCurrentThread {
                attach_current_thread(jvm, &mut jenv_out, std::ptr::null_mut());
            }

            jvm
        };

        unsafe {
            let app = AndroidApp::from_ptr(NonNull::new(native_app).unwrap());

            // We want to specifically catch any panic from the application's android_main
            // so we can finish + destroy the Activity gracefully via the JVM
            catch_unwind(|| {
                // XXX: If we were in control of the Java Activity subclass then
                // we could potentially run the android_main function via a Java native method
                // springboard (e.g. call an Activity subclass method that calls a jni native
                // method that then just calls android_main()) that would make sure there was
                // a Java frame at the base of our call stack which would then be recognised
                // when calling FindClass to lookup a suitable classLoader, instead of
                // defaulting to the system loader. Without this then it's difficult for native
                // code to look up non-standard Java classes.
                android_main(app);
            })
            .unwrap_or_else(|panic| log_panic(panic));

            // Let JVM know that our Activity can be destroyed before detaching from the JVM
            //
            // "Note that this method can be called from any thread; it will send a message
            //  to the main thread of the process where the Java finish call will take place"
            ffi::GameActivity_finish((*native_app).activity);

            if let Some(detach_current_thread) = (*(*jvm)).DetachCurrentThread {
                detach_current_thread(jvm);
            }

            ndk_context::release_android_context();
        }
    })
}
