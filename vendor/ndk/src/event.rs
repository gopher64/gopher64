//! Bindings for [`AInputEvent`, `AKeyEvent` and `AMotionEvent`]
//!
//! Most of these operations directly wrap functions in the NDK.
//!
//! See also the Java docs for [`android.view.InputEvent`], [`android.view.MotionEvent`], and
//! [`android.view.KeyEvent`].
//!
//! [`AInputEvent`, `AKeyEvent` and `AMotionEvent`]: https://developer.android.com/ndk/reference/group/input
//! [`android.view.InputEvent`]: https://developer.android.com/reference/android/view/InputEvent.html
//! [`android.view.MotionEvent`]: https://developer.android.com/reference/android/view/MotionEvent.html
//! [`android.view.KeyEvent`]: https://developer.android.com/reference/android/view/KeyEvent

use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::convert::TryInto;
use std::ptr::NonNull;

/// A native [`AInputEvent *`]
///
/// [`AInputEvent *`]: https://developer.android.com/ndk/reference/group/input#ainputevent
#[derive(Debug)]
pub enum InputEvent {
    MotionEvent(MotionEvent),
    KeyEvent(KeyEvent),
}

/// An enum representing the source of an [`InputEvent`].
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-36)
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Source {
    Unknown = ffi::AINPUT_SOURCE_UNKNOWN,
    Keyboard = ffi::AINPUT_SOURCE_KEYBOARD,
    Dpad = ffi::AINPUT_SOURCE_DPAD,
    Gamepad = ffi::AINPUT_SOURCE_GAMEPAD,
    Touchscreen = ffi::AINPUT_SOURCE_TOUCHSCREEN,
    Mouse = ffi::AINPUT_SOURCE_MOUSE,
    Stylus = ffi::AINPUT_SOURCE_STYLUS,
    BluetoothStylus = ffi::AINPUT_SOURCE_BLUETOOTH_STYLUS,
    Trackball = ffi::AINPUT_SOURCE_TRACKBALL,
    MouseRelative = ffi::AINPUT_SOURCE_MOUSE_RELATIVE,
    Touchpad = ffi::AINPUT_SOURCE_TOUCHPAD,
    TouchNavigation = ffi::AINPUT_SOURCE_TOUCH_NAVIGATION,
    Joystick = ffi::AINPUT_SOURCE_JOYSTICK,
    RotaryEncoder = ffi::AINPUT_SOURCE_ROTARY_ENCODER,
    Any = ffi::AINPUT_SOURCE_ANY,
}

/// An enum representing the class of an [`InputEvent`] source.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-35)
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
enum Class {
    None = ffi::AINPUT_SOURCE_CLASS_NONE,
    Button = ffi::AINPUT_SOURCE_CLASS_BUTTON,
    Pointer = ffi::AINPUT_SOURCE_CLASS_POINTER,
    Navigation = ffi::AINPUT_SOURCE_CLASS_NAVIGATION,
    Position = ffi::AINPUT_SOURCE_CLASS_POSITION,
    Joystick = ffi::AINPUT_SOURCE_CLASS_JOYSTICK,
}

impl InputEvent {
    /// Initialize an [`InputEvent`] from a pointer
    ///
    /// # Safety
    /// By calling this function, you assert that the pointer is a valid pointer to a
    /// native [`ffi::AInputEvent`].
    #[inline]
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AInputEvent>) -> Self {
        match ffi::AInputEvent_getType(ptr.as_ptr()) as u32 {
            ffi::AINPUT_EVENT_TYPE_KEY => InputEvent::KeyEvent(KeyEvent::from_ptr(ptr)),
            ffi::AINPUT_EVENT_TYPE_MOTION => InputEvent::MotionEvent(MotionEvent::from_ptr(ptr)),
            x => panic!("Bad event type received: {}", x),
        }
    }

    /// Returns a pointer to the native [`ffi::AInputEvent`].
    #[inline]
    pub fn ptr(&self) -> NonNull<ffi::AInputEvent> {
        match self {
            InputEvent::MotionEvent(MotionEvent { ptr }) => *ptr,
            InputEvent::KeyEvent(KeyEvent { ptr }) => *ptr,
        }
    }

    /// Get the source of the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#ainputevent_getsource)
    #[inline]
    pub fn source(&self) -> Source {
        let source = unsafe { ffi::AInputEvent_getSource(self.ptr().as_ptr()) as u32 };
        source.try_into().unwrap_or(Source::Unknown)
    }

    /// Get the device id associated with the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#ainputevent_getdeviceid)
    #[inline]
    pub fn device_id(&self) -> i32 {
        unsafe { ffi::AInputEvent_getDeviceId(self.ptr().as_ptr()) }
    }
}

/// A bitfield representing the state of modifier keys during an event.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-25)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MetaState(pub u32);

impl MetaState {
    #[inline]
    pub fn alt_on(self) -> bool {
        self.0 & ffi::AMETA_ALT_ON != 0
    }
    #[inline]
    pub fn alt_left_on(self) -> bool {
        self.0 & ffi::AMETA_ALT_LEFT_ON != 0
    }
    #[inline]
    pub fn alt_right_on(self) -> bool {
        self.0 & ffi::AMETA_ALT_RIGHT_ON != 0
    }
    #[inline]
    pub fn shift_on(self) -> bool {
        self.0 & ffi::AMETA_SHIFT_ON != 0
    }
    #[inline]
    pub fn shift_left_on(self) -> bool {
        self.0 & ffi::AMETA_SHIFT_LEFT_ON != 0
    }
    #[inline]
    pub fn shift_right_on(self) -> bool {
        self.0 & ffi::AMETA_SHIFT_RIGHT_ON != 0
    }
    #[inline]
    pub fn sym_on(self) -> bool {
        self.0 & ffi::AMETA_SYM_ON != 0
    }
    #[inline]
    pub fn function_on(self) -> bool {
        self.0 & ffi::AMETA_FUNCTION_ON != 0
    }
    #[inline]
    pub fn ctrl_on(self) -> bool {
        self.0 & ffi::AMETA_CTRL_ON != 0
    }
    #[inline]
    pub fn ctrl_left_on(self) -> bool {
        self.0 & ffi::AMETA_CTRL_LEFT_ON != 0
    }
    #[inline]
    pub fn ctrl_right_on(self) -> bool {
        self.0 & ffi::AMETA_CTRL_RIGHT_ON != 0
    }
    #[inline]
    pub fn meta_on(self) -> bool {
        self.0 & ffi::AMETA_META_ON != 0
    }
    #[inline]
    pub fn meta_left_on(self) -> bool {
        self.0 & ffi::AMETA_META_LEFT_ON != 0
    }
    #[inline]
    pub fn meta_right_on(self) -> bool {
        self.0 & ffi::AMETA_META_RIGHT_ON != 0
    }
    #[inline]
    pub fn caps_lock_on(self) -> bool {
        self.0 & ffi::AMETA_CAPS_LOCK_ON != 0
    }
    #[inline]
    pub fn num_lock_on(self) -> bool {
        self.0 & ffi::AMETA_NUM_LOCK_ON != 0
    }
    #[inline]
    pub fn scroll_lock_on(self) -> bool {
        self.0 & ffi::AMETA_SCROLL_LOCK_ON != 0
    }
}

