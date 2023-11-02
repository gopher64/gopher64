// GameActivity handles input events by double buffering MotionEvent and KeyEvent structs which
// essentially just mirror state from the corresponding Java objects.
//
// See also the javadocs for
// [`android.view.InputEvent`](https://developer.android.com/reference/android/view/InputEvent.html),
// [`android.view.MotionEvent`](https://developer.android.com/reference/android/view/MotionEvent.html),
// and [`android.view.KeyEvent`](https://developer.android.com/reference/android/view/KeyEvent).
//
// This code is mostly based on https://github.com/rust-windowing/android-ndk-rs/blob/master/ndk/src/event.rs
//
// The `Source` enum was defined based on the Java docs since there are a couple of source types that
// aren't exposed via the AInputQueue API
// The `Class` was also bound differently to `android-ndk-rs` considering how the class is defined
// by masking bits from the `Source`.

use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::{convert::TryInto, ops::Deref};

use crate::game_activity::ffi::{GameActivityKeyEvent, GameActivityMotionEvent};
use crate::input::{Class, Source};

// Note: try to keep this wrapper API compatible with the AInputEvent API if possible

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InputEvent<'a> {
    MotionEvent(MotionEvent<'a>),
    KeyEvent(KeyEvent<'a>),
}

/// A bitfield representing the state of modifier keys during an event.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-25)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MetaState(pub u32);

impl MetaState {
    #[inline]
    pub fn alt_on(self) -> bool {
        self.0 & ndk_sys::AMETA_ALT_ON != 0
    }
    #[inline]
    pub fn alt_left_on(self) -> bool {
        self.0 & ndk_sys::AMETA_ALT_LEFT_ON != 0
    }
    #[inline]
    pub fn alt_right_on(self) -> bool {
        self.0 & ndk_sys::AMETA_ALT_RIGHT_ON != 0
    }
    #[inline]
    pub fn shift_on(self) -> bool {
        self.0 & ndk_sys::AMETA_SHIFT_ON != 0
    }
    #[inline]
    pub fn shift_left_on(self) -> bool {
        self.0 & ndk_sys::AMETA_SHIFT_LEFT_ON != 0
    }
    #[inline]
    pub fn shift_right_on(self) -> bool {
        self.0 & ndk_sys::AMETA_SHIFT_RIGHT_ON != 0
    }
    #[inline]
    pub fn sym_on(self) -> bool {
        self.0 & ndk_sys::AMETA_SYM_ON != 0
    }
    #[inline]
    pub fn function_on(self) -> bool {
        self.0 & ndk_sys::AMETA_FUNCTION_ON != 0
    }
    #[inline]
    pub fn ctrl_on(self) -> bool {
        self.0 & ndk_sys::AMETA_CTRL_ON != 0
    }
    #[inline]
    pub fn ctrl_left_on(self) -> bool {
        self.0 & ndk_sys::AMETA_CTRL_LEFT_ON != 0
    }
    #[inline]
    pub fn ctrl_right_on(self) -> bool {
        self.0 & ndk_sys::AMETA_CTRL_RIGHT_ON != 0
    }
    #[inline]
    pub fn meta_on(self) -> bool {
        self.0 & ndk_sys::AMETA_META_ON != 0
    }
    #[inline]
    pub fn meta_left_on(self) -> bool {
        self.0 & ndk_sys::AMETA_META_LEFT_ON != 0
    }
    #[inline]
    pub fn meta_right_on(self) -> bool {
        self.0 & ndk_sys::AMETA_META_RIGHT_ON != 0
    }
    #[inline]
    pub fn caps_lock_on(self) -> bool {
        self.0 & ndk_sys::AMETA_CAPS_LOCK_ON != 0
    }
    #[inline]
    pub fn num_lock_on(self) -> bool {
        self.0 & ndk_sys::AMETA_NUM_LOCK_ON != 0
    }
    #[inline]
    pub fn scroll_lock_on(self) -> bool {
        self.0 & ndk_sys::AMETA_SCROLL_LOCK_ON != 0
    }
}

/// A motion event.
///
/// For general discussion of motion events in Android, see [the relevant
/// javadoc](https://developer.android.com/reference/android/view/MotionEvent).
#[derive(Clone, Debug)]
pub struct MotionEvent<'a> {
    ga_event: &'a GameActivityMotionEvent,
}

impl<'a> Deref for MotionEvent<'a> {
    type Target = GameActivityMotionEvent;

    fn deref(&self) -> &Self::Target {
        self.ga_event
    }
}

/// A motion action.
///
/// See [the NDK
/// docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-29)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum MotionAction {
    Down = ndk_sys::AMOTION_EVENT_ACTION_DOWN,
    Up = ndk_sys::AMOTION_EVENT_ACTION_UP,
    Move = ndk_sys::AMOTION_EVENT_ACTION_MOVE,
    Cancel = ndk_sys::AMOTION_EVENT_ACTION_CANCEL,
    Outside = ndk_sys::AMOTION_EVENT_ACTION_OUTSIDE,
    PointerDown = ndk_sys::AMOTION_EVENT_ACTION_POINTER_DOWN,
    PointerUp = ndk_sys::AMOTION_EVENT_ACTION_POINTER_UP,
    HoverMove = ndk_sys::AMOTION_EVENT_ACTION_HOVER_MOVE,
    Scroll = ndk_sys::AMOTION_EVENT_ACTION_SCROLL,
    HoverEnter = ndk_sys::AMOTION_EVENT_ACTION_HOVER_ENTER,
    HoverExit = ndk_sys::AMOTION_EVENT_ACTION_HOVER_EXIT,
    ButtonPress = ndk_sys::AMOTION_EVENT_ACTION_BUTTON_PRESS,
    ButtonRelease = ndk_sys::AMOTION_EVENT_ACTION_BUTTON_RELEASE,
}

