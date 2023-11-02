// This file contains generated code. Do not edit directly.
// To regenerate this, run 'make'.

//! Bindings to the `Present` X11 extension.

#![allow(clippy::too_many_arguments)]

#[allow(unused_imports)]
use alloc::borrow::Cow;
#[allow(unused_imports)]
use core::convert::TryInto;
use alloc::vec;
use alloc::vec::Vec;
use core::convert::TryFrom;
use crate::errors::ParseError;
#[allow(unused_imports)]
use crate::x11_utils::TryIntoUSize;
use crate::{BufWithFds, PiecewiseBuf};
#[allow(unused_imports)]
use crate::utils::{RawFdContainer, pretty_print_bitmask, pretty_print_enum};
#[allow(unused_imports)]
use crate::x11_utils::{Request, RequestHeader, Serialize, TryParse, TryParseFd};
#[allow(unused_imports)]
use super::randr;
#[allow(unused_imports)]
use super::sync;
#[allow(unused_imports)]
use super::xfixes;
#[allow(unused_imports)]
use super::xproto;

/// The X11 name of the extension for QueryExtension
pub const X11_EXTENSION_NAME: &str = "Present";

/// The version number of this extension that this client library supports.
///
/// This constant contains the version number of this extension that is supported
/// by this build of x11rb. For most things, it does not make sense to use this
/// information. If you need to send a `QueryVersion`, it is recommended to instead
/// send the maximum version of the extension that you need.
pub const X11_XML_VERSION: (u32, u32) = (1, 2);

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EventEnum(u8);
impl EventEnum {
    pub const CONFIGURE_NOTIFY: Self = Self(0);
    pub const COMPLETE_NOTIFY: Self = Self(1);
    pub const IDLE_NOTIFY: Self = Self(2);
    pub const REDIRECT_NOTIFY: Self = Self(3);
}
impl From<EventEnum> for u8 {
    #[inline]
    fn from(input: EventEnum) -> Self {
        input.0
    }
}
impl From<EventEnum> for std::option::Option<u8> {
    #[inline]
    fn from(input: EventEnum) -> Self {
        Some(input.0)
    }
}
impl From<EventEnum> for u16 {
    #[inline]
    fn from(input: EventEnum) -> Self {
        u16::from(input.0)
    }
}
impl From<EventEnum> for std::option::Option<u16> {
    #[inline]
    fn from(input: EventEnum) -> Self {
        Some(u16::from(input.0))
    }
}
impl From<EventEnum> for u32 {
    #[inline]
    fn from(input: EventEnum) -> Self {
        u32::from(input.0)
    }
}
impl From<EventEnum> for std::option::Option<u32> {
    #[inline]
    fn from(input: EventEnum) -> Self {
        Some(u32::from(input.0))
    }
}
impl From<u8> for EventEnum {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl core::fmt::Debug for EventEnum  {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variants = [
            (Self::CONFIGURE_NOTIFY.0.into(), "CONFIGURE_NOTIFY", "ConfigureNotify"),
            (Self::COMPLETE_NOTIFY.0.into(), "COMPLETE_NOTIFY", "CompleteNotify"),
            (Self::IDLE_NOTIFY.0.into(), "IDLE_NOTIFY", "IdleNotify"),
            (Self::REDIRECT_NOTIFY.0.into(), "REDIRECT_NOTIFY", "RedirectNotify"),
        ];
        pretty_print_enum(fmt, self.0.into(), &variants)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EventMask(u8);
impl EventMask {
    pub const NO_EVENT: Self = Self(0);
    pub const CONFIGURE_NOTIFY: Self = Self(1 << 0);
    pub const COMPLETE_NOTIFY: Self = Self(1 << 1);
    pub const IDLE_NOTIFY: Self = Self(1 << 2);
    pub const REDIRECT_NOTIFY: Self = Self(1 << 3);
}
impl From<EventMask> for u8 {
    #[inline]
    fn from(input: EventMask) -> Self {
        input.0
    }
}
impl From<EventMask> for std::option::Option<u8> {
    #[inline]
    fn from(input: EventMask) -> Self {
        Some(input.0)
    }
}
impl From<EventMask> for u16 {
    #[inline]
    fn from(input: EventMask) -> Self {
        u16::from(input.0)
    }
}
impl From<EventMask> for std::option::Option<u16> {
    #[inline]
    fn from(input: EventMask) -> Self {
        Some(u16::from(input.0))
    }
}
impl From<EventMask> for u32 {
    #[inline]
    fn from(input: EventMask) -> Self {
        u32::from(input.0)
    }
}
impl From<EventMask> for std::option::Option<u32> {
    #[inline]
    fn from(input: EventMask) -> Self {
        Some(u32::from(input.0))
    }
}
impl From<u8> for EventMask {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl core::fmt::Debug for EventMask  {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variants = [
            (Self::NO_EVENT.0.into(), "NO_EVENT", "NoEvent"),
            (Self::CONFIGURE_NOTIFY.0.into(), "CONFIGURE_NOTIFY", "ConfigureNotify"),
            (Self::COMPLETE_NOTIFY.0.into(), "COMPLETE_NOTIFY", "CompleteNotify"),
            (Self::IDLE_NOTIFY.0.into(), "IDLE_NOTIFY", "IdleNotify"),
            (Self::REDIRECT_NOTIFY.0.into(), "REDIRECT_NOTIFY", "RedirectNotify"),
        ];
        pretty_print_bitmask(fmt, self.0.into(), &variants)
    }
}
bitmask_binop!(EventMask, u8);

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Option(u8);
impl Option {
    pub const NONE: Self = Self(0);
    pub const ASYNC: Self = Self(1 << 0);
    pub const COPY: Self = Self(1 << 1);
    pub const UST: Self = Self(1 << 2);
    pub const SUBOPTIMAL: Self = Self(1 << 3);
}
impl From<Option> for u8 {
    #[inline]
    fn from(input: Option) -> Self {
        input.0
    }
}
impl From<Option> for std::option::Option<u8> {
    #[inline]
    fn from(input: Option) -> Self {
        Some(input.0)
    }
}
impl From<Option> for u16 {
    #[inline]
    fn from(input: Option) -> Self {
        u16::from(input.0)
    }
}
impl From<Option> for std::option::Option<u16> {
    #[inline]
    fn from(input: Option) -> Self {
        Some(u16::from(input.0))
    }
}
impl From<Option> for u32 {
    #[inline]
    fn from(input: Option) -> Self {
        u32::from(input.0)
    }
}
impl From<Option> for std::option::Option<u32> {
    #[inline]
    fn from(input: Option) -> Self {
        Some(u32::from(input.0))
    }
}
impl From<u8> for Option {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl core::fmt::Debug for Option  {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variants = [
            (Self::NONE.0.into(), "NONE", "None"),
            (Self::ASYNC.0.into(), "ASYNC", "Async"),
            (Self::COPY.0.into(), "COPY", "Copy"),
            (Self::UST.0.into(), "UST", "UST"),
            (Self::SUBOPTIMAL.0.into(), "SUBOPTIMAL", "Suboptimal"),
        ];
        pretty_print_bitmask(fmt, self.0.into(), &variants)
    }
}
bitmask_binop!(Option, u8);

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Capability(u8);
impl Capability {
    pub const NONE: Self = Self(0);
    pub const ASYNC: Self = Self(1 << 0);
    pub const FENCE: Self = Self(1 << 1);
    pub const UST: Self = Self(1 << 2);
}
impl From<Capability> for u8 {
    #[inline]
    fn from(input: Capability) -> Self {
        input.0
    }
}
impl From<Capability> for std::option::Option<u8> {
    #[inline]
    fn from(input: Capability) -> Self {
        Some(input.0)
    }
}
impl From<Capability> for u16 {
    #[inline]
    fn from(input: Capability) -> Self {
        u16::from(input.0)
    }
}
impl From<Capability> for std::option::Option<u16> {
    #[inline]
    fn from(input: Capability) -> Self {
        Some(u16::from(input.0))
    }
}
impl From<Capability> for u32 {
    #[inline]
    fn from(input: Capability) -> Self {
        u32::from(input.0)
    }
}
impl From<Capability> for std::option::Option<u32> {
    #[inline]
    fn from(input: Capability) -> Self {
        Some(u32::from(input.0))
    }
}
impl From<u8> for Capability {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl core::fmt::Debug for Capability  {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variants = [
            (Self::NONE.0.into(), "NONE", "None"),
            (Self::ASYNC.0.into(), "ASYNC", "Async"),
            (Self::FENCE.0.into(), "FENCE", "Fence"),
            (Self::UST.0.into(), "UST", "UST"),
        ];
        pretty_print_bitmask(fmt, self.0.into(), &variants)
    }
}
bitmask_binop!(Capability, u8);

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompleteKind(u8);
impl CompleteKind {
    pub const PIXMAP: Self = Self(0);
    pub const NOTIFY_MSC: Self = Self(1);
}
impl From<CompleteKind> for u8 {
    #[inline]
    fn from(input: CompleteKind) -> Self {
        input.0
    }
}
impl From<CompleteKind> for std::option::Option<u8> {
    #[inline]
    fn from(input: CompleteKind) -> Self {
        Some(input.0)
    }
}
impl From<CompleteKind> for u16 {
    #[inline]
    fn from(input: CompleteKind) -> Self {
        u16::from(input.0)
    }
}
impl From<CompleteKind> for std::option::Option<u16> {
    #[inline]
    fn from(input: CompleteKind) -> Self {
        Some(u16::from(input.0))
    }
}
impl From<CompleteKind> for u32 {
    #[inline]
    fn from(input: CompleteKind) -> Self {
        u32::from(input.0)
    }
}
impl From<CompleteKind> for std::option::Option<u32> {
    #[inline]
    fn from(input: CompleteKind) -> Self {
        Some(u32::from(input.0))
    }
}
impl From<u8> for CompleteKind {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl core::fmt::Debug for CompleteKind  {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variants = [
            (Self::PIXMAP.0.into(), "PIXMAP", "Pixmap"),
            (Self::NOTIFY_MSC.0.into(), "NOTIFY_MSC", "NotifyMSC"),
        ];
        pretty_print_enum(fmt, self.0.into(), &variants)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompleteMode(u8);
impl CompleteMode {
    pub const COPY: Self = Self(0);
    pub const FLIP: Self = Self(1);
    pub const SKIP: Self = Self(2);
    pub const SUBOPTIMAL_COPY: Self = Self(3);
}
impl From<CompleteMode> for u8 {
    #[inline]
    fn from(input: CompleteMode) -> Self {
        input.0
    }
}
impl From<CompleteMode> for std::option::Option<u8> {
    #[inline]
    fn from(input: CompleteMode) -> Self {
        Some(input.0)
    }
}
impl From<CompleteMode> for u16 {
    #[inline]
    fn from(input: CompleteMode) -> Self {
        u16::from(input.0)
    }
}
impl From<CompleteMode> for std::option::Option<u16> {
    #[inline]
    fn from(input: CompleteMode) -> Self {
        Some(u16::from(input.0))
    }
}
impl From<CompleteMode> for u32 {
    #[inline]
    fn from(input: CompleteMode) -> Self {
        u32::from(input.0)
    }
}
impl From<CompleteMode> for std::option::Option<u32> {
    #[inline]
    fn from(input: CompleteMode) -> Self {
        Some(u32::from(input.0))
    }
}
impl From<u8> for CompleteMode {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl core::fmt::Debug for CompleteMode  {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variants = [
            (Self::COPY.0.into(), "COPY", "Copy"),
            (Self::FLIP.0.into(), "FLIP", "Flip"),
            (Self::SKIP.0.into(), "SKIP", "Skip"),
            (Self::SUBOPTIMAL_COPY.0.into(), "SUBOPTIMAL_COPY", "SuboptimalCopy"),
        ];
        pretty_print_enum(fmt, self.0.into(), &variants)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notify {
    pub window: xproto::Window,
    pub serial: u32,
}
impl TryParse for Notify {
    fn try_parse(remaining: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let (window, remaining) = xproto::Window::try_parse(remaining)?;
        let (serial, remaining) = u32::try_parse(remaining)?;
        let result = Notify { window, serial };
        Ok((result, remaining))
    }
}
impl Serialize for Notify {
    type Bytes = [u8; 8];
    fn serialize(&self) -> [u8; 8] {
        let window_bytes = self.window.serialize();
        let serial_bytes = self.serial.serialize();
        [
            window_bytes[0],
            window_bytes[1],
            window_bytes[2],
            window_bytes[3],
            serial_bytes[0],
            serial_bytes[1],
            serial_bytes[2],
            serial_bytes[3],
        ]
    }
    fn serialize_into(&self, bytes: &mut Vec<u8>) {
        bytes.reserve(8);
        self.window.serialize_into(bytes);
        self.serial.serialize_into(bytes);
    }
}

/// Opcode for the QueryVersion request
pub const QUERY_VERSION_REQUEST: u8 = 0;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryVersionRequest {
    pub major_version: u32,
    pub minor_version: u32,
}
impl QueryVersionRequest {
    /// Serialize this request into bytes for the provided connection
    pub fn serialize(self, major_opcode: u8) -> BufWithFds<PiecewiseBuf<'static>> {
        let length_so_far = 0;
        let major_version_bytes = self.major_version.serialize();
        let minor_version_bytes = self.minor_version.serialize();
        let mut request0 = vec![
            major_opcode,
            QUERY_VERSION_REQUEST,
            0,
            0,
            major_version_bytes[0],
            major_version_bytes[1],
            major_version_bytes[2],
            major_version_bytes[3],
            minor_version_bytes[0],
            minor_version_bytes[1],
            minor_version_bytes[2],
            minor_version_bytes[3],
        ];
        let length_so_far = length_so_far + request0.len();
        assert_eq!(length_so_far % 4, 0);
        let length = u16::try_from(length_so_far / 4).unwrap_or(0);
        request0[2..4].copy_from_slice(&length.to_ne_bytes());
        (vec![request0.into()], vec![])
    }
    /// Parse this request given its header, its body, and any fds that go along with it
    pub fn try_parse_request(header: RequestHeader, value: &[u8]) -> Result<Self, ParseError> {
        if header.minor_opcode != QUERY_VERSION_REQUEST {
            return Err(ParseError::InvalidValue);
        }
        let (major_version, remaining) = u32::try_parse(value)?;
        let (minor_version, remaining) = u32::try_parse(remaining)?;
        let _ = remaining;
        Ok(QueryVersionRequest {
            major_version,
            minor_version,
        })
    }
}
impl Request for QueryVersionRequest {
    const EXTENSION_NAME: std::option::Option<&'static str> = Some(X11_EXTENSION_NAME);

