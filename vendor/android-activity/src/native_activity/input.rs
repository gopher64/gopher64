use std::marker::PhantomData;

pub use ndk::event::{
    Axis, ButtonState, EdgeFlags, KeyAction, KeyEventFlags, Keycode, MetaState, MotionAction,
    MotionEventFlags, Pointer, PointersIter,
};

use crate::input::{Class, Source};

/// A motion event
///
/// For general discussion of motion events in Android, see [the relevant
/// javadoc](https://developer.android.com/reference/android/view/MotionEvent).
#[derive(Debug)]
#[repr(transparent)]
pub struct MotionEvent<'a> {
    ndk_event: ndk::event::MotionEvent,
    _lifetime: PhantomData<&'a ndk::event::MotionEvent>,
}
impl<'a> MotionEvent<'a> {
    pub(crate) fn new(ndk_event: ndk::event::MotionEvent) -> Self {
        Self {
            ndk_event,
            _lifetime: PhantomData,
        }
    }
    pub(crate) fn into_ndk_event(self) -> ndk::event::MotionEvent {
        self.ndk_event
    }

    /// Get the source of the event.
    ///
    #[inline]
    pub fn source(&self) -> Source {
        // XXX: we use `AInputEvent_getSource` directly (instead of calling
        // ndk_event.source()) since we have our own `Source` enum that we
        // share between backends, which may not exactly match the ndk crate's
        // `Source` enum.
        let source =
            unsafe { ndk_sys::AInputEvent_getSource(self.ndk_event.ptr().as_ptr()) as u32 };
        source.try_into().unwrap_or(Source::Unknown)
    }

    /// Get the class of the event source.
    ///
    #[inline]
    pub fn class(&self) -> Class {
        Class::from(self.source())
    }

    /// Get the device id associated with the event.
    ///
    #[inline]
    pub fn device_id(&self) -> i32 {
        self.ndk_event.device_id()
    }

    /// Returns the motion action associated with the event.
    ///
    /// See [the MotionEvent docs](https://developer.android.com/reference/android/view/MotionEvent#getActionMasked())
    #[inline]
    pub fn action(&self) -> MotionAction {
        self.ndk_event.action()
    }

    /// Returns the pointer index of an `Up` or `Down` event.
    ///
    /// Pointer indices can change per motion event.  For an identifier that stays the same, see
    /// [`Pointer::pointer_id()`].
    ///
    /// This only has a meaning when the [action](Self::action) is one of [`Up`](MotionAction::Up),
    /// [`Down`](MotionAction::Down), [`PointerUp`](MotionAction::PointerUp),
    /// or [`PointerDown`](MotionAction::PointerDown).
    #[inline]
    pub fn pointer_index(&self) -> usize {
        self.ndk_event.pointer_index()
    }

    /*
    /// Returns the pointer id associated with the given pointer index.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getpointerid)
    // TODO: look at output with out-of-range pointer index
    // Probably -1 though
    pub fn pointer_id_for(&self, pointer_index: usize) -> i32 {
        unsafe { ndk_sys::AMotionEvent_getPointerId(self.ndk_event.ptr.as_ptr(), pointer_index) }
    }
    */

    /// Returns the number of pointers in this event
    ///
    /// See [the MotionEvent docs](https://developer.android.com/reference/android/view/MotionEvent#getPointerCount())
    #[inline]
    pub fn pointer_count(&self) -> usize {
        self.ndk_event.pointer_count()
    }