/// An axis of a motion event.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-32)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Axis {
    X = ndk_sys::AMOTION_EVENT_AXIS_X,
    Y = ndk_sys::AMOTION_EVENT_AXIS_Y,
    Pressure = ndk_sys::AMOTION_EVENT_AXIS_PRESSURE,
    Size = ndk_sys::AMOTION_EVENT_AXIS_SIZE,
    TouchMajor = ndk_sys::AMOTION_EVENT_AXIS_TOUCH_MAJOR,
    TouchMinor = ndk_sys::AMOTION_EVENT_AXIS_TOUCH_MINOR,
    ToolMajor = ndk_sys::AMOTION_EVENT_AXIS_TOOL_MAJOR,
    ToolMinor = ndk_sys::AMOTION_EVENT_AXIS_TOOL_MINOR,
    Orientation = ndk_sys::AMOTION_EVENT_AXIS_ORIENTATION,
    Vscroll = ndk_sys::AMOTION_EVENT_AXIS_VSCROLL,
    Hscroll = ndk_sys::AMOTION_EVENT_AXIS_HSCROLL,
    Z = ndk_sys::AMOTION_EVENT_AXIS_Z,
    Rx = ndk_sys::AMOTION_EVENT_AXIS_RX,
    Ry = ndk_sys::AMOTION_EVENT_AXIS_RY,
    Rz = ndk_sys::AMOTION_EVENT_AXIS_RZ,
    HatX = ndk_sys::AMOTION_EVENT_AXIS_HAT_X,
    HatY = ndk_sys::AMOTION_EVENT_AXIS_HAT_Y,
    Ltrigger = ndk_sys::AMOTION_EVENT_AXIS_LTRIGGER,
    Rtrigger = ndk_sys::AMOTION_EVENT_AXIS_RTRIGGER,
    Throttle = ndk_sys::AMOTION_EVENT_AXIS_THROTTLE,
    Rudder = ndk_sys::AMOTION_EVENT_AXIS_RUDDER,
    Wheel = ndk_sys::AMOTION_EVENT_AXIS_WHEEL,
    Gas = ndk_sys::AMOTION_EVENT_AXIS_GAS,
    Brake = ndk_sys::AMOTION_EVENT_AXIS_BRAKE,
    Distance = ndk_sys::AMOTION_EVENT_AXIS_DISTANCE,
    Tilt = ndk_sys::AMOTION_EVENT_AXIS_TILT,
    Scroll = ndk_sys::AMOTION_EVENT_AXIS_SCROLL,
    RelativeX = ndk_sys::AMOTION_EVENT_AXIS_RELATIVE_X,
    RelativeY = ndk_sys::AMOTION_EVENT_AXIS_RELATIVE_Y,
    Generic1 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_1,
    Generic2 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_2,
    Generic3 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_3,
    Generic4 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_4,
    Generic5 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_5,
    Generic6 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_6,
    Generic7 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_7,
    Generic8 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_8,
    Generic9 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_9,
    Generic10 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_10,
    Generic11 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_11,
    Generic12 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_12,
    Generic13 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_13,
    Generic14 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_14,
    Generic15 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_15,
    Generic16 = ndk_sys::AMOTION_EVENT_AXIS_GENERIC_16,
}

/// The tool type of a pointer.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-48)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum ToolType {
    Unknown = ndk_sys::AMOTION_EVENT_TOOL_TYPE_UNKNOWN,
    Finger = ndk_sys::AMOTION_EVENT_TOOL_TYPE_FINGER,
    Stylus = ndk_sys::AMOTION_EVENT_TOOL_TYPE_STYLUS,
    Mouse = ndk_sys::AMOTION_EVENT_TOOL_TYPE_MOUSE,
    Eraser = ndk_sys::AMOTION_EVENT_TOOL_TYPE_ERASER,
    Palm = ndk_sys::AMOTION_EVENT_TOOL_TYPE_PALM,
}

/// A bitfield representing the state of buttons during a motion event.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-33)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ButtonState(pub u32);

impl ButtonState {
    #[inline]
    pub fn primary(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_BUTTON_PRIMARY != 0
    }
    #[inline]
    pub fn secondary(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_BUTTON_SECONDARY != 0
    }
    #[inline]
    pub fn teriary(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_BUTTON_TERTIARY != 0
    }
    #[inline]
    pub fn back(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_BUTTON_BACK != 0
    }
    #[inline]
    pub fn forward(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_BUTTON_FORWARD != 0
    }
    #[inline]
    pub fn stylus_primary(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_BUTTON_STYLUS_PRIMARY != 0
    }
    #[inline]
    pub fn stylus_secondary(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_BUTTON_STYLUS_SECONDARY != 0
    }
}

/// A bitfield representing which edges were touched by a motion event.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-31)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EdgeFlags(pub u32);

impl EdgeFlags {
    #[inline]
    pub fn top(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_EDGE_FLAG_TOP != 0
    }
    #[inline]
    pub fn bottom(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_EDGE_FLAG_BOTTOM != 0
    }
    #[inline]
    pub fn left(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_EDGE_FLAG_LEFT != 0
    }
    #[inline]
    pub fn right(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_EDGE_FLAG_RIGHT != 0
    }
}

/// Flags associated with this [`MotionEvent`].
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-30)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MotionEventFlags(pub u32);

impl MotionEventFlags {
    #[inline]
    pub fn window_is_obscured(self) -> bool {
        self.0 & ndk_sys::AMOTION_EVENT_FLAG_WINDOW_IS_OBSCURED != 0
    }
}

impl<'a> MotionEvent<'a> {
    pub(crate) fn new(ga_event: &'a GameActivityMotionEvent) -> Self {
        Self { ga_event }
    }

    /// Get the source of the event.
    ///
    #[inline]
    pub fn source(&self) -> Source {
        let source = self.source as u32;
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
        self.deviceId
    }

    /// Returns the motion action associated with the event.
    ///
    /// See [the MotionEvent docs](https://developer.android.com/reference/android/view/MotionEvent#getActionMasked())
    #[inline]
    pub fn action(&self) -> MotionAction {
        let action = self.action as u32 & ndk_sys::AMOTION_EVENT_ACTION_MASK;
        action.try_into().unwrap()
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
        let action = self.action as u32;
        let index = (action & ndk_sys::AMOTION_EVENT_ACTION_POINTER_INDEX_MASK)
            >> ndk_sys::AMOTION_EVENT_ACTION_POINTER_INDEX_SHIFT;
        index as usize
    }

    /*
    /// Returns the pointer id associated with the given pointer index.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getpointerid)
    // TODO: look at output with out-of-range pointer index
    // Probably -1 though
    pub fn pointer_id_for(&self, pointer_index: usize) -> i32 {
        unsafe { ndk_sys::AMotionEvent_getPointerId(self.ptr.as_ptr(), pointer_index) }
    }
    */

    /// Returns the number of pointers in this event
    ///
    /// See [the MotionEvent docs](https://developer.android.com/reference/android/view/MotionEvent#getPointerCount())
    #[inline]
    pub fn pointer_count(&self) -> usize {
        self.pointerCount as usize
    }

