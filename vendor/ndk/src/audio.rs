//! Bindings for [`AAudioStream`] and [`AAudioStreamBuilder`]
//!
//! See [the NDK guide](https://developer.android.com/ndk/guides/audio/aaudio/aaudio) for
//! design and usage instructions, and [the NDK reference](https://developer.android.com/ndk/reference/group/audio)
//! for an API overview.
//!
//! [`AAudioStream`]: https://developer.android.com/ndk/reference/group/audio#aaudiostream
//! [`AAudioStreamBuilder`]: https://developer.android.com/ndk/reference/group/audio#aaudiostreambuilder
#![cfg(feature = "audio")]

use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::{
    borrow::Cow,
    convert::TryFrom,
    ffi::{c_void, CStr},
    fmt,
    mem::MaybeUninit,
    num::NonZeroI32,
    ptr::NonNull,
};
use thiserror::Error;

/// Specifying if audio may or may not be captured by other apps or the system.
///
/// Note that these match the equivalent values in [`android.media.AudioAttributes`]
/// in the Android Java API.
///
/// [`android.media.AudioAttributes`]: https://developer.android.com/reference/android/media/AudioAttributes
#[cfg(feature = "api-level-29")]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioAllowedCapturePolicy {
    /// Indicates that the audio may be captured by any app.
    ///
    /// For privacy, the following [usages][AudioUsage] can not be recorded: `VoiceCommunication*`,
    /// `Notification*`, `Assistance*` and [`Assistant`][AudioUsage::Assistant].
    ///
    /// On Android Q, this means only [`Media`][AudioUsage::Media] and [`Game`][AudioUsage::Game] may be captured.
    ///
    /// See [`MediaProjection`] and [`AudioStreamBuilder::allowed_capture_policy()`].
    ///
    /// [`MediaProjection`]: https://developer.android.com/reference/android/media/projection/MediaProjection
    AllowCaptureByAll = ffi::AAUDIO_ALLOW_CAPTURE_BY_ALL,
    /// Indicates that the audio may only be captured by system apps.
    ///
    /// System apps can capture for many purposes like accessibility, live captions, user
    /// guidance... but abide to the following restrictions:
    /// - the audio cannot leave the device;
    /// - the audio cannot be passed to a third party app;
    /// - the audio cannot be recorded at a higher quality than 16kHz 16bit mono.
    ///
    /// See [`AudioStreamBuilder::allowed_capture_policy()`].
    AllowCaptureBySystem = ffi::AAUDIO_ALLOW_CAPTURE_BY_SYSTEM,
    /// Indicates that the audio may not be recorded by any app, even if it is a system app.
    ///
    /// It is encouraged to use [`AllowCaptureBySystem`][Self::AllowCaptureBySystem] instead of
    /// this value as system apps provide significant and useful features for the user (such as
    /// live captioning and accessibility).
    AllowCaptureByNone = ffi::AAUDIO_ALLOW_CAPTURE_BY_NONE,
}