    /// An iterator over the pointers in this motion event
    #[inline]
    pub fn pointers(&self) -> PointersIter<'_> {
        self.ndk_event.pointers()
    }

    /// The pointer at a given pointer index. Panics if the pointer index is out of bounds.
    ///
    /// If you need to loop over all the pointers, prefer the [`pointers()`](Self::pointers) method.
    #[inline]
    pub fn pointer_at_index(&self, index: usize) -> Pointer<'_> {
        self.ndk_event.pointer_at_index(index)
    }

    /*
    XXX: Not currently supported with GameActivity so we don't currently expose for NativeActivity
    either, for consistency.

    /// Returns the size of the history contained in this event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_gethistorysize)
    #[inline]
    pub fn history_size(&self) -> usize {
        self.ndk_event.history_size()
    }

    /// An iterator over the historical events contained in this event.
    #[inline]
    pub fn history(&self) -> HistoricalMotionEventsIter<'_> {
        self.ndk_event.history()
    }
    */

    /// Returns the state of any modifier keys that were pressed during the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getmetastate)
    #[inline]
    pub fn meta_state(&self) -> MetaState {
        self.ndk_event.meta_state()
    }

    /// Returns the button state during this event, as a bitfield.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getbuttonstate)
    #[inline]
    pub fn button_state(&self) -> ButtonState {
        self.ndk_event.button_state()
    }

    /// Returns the time of the start of this gesture, in the `java.lang.System.nanoTime()` time
    /// base
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getdowntime)
    #[inline]
    pub fn down_time(&self) -> i64 {
        self.ndk_event.down_time()
    }

    /// Returns a bitfield indicating which edges were touched by this event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getedgeflags)
    #[inline]
    pub fn edge_flags(&self) -> EdgeFlags {
        self.ndk_event.edge_flags()
    }

    /// Returns the time of this event, in the `java.lang.System.nanoTime()` time base
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_geteventtime)
    #[inline]
    pub fn event_time(&self) -> i64 {
        self.ndk_event.event_time()
    }

    /// The flags associated with a motion event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getflags)
    #[inline]
    pub fn flags(&self) -> MotionEventFlags {
        self.ndk_event.flags()
    }

    /* Missing from GameActivity currently...
    /// Returns the offset in the x direction between the coordinates and the raw coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getxoffset)
    #[inline]
    pub fn x_offset(&self) -> f32 {
        self.ndk_event.x_offset()
    }

    /// Returns the offset in the y direction between the coordinates and the raw coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getyoffset)
    #[inline]
    pub fn y_offset(&self) -> f32 {
        self.ndk_event.y_offset()
    }
    */

    /// Returns the precision of the x value of the coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getxprecision)
    #[inline]
    pub fn x_precision(&self) -> f32 {
        self.ndk_event.x_precision()
    }

    /// Returns the precision of the y value of the coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getyprecision)
    #[inline]
    pub fn y_precision(&self) -> f32 {
        self.ndk_event.y_precision()
    }
}

/// A key event
///
/// For general discussion of key events in Android, see [the relevant
/// javadoc](https://developer.android.com/reference/android/view/KeyEvent).
#[derive(Debug)]
#[repr(transparent)]
pub struct KeyEvent<'a> {
    ndk_event: ndk::event::KeyEvent,
    _lifetime: PhantomData<&'a ndk::event::KeyEvent>,
}
impl<'a> KeyEvent<'a> {
    pub(crate) fn new(ndk_event: ndk::event::KeyEvent) -> Self {
        Self {
            ndk_event,
            _lifetime: PhantomData,
        }
    }
    pub(crate) fn into_ndk_event(self) -> ndk::event::KeyEvent {
        self.ndk_event
    }

    /// Get the source of the event.
    ///
    #[inline]
    pub fn source(&self) -> Source {
        // XXX: we use `AInputEvent_getSource` directly (instead of calling
        // ndk_event.source()) since we have our own `Source` enum that we
        // share between backends, which may not exactly match the ndk crate's
        // `Source` enum.
        let source =
            unsafe { ndk_sys::AInputEvent_getSource(self.ndk_event.ptr().as_ptr()) as u32 };
        source.try_into().unwrap_or(Source::Unknown)
    }

    /// Get the class of the event source.
    ///
    #[inline]
    pub fn class(&self) -> Class {
        Class::from(self.source())
    }

    /// Get the device id associated with the event.
    ///
    #[inline]
    pub fn device_id(&self) -> i32 {
        self.ndk_event.device_id()
    }

    /// Returns the key action associated with the event.
    ///
    /// See [the KeyEvent docs](https://developer.android.com/reference/android/view/KeyEvent#getAction())
    #[inline]
    pub fn action(&self) -> KeyAction {
        self.ndk_event.action()
    }

    /// Returns the last time the key was pressed.  This is on the scale of
    /// `java.lang.System.nanoTime()`, which has nanosecond precision, but no defined start time.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getdowntime)
    #[inline]
    pub fn down_time(&self) -> i64 {
        self.ndk_event.down_time()
    }

    /// Returns the time this event occured.  This is on the scale of
    /// `java.lang.System.nanoTime()`, which has nanosecond precision, but no defined start time.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_geteventtime)
    #[inline]
    pub fn event_time(&self) -> i64 {
        self.ndk_event.event_time()
    }

    /// Returns the keycode associated with this key event
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getkeycode)
    #[inline]
    pub fn key_code(&self) -> Keycode {
        self.ndk_event.key_code()
    }

    /// Returns the number of repeats of a key.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getrepeatcount)
    #[inline]
    pub fn repeat_count(&self) -> i32 {
        self.ndk_event.repeat_count()
    }

    /// Returns the hardware keycode of a key.  This varies from device to device.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getscancode)
    #[inline]
    pub fn scan_code(&self) -> i32 {
        self.ndk_event.scan_code()
    }
}

// We use our own wrapper type for input events to have better consistency
// with GameActivity and ensure the enum can be extended without needing a
// semver bump
/// Enum of possible input events
#[derive(Debug)]
#[non_exhaustive]
pub enum InputEvent<'a> {
    MotionEvent(self::MotionEvent<'a>),
    KeyEvent(self::KeyEvent<'a>),
}