    fn serialize(self, major_opcode: u8) -> BufWithFds<Vec<u8>> {
        let (bufs, fds) = self.serialize(major_opcode);
        // Flatten the buffers into a single vector
        let buf = bufs.iter().flat_map(|buf| buf.iter().copied()).collect();
        (buf, fds)
    }
}
impl crate::x11_utils::ReplyRequest for QueryVersionRequest {
    type Reply = QueryVersionReply;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryVersionReply {
    pub sequence: u16,
    pub length: u32,
    pub major_version: u32,
    pub minor_version: u32,
}
impl TryParse for QueryVersionReply {
    fn try_parse(initial_value: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let remaining = initial_value;
        let (response_type, remaining) = u8::try_parse(remaining)?;
        let remaining = remaining.get(1..).ok_or(ParseError::InsufficientData)?;
        let (sequence, remaining) = u16::try_parse(remaining)?;
        let (length, remaining) = u32::try_parse(remaining)?;
        let (major_version, remaining) = u32::try_parse(remaining)?;
        let (minor_version, remaining) = u32::try_parse(remaining)?;
        if response_type != 1 {
            return Err(ParseError::InvalidValue);
        }
        let result = QueryVersionReply { sequence, length, major_version, minor_version };
        let _ = remaining;
        let remaining = initial_value.get(32 + length as usize * 4..)
            .ok_or(ParseError::InsufficientData)?;
        Ok((result, remaining))
    }
}

/// Opcode for the Pixmap request
pub const PIXMAP_REQUEST: u8 = 1;
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PixmapRequest<'input> {
    pub window: xproto::Window,
    pub pixmap: xproto::Pixmap,
    pub serial: u32,
    pub valid: xfixes::Region,
    pub update: xfixes::Region,
    pub x_off: i16,
    pub y_off: i16,
    pub target_crtc: randr::Crtc,
    pub wait_fence: sync::Fence,
    pub idle_fence: sync::Fence,
    pub options: u32,
    pub target_msc: u64,
    pub divisor: u64,
    pub remainder: u64,
    pub notifies: Cow<'input, [Notify]>,
}
impl<'input> PixmapRequest<'input> {
    /// Serialize this request into bytes for the provided connection
    pub fn serialize(self, major_opcode: u8) -> BufWithFds<PiecewiseBuf<'input>> {
        let length_so_far = 0;
        let window_bytes = self.window.serialize();
        let pixmap_bytes = self.pixmap.serialize();
        let serial_bytes = self.serial.serialize();
        let valid_bytes = self.valid.serialize();
        let update_bytes = self.update.serialize();
        let x_off_bytes = self.x_off.serialize();
        let y_off_bytes = self.y_off.serialize();
        let target_crtc_bytes = self.target_crtc.serialize();
        let wait_fence_bytes = self.wait_fence.serialize();
        let idle_fence_bytes = self.idle_fence.serialize();
        let options_bytes = self.options.serialize();
        let target_msc_bytes = self.target_msc.serialize();
        let divisor_bytes = self.divisor.serialize();
        let remainder_bytes = self.remainder.serialize();
        let mut request0 = vec![
            major_opcode,
            PIXMAP_REQUEST,
            0,
            0,
            window_bytes[0],
            window_bytes[1],
            window_bytes[2],
            window_bytes[3],
            pixmap_bytes[0],
            pixmap_bytes[1],
            pixmap_bytes[2],
            pixmap_bytes[3],
            serial_bytes[0],
            serial_bytes[1],
            serial_bytes[2],
            serial_bytes[3],
            valid_bytes[0],
            valid_bytes[1],
            valid_bytes[2],
            valid_bytes[3],
            update_bytes[0],
            update_bytes[1],
            update_bytes[2],
            update_bytes[3],
            x_off_bytes[0],
            x_off_bytes[1],
            y_off_bytes[0],
            y_off_bytes[1],
            target_crtc_bytes[0],
            target_crtc_bytes[1],
            target_crtc_bytes[2],
            target_crtc_bytes[3],
            wait_fence_bytes[0],
            wait_fence_bytes[1],
            wait_fence_bytes[2],
            wait_fence_bytes[3],
            idle_fence_bytes[0],
            idle_fence_bytes[1],
            idle_fence_bytes[2],
            idle_fence_bytes[3],
            options_bytes[0],
            options_bytes[1],
            options_bytes[2],
            options_bytes[3],
            0,
            0,
            0,
            0,
            target_msc_bytes[0],
            target_msc_bytes[1],
            target_msc_bytes[2],
            target_msc_bytes[3],
            target_msc_bytes[4],
            target_msc_bytes[5],
            target_msc_bytes[6],
            target_msc_bytes[7],
            divisor_bytes[0],
            divisor_bytes[1],
            divisor_bytes[2],
            divisor_bytes[3],
            divisor_bytes[4],
            divisor_bytes[5],
            divisor_bytes[6],
            divisor_bytes[7],
            remainder_bytes[0],
            remainder_bytes[1],
            remainder_bytes[2],
            remainder_bytes[3],
            remainder_bytes[4],
            remainder_bytes[5],
            remainder_bytes[6],
            remainder_bytes[7],
        ];
        let length_so_far = length_so_far + request0.len();
        let notifies_bytes = self.notifies.serialize();
        let length_so_far = length_so_far + notifies_bytes.len();
        let padding0 = &[0; 3][..(4 - (length_so_far % 4)) % 4];
        let length_so_far = length_so_far + padding0.len();
        assert_eq!(length_so_far % 4, 0);
        let length = u16::try_from(length_so_far / 4).unwrap_or(0);
        request0[2..4].copy_from_slice(&length.to_ne_bytes());
        (vec![request0.into(), notifies_bytes.into(), padding0.into()], vec![])
    }
    /// Parse this request given its header, its body, and any fds that go along with it
    pub fn try_parse_request(header: RequestHeader, value: &'input [u8]) -> Result<Self, ParseError> {
        if header.minor_opcode != PIXMAP_REQUEST {
            return Err(ParseError::InvalidValue);
        }
        let (window, remaining) = xproto::Window::try_parse(value)?;
        let (pixmap, remaining) = xproto::Pixmap::try_parse(remaining)?;
        let (serial, remaining) = u32::try_parse(remaining)?;
        let (valid, remaining) = xfixes::Region::try_parse(remaining)?;
        let (update, remaining) = xfixes::Region::try_parse(remaining)?;
        let (x_off, remaining) = i16::try_parse(remaining)?;
        let (y_off, remaining) = i16::try_parse(remaining)?;
        let (target_crtc, remaining) = randr::Crtc::try_parse(remaining)?;
        let (wait_fence, remaining) = sync::Fence::try_parse(remaining)?;
        let (idle_fence, remaining) = sync::Fence::try_parse(remaining)?;
        let (options, remaining) = u32::try_parse(remaining)?;
        let remaining = remaining.get(4..).ok_or(ParseError::InsufficientData)?;
        let (target_msc, remaining) = u64::try_parse(remaining)?;
        let (divisor, remaining) = u64::try_parse(remaining)?;
        let (remainder, remaining) = u64::try_parse(remaining)?;
        let mut remaining = remaining;
        // Length is 'everything left in the input'
        let mut notifies = Vec::new();
        while !remaining.is_empty() {
            let (v, new_remaining) = Notify::try_parse(remaining)?;
            remaining = new_remaining;
            notifies.push(v);
        }
        let _ = remaining;
        Ok(PixmapRequest {
            window,
            pixmap,
            serial,
            valid,
            update,
            x_off,
            y_off,
            target_crtc,
            wait_fence,
            idle_fence,
            options,
            target_msc,
            divisor,
            remainder,
            notifies: Cow::Owned(notifies),
        })
    }
    /// Clone all borrowed data in this PixmapRequest.
    pub fn into_owned(self) -> PixmapRequest<'static> {
        PixmapRequest {
            window: self.window,
            pixmap: self.pixmap,
            serial: self.serial,
            valid: self.valid,
            update: self.update,
            x_off: self.x_off,
            y_off: self.y_off,
            target_crtc: self.target_crtc,
            wait_fence: self.wait_fence,
            idle_fence: self.idle_fence,
            options: self.options,
            target_msc: self.target_msc,
            divisor: self.divisor,
            remainder: self.remainder,
            notifies: Cow::Owned(self.notifies.into_owned()),
        }
    }
}
impl<'input> Request for PixmapRequest<'input> {
    const EXTENSION_NAME: std::option::Option<&'static str> = Some(X11_EXTENSION_NAME);