/// The ContentType attribute describes "what" you are playing.
/// It expresses the general category of the content. This information is optional.
/// But in case it is known (for instance `Movie` for a
/// movie streaming service or `Speech` for
/// an audio book application) this information might be used by the audio framework to
/// enforce audio focus.
///
/// Note that these match the equivalent values in [`android.media.AudioAttributes`]
/// in the Android Java API.
///
/// [`android.media.AudioAttributes`]: https://developer.android.com/reference/android/media/AudioAttributes
#[cfg(feature = "api-level-28")]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioContentType {
    /// Use this for spoken voice, audio books, etcetera.
    Speech = ffi::AAUDIO_CONTENT_TYPE_SPEECH,
    /// Use this for pre-recorded or live music.
    Music = ffi::AAUDIO_CONTENT_TYPE_MUSIC,
    /// Use this for a movie or video soundtrack.
    Movie = ffi::AAUDIO_CONTENT_TYPE_MOVIE,
    /// Use this for sound is designed to accompany a user action,
    /// such as a click or beep sound made when the user presses a button.
    Sonification = ffi::AAUDIO_CONTENT_TYPE_SONIFICATION,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioDirection {
    /// Audio data will travel out of the device, for example through a speaker.
    Input = ffi::AAUDIO_DIRECTION_INPUT,
    /// Audio data will travel into the device, for example from a microphone.
    Output = ffi::AAUDIO_DIRECTION_OUTPUT,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[allow(non_camel_case_types)]
pub enum AudioFormat {
    /// This format uses the float data type.
    /// The nominal range of the data is [-1.0f32, 1.0f32).
    /// Values outside that range may be clipped.
    ///
    /// See also `audioData` at
    /// <a href="https://developer.android.com/reference/android/media/AudioTrack#write(float[], int, int, int)"><code>AudioTrack#write(float[], int, int, int)</code></a>.
    PCM_Float = ffi::AAUDIO_FORMAT_PCM_FLOAT,
    /// This format uses the i16 data type.
    /// The maximum range of the data is -32768 to 32767.
    PCM_I16 = ffi::AAUDIO_FORMAT_PCM_I16,
    Invalid = ffi::AAUDIO_FORMAT_INVALID,
    Unspecified = ffi::AAUDIO_FORMAT_UNSPECIFIED,
}

/// Defines the audio source.
/// An audio source defines both a default physical source of audio signal, and a recording
/// configuration.
///
/// Note that these match the equivalent values in MediaRecorder.AudioSource in the Android Java API.
#[cfg(feature = "api-level-28")]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioInputPreset {
    /// Use this preset when other presets do not apply.
    Generic = ffi::AAUDIO_INPUT_PRESET_GENERIC,
    /// Use this preset when recording video.
    Camcorder = ffi::AAUDIO_INPUT_PRESET_CAMCORDER,
    /// Use this preset when doing speech recognition.
    VoiceRecognition = ffi::AAUDIO_INPUT_PRESET_VOICE_RECOGNITION,
    /// Use this preset when doing telephony or voice messaging.
    VoiceCommunication = ffi::AAUDIO_INPUT_PRESET_VOICE_COMMUNICATION,
    /// Use this preset to obtain an input with no effects.
    /// Note that this input will not have automatic gain control
    /// so the recorded volume may be very low.
    Unprocessed = ffi::AAUDIO_INPUT_PRESET_UNPROCESSED,
    /// Use this preset for capturing audio meant to be processed in real time
    /// and played back for live performance (e.g karaoke).
    /// The capture path will minimize latency and coupling with playback path.
    #[cfg(feature = "api-level-29")]
    VoicePerformance = ffi::AAUDIO_INPUT_PRESET_VOICE_PERFORMANCE,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioPerformanceMode {
    /// No particular performance needs. Default.
    None = ffi::AAUDIO_PERFORMANCE_MODE_NONE,
    /// Extending battery life is more important than low latency.
    ///
    /// This mode is not supported in input streams.
    /// For input, mode NONE will be used if this is requested.
    PowerSaving = ffi::AAUDIO_PERFORMANCE_MODE_POWER_SAVING,
    /// Reducing latency is more important than battery life.
    LowLatency = ffi::AAUDIO_PERFORMANCE_MODE_LOW_LATENCY,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioSharingMode {
    /// This will be the only stream using a particular source or sink.
    /// This mode will provide the lowest possible latency.
    /// You should close Exclusive streams immediately when you are not using them.
    Exclusive = ffi::AAUDIO_SHARING_MODE_EXCLUSIVE,
    /// Multiple applications will be mixed by the AAudio Server.
    /// This will have higher latency than the Exclusive mode.
    Shared = ffi::AAUDIO_SHARING_MODE_SHARED,
}

/// The Usage attribute expresses "why" you are playing a sound, what is this sound used for.
/// This information is used by certain platforms or routing policies
/// to make more refined volume or routing decisions.
///
/// Note that these match the equivalent values in [`android.media.AudioAttributes`]
/// in the Android Java API.
///
/// [`android.media.AudioAttributes`]: https://developer.android.com/reference/android/media/AudioAttributes
#[cfg(feature = "api-level-28")]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioUsage {
    /// Use this for streaming media, music performance, video, podcasts, etcetera.
    Media = ffi::AAUDIO_USAGE_MEDIA,
    /// Use this for voice over IP, telephony, etcetera.
    VoiceCommunication = ffi::AAUDIO_USAGE_VOICE_COMMUNICATION,
    /// Use this for sounds associated with telephony such as busy tones, DTMF, etcetera.
    VoiceCommunicationSignalling = ffi::AAUDIO_USAGE_VOICE_COMMUNICATION_SIGNALLING,
    /// Use this to demand the users attention.
    Alarm = ffi::AAUDIO_USAGE_ALARM,
    /// Use this for notifying the user when a message has arrived or some
    /// other background event has occured.
    Notification = ffi::AAUDIO_USAGE_NOTIFICATION,
    /// Use this when the phone rings.
    NotificationRingtone = ffi::AAUDIO_USAGE_NOTIFICATION_RINGTONE,
    /// Use this to attract the users attention when, for example, the battery is low.
    NotificationEvent = ffi::AAUDIO_USAGE_NOTIFICATION_EVENT,
    /// Use this for screen readers, etcetera.
    AssistanceAccessibility = ffi::AAUDIO_USAGE_ASSISTANCE_ACCESSIBILITY,
    /// Use this for driving or navigation directions.
    AssistanceNavigationGuidance = ffi::AAUDIO_USAGE_ASSISTANCE_NAVIGATION_GUIDANCE,
    /// Use this for user interface sounds, beeps, etcetera.
    AssistanceSonification = ffi::AAUDIO_USAGE_ASSISTANCE_SONIFICATION,
    /// Use this for game audio and sound effects.
    Game = ffi::AAUDIO_USAGE_GAME,
    /// Use this for audio responses to user queries, audio instructions or help utterances.
    Assistant = ffi::AAUDIO_USAGE_ASSISTANT,
    /// Use this in case of playing sounds in an emergency.
    /// Privileged MODIFY_AUDIO_ROUTING permission required.
    SystemEmergency = ffi::AAUDIO_SYSTEM_USAGE_EMERGENCY,
    /// Use this for safety sounds and alerts, for example backup camera obstacle detection.
    /// Privileged MODIFY_AUDIO_ROUTING permission required.
    SystemSafety = ffi::AAUDIO_SYSTEM_USAGE_SAFETY,
    /// Use this for vehicle status alerts and information, for example the check engine light.
    /// Privileged MODIFY_AUDIO_ROUTING permission required.
    SystemVehicleStatus = ffi::AAUDIO_SYSTEM_USAGE_VEHICLE_STATUS,
    /// Use this for traffic announcements, etc.
    /// Privileged MODIFY_AUDIO_ROUTING permission required.
    SystemAnnouncement = ffi::AAUDIO_SYSTEM_USAGE_ANNOUNCEMENT,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum AudioStreamState {
    Uninitialized = ffi::AAUDIO_STREAM_STATE_UNINITIALIZED,
    Unknown = ffi::AAUDIO_STREAM_STATE_UNKNOWN,
    Open = ffi::AAUDIO_STREAM_STATE_OPEN,
    Starting = ffi::AAUDIO_STREAM_STATE_STARTING,
    Started = ffi::AAUDIO_STREAM_STATE_STARTED,
    Pausing = ffi::AAUDIO_STREAM_STATE_PAUSING,
    Paused = ffi::AAUDIO_STREAM_STATE_PAUSED,
    Flushing = ffi::AAUDIO_STREAM_STATE_FLUSHING,
    Flushed = ffi::AAUDIO_STREAM_STATE_FLUSHED,
    Stopping = ffi::AAUDIO_STREAM_STATE_STOPPING,
    Stopped = ffi::AAUDIO_STREAM_STATE_STOPPED,
    Closing = ffi::AAUDIO_STREAM_STATE_CLOSING,
    Closed = ffi::AAUDIO_STREAM_STATE_CLOSED,
    Disconnected = ffi::AAUDIO_STREAM_STATE_DISCONNECTED,
}

impl AudioStreamState {
    pub fn to_text(self) -> Cow<'static, str> {
        let ptr = unsafe {
            CStr::from_ptr(ffi::AAudio_convertStreamStateToText(
                self as ffi::aaudio_stream_state_t,
            ))
        };
        ptr.to_string_lossy()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SessionId {
    None,
    Allocated(NonZeroI32),
}

#[derive(Copy, Clone, Debug)]
pub struct Timestamp {
    pub frame_position: i64,
    pub time_nanoseconds: i64,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Clockid {
    Monotonic = ffi::CLOCK_MONOTONIC,
    Boottime = ffi::CLOCK_BOOTTIME,
}

/// Value returned the data callback function.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AudioCallbackResult {
    /// Continue calling the callback.
    Continue = ffi::AAUDIO_CALLBACK_RESULT_CONTINUE,
    /// Stop calling the callback.
    ///
    /// The application will still need to call [`AudioStream::request_pause()`]
    /// or [`AudioStream::request_stop()`].
    Stop = ffi::AAUDIO_CALLBACK_RESULT_STOP,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AudioErrorResult {
    Base = ffi::AAUDIO_ERROR_BASE,
    /// The audio device was disconnected. This could occur, for example, when headphones
    /// are plugged in or unplugged. The stream cannot be used after the device is disconnected.
    /// Applications should stop and close the stream.
    /// If this error is received in an error callback then another thread should be
    /// used to stop and close the stream.
    Disconnected = ffi::AAUDIO_ERROR_DISCONNECTED,
    /// An invalid parameter was passed to AAudio.
    IllegalArgument = ffi::AAUDIO_ERROR_ILLEGAL_ARGUMENT,
    /// The requested operation is not appropriate for the current state of AAudio.
    Internal = ffi::AAUDIO_ERROR_INTERNAL,
    /// The requested operation is not appropriate for the current state of AAudio.
    InvalidState = ffi::AAUDIO_ERROR_INVALID_STATE,
    /// The server rejected the handle used to identify the stream.
    InvalidHandle = ffi::AAUDIO_ERROR_INVALID_HANDLE,
    /// The function is not implemented for this stream.
    Unimplemented = ffi::AAUDIO_ERROR_UNIMPLEMENTED,
    /// A resource or information is unavailable.
    /// This could occur when an application tries to open too many streams,
    /// or a timestamp is not available.
    Unavailable = ffi::AAUDIO_ERROR_UNAVAILABLE,
    /// Memory could not be allocated.
    NoFreeHandles = ffi::AAUDIO_ERROR_NO_FREE_HANDLES,
    /// Memory could not be allocated.
    NoMemory = ffi::AAUDIO_ERROR_NO_MEMORY,
    Null = ffi::AAUDIO_ERROR_NULL,
    Timeout = ffi::AAUDIO_ERROR_TIMEOUT,
    WouldBlock = ffi::AAUDIO_ERROR_WOULD_BLOCK,
    /// The requested data format is not supported.
    InvalidFormat = ffi::AAUDIO_ERROR_INVALID_FORMAT,
    /// A requested was out of range.
    OutOfRange = ffi::AAUDIO_ERROR_OUT_OF_RANGE,
    /// The audio service was not available.
    NoService = ffi::AAUDIO_ERROR_NO_SERVICE,
    /// The requested sample rate was not supported.
    InvalidRate = ffi::AAUDIO_ERROR_INVALID_RATE,
}

impl AudioErrorResult {
    pub fn to_text(self) -> Cow<'static, str> {
        let ptr = unsafe {
            CStr::from_ptr(ffi::AAudio_convertStreamStateToText(
                self as ffi::aaudio_result_t,
            ))
        };
        ptr.to_string_lossy()
    }
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("error Audio result ({0:?})")]
    ErrorResult(AudioErrorResult),
    #[error("unknown AAudio error result ({0})")]
    UnknownResult(i32),
    #[error("unsupported AAudio result value received ({0})")]
    UnsupportedValue(i32),
}

impl AudioError {
    pub(crate) fn from_result<T>(
        result: ffi::aaudio_result_t,
        on_success: impl FnOnce() -> T,
    ) -> Result<T> {
        use AudioErrorResult::*;
        let result = match result {
            value if value >= 0 => return Ok(on_success()),
            ffi::AAUDIO_ERROR_BASE => Base,
            ffi::AAUDIO_ERROR_DISCONNECTED => Disconnected,
            ffi::AAUDIO_ERROR_ILLEGAL_ARGUMENT => IllegalArgument,
            ffi::AAUDIO_ERROR_INTERNAL => Internal,
            ffi::AAUDIO_ERROR_INVALID_STATE => InvalidState,
            ffi::AAUDIO_ERROR_INVALID_HANDLE => InvalidHandle,
            ffi::AAUDIO_ERROR_UNIMPLEMENTED => Unimplemented,
            ffi::AAUDIO_ERROR_UNAVAILABLE => Unavailable,
            ffi::AAUDIO_ERROR_NO_FREE_HANDLES => NoFreeHandles,
            ffi::AAUDIO_ERROR_NO_MEMORY => NoMemory,
            ffi::AAUDIO_ERROR_NULL => Null,
            ffi::AAUDIO_ERROR_TIMEOUT => Timeout,
            ffi::AAUDIO_ERROR_WOULD_BLOCK => WouldBlock,
            ffi::AAUDIO_ERROR_INVALID_FORMAT => InvalidFormat,
            ffi::AAUDIO_ERROR_OUT_OF_RANGE => OutOfRange,
            ffi::AAUDIO_ERROR_NO_SERVICE => NoService,
            ffi::AAUDIO_ERROR_INVALID_RATE => InvalidRate,
            _ => return Err(AudioError::UnknownResult(result)),
        };
        Err(AudioError::ErrorResult(result))
    }
}

pub type Result<T, E = AudioError> = std::result::Result<T, E>;

fn construct<T>(with_ptr: impl FnOnce(*mut T) -> ffi::aaudio_result_t) -> Result<T> {
    let mut result = MaybeUninit::uninit();
    let status = with_ptr(result.as_mut_ptr());
    AudioError::from_result(status, || unsafe { result.assume_init() })
}

fn enum_return_value<T: TryFrom<u32>>(return_value: i32) -> Result<T> {
    u32::try_from(return_value)
        .ok()
        .and_then(|value| T::try_from(value).ok())
        .ok_or(AudioError::UnsupportedValue(return_value))
}

/// A native [`AAudioStreamBuilder *`]
///
/// [`AAudioStreamBuilder *`]: https://developer.android.com/ndk/reference/group/audio#aaudiostreambuilder
pub struct AudioStreamBuilder {
    inner: NonNull<ffi::AAudioStreamBuilder>,
    data_callback: Option<AudioStreamDataCallback>,
    error_callback: Option<AudioStreamErrorCallback>,
}

impl fmt::Debug for AudioStreamBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AAudioStreamBuilder")
            .field("inner", &self.inner)
            .field(
                "data_callback",
                match &self.data_callback {
                    Some(_) => &"Some(_)",
                    None => &"None",
                },
            )
            .field(
                "error_callback",
                match &self.error_callback {
                    Some(_) => &"Some(_)",
                    None => &"None",
                },
            )
            .finish()
    }
}

