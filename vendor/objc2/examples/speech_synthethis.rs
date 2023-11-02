//! Speak synthethized text.
//!
//! This uses `NSSpeechSynthesizer` on macOS, and `AVSpeechSynthesizer` on
//! other Apple platforms. Note that `AVSpeechSynthesizer` _is_ available on
//! macOS, but only since 10.15!
//!
//! TODO: Unsure about when to use `&mut` here?
//!
//! Works on macOS >= 10.7 and iOS > 7.0.
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(feature = "gnustep-1-7", allow(unused))]

use std::thread;
use std::time::Duration;

use objc2::foundation::{NSObject, NSString};
use objc2::rc::{Id, Owned};
use objc2::{extern_class, msg_send, msg_send_id, ns_string, ClassType};

#[cfg(all(feature = "apple", target_os = "macos"))]
mod appkit {
    use objc2::{foundation::NSCopying, rc::Shared};

    use super::*;

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    extern_class!(
        /// <https://developer.apple.com/documentation/appkit/nsspeechsynthesizer?language=objc>
        pub struct NSSpeechSynthesizer;

        unsafe impl ClassType for NSSpeechSynthesizer {
            type Super = NSObject;
        }
    );

    impl NSSpeechSynthesizer {
        // Uses default voice
        pub fn new() -> Id<Self, Owned> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        fn set_rate(&mut self, rate: f32) {
            unsafe { msg_send![self, setRate: rate] }
        }

        fn set_volume(&mut self, volume: f32) {
            unsafe { msg_send![self, setVolume: volume] }
        }

        fn start_speaking(&mut self, s: &NSString) {
            unsafe { msg_send![self, startSpeakingString: s] }
        }

        pub fn speak(&mut self, utterance: &Utterance) {
            // Convert to the range 90-720 that `NSSpeechSynthesizer` seems to
            // support
            //
            // Note that you'd probably want a nonlinear conversion here to
            // make it match `AVSpeechSynthesizer`.
            self.set_rate(90.0 + (utterance.rate * (360.0 - 90.0)));
            self.set_volume(utterance.volume);
            self.start_speaking(&utterance.string);
        }

        pub fn is_speaking(&self) -> bool {
            unsafe { msg_send![self, isSpeaking] }
        }
    }

    // Shim to make NSSpeechSynthesizer work similar to AVSpeechSynthesizer
    pub struct Utterance {
        rate: f32,
        volume: f32,
        string: Id<NSString, Shared>,
    }

    impl Utterance {
        pub fn new(string: &NSString) -> Self {
            Self {
                rate: 0.5,
                volume: 1.0,
                string: string.copy(),
            }
        }

        pub fn set_rate(&mut self, rate: f32) {
            self.rate = rate;
        }

        pub fn set_volume(&mut self, volume: f32) {
            self.volume = volume;
        }
    }
}

#[cfg(all(feature = "apple", not(target_os = "macos")))]
mod avfaudio {
    use super::*;

    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {}

    extern_class!(
        /// <https://developer.apple.com/documentation/avfaudio/avspeechsynthesizer?language=objc>
        #[derive(Debug)]
        pub struct AVSpeechSynthesizer;

        unsafe impl ClassType for AVSpeechSynthesizer {
            type Super = NSObject;
        }
    );

    impl AVSpeechSynthesizer {
        pub fn new() -> Id<Self, Owned> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        pub fn speak(&mut self, utterance: &AVSpeechUtterance) {
            unsafe { msg_send![self, speakUtterance: utterance] }
        }

        pub fn is_speaking(&self) -> bool {
            unsafe { msg_send![self, isSpeaking] }
        }
    }

    extern_class!(
        /// <https://developer.apple.com/documentation/avfaudio/avspeechutterance?language=objc>
        #[derive(Debug)]
        pub struct AVSpeechUtterance;

        unsafe impl ClassType for AVSpeechUtterance {
            type Super = NSObject;
        }
    );

    impl AVSpeechUtterance {
        pub fn new(string: &NSString) -> Id<Self, Owned> {
            unsafe { msg_send_id![msg_send_id![Self::class(), alloc], initWithString: string] }
        }

        pub fn set_rate(&mut self, rate: f32) {
            unsafe { msg_send![self, setRate: rate] }
        }

        pub fn set_volume(&mut self, volume: f32) {
            unsafe { msg_send![self, setVolume: volume] }
        }
    }
}

#[cfg(all(feature = "apple", target_os = "macos"))]
use appkit::{NSSpeechSynthesizer as Synthesizer, Utterance};
#[cfg(all(feature = "apple", not(target_os = "macos")))]
use avfaudio::{AVSpeechSynthesizer as Synthesizer, AVSpeechUtterance as Utterance};

#[cfg(feature = "apple")]
fn main() {
    let mut synthesizer = Synthesizer::new();
    let mut utterance = Utterance::new(ns_string!("Hello from Rust!"));
    utterance.set_rate(0.5);
    utterance.set_volume(0.5);
    synthesizer.speak(&utterance);

    // Wait until speech has properly started up
    thread::sleep(Duration::from_millis(1000));
    // Wait until finished speaking
    while synthesizer.is_speaking() {
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(feature = "gnustep-1-7")]
fn main() {
    panic!("this example is only available on Apple targets");
}
