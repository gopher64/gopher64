use std::borrow::Cow;
use std::convert::TryInto;
use std::io::Read;

use wl_clipboard_rs::{
	copy::{self, Error as CopyError, MimeSource, MimeType, Options, Source},
	paste::{self, get_contents, Error as PasteError, Seat},
	utils::is_primary_selection_supported,
};

#[cfg(feature = "image-data")]
use super::encode_as_png;
use super::{into_unknown, LinuxClipboardKind};
use crate::common::Error;
#[cfg(feature = "image-data")]
use crate::common::ImageData;

#[cfg(feature = "image-data")]
const MIME_PNG: &str = "image/png";

pub(crate) struct Clipboard {}

impl TryInto<copy::ClipboardType> for LinuxClipboardKind {
	type Error = Error;

	fn try_into(self) -> Result<copy::ClipboardType, Self::Error> {
		match self {
			LinuxClipboardKind::Clipboard => Ok(copy::ClipboardType::Regular),
			LinuxClipboardKind::Primary => Ok(copy::ClipboardType::Primary),
			LinuxClipboardKind::Secondary => Err(Error::ClipboardNotSupported),
		}
	}
}

impl TryInto<paste::ClipboardType> for LinuxClipboardKind {
	type Error = Error;

	fn try_into(self) -> Result<paste::ClipboardType, Self::Error> {
		match self {
			LinuxClipboardKind::Clipboard => Ok(paste::ClipboardType::Regular),
			LinuxClipboardKind::Primary => Ok(paste::ClipboardType::Primary),
			LinuxClipboardKind::Secondary => Err(Error::ClipboardNotSupported),
		}
	}
}

impl Clipboard {
	#[allow(clippy::unnecessary_wraps)]
	pub(crate) fn new() -> Result<Self, Error> {
		// Check if it's possible to communicate with the wayland compositor
		if let Err(e) = is_primary_selection_supported() {
			return Err(into_unknown(e));
		}
		Ok(Self {})
	}

	pub(crate) fn get_text(&mut self, selection: LinuxClipboardKind) -> Result<String, Error> {
		use wl_clipboard_rs::paste::MimeType;

		let result = get_contents(selection.try_into()?, Seat::Unspecified, MimeType::Text);
		match result {
			Ok((mut pipe, _)) => {
				let mut contents = vec![];
				pipe.read_to_end(&mut contents).map_err(into_unknown)?;
				String::from_utf8(contents).map_err(|_| Error::ConversionFailure)
			}

			Err(PasteError::ClipboardEmpty) | Err(PasteError::NoMimeType) => {
				Err(Error::ContentNotAvailable)
			}

			Err(PasteError::PrimarySelectionUnsupported) => Err(Error::ClipboardNotSupported),

			Err(err) => Err(Error::Unknown { description: format!("{}", err) }),
		}
	}

	pub(crate) fn set_text(
		&self,
		text: Cow<'_, str>,
		selection: LinuxClipboardKind,
		wait: bool,
	) -> Result<(), Error> {
		let mut opts = Options::new();
		opts.foreground(wait);
		opts.clipboard(selection.try_into()?);
		let source = Source::Bytes(text.into_owned().into_bytes().into_boxed_slice());
		opts.copy(source, MimeType::Text).map_err(|e| match e {
			CopyError::PrimarySelectionUnsupported => Error::ClipboardNotSupported,
			other => into_unknown(other),
		})?;
		Ok(())
	}

	pub(crate) fn set_html(
		&self,
		html: Cow<'_, str>,
		alt: Option<Cow<'_, str>>,
		selection: LinuxClipboardKind,
		wait: bool,
	) -> Result<(), Error> {
		let html_mime = MimeType::Specific(String::from("text/html"));
		let mut opts = Options::new();
		opts.foreground(wait);
		opts.clipboard(selection.try_into()?);
		let html_source = Source::Bytes(html.into_owned().into_bytes().into_boxed_slice());
		match alt {
			Some(alt_text) => {
				let alt_source =
					Source::Bytes(alt_text.into_owned().into_bytes().into_boxed_slice());
				opts.copy_multi(vec![
					MimeSource { source: alt_source, mime_type: MimeType::Text },
					MimeSource { source: html_source, mime_type: html_mime },
				])
			}
			None => opts.copy(html_source, html_mime),
		}
		.map_err(|e| match e {
			CopyError::PrimarySelectionUnsupported => Error::ClipboardNotSupported,
			other => into_unknown(other),
		})?;
		Ok(())
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn get_image(
		&mut self,
		selection: LinuxClipboardKind,
	) -> Result<ImageData<'static>, Error> {
		use std::io::Cursor;
		use wl_clipboard_rs::paste::MimeType;

		let result =
			get_contents(selection.try_into()?, Seat::Unspecified, MimeType::Specific(MIME_PNG));
		match result {
			Ok((mut pipe, _mime_type)) => {
				let mut buffer = vec![];
				pipe.read_to_end(&mut buffer).map_err(into_unknown)?;
				let image = image::io::Reader::new(Cursor::new(buffer))
					.with_guessed_format()
					.map_err(|_| Error::ConversionFailure)?
					.decode()
					.map_err(|_| Error::ConversionFailure)?;
				let image = image.into_rgba8();

				Ok(ImageData {
					width: image.width() as usize,
					height: image.height() as usize,
					bytes: image.into_raw().into(),
				})
			}

			Err(PasteError::ClipboardEmpty) | Err(PasteError::NoMimeType) => {
				Err(Error::ContentNotAvailable)
			}

			Err(err) => Err(Error::Unknown { description: format!("{}", err) }),
		}
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn set_image(
		&mut self,
		image: ImageData,
		selection: LinuxClipboardKind,
		wait: bool,
	) -> Result<(), Error> {
		let image = encode_as_png(&image)?;
		let mut opts = Options::new();
		opts.foreground(wait);
		opts.clipboard(selection.try_into()?);
		let source = Source::Bytes(image.into());
		opts.copy(source, MimeType::Specific(MIME_PNG.into())).map_err(into_unknown)?;
		Ok(())
	}
}