pub type AudioStreamDataCallback =
    Box<dyn FnMut(&AudioStream, *mut c_void, i32) -> AudioCallbackResult>;
pub type AudioStreamErrorCallback = Box<dyn FnMut(&AudioStream, AudioError)>;

impl AudioStreamBuilder {
    fn from_ptr(inner: NonNull<ffi::AAudioStreamBuilder>) -> Self {
        Self {
            inner,
            data_callback: None,
            error_callback: None,
        }
    }

    fn as_ptr(&self) -> *mut ffi::AAudioStreamBuilder {
        self.inner.as_ptr()
    }

    pub fn new() -> Result<Self> {
        unsafe {
            let ptr = construct(|res| ffi::AAudio_createStreamBuilder(res))?;
            Ok(Self::from_ptr(NonNull::new_unchecked(ptr)))
        }
    }

    /// Specify whether this stream audio may or may not be captured by other apps or the system.
    ///
    /// The default is [`AudioAllowedCapturePolicy::AllowCaptureByAll`].
    ///
    /// Note that an application can also set its global policy, in which case the most restrictive
    /// policy is always applied. See [`android.media.AudioAttributes#setAllowedCapturePolicy(int)`].
    ///
    /// Available since API level 29.
    ///
    /// # Arguments
    ///
    /// * `policy` - the desired level of opt-out from being captured.
    ///
    /// [`android.media.AudioAttributes#setAllowedCapturePolicy(int)`]: https://developer.android.com/reference/android/media/AudioAttributes.Builder#setAllowedCapturePolicy(int)
    #[cfg(feature = "api-level-29")]
    pub fn allowed_capture_policy(self, capture_policy: AudioAllowedCapturePolicy) -> Self {
        unsafe {
            ffi::AAudioStreamBuilder_setAllowedCapturePolicy(
                self.as_ptr(),
                capture_policy as ffi::aaudio_allowed_capture_policy_t,
            )
        };
        self
    }

