//! A glue layer for building standalone, Rust applications on Android
//!
//! This crate provides a "glue" layer for building native Rust
//! applications on Android, supporting multiple [`Activity`] base classes.
//! It's comparable to [`android_native_app_glue.c`][ndk_concepts]
//! for C/C++ applications.
//!
//! Currently the crate supports two `Activity` base classes:
//! 1. [`NativeActivity`] - Built in to Android, this doesn't require compiling any Java or Kotlin code.
//! 2. [`GameActivity`] - From the Android Game Development Kit, it has more
//!    sophisticated input handling support than `NativeActivity`. `GameActivity`
//!    is also based on the `AndroidAppCompat` class which can help with supporting
//!    a wider range of devices.
//!
//! Standalone applications based on this crate need to be built as `cdylib` libraries, like:
//! ```
//! [lib]
//! crate_type=["cdylib"]
//! ```
//!
//! and implement a `#[no_mangle]` `android_main` entry point like this:
//! ```rust
//! #[no_mangle]
//! fn android_main(app: AndroidApp) {
//!
//! }
//! ```
//!
//! Once your application's `Activity` class has loaded and it calls `onCreate` then
//! `android-activity` will spawn a dedicated thread to run your `android_main` function,
//! separate from the Java thread that created the corresponding `Activity`.
//!
//! [`AndroidApp`] provides an interface to query state for the application as
//! well as monitor events, such as lifecycle and input events, that are
//! marshalled between the Java thread that owns the `Activity` and the native
//! thread that runs the `android_main()` code.
//!
//! # Main Thread Initialization
//!
//! Before `android_main()` is called, the following application state
//! is also initialized:
//!
//! 1. An I/O thread is spawned that will handle redirecting standard input
//!    and output to the Android log, visible via `logcat`.
//! 2. A `JavaVM` and `Activity` instance will be associated with the [`ndk_context`] crate
//!    so that other, independent, Rust crates are able to find a JavaVM
//!    for making JNI calls.
//! 3. The `JavaVM` will be attached to the native thread
//! 4. A [Looper] is attached to the Rust native thread.
//!
//!
//! These are undone after `android_main()` returns
//!
//! [`Activity`]: https://developer.android.com/reference/android/app/Activity
//! [`NativeActivity`]: https://developer.android.com/reference/android/app/NativeActivity
//! [ndk_concepts]: https://developer.android.com/ndk/guides/concepts#naa
//! [`GameActivity`]: https://developer.android.com/games/agdk/integrate-game-activity
//! [Looper]: https://developer.android.com/reference/android/os/Looper

use std::hash::Hash;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use libc::c_void;
use ndk::asset::AssetManager;
use ndk::native_window::NativeWindow;

use bitflags::bitflags;

#[cfg(not(target_os = "android"))]
compile_error!("android-activity only supports compiling for Android");

#[cfg(all(feature = "game-activity", feature = "native-activity"))]
compile_error!(
    r#"The "game-activity" and "native-activity" features cannot be enabled at the same time"#
);
#[cfg(all(
    not(any(feature = "game-activity", feature = "native-activity")),
    not(doc)
))]
compile_error!(
    r#"Either "game-activity" or "native-activity" must be enabled as features

If you have set one of these features then this error indicates that Cargo is trying to
link together multiple implementations of android-activity (with incompatible versions)
which is not supported.

Since android-activity is responsible for the `android_main` entrypoint of your application
then there can only be a single implementation of android-activity linked with your application.

You can use `cargo tree` (e.g. via `cargo ndk -t arm64-v8a tree`) to identify why multiple
versions have been resolved.

You may need to add a `[patch]` into your Cargo.toml to ensure a specific version of
android-activity is used across all of your application's crates."#
);

#[cfg(any(feature = "native-activity", doc))]
mod native_activity;
#[cfg(any(feature = "native-activity", doc))]
use native_activity as activity_impl;

#[cfg(feature = "game-activity")]
mod game_activity;
#[cfg(feature = "game-activity")]
use game_activity as activity_impl;

pub mod input;

mod config;
pub use config::ConfigurationRef;

mod util;

/// A rectangle with integer edge coordinates. Used to represent window insets, for example.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    /// An empty `Rect` with all components set to zero.
    pub fn empty() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }
}

