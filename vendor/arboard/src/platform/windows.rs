/*
SPDX-License-Identifier: Apache-2.0 OR MIT

Copyright 2022 The Arboard contributors

The project to which this file belongs is licensed under either of
the Apache 2.0 or the MIT license at the licensee's choice. The terms
and conditions of the chosen license apply to this file.
*/

use std::{borrow::Cow, marker::PhantomData};
#[cfg(feature = "image-data")]
use std::{convert::TryInto, mem::size_of};

#[cfg(feature = "image-data")]
use winapi::{
	shared::minwindef::DWORD,
	um::{
		errhandlingapi::GetLastError,
		winbase::{GlobalLock, GlobalUnlock},
		wingdi::{
			CreateDIBitmap, DeleteObject, GetDIBits, LCS_sRGB, BITMAPINFO, BITMAPINFOHEADER,
			BITMAPV5HEADER, BI_RGB, CBM_INIT, DIB_RGB_COLORS, LCS_GM_IMAGES, PROFILE_EMBEDDED,
			PROFILE_LINKED, RGBQUAD,
		},
		winnt::LONG,
		winuser::{GetDC, SetClipboardData},
	},
};

use crate::common::{private, Error};

#[cfg(feature = "image-data")]
use crate::common::{ImageData, ScopeGuard};

#[cfg(feature = "image-data")]
fn add_cf_dibv5(_open_clipboard: OpenClipboard, image: ImageData) -> Result<(), Error> {
	use std::intrinsics::copy_nonoverlapping;
	use winapi::um::{
		winbase::{GlobalAlloc, GHND},
		wingdi::BI_BITFIELDS,
		winuser::CF_DIBV5,
	};

	let header_size = size_of::<BITMAPV5HEADER>();
	let header = BITMAPV5HEADER {
		bV5Size: header_size as u32,
		bV5Width: image.width as LONG,
		bV5Height: image.height as LONG,
		bV5Planes: 1,
		bV5BitCount: 32,
		bV5Compression: BI_BITFIELDS,
		bV5SizeImage: (4 * image.width * image.height) as DWORD,
		bV5XPelsPerMeter: 0,
		bV5YPelsPerMeter: 0,
		bV5ClrUsed: 0,
		bV5ClrImportant: 0,
		bV5RedMask: 0x00ff0000,
		bV5GreenMask: 0x0000ff00,
		bV5BlueMask: 0x000000ff,
		bV5AlphaMask: 0xff000000,
		bV5CSType: LCS_sRGB as u32,
		// SAFETY: Windows ignores this field because `bV5CSType` is not set to `LCS_CALIBRATED_RGB`.
		bV5Endpoints: unsafe { std::mem::zeroed() },
		bV5GammaRed: 0,
		bV5GammaGreen: 0,
		bV5GammaBlue: 0,
		bV5Intent: LCS_GM_IMAGES as u32, // I'm not sure about this.
		bV5ProfileData: 0,
		bV5ProfileSize: 0,
		bV5Reserved: 0,
	};

	// In theory we don't need to flip the image because we could just specify
	// a negative height in the header, which according to the documentation, indicates that the
	// image rows are in top-to-bottom order. HOWEVER: MS Word (and WordPad) cannot paste an image
	// that has a negative height in its header.
	let image = flip_v(image);

	let data_size = header_size + image.bytes.len();
	let hdata = unsafe { GlobalAlloc(GHND, data_size) };
	if hdata.is_null() {
		return Err(Error::Unknown {
			description: format!(
				"Could not allocate global memory object. GlobalAlloc returned null at line {}.",
				line!()
			),
		});
	}
	unsafe {
		let data_ptr = GlobalLock(hdata) as *mut u8;
		if data_ptr.is_null() {
			return Err(Error::Unknown {
				description: format!("Could not lock the global memory object at line {}", line!()),
			});
		}

		let _unlock = ScopeGuard::new(|| {
			let retval = GlobalUnlock(hdata);
			if retval == 0 {
				let lasterr = GetLastError();
				if lasterr != 0 {
					log::error!("Failed calling GlobalUnlock when writing dibv5 data. Error code was 0x{:X}", lasterr);
				}
			}
		});

		copy_nonoverlapping::<u8>((&header) as *const _ as *const u8, data_ptr, header_size);

		// Not using the `add` function, because that has a restriction, that the result cannot overflow isize
		let pixels_dst = (data_ptr as usize + header_size) as *mut u8;
		copy_nonoverlapping::<u8>(image.bytes.as_ptr(), pixels_dst, image.bytes.len());

		let dst_pixels_slice = std::slice::from_raw_parts_mut(pixels_dst, image.bytes.len());

		// If the non-allocating version of the function failed, we need to assign the new bytes to
		// the global allocation.
		if let Cow::Owned(new_pixels) = rgba_to_win(dst_pixels_slice) {
			// SAFETY: `data_ptr` is valid to write to and has no outstanding mutable borrows, and
			// `new_pixels` will be the same length as the original bytes.
			copy_nonoverlapping::<u8>(new_pixels.as_ptr(), data_ptr, new_pixels.len())
		}
	}

	unsafe {
		if SetClipboardData(CF_DIBV5, hdata as _).is_null() {
			DeleteObject(hdata as _);
			return Err(Error::Unknown {
				description: format!(
					"Call to `SetClipboardData` returned NULL at line {}",
					line!()
				),
			});
		}
	}

	Ok(())
}