    /// Set the requested buffer capacity in frames.
    /// The final AAudioStream capacity may differ, but will probably be at least this big.
    ///
    /// The default, if you do not call this function, is unspecified.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `num_frames` - the desired buffer capacity in frames or 0 for unspecified
    pub fn buffer_capacity_in_frames(self, num_frames: i32) -> Self {
        unsafe { ffi::AAudioStreamBuilder_setBufferCapacityInFrames(self.as_ptr(), num_frames) };
        self
    }

    /// Request a number of channels for the stream.
    ///
    /// The default, if you do not call this function, is unspecified.
    /// An optimal value will then be chosen when the stream is opened.
    /// After opening a stream with an unspecified value, the application must
    /// query for the actual value, which may vary by device.
    ///
    /// If an exact value is specified then an opened stream will use that value.
    /// If a stream cannot be opened with the specified value then the open will fail.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `channel_count` - Number of channels desired.
    pub fn channel_count(self, channel_count: i32) -> Self {
        unsafe { ffi::AAudioStreamBuilder_setChannelCount(self.as_ptr(), channel_count) };
        self
    }

    /// Set the type of audio data that the stream will carry.
    ///
    /// The AAudio system will use this information to optimize the
    /// behavior of the stream.
    /// This could, for example, affect whether a stream is paused when a notification occurs.
    ///
    /// The default, if you do not call this function, is [`AudioContentType::Music`].
    ///
    /// Available since API level 28.
    ///
    /// # Arguments
    ///
    /// * `content_type` - the type of audio data, eg. [`AudioContentType::Speech`]
    #[cfg(feature = "api-level-28")]
    pub fn content_type(self, content_type: AudioContentType) -> Self {
        unsafe {
            ffi::AAudioStreamBuilder_setContentType(
                self.as_ptr(),
                content_type as ffi::aaudio_content_type_t,
            )
        };
        self
    }

    /// Request that AAudio call the `data_callback` when the stream is running.
    ///
    /// Note that when using data callback, the audio data will be passed in or out
    /// of the function as an argument.
    /// So you cannot call [`AudioStream::write()`] or [`AudioStream::read()`]
    /// on the same stream that has an active data callback.
    ///
    /// The data callback function will start being called after [`AudioStream::request_start()`]
    /// is called.
    /// It will stop being called after [`AudioStream::request_pause()`] or
    /// [`AudioStream::request_stop()`] is called.
    ///
    /// The `data_callback` function will be called on a real-time thread owned by AAudio.
    /// Note that numFrames can vary unless [`AudioStreamBuilder::frames_per_data_callback()`]
    /// is called.
    ///
    /// Also note that this callback function should be considered a "real-time" function.
    /// It must not do anything that could cause an unbounded delay because that can cause the
    /// audio to glitch or pop.
    ///
    /// These are things the function should NOT do:
    /// * allocate memory using, for example, `malloc()` or new
    /// * any file operations such as opening, closing, reading or writing
    /// * any network operations such as streaming
    /// * use any mutexes or other synchronization primitives
    /// * sleep
    /// * stop or close the stream
    /// * [`AudioStream::read()`]
    /// * [`AudioStream::write()`]
    ///
    /// If you need to move data, eg. MIDI commands, in or out of the callback function then
    /// we recommend the use of non-blocking techniques such as an atomic FIFO.
    ///
    /// Note that the AAudio callbacks will never be called simultaneously from multiple threads.
    ///
    /// Available since API level 26.
    pub fn data_callback(mut self, callback: AudioStreamDataCallback) -> Self {
        let mut boxed = Box::new(callback);
        let ptr: *mut AudioStreamDataCallback = &mut *boxed;
        self.data_callback = Some(boxed);

        unsafe extern "C" fn ffi_callback(
            stream: *mut ffi::AAudioStreamStruct,
            user_data: *mut c_void,
            audio_data: *mut c_void,
            num_frames: i32,
        ) -> ffi::aaudio_data_callback_result_t {
            let callback = user_data as *mut AudioStreamDataCallback;
            let stream = AudioStream {
                inner: NonNull::new_unchecked(stream),
                data_callback: None,
                error_callback: None,
            };
            let result = (*callback)(&stream, audio_data, num_frames);
            std::mem::forget(stream);
            result as ffi::aaudio_data_callback_result_t
        }

        unsafe {
            ffi::AAudioStreamBuilder_setDataCallback(
                self.as_ptr(),
                Some(ffi_callback),
                ptr as *mut c_void,
            )
        };

        self
    }

    /// Request an audio device identified device using an ID.
    /// On Android, for example, the ID could be obtained from the Java AudioManager.
    ///
    /// The default, if you do not call this function, is 0,
    /// in which case the primary device will be used.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `device_id` - device identifier or 0 for unspecified
    pub fn device_id(self, device_id: i32) -> Self {
        unsafe { ffi::AAudioStreamBuilder_setDeviceId(self.as_ptr(), device_id) };
        self
    }

    /// Request the direction for a stream.
    ///
    /// The default, if you do not call this function, is [`Output`][AudioDirection::Output].
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `direction` - [`Output`][AudioDirection::Output] or [`Input`][AudioDirection::Input]
    pub fn direction(self, direction: AudioDirection) -> Self {
        unsafe {
            ffi::AAudioStreamBuilder_setDirection(
                self.as_ptr(),
                direction as ffi::aaudio_direction_t,
            )
        };
        self
    }

