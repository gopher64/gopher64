//! This 'glue' layer acts as an IPC shim between the JVM main thread and the Rust
//! main thread. Notifying Rust of lifecycle events from the JVM and handling
//! synchronization between the two threads.

use std::{
    ffi::{CStr, CString},
    fs::File,
    io::{BufRead, BufReader},
    ops::Deref,
    os::unix::prelude::{FromRawFd, RawFd},
    panic::catch_unwind,
    ptr::{self, NonNull},
    sync::{Arc, Condvar, Mutex, Weak},
};

use log::Level;
use ndk::{configuration::Configuration, input_queue::InputQueue, native_window::NativeWindow};

use crate::{
    util::android_log,
    util::{abort_on_panic, log_panic},
    ConfigurationRef,
};

use super::{AndroidApp, Rect};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AppCmd {
    InputQueueChanged = 0,
    InitWindow = 1,
    TermWindow = 2,
    WindowResized = 3,
    WindowRedrawNeeded = 4,
    ContentRectChanged = 5,
    GainedFocus = 6,
    LostFocus = 7,
    ConfigChanged = 8,
    LowMemory = 9,
    Start = 10,
    Resume = 11,
    SaveState = 12,
    Pause = 13,
    Stop = 14,
    Destroy = 15,
}
impl TryFrom<i8> for AppCmd {
    type Error = ();

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AppCmd::InputQueueChanged),
            1 => Ok(AppCmd::InitWindow),
            2 => Ok(AppCmd::TermWindow),
            3 => Ok(AppCmd::WindowResized),
            4 => Ok(AppCmd::WindowRedrawNeeded),
            5 => Ok(AppCmd::ContentRectChanged),
            6 => Ok(AppCmd::GainedFocus),
            7 => Ok(AppCmd::LostFocus),
            8 => Ok(AppCmd::ConfigChanged),
            9 => Ok(AppCmd::LowMemory),
            10 => Ok(AppCmd::Start),
            11 => Ok(AppCmd::Resume),
            12 => Ok(AppCmd::SaveState),
            13 => Ok(AppCmd::Pause),
            14 => Ok(AppCmd::Stop),
            15 => Ok(AppCmd::Destroy),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum State {
    Init,
    Start,
    Resume,
    Pause,
    Stop,
}

#[derive(Debug)]
pub struct WaitableNativeActivityState {
    pub activity: *mut ndk_sys::ANativeActivity,

    pub mutex: Mutex<NativeActivityState>,
    pub cond: Condvar,
}

#[derive(Debug, Clone)]
pub struct NativeActivityGlue {
    pub inner: Arc<WaitableNativeActivityState>,
}
unsafe impl Send for NativeActivityGlue {}
unsafe impl Sync for NativeActivityGlue {}