#[cfg(feature = "image-data")]
fn read_cf_dibv5(dibv5: &[u8]) -> Result<ImageData<'static>, Error> {
	// The DIBV5 format is a BITMAPV5HEADER followed by the pixel data according to
	// https://docs.microsoft.com/en-us/windows/win32/dataxchg/standard-clipboard-formats

	// so first let's get a pointer to the header
	let header_size = size_of::<BITMAPV5HEADER>();
	if dibv5.len() < header_size {
		return Err(Error::Unknown {
			description: "When reading the DIBV5 data, it contained fewer bytes than the BITMAPV5HEADER size. This is invalid.".into()
		});
	}
	let header = unsafe { &*(dibv5.as_ptr() as *const BITMAPV5HEADER) };

	let has_profile =
		header.bV5CSType as i32 == PROFILE_LINKED || header.bV5CSType as i32 == PROFILE_EMBEDDED;

	let pixel_data_start = if has_profile {
		header.bV5ProfileData as isize + header.bV5ProfileSize as isize
	} else {
		header_size as isize
	};

	unsafe {
		let image_bytes = dibv5.as_ptr().offset(pixel_data_start) as *const _;
		let hdc = GetDC(std::ptr::null_mut());
		let hbitmap = CreateDIBitmap(
			hdc,
			header as *const BITMAPV5HEADER as *const _,
			CBM_INIT,
			image_bytes,
			header as *const BITMAPV5HEADER as *const _,
			DIB_RGB_COLORS,
		);
		if hbitmap.is_null() {
			return Err(Error::Unknown {
				description:
					"Failed to create the HBITMAP while reading DIBV5. CreateDIBitmap returned null"
						.into(),
			});
		}
		// Now extract the pixels in a desired format
		let w = header.bV5Width;
		let h = header.bV5Height.abs();
		let result_size = w as usize * h as usize * 4;

		let mut result_bytes = Vec::<u8>::with_capacity(result_size);

		let mut output_header = BITMAPINFO {
			bmiColors: [RGBQUAD { rgbRed: 0, rgbGreen: 0, rgbBlue: 0, rgbReserved: 0 }],
			bmiHeader: BITMAPINFOHEADER {
				biSize: size_of::<BITMAPINFOHEADER>() as u32,
				biWidth: w,
				biHeight: -h,
				biBitCount: 32,
				biPlanes: 1,
				biCompression: BI_RGB,
				biSizeImage: 0,
				biXPelsPerMeter: 0,
				biYPelsPerMeter: 0,
				biClrUsed: 0,
				biClrImportant: 0,
			},
		};

		let result = GetDIBits(
			hdc,
			hbitmap,
			0,
			h as u32,
			result_bytes.as_mut_ptr() as *mut _,
			&mut output_header as *mut _,
			DIB_RGB_COLORS,
		);
		if result == 0 {
			return Err(Error::Unknown {
				description: "Could not get the bitmap bits, GetDIBits returned 0".into(),
			});
		}
		let read_len = result as usize * w as usize * 4;
		if read_len > result_bytes.capacity() {
			panic!("Segmentation fault. Read more bytes than allocated to pixel buffer");
		}
		result_bytes.set_len(read_len);

		let result_bytes = win_to_rgba(&mut result_bytes);

		let result =
			ImageData { bytes: Cow::Owned(result_bytes), width: w as usize, height: h as usize };
		Ok(result)
	}
}