impl From<Rect> for ndk_sys::ARect {
    fn from(rect: Rect) -> Self {
        Self {
            left: rect.left,
            right: rect.right,
            top: rect.top,
            bottom: rect.bottom,
        }
    }
}

impl From<ndk_sys::ARect> for Rect {
    fn from(arect: ndk_sys::ARect) -> Self {
        Self {
            left: arect.left,
            right: arect.right,
            top: arect.top,
            bottom: arect.bottom,
        }
    }
}

pub use activity_impl::StateLoader;
pub use activity_impl::StateSaver;

/// An application event delivered during [`AndroidApp::poll_events`]
#[non_exhaustive]
#[derive(Debug)]
pub enum MainEvent<'a> {
    /// New input events are available via [`AndroidApp::input_events()`]
    ///
    /// _Note: Even if more input is received this event will not be resent
    /// until [`AndroidApp::input_events()`] has been called, which enables
    /// applications to batch up input processing without there being lots of
    /// redundant event loop wake ups._
    ///
    /// [`AndroidApp::input_events()`]: AndroidApp::input_events
    InputAvailable,

    /// Command from main thread: a new [`NativeWindow`] is ready for use.  Upon
    /// receiving this command, [`AndroidApp::native_window()`] will return the new window
    #[non_exhaustive]
    InitWindow {},

    /// Command from main thread: the existing [`NativeWindow`] needs to be
    /// terminated.  Upon receiving this command, [`AndroidApp::native_window()`] still
    /// returns the existing window; after returning from the [`AndroidApp::poll_events()`]
    /// callback then [`AndroidApp::native_window()`] will return `None`.
    #[non_exhaustive]
    TerminateWindow {},

    // TODO: include the prev and new size in the event
    /// Command from main thread: the current [`NativeWindow`] has been resized.
    /// Please redraw with its new size.
    #[non_exhaustive]
    WindowResized {},

    /// Command from main thread: the current [`NativeWindow`] needs to be redrawn.
    /// You should redraw the window before the [`AndroidApp::poll_events()`]
    /// callback returns in order to avoid transient drawing glitches.
    #[non_exhaustive]
    RedrawNeeded {},

    /// Command from main thread: the content area of the window has changed,
    /// such as from the soft input window being shown or hidden.  You can
    /// get the new content rect by calling [`AndroidApp::content_rect()`]
    #[non_exhaustive]
    ContentRectChanged {},

    /// Command from main thread: the app's activity window has gained
    /// input focus.
    GainedFocus,

    /// Command from main thread: the app's activity window has lost
    /// input focus.
    LostFocus,

    /// Command from main thread: the current device configuration has changed.
    /// You can get a copy of the latest [`ndk::configuration::Configuration`] by calling
    /// [`AndroidApp::config()`]
    #[non_exhaustive]
    ConfigChanged {},

    /// Command from main thread: the system is running low on memory.
    /// Try to reduce your memory use.
    LowMemory,

    /// Command from main thread: the app's activity has been started.
    Start,

    /// Command from main thread: the app's activity has been resumed.
    #[non_exhaustive]
    Resume { loader: StateLoader<'a> },

    /// Command from main thread: the app should generate a new saved state
    /// for itself, to restore from later if needed.  If you have saved state,
    /// allocate it with malloc and place it in android_app.savedState with
    /// the size in android_app.savedStateSize.  The will be freed for you
    /// later.
    #[non_exhaustive]
    SaveState { saver: StateSaver<'a> },

    /// Command from main thread: the app's activity has been paused.
    Pause,

    /// Command from main thread: the app's activity has been stopped.
    Stop,

    /// Command from main thread: the app's activity is being destroyed,
    /// and waiting for the app thread to clean up and exit before proceeding.
    Destroy,

    /// Command from main thread: the app's insets have changed.
    #[non_exhaustive]
    InsetsChanged {},
}

/// An event delivered during [`AndroidApp::poll_events`]
#[derive(Debug)]
#[non_exhaustive]
pub enum PollEvent<'a> {
    Wake,
    Timeout,
    Main(MainEvent<'a>),
}

/// Indicates whether an application has handled or ignored an event
///
/// If an event is not handled by an application then some default handling may happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputStatus {
    Handled,
    Unhandled,
}