impl Deref for NativeActivityGlue {
    type Target = WaitableNativeActivityState;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl NativeActivityGlue {
    pub fn new(
        activity: *mut ndk_sys::ANativeActivity,
        saved_state: *const libc::c_void,
        saved_state_size: libc::size_t,
    ) -> Self {
        let glue = Self {
            inner: Arc::new(WaitableNativeActivityState::new(
                activity,
                saved_state,
                saved_state_size,
            )),
        };

        let weak_ref = Arc::downgrade(&glue.inner);
        let weak_ptr = Weak::into_raw(weak_ref);
        unsafe {
            (*activity).instance = weak_ptr as *mut _;

            (*(*activity).callbacks).onDestroy = Some(on_destroy);
            (*(*activity).callbacks).onStart = Some(on_start);
            (*(*activity).callbacks).onResume = Some(on_resume);
            (*(*activity).callbacks).onSaveInstanceState = Some(on_save_instance_state);
            (*(*activity).callbacks).onPause = Some(on_pause);
            (*(*activity).callbacks).onStop = Some(on_stop);
            (*(*activity).callbacks).onConfigurationChanged = Some(on_configuration_changed);
            (*(*activity).callbacks).onLowMemory = Some(on_low_memory);
            (*(*activity).callbacks).onWindowFocusChanged = Some(on_window_focus_changed);
            (*(*activity).callbacks).onNativeWindowCreated = Some(on_native_window_created);
            (*(*activity).callbacks).onNativeWindowResized = Some(on_native_window_resized);
            (*(*activity).callbacks).onNativeWindowRedrawNeeded =
                Some(on_native_window_redraw_needed);
            (*(*activity).callbacks).onNativeWindowDestroyed = Some(on_native_window_destroyed);
            (*(*activity).callbacks).onInputQueueCreated = Some(on_input_queue_created);
            (*(*activity).callbacks).onInputQueueDestroyed = Some(on_input_queue_destroyed);
            (*(*activity).callbacks).onContentRectChanged = Some(on_content_rect_changed);
        }

        glue
    }

    /// Returns the file descriptor that needs to be polled by the Rust main thread
    /// for events/commands from the JVM thread
    pub fn cmd_read_fd(&self) -> libc::c_int {
        self.mutex.lock().unwrap().msg_read
    }

    /// For the Rust main thread to read a single pending command sent from the JVM main thread
    pub fn read_cmd(&self) -> Option<AppCmd> {
        self.inner.mutex.lock().unwrap().read_cmd()
    }

    /// For the Rust main thread to get an [`InputQueue`] that wraps the AInputQueue pointer
    /// we have and at the same time ensure that the input queue is attached to the given looper.
    ///
    /// NB: it's expected that the input queue is detached as soon as we know there is new
    /// input (knowing the app will be notified) and only re-attached when the application
    /// reads the input (to avoid lots of redundant wake ups)
    pub fn looper_attached_input_queue(
        &self,
        looper: *mut ndk_sys::ALooper,
        ident: libc::c_int,
    ) -> Option<InputQueue> {
        let mut guard = self.mutex.lock().unwrap();

        if guard.input_queue.is_null() {
            return None;
        }

        unsafe {
            // Reattach the input queue to the looper so future input will again deliver an
            // `InputAvailable` event.
            guard.attach_input_queue_to_looper(looper, ident);
            Some(InputQueue::from_ptr(NonNull::new_unchecked(
                guard.input_queue,
            )))
        }
    }

    pub fn detach_input_queue_from_looper(&self) {
        unsafe {
            self.inner
                .mutex
                .lock()
                .unwrap()
                .detach_input_queue_from_looper();
        }
    }

    pub fn config(&self) -> ConfigurationRef {
        self.mutex.lock().unwrap().config.clone()
    }

    pub fn content_rect(&self) -> Rect {
        self.mutex.lock().unwrap().content_rect.into()
    }
}

/// The status of the native thread that's created to run
/// `android_main`
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NativeThreadState {
    /// The `android_main` thread hasn't been created yet
    Init,
    /// The `android_main` thread has been spawned and started running
    Running,
    /// The `android_main` thread has finished
    Stopped,
}

#[derive(Debug)]
pub struct NativeActivityState {
    pub msg_read: libc::c_int,
    pub msg_write: libc::c_int,
    pub config: super::ConfigurationRef,
    pub saved_state: Vec<u8>,
    pub input_queue: *mut ndk_sys::AInputQueue,
    pub window: Option<NativeWindow>,
    pub content_rect: ndk_sys::ARect,
    pub activity_state: State,
    pub destroy_requested: bool,
    pub thread_state: NativeThreadState,
    pub app_has_saved_state: bool,

    /// Set as soon as the Java main thread notifies us of an
    /// `onDestroyed` callback.
    pub destroyed: bool,
    pub redraw_needed: bool,
    pub pending_input_queue: *mut ndk_sys::AInputQueue,
    pub pending_window: Option<NativeWindow>,
}

impl NativeActivityState {
    pub fn read_cmd(&mut self) -> Option<AppCmd> {
        let mut cmd_i: i8 = 0;
        loop {
            match unsafe { libc::read(self.msg_read, &mut cmd_i as *mut _ as *mut _, 1) } {
                1 => {
                    let cmd = AppCmd::try_from(cmd_i);
                    return match cmd {
                        Ok(cmd) => Some(cmd),
                        Err(_) => {
                            log::error!("Spurious, unknown NativeActivityGlue cmd: {}", cmd_i);
                            None
                        }
                    };
                }
                -1 => {
                    let err = std::io::Error::last_os_error();
                    if err.kind() != std::io::ErrorKind::Interrupted {
                        log::error!("Failure reading NativeActivityGlue cmd: {}", err);
                        return None;
                    }
                }
                count => {
                    log::error!(
                        "Spurious read of {count} bytes while reading NativeActivityGlue cmd"
                    );
                    return None;
                }
            }
        }
    }