/// Converts the RGBA (u8) pixel data into the bitmap-native ARGB (u32) format in-place
///
/// Safety: the `bytes` slice must have a length that's a multiple of 4
#[cfg(feature = "image-data")]
#[allow(clippy::identity_op, clippy::erasing_op)]
#[must_use]
unsafe fn rgba_to_win(bytes: &mut [u8]) -> Cow<'_, [u8]> {
	// Check safety invariants to catch obvious bugs.
	debug_assert_eq!(bytes.len() % 4, 0);

	let mut u32pixels_buffer = convert_bytes_to_u32s(bytes);
	let u32pixels = match u32pixels_buffer {
		ImageDataCow::Borrowed(ref mut b) => b,
		ImageDataCow::Owned(ref mut b) => b.as_mut_slice(),
	};

	for p in u32pixels.iter_mut() {
		let [mut r, mut g, mut b, mut a] = p.to_ne_bytes().map(u32::from);
		r <<= 2 * 8;
		g <<= 1 * 8;
		b <<= 0 * 8;
		a <<= 3 * 8;

		*p = r | g | b | a;
	}

	match u32pixels_buffer {
		ImageDataCow::Borrowed(_) => Cow::Borrowed(bytes),
		ImageDataCow::Owned(bytes) => {
			Cow::Owned(bytes.into_iter().flat_map(|b| b.to_ne_bytes()).collect())
		}
	}
}

/// Vertically flips the image pixels in memory
#[cfg(feature = "image-data")]
fn flip_v(image: ImageData) -> ImageData<'static> {
	let w = image.width;
	let h = image.height;

	let mut bytes = image.bytes.into_owned();

	let rowsize = w * 4; // each pixel is 4 bytes
	let mut tmp_a = Vec::new();
	tmp_a.resize(rowsize, 0);
	// I believe this could be done safely with `as_chunks_mut`, but that's not stable yet
	for a_row_id in 0..(h / 2) {
		let b_row_id = h - a_row_id - 1;

		// swap rows `first_id` and `second_id`
		let a_byte_start = a_row_id * rowsize;
		let a_byte_end = a_byte_start + rowsize;
		let b_byte_start = b_row_id * rowsize;
		let b_byte_end = b_byte_start + rowsize;
		tmp_a.copy_from_slice(&bytes[a_byte_start..a_byte_end]);
		bytes.copy_within(b_byte_start..b_byte_end, a_byte_start);
		bytes[b_byte_start..b_byte_end].copy_from_slice(&tmp_a);
	}

	ImageData { width: image.width, height: image.height, bytes: bytes.into() }
}

/// Converts the ARGB (u32) pixel data into the RGBA (u8) format in-place
///
/// Safety: the `bytes` slice must have a length that's a multiple of 4
#[cfg(feature = "image-data")]
#[allow(clippy::identity_op, clippy::erasing_op)]
#[must_use]
unsafe fn win_to_rgba(bytes: &mut [u8]) -> Vec<u8> {
	// Check safety invariants to catch obvious bugs.
	debug_assert_eq!(bytes.len() % 4, 0);

	let mut u32pixels_buffer = convert_bytes_to_u32s(bytes);
	let u32pixels = match u32pixels_buffer {
		ImageDataCow::Borrowed(ref mut b) => b,
		ImageDataCow::Owned(ref mut b) => b.as_mut_slice(),
	};

	for p in u32pixels {
		let mut bytes = p.to_ne_bytes();
		bytes[0] = (*p >> (2 * 8)) as u8;
		bytes[1] = (*p >> (1 * 8)) as u8;
		bytes[2] = (*p >> (0 * 8)) as u8;
		bytes[3] = (*p >> (3 * 8)) as u8;
		*p = u32::from_ne_bytes(bytes);
	}

	match u32pixels_buffer {
		ImageDataCow::Borrowed(_) => bytes.to_vec(),
		ImageDataCow::Owned(bytes) => bytes.into_iter().flat_map(|b| b.to_ne_bytes()).collect(),
	}
}

#[cfg(feature = "image-data")]
// XXX: std's Cow is not usable here because it does not allow mutably
// borrowing data.
enum ImageDataCow<'a> {
	Borrowed(&'a mut [u32]),
	Owned(Vec<u32>),
}