use activity_impl::AndroidAppInner;
pub use activity_impl::AndroidAppWaker;

bitflags! {
    /// Flags for [`AndroidApp::set_window_flags`]
    /// as per the [android.view.WindowManager.LayoutParams Java API](https://developer.android.com/reference/android/view/WindowManager.LayoutParams)
    pub struct WindowManagerFlags: u32 {
        /// As long as this window is visible to the user, allow the lock
        /// screen to activate while the screen is on.  This can be used
        /// independently, or in combination with
        /// [`Self::KEEP_SCREEN_ON`] and/or [`Self::SHOW_WHEN_LOCKED`]
        const ALLOW_LOCK_WHILE_SCREEN_ON = 0x00000001;

        /// Everything behind this window will be dimmed. */
        const DIM_BEHIND = 0x00000002;

        /// Blur everything behind this window.
        #[deprecated = "Blurring is no longer supported"]
        const BLUR_BEHIND = 0x00000004;

        /// This window won't ever get key input focus, so the
        /// user can not send key or other button events to it.  Those will
        /// instead go to whatever focusable window is behind it.  This flag
        /// will also enable [`Self::NOT_TOUCH_MODAL`] whether or not
        /// that is explicitly set.
        ///
        /// Setting this flag also implies that the window will not need to
        /// interact with
        /// a soft input method, so it will be Z-ordered and positioned
        /// independently of any active input method (typically this means it
        /// gets Z-ordered on top of the input method, so it can use the full
        /// screen for its content and cover the input method if needed.  You
        /// can use [`Self::ALT_FOCUSABLE_IM`] to modify this
        /// behavior.
        const NOT_FOCUSABLE = 0x00000008;

        /// This window can never receive touch events.
        const NOT_TOUCHABLE = 0x00000010;

        /// Even when this window is focusable (if
        /// [`Self::NOT_FOCUSABLE`] is not set), allow any pointer
        /// events outside of the window to be sent to the windows behind it.
        /// Otherwise it will consume all pointer events itself, regardless of
        /// whether they are inside of the window.
        const NOT_TOUCH_MODAL = 0x00000020;

        /// When set, if the device is asleep when the touch
        /// screen is pressed, you will receive this first touch event.  Usually
        /// the first touch event is consumed by the system since the user can
        /// not see what they are pressing on.
        #[deprecated]
        const TOUCHABLE_WHEN_WAKING = 0x00000040;

        /// As long as this window is visible to the user, keep
        /// the device's screen turned on and bright.
        const KEEP_SCREEN_ON = 0x00000080;

        /// Place the window within the entire screen, ignoring
        /// decorations around the border (such as the status bar).  The
        /// window must correctly position its contents to take the screen
        /// decoration into account.
        const LAYOUT_IN_SCREEN = 0x00000100;

        /// Allows the window to extend outside of the screen.
        const LAYOUT_NO_LIMITS = 0x00000200;

        /// Hide all screen decorations (such as the status
        /// bar) while this window is displayed.  This allows the window to
        /// use the entire display space for itself -- the status bar will
        /// be hidden when an app window with this flag set is on the top
        /// layer. A fullscreen window will ignore a value of
        /// [`Self::SOFT_INPUT_ADJUST_RESIZE`] the window will stay
        /// fullscreen and will not resize.
        const FULLSCREEN = 0x00000400;

        /// Override [`Self::FULLSCREEN`] and force the
        /// screen decorations (such as the status bar) to be shown.
        const FORCE_NOT_FULLSCREEN = 0x00000800;
        /// Turn on dithering when compositing this window to
        /// the screen.
        #[deprecated="This flag is no longer used"]
        const DITHER = 0x00001000;

        /// Treat the content of the window as secure, preventing
        /// it from appearing in screenshots or from being viewed on non-secure
        /// displays.
        const SECURE = 0x00002000;

        /// A special mode where the layout parameters are used
        /// to perform scaling of the surface when it is composited to the
        /// screen.
        const SCALED = 0x00004000;

        /// Intended for windows that will often be used when the user is
        /// holding the screen against their face, it will aggressively
        /// filter the event stream to prevent unintended presses in this
        /// situation that may not be desired for a particular window, when
        /// such an event stream is detected, the application will receive
        /// a `AMOTION_EVENT_ACTION_CANCEL` to indicate this so
        /// applications can handle this accordingly by taking no action on
        /// the event until the finger is released.
        const IGNORE_CHEEK_PRESSES = 0x00008000;

        /// A special option only for use in combination with
        /// [`Self::LAYOUT_IN_SCREEN`].  When requesting layout in
        /// the screen your window may appear on top of or behind screen decorations
        /// such as the status bar.  By also including this flag, the window
        /// manager will report the inset rectangle needed to ensure your
        /// content is not covered by screen decorations.
        const LAYOUT_INSET_DECOR = 0x00010000;

        /// Invert the state of [`Self::NOT_FOCUSABLE`] with
        /// respect to how this window interacts with the current method.
        /// That is, if [`Self::NOT_FOCUSABLE`] is set and this flag is set,
        /// then the window will behave as if it needs to interact with the
        /// input method and thus be placed behind/away from it; if
        /// [`Self::NOT_FOCUSABLE`] is not set and this flag is set,
        /// then the window will behave as if it doesn't need to interact
        /// with the input method and can be placed to use more space and
        /// cover the input method.
        const ALT_FOCUSABLE_IM = 0x00020000;

        /// If you have set [`Self::NOT_TOUCH_MODAL`], you
        /// can set this flag to receive a single special MotionEvent with
        /// the action
        /// `AMOTION_EVENT_ACTION_OUTSIDE` for
        /// touches that occur outside of your window.  Note that you will not
        /// receive the full down/move/up gesture, only the location of the
        /// first down as an `AMOTION_EVENT_ACTION_OUTSIDE`.
        const WATCH_OUTSIDE_TOUCH = 0x00040000;

        /// Special flag to let windows be shown when the screen
        /// is locked. This will let application windows take precedence over
        /// key guard or any other lock screens. Can be used with
        /// [`Self::KEEP_SCREEN_ON`] to turn screen on and display
        /// windows directly before showing the key guard window.  Can be used with
        /// [`Self::DISMISS_KEYGUARD`] to automatically fully
        /// dismiss non-secure key guards.  This flag only applies to the top-most
        /// full-screen window.
        const SHOW_WHEN_LOCKED = 0x00080000;

        /// Ask that the system wallpaper be shown behind
        /// your window.  The window surface must be translucent to be able
        /// to actually see the wallpaper behind it; this flag just ensures
        /// that the wallpaper surface will be there if this window actually
        /// has translucent regions.
        const SHOW_WALLPAPER = 0x00100000;

        /// When set as a window is being added or made
        /// visible, once the window has been shown then the system will
        /// poke the power manager's user activity (as if the user had woken
        /// up the device) to turn the screen on.
        const TURN_SCREEN_ON = 0x00200000;

        /// When set the window will cause the key guard to
        /// be dismissed, only if it is not a secure lock key guard.  Because such
        /// a key guard is not needed for security, it will never re-appear if
        /// the user navigates to another window (in contrast to
        /// [`Self::SHOW_WHEN_LOCKED`], which will only temporarily
        /// hide both secure and non-secure key guards but ensure they reappear
        /// when the user moves to another UI that doesn't hide them).
        /// If the key guard is currently active and is secure (requires an
        /// unlock pattern) then the user will still need to confirm it before
        /// seeing this window, unless [`Self::SHOW_WHEN_LOCKED`] has
        /// also been set.
        const DISMISS_KEYGUARD = 0x00400000;
    }
}