    /// Request that AAudio call the `data_callback` when the stream is running and the
    /// `error_callback` if any error occurs or the stream is disconnected.
    ///
    /// The `error_callback` will be called, for example, if a headset or a USB device is unplugged causing the stream's
    /// device to be unavailable or "disconnected".
    /// Another possible cause of error would be a timeout or an unanticipated internal error.
    ///
    /// In response, this function should signal or create another thread to stop
    /// and close this stream. The other thread could then reopen a stream on another device.
    /// Do not stop or close the stream, or reopen the new stream, directly from this callback.
    ///
    /// The `error_callback` will not be called because of actions by the application, such as stopping
    /// or closing a stream.
    ///
    /// Note that the AAudio callbacks will never be called simultaneously from multiple threads.
    ///
    /// Available since API level 26.
    pub fn error_callback(mut self, callback: AudioStreamErrorCallback) -> Self {
        let mut boxed = Box::new(callback);
        let ptr: *mut AudioStreamErrorCallback = &mut *boxed;
        self.error_callback = Some(boxed);

        unsafe extern "C" fn ffi_callback(
            stream: *mut ffi::AAudioStreamStruct,
            user_data: *mut c_void,
            error: ffi::aaudio_result_t,
        ) {
            let callback = user_data as *mut AudioStreamErrorCallback;
            let stream = AudioStream {
                inner: NonNull::new_unchecked(stream),
                data_callback: None,
                error_callback: None,
            };
            let err = AudioError::from_result(error, || ()).unwrap_err();
            (*callback)(&stream, err);
            std::mem::forget(stream);
        }

        unsafe {
            ffi::AAudioStreamBuilder_setErrorCallback(
                self.as_ptr(),
                Some(ffi_callback),
                ptr as *mut c_void,
            )
        };

        self
    }

    /// Request a sample data format, for example `Format::I16`.
    ///
    /// The default, if you do not call this function, is [`Unspecified`][AudioFormat::Unspecified].
    /// An optimal value will then be chosen when the stream is opened.
    /// After opening a stream with an unspecified value, the application must
    /// query for the actual value, which may vary by device.
    ///
    /// If an exact value is specified then an opened stream will use that value.
    /// If a stream cannot be opened with the specified value then the open will fail.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `format` - the sample data format.
    pub fn format(self, format: AudioFormat) -> Self {
        unsafe {
            ffi::AAudioStreamBuilder_setFormat(self.as_ptr(), format as ffi::aaudio_format_t)
        };
        self
    }

    /// Set the requested data callback buffer size in frames.
    ///
    /// See [`AudioStreamDataCallback`].
    ///
    /// The default, if you do not call this function, is unspecified.
    ///
    /// For the lowest possible latency, do not call this function. AAudio will then
    /// call the [`data_callback`][Self::data_callback] function with whatever size is optimal.
    /// That size may vary from one callback to another.
    ///
    /// Only use this function if the application requires a specific number of frames for processing.
    /// The application might, for example, be using an FFT that requires
    /// a specific power-of-two sized buffer.
    ///
    /// AAudio may need to add additional buffering in order to adapt between the internal
    /// buffer size and the requested buffer size.
    ///
    /// If you do call this function then the requested size should be less than
    /// half the buffer capacity, to allow double buffering.
    ///
    /// Available since API level 26.
    ///
    /// * `num_frames` - the desired buffer size in frames or 0 for unspecified
    pub fn frames_per_data_callback(self, num_frames: i32) -> Self {
        unsafe { ffi::AAudioStreamBuilder_setFramesPerDataCallback(self.as_ptr(), num_frames) };
        self
    }

    /// Set the input (capture) preset for the stream.
    ///
    /// The AAudio system will use this information to optimize the
    /// behavior of the stream.
    /// This could, for example, affect which microphones are used and how the
    /// recorded data is processed.
    ///
    /// The default, if you do not call this function, is [`VoiceRecognition`][AudioInputPreset::VoiceRecognition]
    /// which is the preset with the lowest latency on many platforms.
    ///
    /// Available since API level 28.
    ///
    /// # Arguments
    ///
    /// * `input_preset` - the desired configuration for recording
    #[cfg(feature = "api-level-28")]
    pub fn input_preset(self, input_preset: AudioInputPreset) -> Self {
        unsafe {
            ffi::AAudioStreamBuilder_setInputPreset(
                self.as_ptr(),
                input_preset as ffi::aaudio_input_preset_t,
            )
        };
        self
    }

    /// Set the requested performance mode.
    ///
    /// Supported modes are None, PowerSaving and LowLatency.
    ///
    /// The default, if you do not call this function, is None.
    ///
    /// You may not get the mode you requested.
    /// You can call [`AudioStream::get_performance_mode()`]
    /// to find out the final mode for the stream.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `mode` - the desired performance mode, eg. LowLatency
    pub fn performance_mode(self, mode: AudioPerformanceMode) -> Self {
        unsafe {
            ffi::AAudioStreamBuilder_setPerformanceMode(
                self.as_ptr(),
                mode as ffi::aaudio_performance_mode_t,
            )
        };
        self
    }

    /// Request a sample rate in Hertz.
    ///
    /// The default, if you do not call this function, is 0 (unspecified).
    /// An optimal value will then be chosen when the stream is opened.
    /// After opening a stream with an unspecified value, the application must
    /// query for the actual value, which may vary by device.
    ///
    /// If an exact value is specified then an opened stream will use that value.
    /// If a stream cannot be opened with the specified value then the open will fail.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - frames per second. Common rates include 44100 and 48000 Hz.
    pub fn sample_rate(self, sample_rate: i32) -> Self {
        unsafe { ffi::AAudioStreamBuilder_setSampleRate(self.as_ptr(), sample_rate) };
        self
    }

    pub fn samples_per_frame(self, samples_per_frame: i32) -> Self {
        unsafe { ffi::AAudioStreamBuilder_setSamplesPerFrame(self.as_ptr(), samples_per_frame) };
        self
    }

    /// The session ID can be used to associate a stream with effects processors.
    /// The effects are controlled using the Android AudioEffect Java API.
    ///
    /// The default, if you do not call this function, is -1 (none).
    ///
    /// If set to [`Option::None`] then a session ID will be allocated when the stream is opened.
    ///
    /// The allocated session ID can be obtained by calling [`AudioStream::get_session_id()`]
    /// and then used with this function when opening another stream.
    /// This allows effects to be shared between streams.
    ///
    /// Session IDs from AAudio can be used with the Android Java APIs and vice versa.
    /// So a session ID from an AAudio stream can be passed to Java
    /// and effects applied using the Java AudioEffect API.
    ///
    /// Note that allocating or setting a session ID may result in a stream with higher latency.
    ///
    /// Allocated session IDs will always be positive and nonzero.
    ///
    /// Available since API level 28.
    ///
    /// # Arguments
    ///
    /// * `session_id` - an allocated sessionID or [`Option::None`] to allocate a new sessionID
    #[cfg(feature = "api-level-28")]
    pub fn session_id(self, session_id_or_allocate: Option<SessionId>) -> Self {
        let session_id = match session_id_or_allocate {
            None => ffi::AAUDIO_SESSION_ID_ALLOCATE,
            Some(SessionId::None) => ffi::AAUDIO_SESSION_ID_NONE,
            Some(SessionId::Allocated(value)) => value.get(),
        };

        unsafe { ffi::AAudioStreamBuilder_setSessionId(self.as_ptr(), session_id) };
        self
    }