    fn serialize(self, major_opcode: u8) -> BufWithFds<Vec<u8>> {
        let (bufs, fds) = self.serialize(major_opcode);
        // Flatten the buffers into a single vector
        let buf = bufs.iter().flat_map(|buf| buf.iter().copied()).collect();
        (buf, fds)
    }
}
impl<'input> crate::x11_utils::VoidRequest for PixmapRequest<'input> {
}

/// Opcode for the NotifyMSC request
pub const NOTIFY_MSC_REQUEST: u8 = 2;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NotifyMSCRequest {
    pub window: xproto::Window,
    pub serial: u32,
    pub target_msc: u64,
    pub divisor: u64,
    pub remainder: u64,
}
impl NotifyMSCRequest {
    /// Serialize this request into bytes for the provided connection
    pub fn serialize(self, major_opcode: u8) -> BufWithFds<PiecewiseBuf<'static>> {
        let length_so_far = 0;
        let window_bytes = self.window.serialize();
        let serial_bytes = self.serial.serialize();
        let target_msc_bytes = self.target_msc.serialize();
        let divisor_bytes = self.divisor.serialize();
        let remainder_bytes = self.remainder.serialize();
        let mut request0 = vec![
            major_opcode,
            NOTIFY_MSC_REQUEST,
            0,
            0,
            window_bytes[0],
            window_bytes[1],
            window_bytes[2],
            window_bytes[3],
            serial_bytes[0],
            serial_bytes[1],
            serial_bytes[2],
            serial_bytes[3],
            0,
            0,
            0,
            0,
            target_msc_bytes[0],
            target_msc_bytes[1],
            target_msc_bytes[2],
            target_msc_bytes[3],
            target_msc_bytes[4],
            target_msc_bytes[5],
            target_msc_bytes[6],
            target_msc_bytes[7],
            divisor_bytes[0],
            divisor_bytes[1],
            divisor_bytes[2],
            divisor_bytes[3],
            divisor_bytes[4],
            divisor_bytes[5],
            divisor_bytes[6],
            divisor_bytes[7],
            remainder_bytes[0],
            remainder_bytes[1],
            remainder_bytes[2],
            remainder_bytes[3],
            remainder_bytes[4],
            remainder_bytes[5],
            remainder_bytes[6],
            remainder_bytes[7],
        ];
        let length_so_far = length_so_far + request0.len();
        assert_eq!(length_so_far % 4, 0);
        let length = u16::try_from(length_so_far / 4).unwrap_or(0);
        request0[2..4].copy_from_slice(&length.to_ne_bytes());
        (vec![request0.into()], vec![])
    }
    /// Parse this request given its header, its body, and any fds that go along with it
    pub fn try_parse_request(header: RequestHeader, value: &[u8]) -> Result<Self, ParseError> {
        if header.minor_opcode != NOTIFY_MSC_REQUEST {
            return Err(ParseError::InvalidValue);
        }
        let (window, remaining) = xproto::Window::try_parse(value)?;
        let (serial, remaining) = u32::try_parse(remaining)?;
        let remaining = remaining.get(4..).ok_or(ParseError::InsufficientData)?;
        let (target_msc, remaining) = u64::try_parse(remaining)?;
        let (divisor, remaining) = u64::try_parse(remaining)?;
        let (remainder, remaining) = u64::try_parse(remaining)?;
        let _ = remaining;
        Ok(NotifyMSCRequest {
            window,
            serial,
            target_msc,
            divisor,
            remainder,
        })
    }
}
impl Request for NotifyMSCRequest {
    const EXTENSION_NAME: std::option::Option<&'static str> = Some(X11_EXTENSION_NAME);