    /// An iterator over the pointers in this motion event
    #[inline]
    pub fn pointers(&self) -> PointersIter<'_> {
        PointersIter {
            event: self,
            next_index: 0,
            count: self.pointer_count(),
        }
    }

    /// The pointer at a given pointer index. Panics if the pointer index is out of bounds.
    ///
    /// If you need to loop over all the pointers, prefer the [`pointers()`](Self::pointers) method.
    #[inline]
    pub fn pointer_at_index(&self, index: usize) -> Pointer<'_> {
        if index >= self.pointer_count() {
            panic!("Pointer index {} is out of bounds", index);
        }
        Pointer { event: self, index }
    }

    /*
    /// Returns the size of the history contained in this event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_gethistorysize)
    #[inline]
    pub fn history_size(&self) -> usize {
        unsafe { ndk_sys::AMotionEvent_getHistorySize(self.ptr.as_ptr()) as usize }
    }

    /// An iterator over the historical events contained in this event.
    #[inline]
    pub fn history(&self) -> HistoricalMotionEventsIter<'_> {
        HistoricalMotionEventsIter {
            event: self.ptr,
            next_history_index: 0,
            history_size: self.history_size(),
            _marker: std::marker::PhantomData,
        }
    }
    */

    /// Returns the state of any modifier keys that were pressed during the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getmetastate)
    #[inline]
    pub fn meta_state(&self) -> MetaState {
        MetaState(self.metaState as u32)
    }

    /// Returns the button state during this event, as a bitfield.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getbuttonstate)
    #[inline]
    pub fn button_state(&self) -> ButtonState {
        ButtonState(self.buttonState as u32)
    }

    /// Returns the time of the start of this gesture, in the `java.lang.System.nanoTime()` time
    /// base
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getdowntime)
    #[inline]
    pub fn down_time(&self) -> i64 {
        self.downTime
    }

    /// Returns a bitfield indicating which edges were touched by this event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getedgeflags)
    #[inline]
    pub fn edge_flags(&self) -> EdgeFlags {
        EdgeFlags(self.edgeFlags as u32)
    }

    /// Returns the time of this event, in the `java.lang.System.nanoTime()` time base
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_geteventtime)
    #[inline]
    pub fn event_time(&self) -> i64 {
        self.eventTime
    }

    /// The flags associated with a motion event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getflags)
    #[inline]
    pub fn flags(&self) -> MotionEventFlags {
        MotionEventFlags(self.flags as u32)
    }

    /* Missing from GameActivity currently...
    /// Returns the offset in the x direction between the coordinates and the raw coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getxoffset)
    #[inline]
    pub fn x_offset(&self) -> f32 {
        self.x_offset
    }

    /// Returns the offset in the y direction between the coordinates and the raw coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getyoffset)
    #[inline]
    pub fn y_offset(&self) -> f32 {
        self.y_offset
    }
    */

    /// Returns the precision of the x value of the coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getxprecision)
    #[inline]
    pub fn x_precision(&self) -> f32 {
        self.precisionX
    }

    /// Returns the precision of the y value of the coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getyprecision)
    #[inline]
    pub fn y_precision(&self) -> f32 {
        self.precisionY
    }
}

/// A view into the data of a specific pointer in a motion event.
#[derive(Debug)]
pub struct Pointer<'a> {
    event: &'a MotionEvent<'a>,
    index: usize,
}

impl<'a> Pointer<'a> {
    #[inline]
    pub fn pointer_index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn pointer_id(&self) -> i32 {
        let pointer = &self.event.pointers[self.index];
        pointer.id
    }

    #[inline]
    pub fn axis_value(&self, axis: Axis) -> f32 {
        let pointer = &self.event.pointers[self.index];
        pointer.axisValues[axis as u32 as usize]
    }

    #[inline]
    pub fn orientation(&self) -> f32 {
        self.axis_value(Axis::Orientation)
    }

    #[inline]
    pub fn pressure(&self) -> f32 {
        self.axis_value(Axis::Pressure)
    }

    #[inline]
    pub fn raw_x(&self) -> f32 {
        let pointer = &self.event.pointers[self.index];
        pointer.rawX
    }

    #[inline]
    pub fn raw_y(&self) -> f32 {
        let pointer = &self.event.pointers[self.index];
        pointer.rawY
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.axis_value(Axis::X)
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.axis_value(Axis::Y)
    }

    #[inline]
    pub fn size(&self) -> f32 {
        self.axis_value(Axis::Size)
    }

    #[inline]
    pub fn tool_major(&self) -> f32 {
        self.axis_value(Axis::ToolMajor)
    }

    #[inline]
    pub fn tool_minor(&self) -> f32 {
        self.axis_value(Axis::ToolMinor)
    }

    #[inline]
    pub fn touch_major(&self) -> f32 {
        self.axis_value(Axis::TouchMajor)
    }

    #[inline]
    pub fn touch_minor(&self) -> f32 {
        self.axis_value(Axis::TouchMinor)
    }

    #[inline]
    pub fn tool_type(&self) -> ToolType {
        let pointer = &self.event.pointers[self.index];
        let tool_type = pointer.toolType as u32;
        tool_type.try_into().unwrap()
    }
}

/// An iterator over the pointers in a [`MotionEvent`].
#[derive(Debug)]
pub struct PointersIter<'a> {
    event: &'a MotionEvent<'a>,
    next_index: usize,
    count: usize,
}