/// Safety: the `bytes` slice must have a length that's a multiple of 4
#[cfg(feature = "image-data")]
unsafe fn convert_bytes_to_u32s(bytes: &mut [u8]) -> ImageDataCow<'_> {
	// When the correct conditions are upheld, `std` should return everything in the well-aligned slice.
	let (prefix, _, suffix) = bytes.align_to::<u32>();

	// Check if `align_to` gave us the optimal result.
	//
	// If it didn't, use the slow path with more allocations
	if prefix.is_empty() && suffix.is_empty() {
		// We know that the newly-aligned slice will contain all the values
		ImageDataCow::Borrowed(bytes.align_to_mut::<u32>().1)
	} else {
		// XXX: Use `as_chunks` when it stabilizes.
		let u32pixels_buffer =
			bytes.chunks(4).map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap())).collect();
		ImageDataCow::Owned(u32pixels_buffer)
	}
}

/// A shim clipboard type that can have operations performed with it, but
/// does not represent an open clipboard itself.
///
/// Windows only allows one thread on the entire system to have the clipboard
/// open at once, so we have to open it very sparingly or risk causing the rest
/// of the system to be unresponsive. Instead, the clipboard is opened for
/// every operation and then closed afterwards.
pub(crate) struct Clipboard(());

// The other platforms have `Drop` implementation on their
// clipboard, so Windows should too for consistently.
impl Drop for Clipboard {
	fn drop(&mut self) {}
}

struct OpenClipboard<'clipboard> {
	_inner: clipboard_win::Clipboard,
	// The Windows clipboard can not be sent between threads once
	// open.
	_marker: PhantomData<*const ()>,
	_for_shim: &'clipboard mut Clipboard,
}

impl Clipboard {
	const DEFAULT_OPEN_ATTEMPTS: usize = 5;

	pub(crate) fn new() -> Result<Self, Error> {
		Ok(Self(()))
	}

	fn open(&mut self) -> Result<OpenClipboard, Error> {
		// Attempt to open the clipboard multiple times. On Windows, its common for something else to temporarily
		// be using it during attempts.
		//
		// For past work/evidence, see Firefox(https://searchfox.org/mozilla-central/source/widget/windows/nsClipboard.cpp#421) and
		// Chromium(https://source.chromium.org/chromium/chromium/src/+/main:ui/base/clipboard/clipboard_win.cc;l=86).
		//
		// Note: This does not use `Clipboard::new_attempts` because its implementation sleeps for `0ms`, which can
		// cause race conditions between closing/opening the clipboard in single-threaded apps.
		let mut attempts = Self::DEFAULT_OPEN_ATTEMPTS;
		let clipboard = loop {
			match clipboard_win::Clipboard::new() {
				Ok(this) => break Ok(this),
				Err(err) => match attempts {
					0 => break Err(err),
					_ => attempts -= 1,
				},
			}

			// The default value matches Chromium's implementation, but could be tweaked later.
			// Safety: This is safe to call with any integer.
			unsafe { winapi::um::synchapi::Sleep(5) };
		}
		.map_err(|_| Error::ClipboardOccupied)?;

		Ok(OpenClipboard { _inner: clipboard, _marker: PhantomData, _for_shim: self })
	}
}

// Note: In all of the builders, a clipboard opening result is stored.
// This is done for a few reasons:
// 1. consistently with the other platforms which can have an occupied clipboard.
// 	It is better if the operation fails at the most similar place on all platforms.
// 2. `{Get, Set, Clear}::new()` don't return a `Result`. Windows is the only case that
// 	needs this kind of handling, so it doesn't need to affect the other APIs.
// 3. Due to how the clipboard works on Windows, we need to open it for every operation
// and keep it open until its finished. This approach allows RAII to still be applicable.

pub(crate) struct Get<'clipboard> {
	clipboard: Result<OpenClipboard<'clipboard>, Error>,
}