    fn serialize(self, major_opcode: u8) -> BufWithFds<Vec<u8>> {
        let (bufs, fds) = self.serialize(major_opcode);
        // Flatten the buffers into a single vector
        let buf = bufs.iter().flat_map(|buf| buf.iter().copied()).collect();
        (buf, fds)
    }
}
impl crate::x11_utils::VoidRequest for NotifyMSCRequest {
}

pub type Event = u32;

/// Opcode for the SelectInput request
pub const SELECT_INPUT_REQUEST: u8 = 3;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SelectInputRequest {
    pub eid: Event,
    pub window: xproto::Window,
    pub event_mask: u32,
}
impl SelectInputRequest {
    /// Serialize this request into bytes for the provided connection
    pub fn serialize(self, major_opcode: u8) -> BufWithFds<PiecewiseBuf<'static>> {
        let length_so_far = 0;
        let eid_bytes = self.eid.serialize();
        let window_bytes = self.window.serialize();
        let event_mask_bytes = self.event_mask.serialize();
        let mut request0 = vec![
            major_opcode,
            SELECT_INPUT_REQUEST,
            0,
            0,
            eid_bytes[0],
            eid_bytes[1],
            eid_bytes[2],
            eid_bytes[3],
            window_bytes[0],
            window_bytes[1],
            window_bytes[2],
            window_bytes[3],
            event_mask_bytes[0],
            event_mask_bytes[1],
            event_mask_bytes[2],
            event_mask_bytes[3],
        ];
        let length_so_far = length_so_far + request0.len();
        assert_eq!(length_so_far % 4, 0);
        let length = u16::try_from(length_so_far / 4).unwrap_or(0);
        request0[2..4].copy_from_slice(&length.to_ne_bytes());
        (vec![request0.into()], vec![])
    }
    /// Parse this request given its header, its body, and any fds that go along with it
    pub fn try_parse_request(header: RequestHeader, value: &[u8]) -> Result<Self, ParseError> {
        if header.minor_opcode != SELECT_INPUT_REQUEST {
            return Err(ParseError::InvalidValue);
        }
        let (eid, remaining) = Event::try_parse(value)?;
        let (window, remaining) = xproto::Window::try_parse(remaining)?;
        let (event_mask, remaining) = u32::try_parse(remaining)?;
        let _ = remaining;
        Ok(SelectInputRequest {
            eid,
            window,
            event_mask,
        })
    }
}
impl Request for SelectInputRequest {
    const EXTENSION_NAME: std::option::Option<&'static str> = Some(X11_EXTENSION_NAME);