impl<'a> Iterator for PointersIter<'a> {
    type Item = Pointer<'a>;
    fn next(&mut self) -> Option<Pointer<'a>> {
        if self.next_index < self.count {
            let ptr = Pointer {
                event: self.event,
                index: self.next_index,
            };
            self.next_index += 1;
            Some(ptr)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.count - self.next_index;
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for PointersIter<'a> {
    fn len(&self) -> usize {
        self.count - self.next_index
    }
}

/*
/// Represents a view into a past moment of a motion event
#[derive(Debug)]
pub struct HistoricalMotionEvent<'a> {
    event: NonNull<ndk_sys::AInputEvent>,
    history_index: usize,
    _marker: std::marker::PhantomData<&'a MotionEvent>,
}

// TODO: thread safety?

impl<'a> HistoricalMotionEvent<'a> {
    /// Returns the "history index" associated with this historical event.  Older events have smaller indices.
    #[inline]
    pub fn history_index(&self) -> usize {
        self.history_index
    }

    /// Returns the time of the historical event, in the `java.lang.System.nanoTime()` time base
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_gethistoricaleventtime)
    #[inline]
    pub fn event_time(&self) -> i64 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalEventTime(
                self.event.as_ptr(),
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    /// An iterator over the pointers of this historical motion event
    #[inline]
    pub fn pointers(&self) -> HistoricalPointersIter<'a> {
        HistoricalPointersIter {
            event: self.event,
            history_index: self.history_index,
            next_pointer_index: 0,
            pointer_count: unsafe {
                ndk_sys::AMotionEvent_getPointerCount(self.event.as_ptr()) as usize
            },
            _marker: std::marker::PhantomData,
        }
    }
}

/// An iterator over all the historical moments in a [`MotionEvent`].
///
/// It iterates from oldest to newest.
#[derive(Debug)]
pub struct HistoricalMotionEventsIter<'a> {
    event: NonNull<ndk_sys::AInputEvent>,
    next_history_index: usize,
    history_size: usize,
    _marker: std::marker::PhantomData<&'a MotionEvent>,
}

// TODO: thread safety?

impl<'a> Iterator for HistoricalMotionEventsIter<'a> {
    type Item = HistoricalMotionEvent<'a>;

    fn next(&mut self) -> Option<HistoricalMotionEvent<'a>> {
        if self.next_history_index < self.history_size {
            let res = HistoricalMotionEvent {
                event: self.event,
                history_index: self.next_history_index,
                _marker: std::marker::PhantomData,
            };
            self.next_history_index += 1;
            Some(res)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.history_size - self.next_history_index;
        (size, Some(size))
    }
}
impl ExactSizeIterator for HistoricalMotionEventsIter<'_> {
    fn len(&self) -> usize {
        self.history_size - self.next_history_index
    }
}
impl<'a> DoubleEndedIterator for HistoricalMotionEventsIter<'a> {
    fn next_back(&mut self) -> Option<HistoricalMotionEvent<'a>> {
        if self.next_history_index < self.history_size {
            self.history_size -= 1;
            Some(HistoricalMotionEvent {
                event: self.event,
                history_index: self.history_size,
                _marker: std::marker::PhantomData,
            })
        } else {
            None
        }
    }
}

/// A view into a pointer at a historical moment
#[derive(Debug)]
pub struct HistoricalPointer<'a> {
    event: NonNull<ndk_sys::AInputEvent>,
    pointer_index: usize,
    history_index: usize,
    _marker: std::marker::PhantomData<&'a MotionEvent>,
}

// TODO: thread safety?

impl<'a> HistoricalPointer<'a> {
    #[inline]
    pub fn pointer_index(&self) -> usize {
        self.pointer_index
    }

    #[inline]
    pub fn pointer_id(&self) -> i32 {
        unsafe {
            ndk_sys::AMotionEvent_getPointerId(self.event.as_ptr(), self.pointer_index as ndk_sys::size_t)
        }
    }

    #[inline]
    pub fn history_index(&self) -> usize {
        self.history_index
    }

    #[inline]
    pub fn axis_value(&self, axis: Axis) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalAxisValue(
                self.event.as_ptr(),
                axis as i32,
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn orientation(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalOrientation(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn pressure(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalPressure(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn raw_x(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalRawX(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn raw_y(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalRawY(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn x(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalX(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn y(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalY(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn size(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalSize(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn tool_major(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalToolMajor(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn tool_minor(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalToolMinor(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn touch_major(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalTouchMajor(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }

    #[inline]
    pub fn touch_minor(&self) -> f32 {
        unsafe {
            ndk_sys::AMotionEvent_getHistoricalTouchMinor(
                self.event.as_ptr(),
                self.pointer_index as ndk_sys::size_t,
                self.history_index as ndk_sys::size_t,
            )
        }
    }
}

/// An iterator over the pointers in a historical motion event
#[derive(Debug)]
pub struct HistoricalPointersIter<'a> {
    event: NonNull<ndk_sys::AInputEvent>,
    history_index: usize,
    next_pointer_index: usize,
    pointer_count: usize,
    _marker: std::marker::PhantomData<&'a MotionEvent>,
}

// TODO: thread safety?

impl<'a> Iterator for HistoricalPointersIter<'a> {
    type Item = HistoricalPointer<'a>;

    fn next(&mut self) -> Option<HistoricalPointer<'a>> {
        if self.next_pointer_index < self.pointer_count {
            let ptr = HistoricalPointer {
                event: self.event,
                history_index: self.history_index,
                pointer_index: self.next_pointer_index,
                _marker: std::marker::PhantomData,
            };
            self.next_pointer_index += 1;
            Some(ptr)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.pointer_count - self.next_pointer_index;
        (size, Some(size))
    }
}
impl ExactSizeIterator for HistoricalPointersIter<'_> {
    fn len(&self) -> usize {
        self.pointer_count - self.next_pointer_index
    }
}

*/

/// A key event.
///
/// For general discussion of key events in Android, see [the relevant
/// javadoc](https://developer.android.com/reference/android/view/KeyEvent).
#[derive(Debug, Clone)]
pub struct KeyEvent<'a> {
    ga_event: &'a GameActivityKeyEvent,
}

impl<'a> Deref for KeyEvent<'a> {
    type Target = GameActivityKeyEvent;

    fn deref(&self) -> &Self::Target {
        self.ga_event
    }
}

/// Key actions.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-27)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum KeyAction {
    Down = ndk_sys::AKEY_EVENT_ACTION_DOWN,
    Up = ndk_sys::AKEY_EVENT_ACTION_UP,
    Multiple = ndk_sys::AKEY_EVENT_ACTION_MULTIPLE,
}