    fn write_cmd(&mut self, cmd: AppCmd) {
        let cmd = cmd as i8;
        loop {
            match unsafe { libc::write(self.msg_write, &cmd as *const _ as *const _, 1) } {
                1 => break,
                -1 => {
                    let err = std::io::Error::last_os_error();
                    if err.kind() != std::io::ErrorKind::Interrupted {
                        log::error!("Failure writing NativeActivityGlue cmd: {}", err);
                        return;
                    }
                }
                count => {
                    log::error!(
                        "Spurious write of {count} bytes while writing NativeActivityGlue cmd"
                    );
                    return;
                }
            }
        }
    }

    pub unsafe fn attach_input_queue_to_looper(
        &mut self,
        looper: *mut ndk_sys::ALooper,
        ident: libc::c_int,
    ) {
        if !self.input_queue.is_null() {
            log::trace!("Attaching input queue to looper");
            ndk_sys::AInputQueue_attachLooper(
                self.input_queue,
                looper,
                ident,
                None,
                ptr::null_mut(),
            );
        }
    }

    pub unsafe fn detach_input_queue_from_looper(&mut self) {
        if !self.input_queue.is_null() {
            log::trace!("Detaching input queue from looper");
            ndk_sys::AInputQueue_detachLooper(self.input_queue);
        }
    }
}

impl Drop for WaitableNativeActivityState {
    fn drop(&mut self) {
        log::debug!("WaitableNativeActivityState::drop!");
        unsafe {
            let mut guard = self.mutex.lock().unwrap();
            guard.detach_input_queue_from_looper();
        }
    }
}

impl WaitableNativeActivityState {
    ///////////////////////////////
    // Java-side callback handling
    ///////////////////////////////