    fn serialize(self, major_opcode: u8) -> BufWithFds<Vec<u8>> {
        let (bufs, fds) = self.serialize(major_opcode);
        // Flatten the buffers into a single vector
        let buf = bufs.iter().flat_map(|buf| buf.iter().copied()).collect();
        (buf, fds)
    }
}
impl crate::x11_utils::VoidRequest for SelectInputRequest {
}

/// Opcode for the QueryCapabilities request
pub const QUERY_CAPABILITIES_REQUEST: u8 = 4;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryCapabilitiesRequest {
    pub target: u32,
}
impl QueryCapabilitiesRequest {
    /// Serialize this request into bytes for the provided connection
    pub fn serialize(self, major_opcode: u8) -> BufWithFds<PiecewiseBuf<'static>> {
        let length_so_far = 0;
        let target_bytes = self.target.serialize();
        let mut request0 = vec![
            major_opcode,
            QUERY_CAPABILITIES_REQUEST,
            0,
            0,
            target_bytes[0],
            target_bytes[1],
            target_bytes[2],
            target_bytes[3],
        ];
        let length_so_far = length_so_far + request0.len();
        assert_eq!(length_so_far % 4, 0);
        let length = u16::try_from(length_so_far / 4).unwrap_or(0);
        request0[2..4].copy_from_slice(&length.to_ne_bytes());
        (vec![request0.into()], vec![])
    }
    /// Parse this request given its header, its body, and any fds that go along with it
    pub fn try_parse_request(header: RequestHeader, value: &[u8]) -> Result<Self, ParseError> {
        if header.minor_opcode != QUERY_CAPABILITIES_REQUEST {
            return Err(ParseError::InvalidValue);
        }
        let (target, remaining) = u32::try_parse(value)?;
        let _ = remaining;
        Ok(QueryCapabilitiesRequest {
            target,
        })
    }
}
impl Request for QueryCapabilitiesRequest {
    const EXTENSION_NAME: std::option::Option<&'static str> = Some(X11_EXTENSION_NAME);