/// Key codes.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-39)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Keycode {
    Unknown = ndk_sys::AKEYCODE_UNKNOWN,
    SoftLeft = ndk_sys::AKEYCODE_SOFT_LEFT,
    SoftRight = ndk_sys::AKEYCODE_SOFT_RIGHT,
    Home = ndk_sys::AKEYCODE_HOME,
    Back = ndk_sys::AKEYCODE_BACK,
    Call = ndk_sys::AKEYCODE_CALL,
    Endcall = ndk_sys::AKEYCODE_ENDCALL,
    Keycode0 = ndk_sys::AKEYCODE_0,
    Keycode1 = ndk_sys::AKEYCODE_1,
    Keycode2 = ndk_sys::AKEYCODE_2,
    Keycode3 = ndk_sys::AKEYCODE_3,
    Keycode4 = ndk_sys::AKEYCODE_4,
    Keycode5 = ndk_sys::AKEYCODE_5,
    Keycode6 = ndk_sys::AKEYCODE_6,
    Keycode7 = ndk_sys::AKEYCODE_7,
    Keycode8 = ndk_sys::AKEYCODE_8,
    Keycode9 = ndk_sys::AKEYCODE_9,
    Star = ndk_sys::AKEYCODE_STAR,
    Pound = ndk_sys::AKEYCODE_POUND,
    DpadUp = ndk_sys::AKEYCODE_DPAD_UP,
    DpadDown = ndk_sys::AKEYCODE_DPAD_DOWN,
    DpadLeft = ndk_sys::AKEYCODE_DPAD_LEFT,
    DpadRight = ndk_sys::AKEYCODE_DPAD_RIGHT,
    DpadCenter = ndk_sys::AKEYCODE_DPAD_CENTER,
    VolumeUp = ndk_sys::AKEYCODE_VOLUME_UP,
    VolumeDown = ndk_sys::AKEYCODE_VOLUME_DOWN,
    Power = ndk_sys::AKEYCODE_POWER,
    Camera = ndk_sys::AKEYCODE_CAMERA,
    Clear = ndk_sys::AKEYCODE_CLEAR,
    A = ndk_sys::AKEYCODE_A,
    B = ndk_sys::AKEYCODE_B,
    C = ndk_sys::AKEYCODE_C,
    D = ndk_sys::AKEYCODE_D,
    E = ndk_sys::AKEYCODE_E,
    F = ndk_sys::AKEYCODE_F,
    G = ndk_sys::AKEYCODE_G,
    H = ndk_sys::AKEYCODE_H,
    I = ndk_sys::AKEYCODE_I,
    J = ndk_sys::AKEYCODE_J,
    K = ndk_sys::AKEYCODE_K,
    L = ndk_sys::AKEYCODE_L,
    M = ndk_sys::AKEYCODE_M,
    N = ndk_sys::AKEYCODE_N,
    O = ndk_sys::AKEYCODE_O,
    P = ndk_sys::AKEYCODE_P,
    Q = ndk_sys::AKEYCODE_Q,
    R = ndk_sys::AKEYCODE_R,
    S = ndk_sys::AKEYCODE_S,
    T = ndk_sys::AKEYCODE_T,
    U = ndk_sys::AKEYCODE_U,
    V = ndk_sys::AKEYCODE_V,
    W = ndk_sys::AKEYCODE_W,
    X = ndk_sys::AKEYCODE_X,
    Y = ndk_sys::AKEYCODE_Y,
    Z = ndk_sys::AKEYCODE_Z,
    Comma = ndk_sys::AKEYCODE_COMMA,
    Period = ndk_sys::AKEYCODE_PERIOD,
    AltLeft = ndk_sys::AKEYCODE_ALT_LEFT,
    AltRight = ndk_sys::AKEYCODE_ALT_RIGHT,
    ShiftLeft = ndk_sys::AKEYCODE_SHIFT_LEFT,
    ShiftRight = ndk_sys::AKEYCODE_SHIFT_RIGHT,
    Tab = ndk_sys::AKEYCODE_TAB,
    Space = ndk_sys::AKEYCODE_SPACE,
    Sym = ndk_sys::AKEYCODE_SYM,
    Explorer = ndk_sys::AKEYCODE_EXPLORER,
    Envelope = ndk_sys::AKEYCODE_ENVELOPE,
    Enter = ndk_sys::AKEYCODE_ENTER,
    Del = ndk_sys::AKEYCODE_DEL,
    Grave = ndk_sys::AKEYCODE_GRAVE,
    Minus = ndk_sys::AKEYCODE_MINUS,
    Equals = ndk_sys::AKEYCODE_EQUALS,
    LeftBracket = ndk_sys::AKEYCODE_LEFT_BRACKET,
    RightBracket = ndk_sys::AKEYCODE_RIGHT_BRACKET,
    Backslash = ndk_sys::AKEYCODE_BACKSLASH,
    Semicolon = ndk_sys::AKEYCODE_SEMICOLON,
    Apostrophe = ndk_sys::AKEYCODE_APOSTROPHE,
    Slash = ndk_sys::AKEYCODE_SLASH,
    At = ndk_sys::AKEYCODE_AT,
    Num = ndk_sys::AKEYCODE_NUM,
    Headsethook = ndk_sys::AKEYCODE_HEADSETHOOK,
    Focus = ndk_sys::AKEYCODE_FOCUS,
    Plus = ndk_sys::AKEYCODE_PLUS,
    Menu = ndk_sys::AKEYCODE_MENU,
    Notification = ndk_sys::AKEYCODE_NOTIFICATION,
    Search = ndk_sys::AKEYCODE_SEARCH,
    MediaPlayPause = ndk_sys::AKEYCODE_MEDIA_PLAY_PAUSE,
    MediaStop = ndk_sys::AKEYCODE_MEDIA_STOP,
    MediaNext = ndk_sys::AKEYCODE_MEDIA_NEXT,
    MediaPrevious = ndk_sys::AKEYCODE_MEDIA_PREVIOUS,
    MediaRewind = ndk_sys::AKEYCODE_MEDIA_REWIND,
    MediaFastForward = ndk_sys::AKEYCODE_MEDIA_FAST_FORWARD,
    Mute = ndk_sys::AKEYCODE_MUTE,
    PageUp = ndk_sys::AKEYCODE_PAGE_UP,
    PageDown = ndk_sys::AKEYCODE_PAGE_DOWN,
    Pictsymbols = ndk_sys::AKEYCODE_PICTSYMBOLS,
    SwitchCharset = ndk_sys::AKEYCODE_SWITCH_CHARSET,
    ButtonA = ndk_sys::AKEYCODE_BUTTON_A,
    ButtonB = ndk_sys::AKEYCODE_BUTTON_B,
    ButtonC = ndk_sys::AKEYCODE_BUTTON_C,
    ButtonX = ndk_sys::AKEYCODE_BUTTON_X,
    ButtonY = ndk_sys::AKEYCODE_BUTTON_Y,
    ButtonZ = ndk_sys::AKEYCODE_BUTTON_Z,
    ButtonL1 = ndk_sys::AKEYCODE_BUTTON_L1,
    ButtonR1 = ndk_sys::AKEYCODE_BUTTON_R1,
    ButtonL2 = ndk_sys::AKEYCODE_BUTTON_L2,
    ButtonR2 = ndk_sys::AKEYCODE_BUTTON_R2,
    ButtonThumbl = ndk_sys::AKEYCODE_BUTTON_THUMBL,
    ButtonThumbr = ndk_sys::AKEYCODE_BUTTON_THUMBR,
    ButtonStart = ndk_sys::AKEYCODE_BUTTON_START,
    ButtonSelect = ndk_sys::AKEYCODE_BUTTON_SELECT,
    ButtonMode = ndk_sys::AKEYCODE_BUTTON_MODE,
    Escape = ndk_sys::AKEYCODE_ESCAPE,
    ForwardDel = ndk_sys::AKEYCODE_FORWARD_DEL,
    CtrlLeft = ndk_sys::AKEYCODE_CTRL_LEFT,
    CtrlRight = ndk_sys::AKEYCODE_CTRL_RIGHT,
    CapsLock = ndk_sys::AKEYCODE_CAPS_LOCK,
    ScrollLock = ndk_sys::AKEYCODE_SCROLL_LOCK,
    MetaLeft = ndk_sys::AKEYCODE_META_LEFT,
    MetaRight = ndk_sys::AKEYCODE_META_RIGHT,
    Function = ndk_sys::AKEYCODE_FUNCTION,
    Sysrq = ndk_sys::AKEYCODE_SYSRQ,
    Break = ndk_sys::AKEYCODE_BREAK,
    MoveHome = ndk_sys::AKEYCODE_MOVE_HOME,
    MoveEnd = ndk_sys::AKEYCODE_MOVE_END,
    Insert = ndk_sys::AKEYCODE_INSERT,
    Forward = ndk_sys::AKEYCODE_FORWARD,
    MediaPlay = ndk_sys::AKEYCODE_MEDIA_PLAY,
    MediaPause = ndk_sys::AKEYCODE_MEDIA_PAUSE,
    MediaClose = ndk_sys::AKEYCODE_MEDIA_CLOSE,
    MediaEject = ndk_sys::AKEYCODE_MEDIA_EJECT,
    MediaRecord = ndk_sys::AKEYCODE_MEDIA_RECORD,
    F1 = ndk_sys::AKEYCODE_F1,
    F2 = ndk_sys::AKEYCODE_F2,
    F3 = ndk_sys::AKEYCODE_F3,
    F4 = ndk_sys::AKEYCODE_F4,
    F5 = ndk_sys::AKEYCODE_F5,
    F6 = ndk_sys::AKEYCODE_F6,
    F7 = ndk_sys::AKEYCODE_F7,
    F8 = ndk_sys::AKEYCODE_F8,
    F9 = ndk_sys::AKEYCODE_F9,
    F10 = ndk_sys::AKEYCODE_F10,
    F11 = ndk_sys::AKEYCODE_F11,
    F12 = ndk_sys::AKEYCODE_F12,
    NumLock = ndk_sys::AKEYCODE_NUM_LOCK,
    Numpad0 = ndk_sys::AKEYCODE_NUMPAD_0,
    Numpad1 = ndk_sys::AKEYCODE_NUMPAD_1,
    Numpad2 = ndk_sys::AKEYCODE_NUMPAD_2,
    Numpad3 = ndk_sys::AKEYCODE_NUMPAD_3,
    Numpad4 = ndk_sys::AKEYCODE_NUMPAD_4,
    Numpad5 = ndk_sys::AKEYCODE_NUMPAD_5,
    Numpad6 = ndk_sys::AKEYCODE_NUMPAD_6,
    Numpad7 = ndk_sys::AKEYCODE_NUMPAD_7,
    Numpad8 = ndk_sys::AKEYCODE_NUMPAD_8,
    Numpad9 = ndk_sys::AKEYCODE_NUMPAD_9,
    NumpadDivide = ndk_sys::AKEYCODE_NUMPAD_DIVIDE,
    NumpadMultiply = ndk_sys::AKEYCODE_NUMPAD_MULTIPLY,
    NumpadSubtract = ndk_sys::AKEYCODE_NUMPAD_SUBTRACT,
    NumpadAdd = ndk_sys::AKEYCODE_NUMPAD_ADD,
    NumpadDot = ndk_sys::AKEYCODE_NUMPAD_DOT,
    NumpadComma = ndk_sys::AKEYCODE_NUMPAD_COMMA,
    NumpadEnter = ndk_sys::AKEYCODE_NUMPAD_ENTER,
    NumpadEquals = ndk_sys::AKEYCODE_NUMPAD_EQUALS,
    NumpadLeftParen = ndk_sys::AKEYCODE_NUMPAD_LEFT_PAREN,
    NumpadRightParen = ndk_sys::AKEYCODE_NUMPAD_RIGHT_PAREN,
    VolumeMute = ndk_sys::AKEYCODE_VOLUME_MUTE,
    Info = ndk_sys::AKEYCODE_INFO,
    ChannelUp = ndk_sys::AKEYCODE_CHANNEL_UP,
    ChannelDown = ndk_sys::AKEYCODE_CHANNEL_DOWN,
    ZoomIn = ndk_sys::AKEYCODE_ZOOM_IN,
    ZoomOut = ndk_sys::AKEYCODE_ZOOM_OUT,
    Tv = ndk_sys::AKEYCODE_TV,
    Window = ndk_sys::AKEYCODE_WINDOW,
    Guide = ndk_sys::AKEYCODE_GUIDE,
    Dvr = ndk_sys::AKEYCODE_DVR,
    Bookmark = ndk_sys::AKEYCODE_BOOKMARK,
    Captions = ndk_sys::AKEYCODE_CAPTIONS,
    Settings = ndk_sys::AKEYCODE_SETTINGS,
    TvPower = ndk_sys::AKEYCODE_TV_POWER,
    TvInput = ndk_sys::AKEYCODE_TV_INPUT,
    StbPower = ndk_sys::AKEYCODE_STB_POWER,
    StbInput = ndk_sys::AKEYCODE_STB_INPUT,
    AvrPower = ndk_sys::AKEYCODE_AVR_POWER,
    AvrInput = ndk_sys::AKEYCODE_AVR_INPUT,
    ProgRed = ndk_sys::AKEYCODE_PROG_RED,
    ProgGreen = ndk_sys::AKEYCODE_PROG_GREEN,
    ProgYellow = ndk_sys::AKEYCODE_PROG_YELLOW,
    ProgBlue = ndk_sys::AKEYCODE_PROG_BLUE,
    AppSwitch = ndk_sys::AKEYCODE_APP_SWITCH,
    Button1 = ndk_sys::AKEYCODE_BUTTON_1,
    Button2 = ndk_sys::AKEYCODE_BUTTON_2,
    Button3 = ndk_sys::AKEYCODE_BUTTON_3,
    Button4 = ndk_sys::AKEYCODE_BUTTON_4,
    Button5 = ndk_sys::AKEYCODE_BUTTON_5,
    Button6 = ndk_sys::AKEYCODE_BUTTON_6,
    Button7 = ndk_sys::AKEYCODE_BUTTON_7,
    Button8 = ndk_sys::AKEYCODE_BUTTON_8,
    Button9 = ndk_sys::AKEYCODE_BUTTON_9,
    Button10 = ndk_sys::AKEYCODE_BUTTON_10,
    Button11 = ndk_sys::AKEYCODE_BUTTON_11,
    Button12 = ndk_sys::AKEYCODE_BUTTON_12,
    Button13 = ndk_sys::AKEYCODE_BUTTON_13,
    Button14 = ndk_sys::AKEYCODE_BUTTON_14,
    Button15 = ndk_sys::AKEYCODE_BUTTON_15,
    Button16 = ndk_sys::AKEYCODE_BUTTON_16,
    LanguageSwitch = ndk_sys::AKEYCODE_LANGUAGE_SWITCH,
    MannerMode = ndk_sys::AKEYCODE_MANNER_MODE,
    Keycode3dMode = ndk_sys::AKEYCODE_3D_MODE,
    Contacts = ndk_sys::AKEYCODE_CONTACTS,
    Calendar = ndk_sys::AKEYCODE_CALENDAR,
    Music = ndk_sys::AKEYCODE_MUSIC,
    Calculator = ndk_sys::AKEYCODE_CALCULATOR,
    ZenkakuHankaku = ndk_sys::AKEYCODE_ZENKAKU_HANKAKU,
    Eisu = ndk_sys::AKEYCODE_EISU,
    Muhenkan = ndk_sys::AKEYCODE_MUHENKAN,
    Henkan = ndk_sys::AKEYCODE_HENKAN,
    KatakanaHiragana = ndk_sys::AKEYCODE_KATAKANA_HIRAGANA,
    Yen = ndk_sys::AKEYCODE_YEN,
    Ro = ndk_sys::AKEYCODE_RO,
    Kana = ndk_sys::AKEYCODE_KANA,
    Assist = ndk_sys::AKEYCODE_ASSIST,
    BrightnessDown = ndk_sys::AKEYCODE_BRIGHTNESS_DOWN,
    BrightnessUp = ndk_sys::AKEYCODE_BRIGHTNESS_UP,
    MediaAudioTrack = ndk_sys::AKEYCODE_MEDIA_AUDIO_TRACK,
    Sleep = ndk_sys::AKEYCODE_SLEEP,
    Wakeup = ndk_sys::AKEYCODE_WAKEUP,
    Pairing = ndk_sys::AKEYCODE_PAIRING,
    MediaTopMenu = ndk_sys::AKEYCODE_MEDIA_TOP_MENU,
    Keycode11 = ndk_sys::AKEYCODE_11,
    Keycode12 = ndk_sys::AKEYCODE_12,
    LastChannel = ndk_sys::AKEYCODE_LAST_CHANNEL,
    TvDataService = ndk_sys::AKEYCODE_TV_DATA_SERVICE,
    VoiceAssist = ndk_sys::AKEYCODE_VOICE_ASSIST,
    TvRadioService = ndk_sys::AKEYCODE_TV_RADIO_SERVICE,
    TvTeletext = ndk_sys::AKEYCODE_TV_TELETEXT,
    TvNumberEntry = ndk_sys::AKEYCODE_TV_NUMBER_ENTRY,
    TvTerrestrialAnalog = ndk_sys::AKEYCODE_TV_TERRESTRIAL_ANALOG,
    TvTerrestrialDigital = ndk_sys::AKEYCODE_TV_TERRESTRIAL_DIGITAL,
    TvSatellite = ndk_sys::AKEYCODE_TV_SATELLITE,
    TvSatelliteBs = ndk_sys::AKEYCODE_TV_SATELLITE_BS,
    TvSatelliteCs = ndk_sys::AKEYCODE_TV_SATELLITE_CS,
    TvSatelliteService = ndk_sys::AKEYCODE_TV_SATELLITE_SERVICE,
    TvNetwork = ndk_sys::AKEYCODE_TV_NETWORK,
    TvAntennaCable = ndk_sys::AKEYCODE_TV_ANTENNA_CABLE,
    TvInputHdmi1 = ndk_sys::AKEYCODE_TV_INPUT_HDMI_1,
    TvInputHdmi2 = ndk_sys::AKEYCODE_TV_INPUT_HDMI_2,
    TvInputHdmi3 = ndk_sys::AKEYCODE_TV_INPUT_HDMI_3,
    TvInputHdmi4 = ndk_sys::AKEYCODE_TV_INPUT_HDMI_4,
    TvInputComposite1 = ndk_sys::AKEYCODE_TV_INPUT_COMPOSITE_1,
    TvInputComposite2 = ndk_sys::AKEYCODE_TV_INPUT_COMPOSITE_2,
    TvInputComponent1 = ndk_sys::AKEYCODE_TV_INPUT_COMPONENT_1,
    TvInputComponent2 = ndk_sys::AKEYCODE_TV_INPUT_COMPONENT_2,
    TvInputVga1 = ndk_sys::AKEYCODE_TV_INPUT_VGA_1,
    TvAudioDescription = ndk_sys::AKEYCODE_TV_AUDIO_DESCRIPTION,
    TvAudioDescriptionMixUp = ndk_sys::AKEYCODE_TV_AUDIO_DESCRIPTION_MIX_UP,
    TvAudioDescriptionMixDown = ndk_sys::AKEYCODE_TV_AUDIO_DESCRIPTION_MIX_DOWN,
    TvZoomMode = ndk_sys::AKEYCODE_TV_ZOOM_MODE,
    TvContentsMenu = ndk_sys::AKEYCODE_TV_CONTENTS_MENU,
    TvMediaContextMenu = ndk_sys::AKEYCODE_TV_MEDIA_CONTEXT_MENU,
    TvTimerProgramming = ndk_sys::AKEYCODE_TV_TIMER_PROGRAMMING,
    Help = ndk_sys::AKEYCODE_HELP,
    NavigatePrevious = ndk_sys::AKEYCODE_NAVIGATE_PREVIOUS,
    NavigateNext = ndk_sys::AKEYCODE_NAVIGATE_NEXT,
    NavigateIn = ndk_sys::AKEYCODE_NAVIGATE_IN,
    NavigateOut = ndk_sys::AKEYCODE_NAVIGATE_OUT,
    StemPrimary = ndk_sys::AKEYCODE_STEM_PRIMARY,
    Stem1 = ndk_sys::AKEYCODE_STEM_1,
    Stem2 = ndk_sys::AKEYCODE_STEM_2,
    Stem3 = ndk_sys::AKEYCODE_STEM_3,
    DpadUpLeft = ndk_sys::AKEYCODE_DPAD_UP_LEFT,
    DpadDownLeft = ndk_sys::AKEYCODE_DPAD_DOWN_LEFT,
    DpadUpRight = ndk_sys::AKEYCODE_DPAD_UP_RIGHT,
    DpadDownRight = ndk_sys::AKEYCODE_DPAD_DOWN_RIGHT,
    MediaSkipForward = ndk_sys::AKEYCODE_MEDIA_SKIP_FORWARD,
    MediaSkipBackward = ndk_sys::AKEYCODE_MEDIA_SKIP_BACKWARD,
    MediaStepForward = ndk_sys::AKEYCODE_MEDIA_STEP_FORWARD,
    MediaStepBackward = ndk_sys::AKEYCODE_MEDIA_STEP_BACKWARD,
    SoftSleep = ndk_sys::AKEYCODE_SOFT_SLEEP,
    Cut = ndk_sys::AKEYCODE_CUT,
    Copy = ndk_sys::AKEYCODE_COPY,
    Paste = ndk_sys::AKEYCODE_PASTE,
    SystemNavigationUp = ndk_sys::AKEYCODE_SYSTEM_NAVIGATION_UP,
    SystemNavigationDown = ndk_sys::AKEYCODE_SYSTEM_NAVIGATION_DOWN,
    SystemNavigationLeft = ndk_sys::AKEYCODE_SYSTEM_NAVIGATION_LEFT,
    SystemNavigationRight = ndk_sys::AKEYCODE_SYSTEM_NAVIGATION_RIGHT,
    AllApps = ndk_sys::AKEYCODE_ALL_APPS,
    Refresh = ndk_sys::AKEYCODE_REFRESH,
    ThumbsUp = ndk_sys::AKEYCODE_THUMBS_UP,
    ThumbsDown = ndk_sys::AKEYCODE_THUMBS_DOWN,
    ProfileSwitch = ndk_sys::AKEYCODE_PROFILE_SWITCH,
}