    /// Request a mode for sharing the device.
    ///
    /// The default, if you do not call this function, is [`AudioSharingMode::Shared`].
    ///
    /// The requested sharing mode may not be available.
    /// The application can query for the actual mode after the stream is opened.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `sharing_mode` - [`AudioSharingMode::Shared`] or [`AudioSharingMode::Exclusive`]
    pub fn sharing_mode(self, sharing_mode: AudioSharingMode) -> Self {
        unsafe {
            ffi::AAudioStreamBuilder_setSharingMode(
                self.as_ptr(),
                sharing_mode as ffi::aaudio_sharing_mode_t,
            )
        };
        self
    }

    /// Set the intended use case for the stream.
    ///
    /// The AAudio system will use this information to optimize the
    /// behavior of the stream.
    /// This could, for example, affect how volume and focus is handled for the stream.
    ///
    /// The default, if you do not call this function, is [`AudioUsage::Media`].
    ///
    /// Available since API level 28.
    ///
    /// * `usage` - the desired usage, eg. [`AudioUsage::Game`]
    #[cfg(feature = "api-level-28")]
    pub fn usage(self, usage: AudioUsage) -> Self {
        unsafe { ffi::AAudioStreamBuilder_setUsage(self.as_ptr(), usage as ffi::aaudio_usage_t) };
        self
    }

    /// Open a stream based on the options in the AAudioStreamBuilder.
    pub fn open_stream(mut self) -> Result<AudioStream> {
        unsafe {
            let ptr = construct(|res| ffi::AAudioStreamBuilder_openStream(self.as_ptr(), res))?;

            Ok(AudioStream {
                inner: NonNull::new_unchecked(ptr),
                data_callback: self.data_callback.take(),
                error_callback: self.error_callback.take(),
            })
        }
    }
}

impl Drop for AudioStreamBuilder {
    fn drop(&mut self) {
        let status = unsafe { ffi::AAudioStreamBuilder_delete(self.as_ptr()) };
        AudioError::from_result(status, || ()).unwrap();
    }
}

/// A native [`AAudioStream *`]
///
/// [`AAudioStream *`]: https://developer.android.com/ndk/reference/group/audio#aaudiostream
pub struct AudioStream {
    inner: NonNull<ffi::AAudioStream>,
    data_callback: Option<AudioStreamDataCallback>,
    error_callback: Option<AudioStreamErrorCallback>,
}

impl fmt::Debug for AudioStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AAudioStream")
            .field("inner", &self.inner)
            .field(
                "data_callback",
                match &self.data_callback {
                    Some(_) => &"Some(_)",
                    None => &"None",
                },
            )
            .field(
                "error_callback",
                match &self.error_callback {
                    Some(_) => &"Some(_)",
                    None => &"None",
                },
            )
            .finish()
    }
}

impl AudioStream {
    fn as_ptr(&self) -> *mut ffi::AAudioStream {
        self.inner.as_ptr()
    }

    /// Returns the policy that determines whether the audio may or
    /// may not be captured by other apps or the system.
    #[cfg(feature = "api-level-29")]
    pub fn get_allowed_capture_policy(self) -> Result<AudioAllowedCapturePolicy> {
        enum_return_value(unsafe { ffi::AAudioStream_getAllowedCapturePolicy(self.as_ptr()) })
    }

    /// Query maximum buffer capacity in frames.
    ///
    /// Available since API level 26.
    pub fn get_buffer_capacity_in_frames(&self) -> i32 {
        unsafe { ffi::AAudioStream_getBufferCapacityInFrames(self.as_ptr()) }
    }

    /// Query the maximum number of frames that can be filled without blocking.
    ///
    /// Available since API level 26.
    pub fn get_buffer_size_in_frames(&self) -> i32 {
        unsafe { ffi::AAudioStream_getBufferSizeInFrames(self.as_ptr()) }
    }

    /// A stream has one or more channels of data.
    /// A frame will contain one sample for each channel.
    ///
    /// Available since API level 26.
    pub fn get_channel_count(&self) -> i32 {
        unsafe { ffi::AAudioStream_getChannelCount(self.as_ptr()) }
    }

    #[cfg(feature = "api-level-28")]
    pub fn get_content_type(&self) -> Result<AudioContentType> {
        let value = unsafe { ffi::AAudioStream_getContentType(self.as_ptr()) };
        enum_return_value(value)
    }

    /// Returns the actual device ID.
    ///
    /// Available since API level 26.
    pub fn get_device_id(&self) -> i32 {
        unsafe { ffi::AAudioStream_getDeviceId(self.as_ptr()) }
    }

    /// Available since API level 26.
    pub fn get_direction(&self) -> Result<AudioDirection> {
        let value = unsafe { ffi::AAudioStream_getDirection(self.as_ptr()) };
        enum_return_value(value)
    }

    /// Returns the actual data format.
    ///
    /// Available since API level 26.
    pub fn get_format(&self) -> Result<AudioFormat> {
        let value = unsafe { ffi::AAudioStream_getFormat(self.as_ptr()) };
        AudioFormat::try_from(value).map_err(|_| AudioError::UnsupportedValue(value))
    }

    /// Query the number of frames that the application should read or write at
    /// one time for optimal performance. It is OK if an application writes
    /// a different number of frames. But the buffer size may need to be larger
    /// in order to avoid underruns or overruns.
    ///
    /// Note that this may or may not match the actual device burst size.
    /// For some endpoints, the burst size can vary dynamically.
    /// But these tend to be devices with high latency.
    ///
    /// Available since API level 26.
    pub fn get_frames_per_burst(&self) -> i32 {
        unsafe { ffi::AAudioStream_getFramesPerBurst(self.as_ptr()) }
    }