impl<'clipboard> Get<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { clipboard: clipboard.open() }
	}

	pub(crate) fn text(self) -> Result<String, Error> {
		const FORMAT: u32 = clipboard_win::formats::CF_UNICODETEXT;

		let _clipboard_assertion = self.clipboard?;

		// XXX: ToC/ToU race conditions are not possible because we are the sole owners of the clipboard currently.
		if !clipboard_win::is_format_avail(FORMAT) {
			return Err(Error::ContentNotAvailable);
		}

		let text_size = clipboard_win::raw::size(FORMAT).ok_or_else(|| Error::Unknown {
			description: "failed to read clipboard text size".into(),
		})?;

		// Allocate the specific number of WTF-16 characters we need to receive.
		// This division is always accurate because Windows uses 16-bit characters.
		let mut out: Vec<u16> = vec![0u16; text_size.get() / 2];

		let bytes_read = {
			// SAFETY: The source slice has a greater alignment than the resulting one.
			let out: &mut [u8] =
				unsafe { std::slice::from_raw_parts_mut(out.as_mut_ptr().cast(), out.len() * 2) };

			let mut bytes_read = clipboard_win::raw::get(FORMAT, out).map_err(|_| {
				Error::Unknown { description: "failed to read clipboard string".into() }
			})?;

			// Convert the number of bytes read to the number of `u16`s
			bytes_read /= 2;

			// Remove the NUL terminator, if it existed.
			if let Some(last) = out.last().copied() {
				if last == 0 {
					bytes_read -= 1;
				}
			}

			bytes_read
		};

		// Create a UTF-8 string from WTF-16 data, if it was valid.
		String::from_utf16(&out[..bytes_read]).map_err(|_| Error::ConversionFailure)
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn image(self) -> Result<ImageData<'static>, Error> {
		const FORMAT: u32 = clipboard_win::formats::CF_DIBV5;

		let _clipboard_assertion = self.clipboard?;

		if !clipboard_win::is_format_avail(FORMAT) {
			return Err(Error::ContentNotAvailable);
		}

		let mut data = Vec::new();

		clipboard_win::raw::get_vec(FORMAT, &mut data).map_err(|_| Error::Unknown {
			description: "failed to read clipboard image data".into(),
		})?;

		read_cf_dibv5(&data)
	}
}

pub(crate) struct Set<'clipboard> {
	clipboard: Result<OpenClipboard<'clipboard>, Error>,
	exclude_from_cloud: bool,
	exclude_from_history: bool,
}

impl<'clipboard> Set<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { clipboard: clipboard.open(), exclude_from_cloud: false, exclude_from_history: false }
	}

	pub(crate) fn text(self, data: Cow<'_, str>) -> Result<(), Error> {
		let open_clipboard = self.clipboard?;

		clipboard_win::raw::set_string(&data).map_err(|_| Error::Unknown {
			description: "Could not place the specified text to the clipboard".into(),
		})?;

		add_clipboard_exclusions(open_clipboard, self.exclude_from_cloud, self.exclude_from_history)
	}

	pub(crate) fn html(self, html: Cow<'_, str>, alt: Option<Cow<'_, str>>) -> Result<(), Error> {
		let open_clipboard = self.clipboard?;

		let alt = match alt {
			Some(s) => s.into(),
			None => String::new(),
		};
		clipboard_win::raw::set_string(&alt).map_err(|_| Error::Unknown {
			description: "Could not place the specified text to the clipboard".into(),
		})?;

		if let Some(format) = clipboard_win::register_format("HTML Format") {
			let html = wrap_html(&html);
			clipboard_win::raw::set_without_clear(format.get(), html.as_bytes())
				.map_err(|e| Error::Unknown { description: e.to_string() })?;
		}

		add_clipboard_exclusions(open_clipboard, self.exclude_from_cloud, self.exclude_from_history)
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn image(self, image: ImageData) -> Result<(), Error> {
		let open_clipboard = self.clipboard?;

		if let Err(e) = clipboard_win::raw::empty() {
			return Err(Error::Unknown {
				description: format!("Failed to empty the clipboard. Got error code: {}", e),
			});
		};

		add_cf_dibv5(open_clipboard, image)
	}
}