/// A motion event
///
/// Wraps an [`AInputEvent *`] of the [`ffi::AINPUT_EVENT_TYPE_MOTION`] type.
///
/// For general discussion of motion events in Android, see [the relevant
/// javadoc](https://developer.android.com/reference/android/view/MotionEvent).
///
/// [`AInputEvent *`]: https://developer.android.com/ndk/reference/group/input#ainputevent
#[derive(Clone, Debug)]
pub struct MotionEvent {
    ptr: NonNull<ffi::AInputEvent>,
}

// TODO: thread safety?

/// A motion action.
///
/// See [the NDK
/// docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-29)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum MotionAction {
    Down = ffi::AMOTION_EVENT_ACTION_DOWN,
    Up = ffi::AMOTION_EVENT_ACTION_UP,
    Move = ffi::AMOTION_EVENT_ACTION_MOVE,
    Cancel = ffi::AMOTION_EVENT_ACTION_CANCEL,
    Outside = ffi::AMOTION_EVENT_ACTION_OUTSIDE,
    PointerDown = ffi::AMOTION_EVENT_ACTION_POINTER_DOWN,
    PointerUp = ffi::AMOTION_EVENT_ACTION_POINTER_UP,
    HoverMove = ffi::AMOTION_EVENT_ACTION_HOVER_MOVE,
    Scroll = ffi::AMOTION_EVENT_ACTION_SCROLL,
    HoverEnter = ffi::AMOTION_EVENT_ACTION_HOVER_ENTER,
    HoverExit = ffi::AMOTION_EVENT_ACTION_HOVER_EXIT,
    ButtonPress = ffi::AMOTION_EVENT_ACTION_BUTTON_PRESS,
    ButtonRelease = ffi::AMOTION_EVENT_ACTION_BUTTON_RELEASE,
}

/// An axis of a motion event.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-32)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Axis {
    X = ffi::AMOTION_EVENT_AXIS_X,
    Y = ffi::AMOTION_EVENT_AXIS_Y,
    Pressure = ffi::AMOTION_EVENT_AXIS_PRESSURE,
    Size = ffi::AMOTION_EVENT_AXIS_SIZE,
    TouchMajor = ffi::AMOTION_EVENT_AXIS_TOUCH_MAJOR,
    TouchMinor = ffi::AMOTION_EVENT_AXIS_TOUCH_MINOR,
    ToolMajor = ffi::AMOTION_EVENT_AXIS_TOOL_MAJOR,
    ToolMinor = ffi::AMOTION_EVENT_AXIS_TOOL_MINOR,
    Orientation = ffi::AMOTION_EVENT_AXIS_ORIENTATION,
    Vscroll = ffi::AMOTION_EVENT_AXIS_VSCROLL,
    Hscroll = ffi::AMOTION_EVENT_AXIS_HSCROLL,
    Z = ffi::AMOTION_EVENT_AXIS_Z,
    Rx = ffi::AMOTION_EVENT_AXIS_RX,
    Ry = ffi::AMOTION_EVENT_AXIS_RY,
    Rz = ffi::AMOTION_EVENT_AXIS_RZ,
    HatX = ffi::AMOTION_EVENT_AXIS_HAT_X,
    HatY = ffi::AMOTION_EVENT_AXIS_HAT_Y,
    Ltrigger = ffi::AMOTION_EVENT_AXIS_LTRIGGER,
    Rtrigger = ffi::AMOTION_EVENT_AXIS_RTRIGGER,
    Throttle = ffi::AMOTION_EVENT_AXIS_THROTTLE,
    Rudder = ffi::AMOTION_EVENT_AXIS_RUDDER,
    Wheel = ffi::AMOTION_EVENT_AXIS_WHEEL,
    Gas = ffi::AMOTION_EVENT_AXIS_GAS,
    Brake = ffi::AMOTION_EVENT_AXIS_BRAKE,
    Distance = ffi::AMOTION_EVENT_AXIS_DISTANCE,
    Tilt = ffi::AMOTION_EVENT_AXIS_TILT,
    Scroll = ffi::AMOTION_EVENT_AXIS_SCROLL,
    RelativeX = ffi::AMOTION_EVENT_AXIS_RELATIVE_X,
    RelativeY = ffi::AMOTION_EVENT_AXIS_RELATIVE_Y,
    Generic1 = ffi::AMOTION_EVENT_AXIS_GENERIC_1,
    Generic2 = ffi::AMOTION_EVENT_AXIS_GENERIC_2,
    Generic3 = ffi::AMOTION_EVENT_AXIS_GENERIC_3,
    Generic4 = ffi::AMOTION_EVENT_AXIS_GENERIC_4,
    Generic5 = ffi::AMOTION_EVENT_AXIS_GENERIC_5,
    Generic6 = ffi::AMOTION_EVENT_AXIS_GENERIC_6,
    Generic7 = ffi::AMOTION_EVENT_AXIS_GENERIC_7,
    Generic8 = ffi::AMOTION_EVENT_AXIS_GENERIC_8,
    Generic9 = ffi::AMOTION_EVENT_AXIS_GENERIC_9,
    Generic10 = ffi::AMOTION_EVENT_AXIS_GENERIC_10,
    Generic11 = ffi::AMOTION_EVENT_AXIS_GENERIC_11,
    Generic12 = ffi::AMOTION_EVENT_AXIS_GENERIC_12,
    Generic13 = ffi::AMOTION_EVENT_AXIS_GENERIC_13,
    Generic14 = ffi::AMOTION_EVENT_AXIS_GENERIC_14,
    Generic15 = ffi::AMOTION_EVENT_AXIS_GENERIC_15,
    Generic16 = ffi::AMOTION_EVENT_AXIS_GENERIC_16,
}