    /// Query the size of the buffer that will be passed to the data callback in the `numFrames` parameter.
    /// This call can be used if the application needs to know the value of numFrames before
    /// the stream is started. This is not normally necessary.
    ///
    /// If a specific size was requested by calling
    /// [`AudioStreamBuilder::frames_per_data_callback()`] then this will be the same size.
    ///
    /// If [`AudioStreamBuilder::frames_per_data_callback()`] was not called then this will
    /// return the size chosen by AAudio, or 0.
    ///
    /// `None` indicates that the callback buffer size for this stream may vary from one dataProc callback to the next.
    ///
    /// Available since API level 26.
    pub fn get_frames_per_data_callback(&self) -> Option<i32> {
        let value = unsafe { ffi::AAudioStream_getFramesPerDataCallback(self.as_ptr()) };
        const AAUDIO_UNSPECIFIED: i32 = ffi::AAUDIO_UNSPECIFIED as i32;
        match value {
            AAUDIO_UNSPECIFIED => None,
            val => Some(val),
        }
    }

    /// Returns the number of frames that have been read since the stream was created.
    /// For an output stream, this will be advanced by the endpoint.
    /// For an input stream, this will be advanced by the application calling [`read()`][Self::read]
    /// or by a data callback.
    ///
    /// The frame position is monotonically increasing.
    ///
    /// Available since API level 26.
    pub fn get_frames_read(&self) -> i64 {
        unsafe { ffi::AAudioStream_getFramesRead(self.as_ptr()) }
    }

    /// Returns the number of frames that have been written since the stream was created.
    /// For an output stream, this will be advanced by the application calling write()
    /// or by a data callback.
    /// For an input stream, this will be advanced by the endpoint.
    ///
    /// The frame position is monotonically increasing.
    ///
    /// Available since API level 26.
    pub fn get_frames_written(&self) -> i64 {
        unsafe { ffi::AAudioStream_getFramesWritten(self.as_ptr()) }
    }

    #[cfg(feature = "api-level-28")]
    pub fn get_input_preset(&self) -> Result<AudioInputPreset> {
        let value = unsafe { ffi::AAudioStream_getInputPreset(self.as_ptr()) };
        enum_return_value(value)
    }

    /// Get the performance mode used by the stream.
    ///
    /// Available since API level 26.
    pub fn get_performance_mode(&self) -> Result<AudioPerformanceMode> {
        let value = unsafe { ffi::AAudioStream_getPerformanceMode(self.as_ptr()) };
        enum_return_value(value)
    }

    /// Returns the actual sample rate.
    ///
    /// Available since API level 26.
    pub fn get_sample_rate(&self) -> i32 {
        unsafe { ffi::AAudioStream_getSampleRate(self.as_ptr()) }
    }

    pub fn get_samples_per_frame(&self) -> i32 {
        unsafe { ffi::AAudioStream_getSamplesPerFrame(self.as_ptr()) }
    }

    /// Passes back the session ID associated with this stream.
    ///
    /// The session ID can be used to associate a stream with effects processors.
    /// The effects are controlled using the Android AudioEffect Java API.
    ///
    /// If [`AudioStreamBuilder::session_id()`] was called with `0`
    /// then a new session ID should be allocated once when the stream is opened.
    ///
    /// If [`AudioStreamBuilder::session_id()`] was called with a previously allocated
    /// session ID then that value should be returned.
    ///
    /// If [`AudioStreamBuilder::session_id()`] was not called then this function should
    /// return `-1`.
    ///
    /// The sessionID for a stream should not change once the stream has been opened.
    ///
    /// Available since API level 28.
    #[cfg(feature = "api-level-28")]
    pub fn get_session_id(&self) -> SessionId {
        let value = unsafe { ffi::AAudioStream_getSessionId(self.as_ptr()) };
        match value {
            ffi::AAUDIO_SESSION_ID_NONE => SessionId::None,
            allocated => SessionId::Allocated(NonZeroI32::new(allocated).unwrap()),
        }
    }

    /// Provide actual sharing mode.
    ///
    /// Available since API level 26.
    pub fn get_sharing_mode(&self) -> Result<AudioSharingMode> {
        let value = unsafe { ffi::AAudioStream_getSharingMode(self.as_ptr()) };
        enum_return_value(value)
    }

    /// Query the current state of the client, eg. [`Pausing`][AudioStreamState::Pausing].
    ///
    /// This function will immediately return the state without updating the state.
    /// If you want to update the client state based on the server state then
    /// call [`AudioStream::wait_for_state_change()`] with currentState
    /// set to [`Unknown`][AudioStreamState::Unknown] and a zero timeout.
    ///
    /// Available since API level 26.
    pub fn get_state(&self) -> Result<AudioStreamState> {
        let value = unsafe { ffi::AAudioStream_getState(self.as_ptr()) };
        enum_return_value(value)
    }

    /// Returns the time at which a particular frame was presented.
    /// This can be used to synchronize audio with video or MIDI.
    /// It can also be used to align a recorded stream with a playback stream.
    ///
    /// Timestamps are only valid when the stream is in `Started` state.
    /// [`InvalidState`][AudioErrorResult::InvalidState] will be returned
    /// if the stream is not started.
    /// Note that because [`AudioStream::request_start()`] is asynchronous,
    /// timestamps will not be valid until a short time after calling
    /// [`AudioStream::request_start()`].
    /// So [`InvalidState`][AudioErrorResult::InvalidState] should not be
    /// considered a fatal error.
    /// Just try calling again later.
    ///
    /// If an error occurs, then the position and time will not be modified.
    ///
    /// The position and time passed back are monotonically increasing.
    ///
    /// Available since API level 26.
    pub fn get_timestamp(&self, clockid: Clockid) -> Result<Timestamp> {
        let frame_position;
        let time_nanoseconds = unsafe {
            let mut nanoseconds = MaybeUninit::uninit();
            frame_position = construct(|ptr| {
                ffi::AAudioStream_getTimestamp(
                    self.as_ptr(),
                    clockid as ffi::clockid_t,
                    ptr,
                    nanoseconds.as_mut_ptr(),
                )
            })?;
            nanoseconds.assume_init()
        };

        Ok(Timestamp {
            frame_position,
            time_nanoseconds,
        })
    }

    #[cfg(feature = "api-level-28")]
    pub fn get_usage(&self) -> Result<AudioUsage> {
        let value = unsafe { ffi::AAudioStream_getUsage(self.as_ptr()) };
        enum_return_value(value)
    }

    /// An XRun is an Underrun or an Overrun.
    /// During playing, an underrun will occur if the stream is not written in time
    /// and the system runs out of valid data.
    /// During recording, an overrun will occur if the stream is not read in time
    /// and there is no place to put the incoming data so it is discarded.
    ///
    /// An underrun or overrun can cause an audible "pop" or "glitch".
    ///
    /// Note that some INPUT devices may not support this function.
    /// In that case a 0 will always be returned.
    ///
    /// Available since API level 26.
    pub fn get_x_run_count(&self) -> i32 {
        unsafe { ffi::AAudioStream_getXRunCount(self.as_ptr()) }
    }