/// The top-level state and interface for a native Rust application
///
/// `AndroidApp` provides an interface to query state for the application as
/// well as monitor events, such as lifecycle and input events, that are
/// marshalled between the Java thread that owns the `Activity` and the native
/// thread that runs the `android_main()` code.
///
#[derive(Debug, Clone)]
pub struct AndroidApp {
    pub(crate) inner: Arc<RwLock<AndroidAppInner>>,
}

impl PartialEq for AndroidApp {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for AndroidApp {}

impl Hash for AndroidApp {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.inner).hash(state);
    }
}

impl AndroidApp {
    /// Queries the current [`NativeWindow`] for the application.
    ///
    /// This will only return `Some(window)` between
    /// [`MainEvent::InitWindow`] and [`MainEvent::TerminateWindow`]
    /// events.
    pub fn native_window(&self) -> Option<NativeWindow> {
        self.inner.read().unwrap().native_window()
    }

    /// Returns a pointer to the Java Virtual Machine, for making JNI calls
    ///
    /// This returns a pointer to the Java Virtual Machine which can be used
    /// with the [`jni`] crate (or similar crates) to make JNI calls that bridge
    /// between native Rust code and Java/Kotlin code running within the JVM.
    ///
    /// If you use the [`jni`] crate you can wrap this as a [`JavaVM`] via:
    /// ```ignore
    /// # use jni::JavaVM;
    /// # let app: AndroidApp = todo!();
    /// let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr()) };
    /// ```
    ///
    /// [`jni`]: https://crates.io/crates/jni
    /// [`JavaVM`]: https://docs.rs/jni/latest/jni/struct.JavaVM.html
    pub fn vm_as_ptr(&self) -> *mut c_void {
        self.inner.read().unwrap().vm_as_ptr()
    }