/// A bitfield representing the state of buttons during a motion event.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-33)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ButtonState(pub u32);

impl ButtonState {
    #[inline]
    pub fn primary(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_BUTTON_PRIMARY != 0
    }
    #[inline]
    pub fn secondary(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_BUTTON_SECONDARY != 0
    }
    #[inline]
    pub fn teriary(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_BUTTON_TERTIARY != 0
    }
    #[inline]
    pub fn back(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_BUTTON_BACK != 0
    }
    #[inline]
    pub fn forward(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_BUTTON_FORWARD != 0
    }
    #[inline]
    pub fn stylus_primary(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_BUTTON_STYLUS_PRIMARY != 0
    }
    #[inline]
    pub fn stylus_secondary(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_BUTTON_STYLUS_SECONDARY != 0
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
        self.0 & ffi::AMOTION_EVENT_EDGE_FLAG_TOP != 0
    }
    #[inline]
    pub fn bottom(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_EDGE_FLAG_BOTTOM != 0
    }
    #[inline]
    pub fn left(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_EDGE_FLAG_LEFT != 0
    }
    #[inline]
    pub fn right(self) -> bool {
        self.0 & ffi::AMOTION_EVENT_EDGE_FLAG_RIGHT != 0
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
        self.0 & ffi::AMOTION_EVENT_FLAG_WINDOW_IS_OBSCURED != 0
    }
}

impl MotionEvent {
    /// Constructs a MotionEvent from a pointer to a native [`ffi::AInputEvent`]
    ///
    /// # Safety
    /// By calling this method, you assert that the pointer is a valid, non-null pointer to a
    /// native [`ffi::AInputEvent`] and that [`ffi::AInputEvent`]
    /// is an `AMotionEvent`.
    #[inline]
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AInputEvent>) -> Self {
        Self { ptr }
    }

    /// Returns a pointer to the native [`ffi::AInputEvent`]
    #[inline]
    pub fn ptr(&self) -> NonNull<ffi::AInputEvent> {
        self.ptr
    }

    /// Get the source of the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#ainputevent_getsource)
    #[inline]
    pub fn source(&self) -> Source {
        let source = unsafe { ffi::AInputEvent_getSource(self.ptr.as_ptr()) as u32 };
        source.try_into().unwrap_or(Source::Unknown)
    }

    /// Get the device id associated with the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#ainputevent_getdeviceid)
    #[inline]
    pub fn device_id(&self) -> i32 {
        unsafe { ffi::AInputEvent_getDeviceId(self.ptr.as_ptr()) }
    }

    /// Returns the motion action associated with the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getaction)
    #[inline]
    pub fn action(&self) -> MotionAction {
        let action = unsafe {
            ffi::AMotionEvent_getAction(self.ptr.as_ptr()) as u32 & ffi::AMOTION_EVENT_ACTION_MASK
        };
        action.try_into().unwrap()
    }

    /// Returns the pointer index of an `Up` or `Down` event.
    ///
    /// Pointer indices can change per motion event.  For an identifier that stays the same, see
    /// [`Pointer::pointer_id()`].
    ///
    /// This only has a meaning when the [action][Self::action] is one of [`Up`][MotionAction::Up],
    /// [`Down`][MotionAction::Down], [`PointerUp`][MotionAction::PointerUp],
    /// or [`PointerDown`][MotionAction::PointerDown].
    #[inline]
    pub fn pointer_index(&self) -> usize {
        let action = unsafe { ffi::AMotionEvent_getAction(self.ptr.as_ptr()) as u32 };
        let index = (action & ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_MASK)
            >> ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_SHIFT;
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
        unsafe { ffi::AMotionEvent_getPointerId(self.ptr.as_ptr(), pointer_index) }
    }
    */

    /// Returns the number of pointers in this event
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getpointercount)
    #[inline]
    pub fn pointer_count(&self) -> usize {
        unsafe { ffi::AMotionEvent_getPointerCount(self.ptr.as_ptr()) as usize }
    }

    /// An iterator over the pointers in this motion event
    #[inline]
    pub fn pointers(&self) -> PointersIter<'_> {
        PointersIter {
            event: self.ptr,
            next_index: 0,
            count: self.pointer_count(),
            _marker: std::marker::PhantomData,
        }
    }

    /// The pointer at a given pointer index. Panics if the pointer index is out of bounds.
    ///
    /// If you need to loop over all the pointers, prefer the [`pointers()`][Self::pointers] method.
    #[inline]
    pub fn pointer_at_index(&self, index: usize) -> Pointer<'_> {
        if index >= self.pointer_count() {
            panic!("Pointer index {} is out of bounds", index);
        }
        Pointer {
            event: self.ptr,
            index,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the size of the history contained in this event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_gethistorysize)
    #[inline]
    pub fn history_size(&self) -> usize {
        unsafe { ffi::AMotionEvent_getHistorySize(self.ptr.as_ptr()) as usize }
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

    /// Returns the state of any modifier keys that were pressed during the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getmetastate)
    #[inline]
    pub fn meta_state(&self) -> MetaState {
        unsafe { MetaState(ffi::AMotionEvent_getMetaState(self.ptr.as_ptr()) as u32) }
    }

    /// Returns the button state during this event, as a bitfield.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getbuttonstate)
    #[inline]
    pub fn button_state(&self) -> ButtonState {
        unsafe { ButtonState(ffi::AMotionEvent_getButtonState(self.ptr.as_ptr()) as u32) }
    }

    /// Returns the time of the start of this gesture, in the `java.lang.System.nanoTime()` time
    /// base
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getdowntime)
    #[inline]
    pub fn down_time(&self) -> i64 {
        unsafe { ffi::AMotionEvent_getDownTime(self.ptr.as_ptr()) }
    }

    /// Returns a bitfield indicating which edges were touched by this event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getedgeflags)
    #[inline]
    pub fn edge_flags(&self) -> EdgeFlags {
        unsafe { EdgeFlags(ffi::AMotionEvent_getEdgeFlags(self.ptr.as_ptr()) as u32) }
    }

    /// Returns the time of this event, in the `java.lang.System.nanoTime()` time base
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_geteventtime)
    #[inline]
    pub fn event_time(&self) -> i64 {
        unsafe { ffi::AMotionEvent_getEventTime(self.ptr.as_ptr()) }
    }

    /// The flags associated with a motion event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getflags)
    #[inline]
    pub fn flags(&self) -> MotionEventFlags {
        unsafe { MotionEventFlags(ffi::AMotionEvent_getFlags(self.ptr.as_ptr()) as u32) }
    }

    /// Returns the offset in the x direction between the coordinates and the raw coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getxoffset)
    #[inline]
    pub fn x_offset(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getXOffset(self.ptr.as_ptr()) }
    }

    /// Returns the offset in the y direction between the coordinates and the raw coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getyoffset)
    #[inline]
    pub fn y_offset(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getYOffset(self.ptr.as_ptr()) }
    }

    /// Returns the precision of the x value of the coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getxprecision)
    #[inline]
    pub fn x_precision(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getXPrecision(self.ptr.as_ptr()) }
    }

    /// Returns the precision of the y value of the coordinates
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#amotionevent_getyprecision)
    #[inline]
    pub fn y_precision(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getYPrecision(self.ptr.as_ptr()) }
    }
}