    pub fn new(
        activity: *mut ndk_sys::ANativeActivity,
        saved_state_in: *const libc::c_void,
        saved_state_size: libc::size_t,
    ) -> Self {
        let mut msgpipe: [libc::c_int; 2] = [-1, -1];
        unsafe {
            if libc::pipe(msgpipe.as_mut_ptr()) != 0 {
                panic!(
                    "could not create  Rust <-> Java IPC pipe: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        let saved_state = unsafe {
            std::slice::from_raw_parts(saved_state_in as *const u8, saved_state_size as _)
        };

        let config = unsafe {
            let config = ndk_sys::AConfiguration_new();
            ndk_sys::AConfiguration_fromAssetManager(config, (*activity).assetManager);

            let config = super::ConfigurationRef::new(Configuration::from_ptr(
                NonNull::new_unchecked(config),
            ));
            log::trace!("Config: {:#?}", config);
            config
        };

        Self {
            activity,
            mutex: Mutex::new(NativeActivityState {
                msg_read: msgpipe[0],
                msg_write: msgpipe[1],
                config,
                saved_state: saved_state.into(),
                input_queue: ptr::null_mut(),
                window: None,
                content_rect: Rect::empty().into(),
                activity_state: State::Init,
                destroy_requested: false,
                thread_state: NativeThreadState::Init,
                app_has_saved_state: false,
                destroyed: false,
                redraw_needed: false,
                pending_input_queue: ptr::null_mut(),
                pending_window: None,
            }),
            cond: Condvar::new(),
        }
    }

    pub fn notify_destroyed(&self) {
        let mut guard = self.mutex.lock().unwrap();
        guard.destroyed = true;

        unsafe {
            guard.write_cmd(AppCmd::Destroy);
            while guard.thread_state != NativeThreadState::Stopped {
                guard = self.cond.wait(guard).unwrap();
            }

            libc::close(guard.msg_read);
            guard.msg_read = -1;
            libc::close(guard.msg_write);
            guard.msg_write = -1;
        }
    }

    pub fn notify_config_changed(&self) {
        let mut guard = self.mutex.lock().unwrap();
        guard.write_cmd(AppCmd::ConfigChanged);
    }

    pub fn notify_low_memory(&self) {
        let mut guard = self.mutex.lock().unwrap();
        guard.write_cmd(AppCmd::LowMemory);
    }

    pub fn notify_focus_changed(&self, focused: bool) {
        let mut guard = self.mutex.lock().unwrap();
        guard.write_cmd(if focused {
            AppCmd::GainedFocus
        } else {
            AppCmd::LostFocus
        });
    }

    pub fn notify_window_resized(&self, native_window: *mut ndk_sys::ANativeWindow) {
        let mut guard = self.mutex.lock().unwrap();
        // set_window always syncs .pending_window back to .window before returning. This callback
        // from Android can never arrive at an interim state, and validates that Android:
        // 1. Only provides resizes in between onNativeWindowCreated and onNativeWindowDestroyed;
        // 2. Doesn't call it on a bogus window pointer that we don't know about.
        debug_assert_eq!(guard.window.as_ref().unwrap().ptr().as_ptr(), native_window);
        guard.write_cmd(AppCmd::WindowResized);
    }

    pub fn notify_window_redraw_needed(&self, native_window: *mut ndk_sys::ANativeWindow) {
        let mut guard = self.mutex.lock().unwrap();
        // set_window always syncs .pending_window back to .window before returning. This callback
        // from Android can never arrive at an interim state, and validates that Android:
        // 1. Only provides resizes in between onNativeWindowCreated and onNativeWindowDestroyed;
        // 2. Doesn't call it on a bogus window pointer that we don't know about.
        debug_assert_eq!(guard.window.as_ref().unwrap().ptr().as_ptr(), native_window);
        guard.write_cmd(AppCmd::WindowRedrawNeeded);
    }

    unsafe fn set_input(&self, input_queue: *mut ndk_sys::AInputQueue) {
        let mut guard = self.mutex.lock().unwrap();

        // The pending_input_queue state should only be set while in this method, and since
        // it doesn't allow re-entrance and is cleared before returning then we expect
        // this to be null
        debug_assert!(
            guard.pending_input_queue.is_null(),
            "InputQueue update clash"
        );

        guard.pending_input_queue = input_queue;
        guard.write_cmd(AppCmd::InputQueueChanged);
        while guard.input_queue != guard.pending_input_queue {
            guard = self.cond.wait(guard).unwrap();
        }
        guard.pending_input_queue = ptr::null_mut();
    }

    unsafe fn set_window(&self, window: Option<NativeWindow>) {
        let mut guard = self.mutex.lock().unwrap();

        // The pending_window state should only be set while in this method, and since
        // it doesn't allow re-entrance and is cleared before returning then we expect
        // this to be None
        debug_assert!(guard.pending_window.is_none(), "NativeWindow update clash");

        if guard.window.is_some() {
            guard.write_cmd(AppCmd::TermWindow);
        }
        guard.pending_window = window;
        if guard.pending_window.is_some() {
            guard.write_cmd(AppCmd::InitWindow);
        }
        while guard.window != guard.pending_window {
            guard = self.cond.wait(guard).unwrap();
        }
        guard.pending_window = None;
    }

    unsafe fn set_content_rect(&self, rect: *const ndk_sys::ARect) {
        let mut guard = self.mutex.lock().unwrap();
        guard.content_rect = *rect;
        guard.write_cmd(AppCmd::ContentRectChanged);
    }

    unsafe fn set_activity_state(&self, state: State) {
        let mut guard = self.mutex.lock().unwrap();

        let cmd = match state {
            State::Init => panic!("Can't explicitly transition into 'init' state"),
            State::Start => AppCmd::Start,
            State::Resume => AppCmd::Resume,
            State::Pause => AppCmd::Pause,
            State::Stop => AppCmd::Stop,
        };
        guard.write_cmd(cmd);

        while guard.activity_state != state {
            guard = self.cond.wait(guard).unwrap();
        }
    }

    fn request_save_state(&self) -> (*mut libc::c_void, libc::size_t) {
        let mut guard = self.mutex.lock().unwrap();

        // The state_saved flag should only be set while in this method, and since
        // it doesn't allow re-entrance and is cleared before returning then we expect
        // this to be None
        debug_assert!(!guard.app_has_saved_state, "SaveState request clash");
        guard.write_cmd(AppCmd::SaveState);
        while !guard.app_has_saved_state {
            guard = self.cond.wait(guard).unwrap();
        }
        guard.app_has_saved_state = false;

        // `ANativeActivity` explicitly documents that it expects save state to be
        // given via a `malloc()` allocated pointer since it will automatically
        // `free()` the state after it has been converted to a buffer for the JVM.
        if !guard.saved_state.is_empty() {
            let saved_state_size = guard.saved_state.len() as _;
            let saved_state_src_ptr = guard.saved_state.as_ptr();
            unsafe {
                let saved_state = libc::malloc(saved_state_size);
                assert!(
                    !saved_state.is_null(),
                    "Failed to allocate {} bytes for restoring saved application state",
                    saved_state_size
                );
                libc::memcpy(saved_state, saved_state_src_ptr as _, saved_state_size);
                (saved_state, saved_state_size)
            }
        } else {
            (ptr::null_mut(), 0)
        }
    }

    pub fn saved_state(&self) -> Option<Vec<u8>> {
        let guard = self.mutex.lock().unwrap();
        if !guard.saved_state.is_empty() {
            Some(guard.saved_state.clone())
        } else {
            None
        }
    }

    pub fn set_saved_state(&self, state: &[u8]) {
        let mut guard = self.mutex.lock().unwrap();

        guard.saved_state.clear();
        guard.saved_state.extend_from_slice(state);
    }

    ////////////////////////////
    // Rust-side event loop
    ////////////////////////////

    pub fn notify_main_thread_running(&self) {
        let mut guard = self.mutex.lock().unwrap();
        guard.thread_state = NativeThreadState::Running;
        self.cond.notify_one();
    }

    pub fn notify_main_thread_stopped_running(&self) {
        let mut guard = self.mutex.lock().unwrap();
        guard.thread_state = NativeThreadState::Stopped;
        self.cond.notify_one();
    }

    pub unsafe fn pre_exec_cmd(
        &self,
        cmd: AppCmd,
        looper: *mut ndk_sys::ALooper,
        input_queue_ident: libc::c_int,
    ) {
        log::trace!("Pre: AppCmd::{:#?}", cmd);
        match cmd {
            AppCmd::InputQueueChanged => {
                let mut guard = self.mutex.lock().unwrap();
                guard.detach_input_queue_from_looper();
                guard.input_queue = guard.pending_input_queue;
                if !guard.input_queue.is_null() {
                    guard.attach_input_queue_to_looper(looper, input_queue_ident);
                }
                self.cond.notify_one();
            }
            AppCmd::InitWindow => {
                let mut guard = self.mutex.lock().unwrap();
                guard.window = guard.pending_window.clone();
                self.cond.notify_one();
            }
            AppCmd::Resume | AppCmd::Start | AppCmd::Pause | AppCmd::Stop => {
                let mut guard = self.mutex.lock().unwrap();
                guard.activity_state = match cmd {
                    AppCmd::Start => State::Start,
                    AppCmd::Pause => State::Pause,
                    AppCmd::Resume => State::Resume,
                    AppCmd::Stop => State::Stop,
                    _ => unreachable!(),
                };
                self.cond.notify_one();
            }
            AppCmd::ConfigChanged => {
                let guard = self.mutex.lock().unwrap();
                let config = ndk_sys::AConfiguration_new();
                ndk_sys::AConfiguration_fromAssetManager(config, (*self.activity).assetManager);
                let config = Configuration::from_ptr(NonNull::new_unchecked(config));
                guard.config.replace(config);
                log::debug!("Config: {:#?}", guard.config);
            }
            AppCmd::Destroy => {
                let mut guard = self.mutex.lock().unwrap();
                guard.destroy_requested = true;
            }
            _ => {}
        }
    }

    pub unsafe fn post_exec_cmd(&self, cmd: AppCmd) {
        log::trace!("Post: AppCmd::{:#?}", cmd);
        match cmd {
            AppCmd::TermWindow => {
                let mut guard = self.mutex.lock().unwrap();
                guard.window = None;
                self.cond.notify_one();
            }
            AppCmd::SaveState => {
                let mut guard = self.mutex.lock().unwrap();
                guard.app_has_saved_state = true;
                self.cond.notify_one();
            }
            _ => {}
        }
    }
}

extern "Rust" {
    pub fn android_main(app: AndroidApp);
}

unsafe fn try_with_waitable_activity_ref(
    activity: *mut ndk_sys::ANativeActivity,
    closure: impl FnOnce(Arc<WaitableNativeActivityState>),
) {
    assert!(!(*activity).instance.is_null());
    let weak_ptr: *const WaitableNativeActivityState = (*activity).instance.cast();
    let weak_ref = Weak::from_raw(weak_ptr);
    if let Some(waitable_activity) = weak_ref.upgrade() {
        closure(waitable_activity);
    } else {
        log::error!("Ignoring spurious JVM callback after last activity reference was dropped!")
    }
    let _ = weak_ref.into_raw();
}

unsafe extern "C" fn on_destroy(activity: *mut ndk_sys::ANativeActivity) {
    abort_on_panic(|| {
        log::debug!("Destroy: {:p}\n", activity);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.notify_destroyed()
        });
    })
}

unsafe extern "C" fn on_start(activity: *mut ndk_sys::ANativeActivity) {
    abort_on_panic(|| {
        log::debug!("Start: {:p}\n", activity);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.set_activity_state(State::Start);
        });
    })
}