impl<'a> KeyEvent<'a> {
    pub(crate) fn new(ga_event: &'a GameActivityKeyEvent) -> Self {
        Self { ga_event }
    }

    /// Get the source of the event.
    ///
    #[inline]
    pub fn source(&self) -> Source {
        let source = self.source as u32;
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
        self.deviceId
    }

    /// Returns the key action associated with the event.
    ///
    /// See [the KeyEvent docs](https://developer.android.com/reference/android/view/KeyEvent#getAction())
    #[inline]
    pub fn action(&self) -> KeyAction {
        let action = self.action as u32;
        action.try_into().unwrap()
    }

    /// Returns the last time the key was pressed.  This is on the scale of
    /// `java.lang.System.nanoTime()`, which has nanosecond precision, but no defined start time.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getdowntime)
    #[inline]
    pub fn down_time(&self) -> i64 {
        self.downTime
    }

    /// Returns the time this event occured.  This is on the scale of
    /// `java.lang.System.nanoTime()`, which has nanosecond precision, but no defined start time.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_geteventtime)
    #[inline]
    pub fn event_time(&self) -> i64 {
        self.eventTime
    }

    /// Returns the keycode associated with this key event
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getkeycode)
    #[inline]
    pub fn key_code(&self) -> Keycode {
        let keycode = self.keyCode as u32;
        keycode.try_into().unwrap_or(Keycode::Unknown)
    }

    /// Returns the number of repeats of a key.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getrepeatcount)
    #[inline]
    pub fn repeat_count(&self) -> i32 {
        self.repeatCount
    }

    /// Returns the hardware keycode of a key.  This varies from device to device.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getscancode)
    #[inline]
    pub fn scan_code(&self) -> i32 {
        self.scanCode
    }
}