    /// Read data from the stream.
    /// Returns the number of frames actually read or a negative error.
    ///
    /// The call will wait until the read is complete or until it runs out of time.
    /// If timeoutNanos is zero then this call will not wait.
    ///
    /// Note that timeoutNanoseconds is a relative duration in wall clock time.
    /// Time will not stop if the thread is asleep.
    /// So it will be implemented using CLOCK_BOOTTIME.
    ///
    /// This call is "strong non-blocking" unless it has to wait for data.
    ///
    /// If the call times out then zero or a partial frame count will be returned.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The slice with the samples.
    /// * `num_frames` - Number of frames to read. Only complete frames will be written.
    /// * `timeout_nanoseconds` - Maximum number of nanoseconds to wait for completion.
    ///
    /// # Safety
    /// `buffer` must be a valid pointer to at least `num_frames` samples.
    pub unsafe fn read(
        &self,
        buffer: *mut c_void,
        num_frames: i32,
        timeout_nanoseconds: i64,
    ) -> Result<u32> {
        let result = ffi::AAudioStream_read(self.as_ptr(), buffer, num_frames, timeout_nanoseconds);

        AudioError::from_result(result, || result as u32)
    }

    /// Asynchronous request for the stream to flush.
    /// Flushing will discard any pending data.
    /// This call only works if the stream is pausing or paused.
    /// Frame counters are not reset by a flush. They may be advanced.
    /// After this call the state will be in [`Flushing`][AudioStreamState::Flushing] or
    /// [`Flushed`][AudioStreamState::Flushed].
    ///
    /// This will return [`Unimplemented`][AudioErrorResult::Unimplemented] for input streams.
    ///
    /// Available since API level 26.
    pub fn request_flush(&self) -> Result<()> {
        let result = unsafe { ffi::AAudioStream_requestFlush(self.as_ptr()) };
        AudioError::from_result(result, || ())
    }

    /// Asynchronous request for the stream to pause.
    /// Pausing a stream will freeze the data flow but not flush any buffers.
    /// Use [`AudioStream::request_start()`] to resume playback after a pause.
    /// After this call the state will be in [`Pausing`][AudioStreamState::Pausing] or
    /// [`Paused`][AudioStreamState::Paused].
    ///
    /// This will return [`Unimplemented`][AudioErrorResult::Unimplemented] for input streams.
    /// For input streams use [`AudioStream::request_stop()`].
    ///
    /// Available since API level 26.
    pub fn request_pause(&self) -> Result<()> {
        let result = unsafe { ffi::AAudioStream_requestPause(self.as_ptr()) };
        AudioError::from_result(result, || ())
    }

    /// Asynchronously request to start playing the stream. For output streams, one should
    /// write to the stream to fill the buffer before starting.
    /// Otherwise it will underflow.
    /// After this call the state will be in [`Starting`][AudioStreamState::Starting] or
    /// [`Started`][AudioStreamState::Started].
    ///
    /// Returns 0 for OK or a negative error.
    ///
    /// Available since API level 26.
    pub fn request_start(&self) -> Result<()> {
        let result = unsafe { ffi::AAudioStream_requestStart(self.as_ptr()) };
        AudioError::from_result(result, || ())
    }

    /// Asynchronous request for the stream to stop.
    /// The stream will stop after all of the data currently buffered has been played.
    /// After this call the state will be in [`Stopping`][AudioStreamState::Stopping] or
    /// [`Stopped`][AudioStreamState::Stopped].
    ///
    /// Available since API level 26.
    pub fn request_stop(&self) -> Result<()> {
        let result = unsafe { ffi::AAudioStream_requestStop(self.as_ptr()) };
        AudioError::from_result(result, || ())
    }

    /// This can be used to adjust the latency of the buffer by changing
    /// the threshold where blocking will occur.
    /// By combining this with [`AudioStream::get_x_run_count()`], the latency can be tuned
    /// at run-time for each device.
    /// Returns actual buffer size in frames or a negative error.
    ///
    /// This cannot be set higher than [`AudioStream::get_buffer_capacity_in_frames()`].
    ///
    /// Note that you will probably not get the exact size you request.
    /// You can check the return value or call [`AudioStream::get_buffer_size_in_frames()`]
    /// to see what the actual final size is.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `num_frames` - requested number of frames that can be filled without blocking
    pub fn set_buffer_size_in_frames(&self, num_frames: i32) -> Result<i32> {
        let result = unsafe { ffi::AAudioStream_setBufferSizeInFrames(self.as_ptr(), num_frames) };
        AudioError::from_result(result, || result)
    }

    /// Wait until the current state no longer matches the input state.
    ///
    /// This will update the current client state.
    ///
    /// Returns the new state.
    ///
    /// Available since API level 26.
    pub fn wait_for_state_change(
        &self,
        input_state: AudioStreamState,
        timeout_nanoseconds: i64,
    ) -> Result<AudioStreamState> {
        let value = construct(|ptr| unsafe {
            ffi::AAudioStream_waitForStateChange(
                self.as_ptr(),
                input_state as ffi::aaudio_stream_state_t,
                ptr,
                timeout_nanoseconds,
            )
        })?;
        enum_return_value(value)
    }

    /// Write data to the stream.
    /// Returns the number of frames actually written or a negative error.
    ///
    /// The call will wait until the write is complete or until it runs out of time.
    /// If `timeout_nanoseconds` is zero then this call will not wait.
    ///
    /// Note that `timeout_nanoseconds` is a relative duration in wall clock time.
    /// Time will not stop if the thread is asleep.
    /// So it will be implemented using `CLOCK_BOOTTIME`.
    ///
    /// This call is "strong non-blocking" unless it has to wait for room in the buffer.
    ///
    /// If the call times out then zero or a partial frame count will be returned.
    ///
    /// Available since API level 26.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The address of the first sample.
    /// * `num_frames` - Number of frames to write. Only complete frames will be written.
    /// * `timeout_nanoseconds` - Maximum number of nanoseconds to wait for completion.
    ///
    /// # Safety
    /// `buffer` must be a valid pointer to at least `num_frames` samples.
    pub unsafe fn write(
        &self,
        buffer: *const c_void,
        num_frames: i32,
        timeout_nanoseconds: i64,
    ) -> Result<u32> {
        let result =
            ffi::AAudioStream_write(self.as_ptr(), buffer, num_frames, timeout_nanoseconds);

        AudioError::from_result(result, || result as u32)
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        let status = unsafe { ffi::AAudioStream_close(self.as_ptr()) };
        AudioError::from_result(status, || ()).unwrap();
    }
}