unsafe extern "C" fn on_resume(activity: *mut ndk_sys::ANativeActivity) {
    abort_on_panic(|| {
        log::debug!("Resume: {:p}\n", activity);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.set_activity_state(State::Resume);
        });
    })
}

unsafe extern "C" fn on_save_instance_state(
    activity: *mut ndk_sys::ANativeActivity,
    out_len: *mut ndk_sys::size_t,
) -> *mut libc::c_void {
    abort_on_panic(|| {
        log::debug!("SaveInstanceState: {:p}\n", activity);
        *out_len = 0;
        let mut ret = ptr::null_mut();
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            let (state, len) = waitable_activity.request_save_state();
            *out_len = len as ndk_sys::size_t;
            ret = state
        });

        log::debug!("Saved state = {:p}, len = {}", ret, *out_len);
        ret
    })
}

unsafe extern "C" fn on_pause(activity: *mut ndk_sys::ANativeActivity) {
    abort_on_panic(|| {
        log::debug!("Pause: {:p}\n", activity);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.set_activity_state(State::Pause);
        });
    })
}

unsafe extern "C" fn on_stop(activity: *mut ndk_sys::ANativeActivity) {
    abort_on_panic(|| {
        log::debug!("Stop: {:p}\n", activity);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.set_activity_state(State::Stop);
        });
    })
}