    fn serialize(self, major_opcode: u8) -> BufWithFds<Vec<u8>> {
        let (bufs, fds) = self.serialize(major_opcode);
        // Flatten the buffers into a single vector
        let buf = bufs.iter().flat_map(|buf| buf.iter().copied()).collect();
        (buf, fds)
    }
}
impl crate::x11_utils::ReplyRequest for QueryCapabilitiesRequest {
    type Reply = QueryCapabilitiesReply;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryCapabilitiesReply {
    pub sequence: u16,
    pub length: u32,
    pub capabilities: u32,
}
impl TryParse for QueryCapabilitiesReply {
    fn try_parse(initial_value: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let remaining = initial_value;
        let (response_type, remaining) = u8::try_parse(remaining)?;
        let remaining = remaining.get(1..).ok_or(ParseError::InsufficientData)?;
        let (sequence, remaining) = u16::try_parse(remaining)?;
        let (length, remaining) = u32::try_parse(remaining)?;
        let (capabilities, remaining) = u32::try_parse(remaining)?;
        if response_type != 1 {
            return Err(ParseError::InvalidValue);
        }
        let result = QueryCapabilitiesReply { sequence, length, capabilities };
        let _ = remaining;
        let remaining = initial_value.get(32 + length as usize * 4..)
            .ok_or(ParseError::InsufficientData)?;
        Ok((result, remaining))
    }
}

/// Opcode for the Generic event
pub const GENERIC_EVENT: u8 = 0;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GenericEvent {
    pub response_type: u8,
    pub extension: u8,
    pub sequence: u16,
    pub length: u32,
    pub evtype: u16,
    pub event: Event,
}
impl TryParse for GenericEvent {
    fn try_parse(initial_value: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let remaining = initial_value;
        let (response_type, remaining) = u8::try_parse(remaining)?;
        let (extension, remaining) = u8::try_parse(remaining)?;
        let (sequence, remaining) = u16::try_parse(remaining)?;
        let (length, remaining) = u32::try_parse(remaining)?;
        let (evtype, remaining) = u16::try_parse(remaining)?;
        let remaining = remaining.get(2..).ok_or(ParseError::InsufficientData)?;
        let (event, remaining) = Event::try_parse(remaining)?;
        let result = GenericEvent { response_type, extension, sequence, length, evtype, event };
        let _ = remaining;
        let remaining = initial_value.get(32..)
            .ok_or(ParseError::InsufficientData)?;
        Ok((result, remaining))
    }
}
impl From<&GenericEvent> for [u8; 32] {
    fn from(input: &GenericEvent) -> Self {
        let response_type_bytes = input.response_type.serialize();
        let extension_bytes = input.extension.serialize();
        let sequence_bytes = input.sequence.serialize();
        let length_bytes = input.length.serialize();
        let evtype_bytes = input.evtype.serialize();
        let event_bytes = input.event.serialize();
        [
            response_type_bytes[0],
            extension_bytes[0],
            sequence_bytes[0],
            sequence_bytes[1],
            length_bytes[0],
            length_bytes[1],
            length_bytes[2],
            length_bytes[3],
            evtype_bytes[0],
            evtype_bytes[1],
            0,
            0,
            event_bytes[0],
            event_bytes[1],
            event_bytes[2],
            event_bytes[3],
            // trailing padding
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ]
    }
}
impl From<GenericEvent> for [u8; 32] {
    fn from(input: GenericEvent) -> Self {
        Self::from(&input)
    }
}

/// Opcode for the ConfigureNotify event
pub const CONFIGURE_NOTIFY_EVENT: u16 = 0;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConfigureNotifyEvent {
    pub response_type: u8,
    pub extension: u8,
    pub sequence: u16,
    pub length: u32,
    pub event_type: u16,
    pub event: Event,
    pub window: xproto::Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub off_x: i16,
    pub off_y: i16,
    pub pixmap_width: u16,
    pub pixmap_height: u16,
    pub pixmap_flags: u32,
}
impl TryParse for ConfigureNotifyEvent {
    fn try_parse(initial_value: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let remaining = initial_value;
        let (response_type, remaining) = u8::try_parse(remaining)?;
        let (extension, remaining) = u8::try_parse(remaining)?;
        let (sequence, remaining) = u16::try_parse(remaining)?;
        let (length, remaining) = u32::try_parse(remaining)?;
        let (event_type, remaining) = u16::try_parse(remaining)?;
        let remaining = remaining.get(2..).ok_or(ParseError::InsufficientData)?;
        let (event, remaining) = Event::try_parse(remaining)?;
        let (window, remaining) = xproto::Window::try_parse(remaining)?;
        let (x, remaining) = i16::try_parse(remaining)?;
        let (y, remaining) = i16::try_parse(remaining)?;
        let (width, remaining) = u16::try_parse(remaining)?;
        let (height, remaining) = u16::try_parse(remaining)?;
        let (off_x, remaining) = i16::try_parse(remaining)?;
        let (off_y, remaining) = i16::try_parse(remaining)?;
        let (pixmap_width, remaining) = u16::try_parse(remaining)?;
        let (pixmap_height, remaining) = u16::try_parse(remaining)?;
        let (pixmap_flags, remaining) = u32::try_parse(remaining)?;
        let result = ConfigureNotifyEvent { response_type, extension, sequence, length, event_type, event, window, x, y, width, height, off_x, off_y, pixmap_width, pixmap_height, pixmap_flags };
        let _ = remaining;
        let remaining = initial_value.get(32 + length as usize * 4..)
            .ok_or(ParseError::InsufficientData)?;
        Ok((result, remaining))
    }
}

/// Opcode for the CompleteNotify event
pub const COMPLETE_NOTIFY_EVENT: u16 = 1;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompleteNotifyEvent {
    pub response_type: u8,
    pub extension: u8,
    pub sequence: u16,
    pub length: u32,
    pub event_type: u16,
    pub kind: CompleteKind,
    pub mode: CompleteMode,
    pub event: Event,
    pub window: xproto::Window,
    pub serial: u32,
    pub ust: u64,
    pub msc: u64,
}
impl TryParse for CompleteNotifyEvent {
    fn try_parse(initial_value: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let remaining = initial_value;
        let (response_type, remaining) = u8::try_parse(remaining)?;
        let (extension, remaining) = u8::try_parse(remaining)?;
        let (sequence, remaining) = u16::try_parse(remaining)?;
        let (length, remaining) = u32::try_parse(remaining)?;
        let (event_type, remaining) = u16::try_parse(remaining)?;
        let (kind, remaining) = u8::try_parse(remaining)?;
        let (mode, remaining) = u8::try_parse(remaining)?;
        let (event, remaining) = Event::try_parse(remaining)?;
        let (window, remaining) = xproto::Window::try_parse(remaining)?;
        let (serial, remaining) = u32::try_parse(remaining)?;
        let (ust, remaining) = u64::try_parse(remaining)?;
        let (msc, remaining) = u64::try_parse(remaining)?;
        let kind = kind.into();
        let mode = mode.into();
        let result = CompleteNotifyEvent { response_type, extension, sequence, length, event_type, kind, mode, event, window, serial, ust, msc };
        let _ = remaining;
        let remaining = initial_value.get(32 + length as usize * 4..)
            .ok_or(ParseError::InsufficientData)?;
        Ok((result, remaining))
    }
}

/// Opcode for the IdleNotify event
pub const IDLE_NOTIFY_EVENT: u16 = 2;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdleNotifyEvent {
    pub response_type: u8,
    pub extension: u8,
    pub sequence: u16,
    pub length: u32,
    pub event_type: u16,
    pub event: Event,
    pub window: xproto::Window,
    pub serial: u32,
    pub pixmap: xproto::Pixmap,
    pub idle_fence: sync::Fence,
}
impl TryParse for IdleNotifyEvent {
    fn try_parse(initial_value: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let remaining = initial_value;
        let (response_type, remaining) = u8::try_parse(remaining)?;
        let (extension, remaining) = u8::try_parse(remaining)?;
        let (sequence, remaining) = u16::try_parse(remaining)?;
        let (length, remaining) = u32::try_parse(remaining)?;
        let (event_type, remaining) = u16::try_parse(remaining)?;
        let remaining = remaining.get(2..).ok_or(ParseError::InsufficientData)?;
        let (event, remaining) = Event::try_parse(remaining)?;
        let (window, remaining) = xproto::Window::try_parse(remaining)?;
        let (serial, remaining) = u32::try_parse(remaining)?;
        let (pixmap, remaining) = xproto::Pixmap::try_parse(remaining)?;
        let (idle_fence, remaining) = sync::Fence::try_parse(remaining)?;
        let result = IdleNotifyEvent { response_type, extension, sequence, length, event_type, event, window, serial, pixmap, idle_fence };
        let _ = remaining;
        let remaining = initial_value.get(32 + length as usize * 4..)
            .ok_or(ParseError::InsufficientData)?;
        Ok((result, remaining))
    }
}

/// Opcode for the RedirectNotify event
pub const REDIRECT_NOTIFY_EVENT: u16 = 3;
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RedirectNotifyEvent {
    pub response_type: u8,
    pub extension: u8,
    pub sequence: u16,
    pub length: u32,
    pub event_type: u16,
    pub update_window: bool,
    pub event: Event,
    pub event_window: xproto::Window,
    pub window: xproto::Window,
    pub pixmap: xproto::Pixmap,
    pub serial: u32,
    pub valid_region: xfixes::Region,
    pub update_region: xfixes::Region,
    pub valid_rect: xproto::Rectangle,
    pub update_rect: xproto::Rectangle,
    pub x_off: i16,
    pub y_off: i16,
    pub target_crtc: randr::Crtc,
    pub wait_fence: sync::Fence,
    pub idle_fence: sync::Fence,
    pub options: u32,
    pub target_msc: u64,
    pub divisor: u64,
    pub remainder: u64,
    pub notifies: Vec<Notify>,
}
impl TryParse for RedirectNotifyEvent {
    fn try_parse(initial_value: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        let remaining = initial_value;
        let (response_type, remaining) = u8::try_parse(remaining)?;
        let (extension, remaining) = u8::try_parse(remaining)?;
        let (sequence, remaining) = u16::try_parse(remaining)?;
        let (length, remaining) = u32::try_parse(remaining)?;
        let (event_type, remaining) = u16::try_parse(remaining)?;
        let (update_window, remaining) = bool::try_parse(remaining)?;
        let remaining = remaining.get(1..).ok_or(ParseError::InsufficientData)?;
        let (event, remaining) = Event::try_parse(remaining)?;
        let (event_window, remaining) = xproto::Window::try_parse(remaining)?;
        let (window, remaining) = xproto::Window::try_parse(remaining)?;
        let (pixmap, remaining) = xproto::Pixmap::try_parse(remaining)?;
        let (serial, remaining) = u32::try_parse(remaining)?;
        let (valid_region, remaining) = xfixes::Region::try_parse(remaining)?;
        let (update_region, remaining) = xfixes::Region::try_parse(remaining)?;
        let (valid_rect, remaining) = xproto::Rectangle::try_parse(remaining)?;
        let (update_rect, remaining) = xproto::Rectangle::try_parse(remaining)?;
        let (x_off, remaining) = i16::try_parse(remaining)?;
        let (y_off, remaining) = i16::try_parse(remaining)?;
        let (target_crtc, remaining) = randr::Crtc::try_parse(remaining)?;
        let (wait_fence, remaining) = sync::Fence::try_parse(remaining)?;
        let (idle_fence, remaining) = sync::Fence::try_parse(remaining)?;
        let (options, remaining) = u32::try_parse(remaining)?;
        let remaining = remaining.get(4..).ok_or(ParseError::InsufficientData)?;
        let (target_msc, remaining) = u64::try_parse(remaining)?;
        let (divisor, remaining) = u64::try_parse(remaining)?;
        let (remainder, remaining) = u64::try_parse(remaining)?;
        let mut remaining = remaining;
        // Length is 'everything left in the input'
        let mut notifies = Vec::new();
        while !remaining.is_empty() {
            let (v, new_remaining) = Notify::try_parse(remaining)?;
            remaining = new_remaining;
            notifies.push(v);
        }
        let result = RedirectNotifyEvent { response_type, extension, sequence, length, event_type, update_window, event, event_window, window, pixmap, serial, valid_region, update_region, valid_rect, update_rect, x_off, y_off, target_crtc, wait_fence, idle_fence, options, target_msc, divisor, remainder, notifies };
        let _ = remaining;
        let remaining = initial_value.get(32 + length as usize * 4..)
            .ok_or(ParseError::InsufficientData)?;
        Ok((result, remaining))
    }
}