/// A view into the data of a specific pointer in a motion event.
#[derive(Debug)]
pub struct Pointer<'a> {
    event: NonNull<ffi::AInputEvent>,
    index: usize,
    _marker: std::marker::PhantomData<&'a MotionEvent>,
}

// TODO: thread safety?

impl<'a> Pointer<'a> {
    #[inline]
    pub fn pointer_index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn pointer_id(&self) -> i32 {
        unsafe { ffi::AMotionEvent_getPointerId(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn axis_value(&self, axis: Axis) -> f32 {
        unsafe {
            ffi::AMotionEvent_getAxisValue(
                self.event.as_ptr(),
                axis as i32,
                self.index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn orientation(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getOrientation(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn pressure(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getPressure(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn raw_x(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getRawX(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn raw_y(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getRawY(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn x(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getX(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn y(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getY(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn size(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getSize(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn tool_major(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getToolMajor(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn tool_minor(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getToolMinor(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn touch_major(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getTouchMajor(self.event.as_ptr(), self.index as ffi::size_t) }
    }

    #[inline]
    pub fn touch_minor(&self) -> f32 {
        unsafe { ffi::AMotionEvent_getTouchMinor(self.event.as_ptr(), self.index as ffi::size_t) }
    }
}

/// An iterator over the pointers in a [`MotionEvent`].
#[derive(Debug)]
pub struct PointersIter<'a> {
    event: NonNull<ffi::AInputEvent>,
    next_index: usize,
    count: usize,
    _marker: std::marker::PhantomData<&'a MotionEvent>,
}

// TODO: thread safety?

impl<'a> Iterator for PointersIter<'a> {
    type Item = Pointer<'a>;
    fn next(&mut self) -> Option<Pointer<'a>> {
        if self.next_index < self.count {
            let ptr = Pointer {
                event: self.event,
                index: self.next_index,
                _marker: std::marker::PhantomData,
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

/// Represents a view into a past moment of a motion event
#[derive(Debug)]
pub struct HistoricalMotionEvent<'a> {
    event: NonNull<ffi::AInputEvent>,
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
            ffi::AMotionEvent_getHistoricalEventTime(
                self.event.as_ptr(),
                self.history_index as ffi::size_t,
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
                ffi::AMotionEvent_getPointerCount(self.event.as_ptr()) as usize
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
    event: NonNull<ffi::AInputEvent>,
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
    event: NonNull<ffi::AInputEvent>,
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
            ffi::AMotionEvent_getPointerId(self.event.as_ptr(), self.pointer_index as ffi::size_t)
        }
    }

    #[inline]
    pub fn history_index(&self) -> usize {
        self.history_index
    }

    #[inline]
    pub fn axis_value(&self, axis: Axis) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalAxisValue(
                self.event.as_ptr(),
                axis as i32,
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn orientation(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalOrientation(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn pressure(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalPressure(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn raw_x(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalRawX(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn raw_y(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalRawY(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn x(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalX(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn y(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalY(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn size(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalSize(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn tool_major(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalToolMajor(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn tool_minor(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalToolMinor(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn touch_major(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalTouchMajor(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }

    #[inline]
    pub fn touch_minor(&self) -> f32 {
        unsafe {
            ffi::AMotionEvent_getHistoricalTouchMinor(
                self.event.as_ptr(),
                self.pointer_index as ffi::size_t,
                self.history_index as ffi::size_t,
            )
        }
    }
}

/// An iterator over the pointers in a historical motion event
#[derive(Debug)]
pub struct HistoricalPointersIter<'a> {
    event: NonNull<ffi::AInputEvent>,
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

/// A key event
///
/// Wraps an [`AInputEvent *`] of the [`ffi::AINPUT_EVENT_TYPE_KEY`] type.
///
/// For general discussion of key events in Android, see [the relevant
/// javadoc](https://developer.android.com/reference/android/view/KeyEvent).
///
/// [`AInputEvent *`]: https://developer.android.com/ndk/reference/group/input#ainputevent
#[derive(Debug)]
pub struct KeyEvent {
    ptr: NonNull<ffi::AInputEvent>,
}

// TODO: thread safety?

/// Key actions.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-27)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum KeyAction {
    Down = ffi::AKEY_EVENT_ACTION_DOWN,
    Up = ffi::AKEY_EVENT_ACTION_UP,
    Multiple = ffi::AKEY_EVENT_ACTION_MULTIPLE,
}

/// Key codes.
///
/// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#anonymous-enum-39)
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Keycode {
    Unknown = ffi::AKEYCODE_UNKNOWN,
    SoftLeft = ffi::AKEYCODE_SOFT_LEFT,
    SoftRight = ffi::AKEYCODE_SOFT_RIGHT,
    Home = ffi::AKEYCODE_HOME,
    Back = ffi::AKEYCODE_BACK,
    Call = ffi::AKEYCODE_CALL,
    Endcall = ffi::AKEYCODE_ENDCALL,
    Keycode0 = ffi::AKEYCODE_0,
    Keycode1 = ffi::AKEYCODE_1,
    Keycode2 = ffi::AKEYCODE_2,
    Keycode3 = ffi::AKEYCODE_3,
    Keycode4 = ffi::AKEYCODE_4,
    Keycode5 = ffi::AKEYCODE_5,
    Keycode6 = ffi::AKEYCODE_6,
    Keycode7 = ffi::AKEYCODE_7,
    Keycode8 = ffi::AKEYCODE_8,
    Keycode9 = ffi::AKEYCODE_9,
    Star = ffi::AKEYCODE_STAR,
    Pound = ffi::AKEYCODE_POUND,
    DpadUp = ffi::AKEYCODE_DPAD_UP,
    DpadDown = ffi::AKEYCODE_DPAD_DOWN,
    DpadLeft = ffi::AKEYCODE_DPAD_LEFT,
    DpadRight = ffi::AKEYCODE_DPAD_RIGHT,
    DpadCenter = ffi::AKEYCODE_DPAD_CENTER,
    VolumeUp = ffi::AKEYCODE_VOLUME_UP,
    VolumeDown = ffi::AKEYCODE_VOLUME_DOWN,
    Power = ffi::AKEYCODE_POWER,
    Camera = ffi::AKEYCODE_CAMERA,
    Clear = ffi::AKEYCODE_CLEAR,
    A = ffi::AKEYCODE_A,
    B = ffi::AKEYCODE_B,
    C = ffi::AKEYCODE_C,
    D = ffi::AKEYCODE_D,
    E = ffi::AKEYCODE_E,
    F = ffi::AKEYCODE_F,
    G = ffi::AKEYCODE_G,
    H = ffi::AKEYCODE_H,
    I = ffi::AKEYCODE_I,
    J = ffi::AKEYCODE_J,
    K = ffi::AKEYCODE_K,
    L = ffi::AKEYCODE_L,
    M = ffi::AKEYCODE_M,
    N = ffi::AKEYCODE_N,
    O = ffi::AKEYCODE_O,
    P = ffi::AKEYCODE_P,
    Q = ffi::AKEYCODE_Q,
    R = ffi::AKEYCODE_R,
    S = ffi::AKEYCODE_S,
    T = ffi::AKEYCODE_T,
    U = ffi::AKEYCODE_U,
    V = ffi::AKEYCODE_V,
    W = ffi::AKEYCODE_W,
    X = ffi::AKEYCODE_X,
    Y = ffi::AKEYCODE_Y,
    Z = ffi::AKEYCODE_Z,
    Comma = ffi::AKEYCODE_COMMA,
    Period = ffi::AKEYCODE_PERIOD,
    AltLeft = ffi::AKEYCODE_ALT_LEFT,
    AltRight = ffi::AKEYCODE_ALT_RIGHT,
    ShiftLeft = ffi::AKEYCODE_SHIFT_LEFT,
    ShiftRight = ffi::AKEYCODE_SHIFT_RIGHT,
    Tab = ffi::AKEYCODE_TAB,
    Space = ffi::AKEYCODE_SPACE,
    Sym = ffi::AKEYCODE_SYM,
    Explorer = ffi::AKEYCODE_EXPLORER,
    Envelope = ffi::AKEYCODE_ENVELOPE,
    Enter = ffi::AKEYCODE_ENTER,
    Del = ffi::AKEYCODE_DEL,
    Grave = ffi::AKEYCODE_GRAVE,
    Minus = ffi::AKEYCODE_MINUS,
    Equals = ffi::AKEYCODE_EQUALS,
    LeftBracket = ffi::AKEYCODE_LEFT_BRACKET,
    RightBracket = ffi::AKEYCODE_RIGHT_BRACKET,
    Backslash = ffi::AKEYCODE_BACKSLASH,
    Semicolon = ffi::AKEYCODE_SEMICOLON,
    Apostrophe = ffi::AKEYCODE_APOSTROPHE,
    Slash = ffi::AKEYCODE_SLASH,
    At = ffi::AKEYCODE_AT,
    Num = ffi::AKEYCODE_NUM,
    Headsethook = ffi::AKEYCODE_HEADSETHOOK,
    Focus = ffi::AKEYCODE_FOCUS,
    Plus = ffi::AKEYCODE_PLUS,
    Menu = ffi::AKEYCODE_MENU,
    Notification = ffi::AKEYCODE_NOTIFICATION,
    Search = ffi::AKEYCODE_SEARCH,
    MediaPlayPause = ffi::AKEYCODE_MEDIA_PLAY_PAUSE,
    MediaStop = ffi::AKEYCODE_MEDIA_STOP,
    MediaNext = ffi::AKEYCODE_MEDIA_NEXT,
    MediaPrevious = ffi::AKEYCODE_MEDIA_PREVIOUS,
    MediaRewind = ffi::AKEYCODE_MEDIA_REWIND,
    MediaFastForward = ffi::AKEYCODE_MEDIA_FAST_FORWARD,
    Mute = ffi::AKEYCODE_MUTE,
    PageUp = ffi::AKEYCODE_PAGE_UP,
    PageDown = ffi::AKEYCODE_PAGE_DOWN,
    Pictsymbols = ffi::AKEYCODE_PICTSYMBOLS,
    SwitchCharset = ffi::AKEYCODE_SWITCH_CHARSET,
    ButtonA = ffi::AKEYCODE_BUTTON_A,
    ButtonB = ffi::AKEYCODE_BUTTON_B,
    ButtonC = ffi::AKEYCODE_BUTTON_C,
    ButtonX = ffi::AKEYCODE_BUTTON_X,
    ButtonY = ffi::AKEYCODE_BUTTON_Y,
    ButtonZ = ffi::AKEYCODE_BUTTON_Z,
    ButtonL1 = ffi::AKEYCODE_BUTTON_L1,
    ButtonR1 = ffi::AKEYCODE_BUTTON_R1,
    ButtonL2 = ffi::AKEYCODE_BUTTON_L2,
    ButtonR2 = ffi::AKEYCODE_BUTTON_R2,
    ButtonThumbl = ffi::AKEYCODE_BUTTON_THUMBL,
    ButtonThumbr = ffi::AKEYCODE_BUTTON_THUMBR,
    ButtonStart = ffi::AKEYCODE_BUTTON_START,
    ButtonSelect = ffi::AKEYCODE_BUTTON_SELECT,
    ButtonMode = ffi::AKEYCODE_BUTTON_MODE,
    Escape = ffi::AKEYCODE_ESCAPE,
    ForwardDel = ffi::AKEYCODE_FORWARD_DEL,
    CtrlLeft = ffi::AKEYCODE_CTRL_LEFT,
    CtrlRight = ffi::AKEYCODE_CTRL_RIGHT,
    CapsLock = ffi::AKEYCODE_CAPS_LOCK,
    ScrollLock = ffi::AKEYCODE_SCROLL_LOCK,
    MetaLeft = ffi::AKEYCODE_META_LEFT,
    MetaRight = ffi::AKEYCODE_META_RIGHT,
    Function = ffi::AKEYCODE_FUNCTION,
    Sysrq = ffi::AKEYCODE_SYSRQ,
    Break = ffi::AKEYCODE_BREAK,
    MoveHome = ffi::AKEYCODE_MOVE_HOME,
    MoveEnd = ffi::AKEYCODE_MOVE_END,
    Insert = ffi::AKEYCODE_INSERT,
    Forward = ffi::AKEYCODE_FORWARD,
    MediaPlay = ffi::AKEYCODE_MEDIA_PLAY,
    MediaPause = ffi::AKEYCODE_MEDIA_PAUSE,
    MediaClose = ffi::AKEYCODE_MEDIA_CLOSE,
    MediaEject = ffi::AKEYCODE_MEDIA_EJECT,
    MediaRecord = ffi::AKEYCODE_MEDIA_RECORD,
    F1 = ffi::AKEYCODE_F1,
    F2 = ffi::AKEYCODE_F2,
    F3 = ffi::AKEYCODE_F3,
    F4 = ffi::AKEYCODE_F4,
    F5 = ffi::AKEYCODE_F5,
    F6 = ffi::AKEYCODE_F6,
    F7 = ffi::AKEYCODE_F7,
    F8 = ffi::AKEYCODE_F8,
    F9 = ffi::AKEYCODE_F9,
    F10 = ffi::AKEYCODE_F10,
    F11 = ffi::AKEYCODE_F11,
    F12 = ffi::AKEYCODE_F12,
    NumLock = ffi::AKEYCODE_NUM_LOCK,
    Numpad0 = ffi::AKEYCODE_NUMPAD_0,
    Numpad1 = ffi::AKEYCODE_NUMPAD_1,
    Numpad2 = ffi::AKEYCODE_NUMPAD_2,
    Numpad3 = ffi::AKEYCODE_NUMPAD_3,
    Numpad4 = ffi::AKEYCODE_NUMPAD_4,
    Numpad5 = ffi::AKEYCODE_NUMPAD_5,
    Numpad6 = ffi::AKEYCODE_NUMPAD_6,
    Numpad7 = ffi::AKEYCODE_NUMPAD_7,
    Numpad8 = ffi::AKEYCODE_NUMPAD_8,
    Numpad9 = ffi::AKEYCODE_NUMPAD_9,
    NumpadDivide = ffi::AKEYCODE_NUMPAD_DIVIDE,
    NumpadMultiply = ffi::AKEYCODE_NUMPAD_MULTIPLY,
    NumpadSubtract = ffi::AKEYCODE_NUMPAD_SUBTRACT,
    NumpadAdd = ffi::AKEYCODE_NUMPAD_ADD,
    NumpadDot = ffi::AKEYCODE_NUMPAD_DOT,
    NumpadComma = ffi::AKEYCODE_NUMPAD_COMMA,
    NumpadEnter = ffi::AKEYCODE_NUMPAD_ENTER,
    NumpadEquals = ffi::AKEYCODE_NUMPAD_EQUALS,
    NumpadLeftParen = ffi::AKEYCODE_NUMPAD_LEFT_PAREN,
    NumpadRightParen = ffi::AKEYCODE_NUMPAD_RIGHT_PAREN,
    VolumeMute = ffi::AKEYCODE_VOLUME_MUTE,
    Info = ffi::AKEYCODE_INFO,
    ChannelUp = ffi::AKEYCODE_CHANNEL_UP,
    ChannelDown = ffi::AKEYCODE_CHANNEL_DOWN,
    ZoomIn = ffi::AKEYCODE_ZOOM_IN,
    ZoomOut = ffi::AKEYCODE_ZOOM_OUT,
    Tv = ffi::AKEYCODE_TV,
    Window = ffi::AKEYCODE_WINDOW,
    Guide = ffi::AKEYCODE_GUIDE,
    Dvr = ffi::AKEYCODE_DVR,
    Bookmark = ffi::AKEYCODE_BOOKMARK,
    Captions = ffi::AKEYCODE_CAPTIONS,
    Settings = ffi::AKEYCODE_SETTINGS,
    TvPower = ffi::AKEYCODE_TV_POWER,
    TvInput = ffi::AKEYCODE_TV_INPUT,
    StbPower = ffi::AKEYCODE_STB_POWER,
    StbInput = ffi::AKEYCODE_STB_INPUT,
    AvrPower = ffi::AKEYCODE_AVR_POWER,
    AvrInput = ffi::AKEYCODE_AVR_INPUT,
    ProgRed = ffi::AKEYCODE_PROG_RED,
    ProgGreen = ffi::AKEYCODE_PROG_GREEN,
    ProgYellow = ffi::AKEYCODE_PROG_YELLOW,
    ProgBlue = ffi::AKEYCODE_PROG_BLUE,
    AppSwitch = ffi::AKEYCODE_APP_SWITCH,
    Button1 = ffi::AKEYCODE_BUTTON_1,
    Button2 = ffi::AKEYCODE_BUTTON_2,
    Button3 = ffi::AKEYCODE_BUTTON_3,
    Button4 = ffi::AKEYCODE_BUTTON_4,
    Button5 = ffi::AKEYCODE_BUTTON_5,
    Button6 = ffi::AKEYCODE_BUTTON_6,
    Button7 = ffi::AKEYCODE_BUTTON_7,
    Button8 = ffi::AKEYCODE_BUTTON_8,
    Button9 = ffi::AKEYCODE_BUTTON_9,
    Button10 = ffi::AKEYCODE_BUTTON_10,
    Button11 = ffi::AKEYCODE_BUTTON_11,
    Button12 = ffi::AKEYCODE_BUTTON_12,
    Button13 = ffi::AKEYCODE_BUTTON_13,
    Button14 = ffi::AKEYCODE_BUTTON_14,
    Button15 = ffi::AKEYCODE_BUTTON_15,
    Button16 = ffi::AKEYCODE_BUTTON_16,
    LanguageSwitch = ffi::AKEYCODE_LANGUAGE_SWITCH,
    MannerMode = ffi::AKEYCODE_MANNER_MODE,
    Keycode3dMode = ffi::AKEYCODE_3D_MODE,
    Contacts = ffi::AKEYCODE_CONTACTS,
    Calendar = ffi::AKEYCODE_CALENDAR,
    Music = ffi::AKEYCODE_MUSIC,
    Calculator = ffi::AKEYCODE_CALCULATOR,
    ZenkakuHankaku = ffi::AKEYCODE_ZENKAKU_HANKAKU,
    Eisu = ffi::AKEYCODE_EISU,
    Muhenkan = ffi::AKEYCODE_MUHENKAN,
    Henkan = ffi::AKEYCODE_HENKAN,
    KatakanaHiragana = ffi::AKEYCODE_KATAKANA_HIRAGANA,
    Yen = ffi::AKEYCODE_YEN,
    Ro = ffi::AKEYCODE_RO,
    Kana = ffi::AKEYCODE_KANA,
    Assist = ffi::AKEYCODE_ASSIST,
    BrightnessDown = ffi::AKEYCODE_BRIGHTNESS_DOWN,
    BrightnessUp = ffi::AKEYCODE_BRIGHTNESS_UP,
    MediaAudioTrack = ffi::AKEYCODE_MEDIA_AUDIO_TRACK,
    Sleep = ffi::AKEYCODE_SLEEP,
    Wakeup = ffi::AKEYCODE_WAKEUP,
    Pairing = ffi::AKEYCODE_PAIRING,
    MediaTopMenu = ffi::AKEYCODE_MEDIA_TOP_MENU,
    Keycode11 = ffi::AKEYCODE_11,
    Keycode12 = ffi::AKEYCODE_12,
    LastChannel = ffi::AKEYCODE_LAST_CHANNEL,
    TvDataService = ffi::AKEYCODE_TV_DATA_SERVICE,
    VoiceAssist = ffi::AKEYCODE_VOICE_ASSIST,
    TvRadioService = ffi::AKEYCODE_TV_RADIO_SERVICE,
    TvTeletext = ffi::AKEYCODE_TV_TELETEXT,
    TvNumberEntry = ffi::AKEYCODE_TV_NUMBER_ENTRY,
    TvTerrestrialAnalog = ffi::AKEYCODE_TV_TERRESTRIAL_ANALOG,
    TvTerrestrialDigital = ffi::AKEYCODE_TV_TERRESTRIAL_DIGITAL,
    TvSatellite = ffi::AKEYCODE_TV_SATELLITE,
    TvSatelliteBs = ffi::AKEYCODE_TV_SATELLITE_BS,
    TvSatelliteCs = ffi::AKEYCODE_TV_SATELLITE_CS,
    TvSatelliteService = ffi::AKEYCODE_TV_SATELLITE_SERVICE,
    TvNetwork = ffi::AKEYCODE_TV_NETWORK,
    TvAntennaCable = ffi::AKEYCODE_TV_ANTENNA_CABLE,
    TvInputHdmi1 = ffi::AKEYCODE_TV_INPUT_HDMI_1,
    TvInputHdmi2 = ffi::AKEYCODE_TV_INPUT_HDMI_2,
    TvInputHdmi3 = ffi::AKEYCODE_TV_INPUT_HDMI_3,
    TvInputHdmi4 = ffi::AKEYCODE_TV_INPUT_HDMI_4,
    TvInputComposite1 = ffi::AKEYCODE_TV_INPUT_COMPOSITE_1,
    TvInputComposite2 = ffi::AKEYCODE_TV_INPUT_COMPOSITE_2,
    TvInputComponent1 = ffi::AKEYCODE_TV_INPUT_COMPONENT_1,
    TvInputComponent2 = ffi::AKEYCODE_TV_INPUT_COMPONENT_2,
    TvInputVga1 = ffi::AKEYCODE_TV_INPUT_VGA_1,
    TvAudioDescription = ffi::AKEYCODE_TV_AUDIO_DESCRIPTION,
    TvAudioDescriptionMixUp = ffi::AKEYCODE_TV_AUDIO_DESCRIPTION_MIX_UP,
    TvAudioDescriptionMixDown = ffi::AKEYCODE_TV_AUDIO_DESCRIPTION_MIX_DOWN,
    TvZoomMode = ffi::AKEYCODE_TV_ZOOM_MODE,
    TvContentsMenu = ffi::AKEYCODE_TV_CONTENTS_MENU,
    TvMediaContextMenu = ffi::AKEYCODE_TV_MEDIA_CONTEXT_MENU,
    TvTimerProgramming = ffi::AKEYCODE_TV_TIMER_PROGRAMMING,
    Help = ffi::AKEYCODE_HELP,
    NavigatePrevious = ffi::AKEYCODE_NAVIGATE_PREVIOUS,
    NavigateNext = ffi::AKEYCODE_NAVIGATE_NEXT,
    NavigateIn = ffi::AKEYCODE_NAVIGATE_IN,
    NavigateOut = ffi::AKEYCODE_NAVIGATE_OUT,
    StemPrimary = ffi::AKEYCODE_STEM_PRIMARY,
    Stem1 = ffi::AKEYCODE_STEM_1,
    Stem2 = ffi::AKEYCODE_STEM_2,
    Stem3 = ffi::AKEYCODE_STEM_3,
    DpadUpLeft = ffi::AKEYCODE_DPAD_UP_LEFT,
    DpadDownLeft = ffi::AKEYCODE_DPAD_DOWN_LEFT,
    DpadUpRight = ffi::AKEYCODE_DPAD_UP_RIGHT,
    DpadDownRight = ffi::AKEYCODE_DPAD_DOWN_RIGHT,
    MediaSkipForward = ffi::AKEYCODE_MEDIA_SKIP_FORWARD,
    MediaSkipBackward = ffi::AKEYCODE_MEDIA_SKIP_BACKWARD,
    MediaStepForward = ffi::AKEYCODE_MEDIA_STEP_FORWARD,
    MediaStepBackward = ffi::AKEYCODE_MEDIA_STEP_BACKWARD,
    SoftSleep = ffi::AKEYCODE_SOFT_SLEEP,
    Cut = ffi::AKEYCODE_CUT,
    Copy = ffi::AKEYCODE_COPY,
    Paste = ffi::AKEYCODE_PASTE,
    SystemNavigationUp = ffi::AKEYCODE_SYSTEM_NAVIGATION_UP,
    SystemNavigationDown = ffi::AKEYCODE_SYSTEM_NAVIGATION_DOWN,
    SystemNavigationLeft = ffi::AKEYCODE_SYSTEM_NAVIGATION_LEFT,
    SystemNavigationRight = ffi::AKEYCODE_SYSTEM_NAVIGATION_RIGHT,
    AllApps = ffi::AKEYCODE_ALL_APPS,
    Refresh = ffi::AKEYCODE_REFRESH,
    ThumbsUp = ffi::AKEYCODE_THUMBS_UP,
    ThumbsDown = ffi::AKEYCODE_THUMBS_DOWN,
    ProfileSwitch = ffi::AKEYCODE_PROFILE_SWITCH,
}

impl KeyEvent {
    /// Constructs a KeyEvent from a pointer to a native [`ffi::AInputEvent`]
    ///
    /// # Safety
    /// By calling this method, you assert that the pointer is a valid, non-null pointer to an
    /// [`ffi::AInputEvent`], and that [`ffi::AInputEvent`] is an `AKeyEvent`.
    #[inline]
    pub unsafe fn from_ptr(ptr: NonNull<ffi::AInputEvent>) -> Self {
        Self { ptr }
    }

    /// Returns a pointer to the native [`ffi::AInputEvent`]
    #[inline]
    pub fn ptr(&self) -> NonNull<ffi::AInputEvent> {
        self.ptr
    }

    /// Returns the key action represented by this event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getaction)
    #[inline]
    pub fn action(&self) -> KeyAction {
        let action = unsafe { ffi::AKeyEvent_getAction(self.ptr.as_ptr()) as u32 };
        action.try_into().unwrap()
    }

    /// Get the source of the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#ainputevent_getsource)
    #[inline]
    pub fn source(&self) -> Source {
        let source = unsafe { ffi::AInputEvent_getSource(self.ptr.as_ptr()) as u32 };
        source.try_into().unwrap_or(Source::Unknown)
    }

    /// Get the device id associated with the event.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#ainputevent_getdeviceid)
    #[inline]
    pub fn device_id(&self) -> i32 {
        unsafe { ffi::AInputEvent_getDeviceId(self.ptr.as_ptr()) }
    }

    /// Returns the last time the key was pressed.  This is on the scale of
    /// `java.lang.System.nanoTime()`, which has nanosecond precision, but no defined start time.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getdowntime)
    #[inline]
    pub fn down_time(&self) -> i64 {
        unsafe { ffi::AKeyEvent_getDownTime(self.ptr.as_ptr()) }
    }

    /// Returns the time this event occured.  This is on the scale of
    /// `java.lang.System.nanoTime()`, which has nanosecond precision, but no defined start time.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_geteventtime)
    #[inline]
    pub fn event_time(&self) -> i64 {
        unsafe { ffi::AKeyEvent_getEventTime(self.ptr.as_ptr()) }
    }

    /// Returns the keycode associated with this key event
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getkeycode)
    #[inline]
    pub fn key_code(&self) -> Keycode {
        let keycode = unsafe { ffi::AKeyEvent_getKeyCode(self.ptr.as_ptr()) as u32 };
        keycode.try_into().unwrap_or(Keycode::Unknown)
    }

    /// Returns the number of repeats of a key.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getrepeatcount)
    #[inline]
    pub fn repeat_count(&self) -> i32 {
        unsafe { ffi::AKeyEvent_getRepeatCount(self.ptr.as_ptr()) }
    }

    /// Returns the hardware keycode of a key.  This varies from device to device.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getscancode)
    #[inline]
    pub fn scan_code(&self) -> i32 {
        unsafe { ffi::AKeyEvent_getScanCode(self.ptr.as_ptr()) }
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
        self.0 & ffi::AKEY_EVENT_FLAG_CANCELED != 0
    }
    #[inline]
    pub fn cancelled_long_press(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_CANCELED_LONG_PRESS != 0
    }
    #[inline]
    pub fn editor_action(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_EDITOR_ACTION != 0
    }
    #[inline]
    pub fn fallback(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_FALLBACK != 0
    }
    #[inline]
    pub fn from_system(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_FROM_SYSTEM != 0
    }
    #[inline]
    pub fn keep_touch_mode(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_KEEP_TOUCH_MODE != 0
    }
    #[inline]
    pub fn long_press(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_LONG_PRESS != 0
    }
    #[inline]
    pub fn soft_keyboard(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_SOFT_KEYBOARD != 0
    }
    #[inline]
    pub fn tracking(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_TRACKING != 0
    }
    #[inline]
    pub fn virtual_hard_key(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_VIRTUAL_HARD_KEY != 0
    }
    #[inline]
    pub fn woke_here(&self) -> bool {
        self.0 & ffi::AKEY_EVENT_FLAG_WOKE_HERE != 0
    }
}

impl KeyEvent {
    /// Flags associated with this [`KeyEvent`].
    ///
    /// See [the NDK docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getflags)
    #[inline]
    pub fn flags(&self) -> KeyEventFlags {
        unsafe { KeyEventFlags(ffi::AKeyEvent_getFlags(self.ptr.as_ptr()) as u32) }
    }

    /// Returns the state of the modifiers during this key event, represented by a bitmask.
    ///
    /// See [the NDK
    /// docs](https://developer.android.com/ndk/reference/group/input#akeyevent_getmetastate)
    #[inline]
    pub fn meta_state(&self) -> MetaState {
        unsafe { MetaState(ffi::AKeyEvent_getMetaState(self.ptr.as_ptr()) as u32) }
    }
}