unsafe extern "C" fn on_configuration_changed(activity: *mut ndk_sys::ANativeActivity) {
    abort_on_panic(|| {
        log::debug!("ConfigurationChanged: {:p}\n", activity);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.notify_config_changed();
        });
    })
}

unsafe extern "C" fn on_low_memory(activity: *mut ndk_sys::ANativeActivity) {
    abort_on_panic(|| {
        log::debug!("LowMemory: {:p}\n", activity);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.notify_low_memory();
        });
    })
}

unsafe extern "C" fn on_window_focus_changed(
    activity: *mut ndk_sys::ANativeActivity,
    focused: libc::c_int,
) {
    abort_on_panic(|| {
        log::debug!("WindowFocusChanged: {:p} -- {}\n", activity, focused);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.notify_focus_changed(focused != 0);
        });
    })
}

unsafe extern "C" fn on_native_window_created(
    activity: *mut ndk_sys::ANativeActivity,
    window: *mut ndk_sys::ANativeWindow,
) {
    abort_on_panic(|| {
        log::debug!("NativeWindowCreated: {:p} -- {:p}\n", activity, window);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            // Use clone_from_ptr to acquire additional ownership on the NativeWindow,
            // which will unconditionally be _release()'d on Drop.
            let window = NativeWindow::clone_from_ptr(NonNull::new_unchecked(window));
            waitable_activity.set_window(Some(window));
        });
    })
}

unsafe extern "C" fn on_native_window_resized(
    activity: *mut ndk_sys::ANativeActivity,
    window: *mut ndk_sys::ANativeWindow,
) {
    log::debug!("NativeWindowResized: {:p} -- {:p}\n", activity, window);
    try_with_waitable_activity_ref(activity, |waitable_activity| {
        waitable_activity.notify_window_resized(window);
    });
}

unsafe extern "C" fn on_native_window_redraw_needed(
    activity: *mut ndk_sys::ANativeActivity,
    window: *mut ndk_sys::ANativeWindow,
) {
    log::debug!("NativeWindowRedrawNeeded: {:p} -- {:p}\n", activity, window);
    try_with_waitable_activity_ref(activity, |waitable_activity| {
        waitable_activity.notify_window_redraw_needed(window)
    });
}