    /// Returns a JNI object reference for this application's JVM `Activity` as a pointer
    ///
    /// If you use the [`jni`] crate you can wrap this as an object reference via:
    /// ```ignore
    /// # use jni::objects::JObject;
    /// # let app: AndroidApp = todo!();
    /// let activity = unsafe { JObject::from_raw(app.activity_as_ptr()) };
    /// ```
    ///
    /// # JNI Safety
    ///
    /// Note that the object reference will be a JNI global reference, not a
    /// local reference and it should not be deleted. Don't wrap the reference
    /// in an [`AutoLocal`] which would try to explicitly delete the reference
    /// when dropped. Similarly, don't wrap the reference as a [`GlobalRef`]
    /// which would also try to explicitly delete the reference when dropped.
    ///
    /// [`jni`]: https://crates.io/crates/jni
    /// [`AutoLocal`]: https://docs.rs/jni/latest/jni/objects/struct.AutoLocal.html
    /// [`GlobalRef`]: https://docs.rs/jni/latest/jni/objects/struct.GlobalRef.html
    pub fn activity_as_ptr(&self) -> *mut c_void {
        self.inner.read().unwrap().activity_as_ptr()
    }

    /// Polls for any events associated with this [AndroidApp] and processes those events
    /// (such as lifecycle events) via the given `callback`.
    ///
    /// It's important to use this API for polling, and not call [`ALooper_pollAll`] directly since
    /// some events require pre- and post-processing either side of the callback. For correct
    /// behavior events should be handled immediately, before returning from the callback and
    /// not simply queued for batch processing later. For example the existing [`NativeWindow`]
    /// is accessible during a [`MainEvent::TerminateWindow`] callback and will be
    /// set to `None` once the callback returns, and this is also synchronized with the Java
    /// main thread. The [`MainEvent::SaveState`] event is also synchronized with the
    /// Java main thread.
    ///
    /// # Panics
    ///
    /// This must only be called from your `android_main()` thread and it may panic if called
    /// from another thread.
    ///
    /// [`ALooper_pollAll`]: ndk::looper::ThreadLooper::poll_all
    pub fn poll_events<F>(&self, timeout: Option<Duration>, callback: F)
    where
        F: FnMut(PollEvent),
    {
        self.inner.read().unwrap().poll_events(timeout, callback);
    }

    /// Creates a means to wake up the main loop while it is blocked waiting for
    /// events within [`AndroidApp::poll_events()`].
    pub fn create_waker(&self) -> activity_impl::AndroidAppWaker {
        self.inner.read().unwrap().create_waker()
    }

    /// Returns a (cheaply clonable) reference to this application's [`ndk::configuration::Configuration`]
    pub fn config(&self) -> ConfigurationRef {
        self.inner.read().unwrap().config()
    }

    /// Queries the current content rectangle of the window; this is the area where the
    /// window's content should be placed to be seen by the user.
    pub fn content_rect(&self) -> Rect {
        self.inner.read().unwrap().content_rect()
    }