/// Flags associated with [`KeyEvent`].
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-28)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeyEventFlags(pub u32);

impl KeyEventFlags {
    #[inline]
    pub fn cancelled(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_CANCELED != 0
    }
    #[inline]
    pub fn cancelled_long_press(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_CANCELED_LONG_PRESS != 0
    }
    #[inline]
    pub fn editor_action(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_EDITOR_ACTION != 0
    }
    #[inline]
    pub fn fallback(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_FALLBACK != 0
    }
    #[inline]
    pub fn from_system(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_FROM_SYSTEM != 0
    }
    #[inline]
    pub fn keep_touch_mode(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_KEEP_TOUCH_MODE != 0
    }
    #[inline]
    pub fn long_press(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_LONG_PRESS != 0
    }
    #[inline]
    pub fn soft_keyboard(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_SOFT_KEYBOARD != 0
    }
    #[inline]
    pub fn tracking(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_TRACKING != 0
    }
    #[inline]
    pub fn virtual_hard_key(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_VIRTUAL_HARD_KEY != 0
    }
    #[inline]
    pub fn woke_here(&self) -> bool {
        self.0 & ndk_sys::AKEY_EVENT_FLAG_WOKE_HERE != 0
    }
}

impl<'a> KeyEvent<'a> {
    /// Flags associated with this [`KeyEvent`].
    ///
    /// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getflags)
    #[inline]
    pub fn flags(&self) -> KeyEventFlags {
        KeyEventFlags(self.flags as u32)
    }

    /// Returns the state of the modifiers during this key event, represented by a bitmask.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getmetastate)
    #[inline]
    pub fn meta_state(&self) -> MetaState {
        MetaState(self.metaState as u32)
    }
}