unsafe extern "C" fn on_native_window_destroyed(
    activity: *mut ndk_sys::ANativeActivity,
    window: *mut ndk_sys::ANativeWindow,
) {
    abort_on_panic(|| {
        log::debug!("NativeWindowDestroyed: {:p} -- {:p}\n", activity, window);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.set_window(None);
        });
    })
}

unsafe extern "C" fn on_input_queue_created(
    activity: *mut ndk_sys::ANativeActivity,
    queue: *mut ndk_sys::AInputQueue,
) {
    abort_on_panic(|| {
        log::debug!("InputQueueCreated: {:p} -- {:p}\n", activity, queue);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.set_input(queue);
        });
    })
}

unsafe extern "C" fn on_input_queue_destroyed(
    activity: *mut ndk_sys::ANativeActivity,
    queue: *mut ndk_sys::AInputQueue,
) {
    abort_on_panic(|| {
        log::debug!("InputQueueDestroyed: {:p} -- {:p}\n", activity, queue);
        try_with_waitable_activity_ref(activity, |waitable_activity| {
            waitable_activity.set_input(ptr::null_mut());
        });
    })
}

unsafe extern "C" fn on_content_rect_changed(
    activity: *mut ndk_sys::ANativeActivity,
    rect: *const ndk_sys::ARect,
) {
    log::debug!("ContentRectChanged: {:p} -- {:p}\n", activity, rect);
    try_with_waitable_activity_ref(activity, |waitable_activity| {
        waitable_activity.set_content_rect(rect)
    });
}

/// This is the native entrypoint for our cdylib library that `ANativeActivity` will look for via `dlsym`
#[no_mangle]
#[allow(unused_unsafe)] // Otherwise rust 1.64 moans about using unsafe{} in unsafe functions
extern "C" fn ANativeActivity_onCreate(
    activity: *mut ndk_sys::ANativeActivity,
    saved_state: *const libc::c_void,
    saved_state_size: libc::size_t,
) {
    abort_on_panic(|| {
        // Maybe make this stdout/stderr redirection an optional / opt-in feature?...
        unsafe {
            let mut logpipe: [RawFd; 2] = Default::default();
            libc::pipe(logpipe.as_mut_ptr());
            libc::dup2(logpipe[1], libc::STDOUT_FILENO);
            libc::dup2(logpipe[1], libc::STDERR_FILENO);
            std::thread::spawn(move || {
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
        }

        log::trace!(
            "Creating: {:p}, saved_state = {:p}, save_state_size = {}",
            activity,
            saved_state,
            saved_state_size
        );

        // Conceptually we associate a glue reference with the JVM main thread, and another
        // reference with the Rust main thread
        let jvm_glue = NativeActivityGlue::new(activity, saved_state, saved_state_size);

        let rust_glue = jvm_glue.clone();
        // Let us Send the NativeActivity pointer to the Rust main() thread without a wrapper type
        let activity_ptr: libc::intptr_t = activity as _;

        // Note: we drop the thread handle which will detach the thread
        std::thread::spawn(move || {
            let activity: *mut ndk_sys::ANativeActivity = activity_ptr as *mut _;

            let jvm = unsafe {
                let na = activity;
                let jvm = (*na).vm;
                let activity = (*na).clazz; // Completely bogus name; this is the _instance_ not class pointer
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

            let app = AndroidApp::new(rust_glue.clone());

            rust_glue.notify_main_thread_running();

            unsafe {
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
                ndk_sys::ANativeActivity_finish(activity);

                if let Some(detach_current_thread) = (*(*jvm)).DetachCurrentThread {
                    detach_current_thread(jvm);
                }

                ndk_context::release_android_context();
            }

            rust_glue.notify_main_thread_stopped_running();
        });

        // Wait for thread to start.
        let mut guard = jvm_glue.mutex.lock().unwrap();

        // Don't specifically wait for `Running` just in case `android_main` returns
        // immediately and the state is set to `Stopped`
        while guard.thread_state == NativeThreadState::Init {
            guard = jvm_glue.cond.wait(guard).unwrap();
        }
    })
}