    /// Queries the Asset Manager instance for the application.
    ///
    /// Use this to access binary assets bundled inside your application's .apk file.
    pub fn asset_manager(&self) -> AssetManager {
        self.inner.read().unwrap().asset_manager()
    }

    /// Change the window flags of the given activity.
    ///
    /// Note that some flags must be set before the window decoration is created,
    /// see
    /// `<https://developer.android.com/reference/android/view/Window#setFlags(int,%20int)>`.
    pub fn set_window_flags(
        &self,
        add_flags: WindowManagerFlags,
        remove_flags: WindowManagerFlags,
    ) {
        self.inner
            .write()
            .unwrap()
            .set_window_flags(add_flags, remove_flags);
    }

    /// Enable additional input axis
    ///
    /// To reduce overhead, by default only [`input::Axis::X`] and [`input::Axis::Y`] are enabled
    /// and other axis should be enabled explicitly.
    pub fn enable_motion_axis(&self, axis: input::Axis) {
        self.inner.write().unwrap().enable_motion_axis(axis);
    }

    /// Disable input axis
    ///
    /// To reduce overhead, by default only [`input::Axis::X`] and [`input::Axis::Y`] are enabled
    /// and other axis should be enabled explicitly.
    pub fn disable_motion_axis(&self, axis: input::Axis) {
        self.inner.write().unwrap().disable_motion_axis(axis);
    }

    /// Explicitly request that the current input method's soft input area be
    /// shown to the user, if needed.
    ///
    /// Call this if the user interacts with your view in such a way that they
    /// have expressed they would like to start performing input into it.
    pub fn show_soft_input(&self, show_implicit: bool) {
        self.inner.read().unwrap().show_soft_input(show_implicit);
    }

    /// Request to hide the soft input window from the context of the window
    /// that is currently accepting input.
    ///
    /// This should be called as a result of the user doing some action that
    /// fairly explicitly requests to have the input window hidden.
    pub fn hide_soft_input(&self, hide_implicit_only: bool) {
        self.inner
            .read()
            .unwrap()
            .hide_soft_input(hide_implicit_only);
    }

    /// Query and process all out-standing input event
    ///
    /// `callback` should return [`InputStatus::Unhandled`] for any input events that aren't directly
    /// handled by the application, or else [`InputStatus::Handled`]. Unhandled events may lead to a
    /// fallback interpretation of the event.
    ///
    /// Applications are generally either expected to call this in-sync with their rendering or
    /// in response to a [`MainEvent::InputAvailable`] event being delivered. _Note though that your
    /// application is will only be delivered a single [`MainEvent::InputAvailable`] event between calls
    /// to this API._
    ///
    /// To reduce overhead, by default only [`input::Axis::X`] and [`input::Axis::Y`] are enabled
    /// and other axis should be enabled explicitly via [`Self::enable_motion_axis`].
    pub fn input_events<F>(&self, callback: F)
    where
        F: FnMut(&input::InputEvent) -> InputStatus,
    {
        self.inner.read().unwrap().input_events(callback)
    }

    /// The user-visible SDK version of the framework
    ///
    /// Also referred to as [`Build.VERSION_CODES`](https://developer.android.com/reference/android/os/Build.VERSION_CODES)
    pub fn sdk_version() -> i32 {
        let mut prop = android_properties::getprop("ro.build.version.sdk");
        if let Some(val) = prop.value() {
            val.parse::<i32>()
                .expect("Failed to parse ro.build.version.sdk property")
        } else {
            panic!("Couldn't read ro.build.version.sdk system property");
        }
    }

    /// Path to this application's internal data directory
    pub fn internal_data_path(&self) -> Option<std::path::PathBuf> {
        self.inner.read().unwrap().internal_data_path()
    }

    /// Path to this application's external data directory
    pub fn external_data_path(&self) -> Option<std::path::PathBuf> {
        self.inner.read().unwrap().external_data_path()
    }

    /// Path to the directory containing the application's OBB files (if any).
    pub fn obb_path(&self) -> Option<std::path::PathBuf> {
        self.inner.read().unwrap().obb_path()
    }
}

#[test]
fn test_app_is_send_sync() {
    fn needs_send_sync<T: Send + Sync>() {}
    needs_send_sync::<AndroidApp>();
}