fn add_clipboard_exclusions(
	_open_clipboard: OpenClipboard<'_>,
	exclude_from_cloud: bool,
	exclude_from_history: bool,
) -> Result<(), Error> {
	/// `set` should be called with the registered format and a DWORD value of 0.
	///
	/// See https://docs.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#cloud-clipboard-and-clipboard-history-formats
	const CLIPBOARD_EXCLUSION_DATA: &[u8] = &0u32.to_ne_bytes();

	// Clipboard exclusions are applied retroactively to the item that is currently in the clipboard.
	// See the MS docs on `CLIPBOARD_EXCLUSION_DATA` for specifics. Once the item is added to the clipboard,
	// tell Windows to remove it from cloud syncing and history.

	if exclude_from_cloud {
		if let Some(format) = clipboard_win::register_format("CanUploadToCloudClipboard") {
			// We believe that it would be a logic error if this call failed, since we've validated the format is supported,
			// we still have full ownership of the clipboard and aren't moving it to another thread, and this is a well-documented operation.
			// Due to these reasons, `Error::Unknown` is used because we never expect the error path to be taken.
			clipboard_win::raw::set_without_clear(format.get(), CLIPBOARD_EXCLUSION_DATA).map_err(
				|_| Error::Unknown {
					description: "Failed to exclude data from cloud clipboard".into(),
				},
			)?;
		}
	}

	if exclude_from_history {
		if let Some(format) = clipboard_win::register_format("CanIncludeInClipboardHistory") {
			// See above for reasoning about using `Error::Unknown`.
			clipboard_win::raw::set_without_clear(format.get(), CLIPBOARD_EXCLUSION_DATA).map_err(
				|_| Error::Unknown {
					description: "Failed to exclude data from clipboard history".into(),
				},
			)?;
		}
	}

	Ok(())
}

/// Windows-specific extensions to the [`Set`](crate::Set) builder.
pub trait SetExtWindows: private::Sealed {
	/// Excludes the data which will be set on the clipboard from being uploaded to
	/// the Windows 10/11 [cloud clipboard].
	///
	/// [cloud clipboard]: https://support.microsoft.com/en-us/windows/clipboard-in-windows-c436501e-985d-1c8d-97ea-fe46ddf338c6
	fn exclude_from_cloud(self) -> Self;

	/// Excludes the data which will be set on the clipboard from being added to
	/// the system's [clipboard history] list.
	///
	/// [clipboard history]: https://support.microsoft.com/en-us/windows/get-help-with-clipboard-30375039-ce71-9fe4-5b30-21b7aab6b13f
	fn exclude_from_history(self) -> Self;
}

impl SetExtWindows for crate::Set<'_> {
	fn exclude_from_cloud(mut self) -> Self {
		self.platform.exclude_from_cloud = true;
		self
	}

	fn exclude_from_history(mut self) -> Self {
		self.platform.exclude_from_history = true;
		self
	}
}

pub(crate) struct Clear<'clipboard> {
	clipboard: Result<OpenClipboard<'clipboard>, Error>,
}

impl<'clipboard> Clear<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { clipboard: clipboard.open() }
	}

	pub(crate) fn clear(self) -> Result<(), Error> {
		let _clipboard_assertion = self.clipboard?;
		clipboard_win::empty()
			.map_err(|_| Error::Unknown { description: "failed to clear clipboard".into() })
	}
}

fn wrap_html(ctn: &str) -> String {
	let h_version = "Version:0.9";
	let h_start_html = "\r\nStartHTML:";
	let h_end_html = "\r\nEndHTML:";
	let h_start_frag = "\r\nStartFragment:";
	let h_end_frag = "\r\nEndFragment:";
	let c_start_frag = "\r\n<html>\r\n<body>\r\n<!--StartFragment-->\r\n";
	let c_end_frag = "\r\n<!--EndFragment-->\r\n</body>\r\n</html>";
	let h_len = h_version.len()
		+ h_start_html.len()
		+ 10 + h_end_html.len()
		+ 10 + h_start_frag.len()
		+ 10 + h_end_frag.len()
		+ 10;
	let n_start_html = h_len + 2;
	let n_start_frag = h_len + c_start_frag.len();
	let n_end_frag = n_start_frag + ctn.len();
	let n_end_html = n_end_frag + c_end_frag.len();
	format!(
		"{}{}{:010}{}{:010}{}{:010}{}{:010}{}{}{}",
		h_version,
		h_start_html,
		n_start_html,
		h_end_html,
		n_end_html,
		h_start_frag,
		n_start_frag,
		h_end_frag,
		n_end_frag,
		c_start_frag,
		ctn,
		c_end_frag,
	)
}

#[cfg(all(test, feature = "image-data"))]
mod tests {
	use super::{rgba_to_win, win_to_rgba};

	const DATA: [u8; 16] =
		[100, 100, 255, 100, 0, 0, 0, 255, 255, 100, 100, 255, 100, 255, 100, 100];

	#[test]
	fn check_win_to_rgba_conversion() {
		let mut data = DATA;
		unsafe { win_to_rgba(&mut data) };
	}

	#[test]
	fn check_rgba_to_win_conversion() {
		let mut data = DATA;
		unsafe { rgba_to_win(&mut data) };
	}
}
