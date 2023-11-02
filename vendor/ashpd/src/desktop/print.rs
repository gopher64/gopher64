//! # Examples
//!
//! Print a file
//!
//! ```rust,no_run
//! use std::fs::File;
//!
//! use ashpd::{desktop::print::PrintProxy, WindowIdentifier};
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = PrintProxy::new().await?;
//!     let identifier = WindowIdentifier::default();
//!
//!     let file =
//!         File::open("/home/bilelmoussaoui/gitlog.pdf").expect("file to print was not found");
//!     let pre_print = proxy
//!         .prepare_print(
//!             &identifier,
//!             "prepare print",
//!             Default::default(),
//!             Default::default(),
//!             true,
//!         )
//!         .await?
//!         .response()?;
//!     proxy
//!         .print(&identifier, "test", &file, Some(pre_print.token), true)
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

use std::{fmt, os::unix::prelude::AsRawFd, str::FromStr};

use serde::{Deserialize, Serialize};
use zbus::zvariant::{DeserializeDict, Fd, SerializeDict, Type};

use super::{HandleToken, Request};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Type)]
#[zvariant(signature = "s")]
#[serde(rename_all = "snake_case")]
/// The page orientation.
pub enum Orientation {
    /// Landscape.
    Landscape,
    /// Portrait.
    Portrait,
    /// Reverse landscape.
    ReverseLandscape,
    /// Reverse portrait.
    ReversePortrait,
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Landscape => write!(f, "Landscape"),
            Self::Portrait => write!(f, "Portrait"),
            Self::ReverseLandscape => write!(f, "Reverse Landscape"),
            Self::ReversePortrait => write!(f, "Reverse Portrait"),
        }
    }
}

impl AsRef<str> for Orientation {
    fn as_ref(&self) -> &str {
        match self {
            Self::Landscape => "Landscape",
            Self::Portrait => "Portrait",
            Self::ReverseLandscape => "Reverse Landscape",
            Self::ReversePortrait => "Reverse Portrait",
        }
    }
}

impl From<Orientation> for &'static str {
    fn from(o: Orientation) -> Self {
        match o {
            Orientation::Landscape => "Landscape",
            Orientation::Portrait => "Portrait",
            Orientation::ReverseLandscape => "Reverse Landscape",
            Orientation::ReversePortrait => "Reverse Portrait",
        }
    }
}

impl FromStr for Orientation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Landscape" | "landscape" => Ok(Orientation::Landscape),
            "Portrait" | "portrait" => Ok(Orientation::Portrait),
            "ReverseLandscape" | "reverse_landscape" => Ok(Orientation::ReverseLandscape),
            "ReversePortrait" | "reverse_portrait" => Ok(Orientation::ReversePortrait),
            _ => Err(Error::ParseError(
                "Failed to parse orientation, invalid value",
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Type)]
#[zvariant(signature = "s")]
#[serde(rename_all = "lowercase")]
/// The print quality.
pub enum Quality {
    /// Draft quality.
    Draft,
    /// Low quality.
    Low,
    /// Normal quality.
    Normal,
    /// High quality.
    High,
}

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::Low => write!(f, "Low"),
            Self::Normal => write!(f, "Normal"),
            Self::High => write!(f, "High"),
        }
    }
}

impl AsRef<str> for Quality {
    fn as_ref(&self) -> &str {
        match self {
            Self::Draft => "Draft",
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
        }
    }
}

impl From<Quality> for &'static str {
    fn from(q: Quality) -> Self {
        match q {
            Quality::Draft => "Draft",
            Quality::Low => "Low",
            Quality::Normal => "Normal",
            Quality::High => "High",
        }
    }
}

impl FromStr for Quality {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" | "draft" => Ok(Quality::Draft),
            "Low" | "low" => Ok(Quality::Low),
            "Normal" | "normal" => Ok(Quality::Normal),
            "High" | "high" => Ok(Quality::High),
            _ => Err(Error::ParseError("Failed to parse quality, invalid value")),
        }
    }
}

#[derive(SerializeDict, DeserializeDict, Type, Debug, Default)]
/// Print settings to set in the print dialog.
#[zvariant(signature = "dict")]
pub struct Settings {
    /// One of landscape, portrait, reverse_landscape or reverse_portrait.
    pub orientation: Option<Orientation>,
    /// A paper name according to [PWG 5101.1-2002](ftp://ftp.pwg.org/pub/pwg/candidates/cs-pwgmsn10-20020226-5101.1.pdf)
    #[zvariant(rename = "paper-format")]
    pub paper_format: Option<String>,
    /// Paper width, in millimeters.
    #[zvariant(rename = "paper-width")]
    pub paper_width: Option<String>,
    /// Paper height, in millimeters.
    #[zvariant(rename = "paper-height")]
    pub paper_height: Option<String>,
    /// The number of copies to print.
    #[zvariant(rename = "n-copies")]
    pub n_copies: Option<String>,
    /// The default paper source.
    #[zvariant(rename = "default-source")]
    pub default_source: Option<String>,
    /// Print quality.
    pub quality: Option<Quality>,
    /// The resolution, sets both resolution-x & resolution-y
    pub resolution: Option<String>,
    /// Whether to use color.
    #[zvariant(rename = "use-color")]
    pub use_color: Option<bool>,
    /// Duplex printing mode, one of simplex, horizontal or vertical.
    pub duplex: Option<String>,
    /// Whether to collate copies.
    pub collate: Option<String>,
    /// Whether to reverse the order of printed pages.
    pub reverse: Option<String>,
    /// A media type according to [PWG 5101.1-2002](ftp://ftp.pwg.org/pub/pwg/candidates/cs-pwgmsn10-20020226-5101.1.pdf)
    #[zvariant(rename = "media-type")]
    pub media_type: Option<String>,
    /// The dithering to use, one of fine, none, coarse, lineart, grayscale or
    /// error-diffusion.
    pub dither: Option<String>,
    /// The scale in percent
    pub scale: Option<String>,
    /// What pages to print, one of all, selection, current or ranges.
    #[zvariant(rename = "print-pages")]
    pub print_pages: Option<String>,
    /// A list of page ranges, formatted like this: 0-2,4,9-11.
    #[zvariant(rename = "page-ranges")]
    pub page_ranges: Option<String>,
    /// What pages to print, one of all, even or odd.
    #[zvariant(rename = "page-set")]
    pub page_set: Option<String>,
    /// The finishings.
    pub finishings: Option<String>,
    /// The number of pages per sheet.
    #[zvariant(rename = "number-up")]
    pub number_up: Option<String>,
    /// One of lrtb, lrbt, rltb, rlbt, tblr, tbrl, btlr, btrl.
    #[zvariant(rename = "number-up-layout")]
    pub number_up_layout: Option<String>,
    #[zvariant(rename = "output-bin")]
    /// The output bin.
    pub output_bin: Option<String>,
    /// The horizontal resolution in dpi.
    #[zvariant(rename = "resolution-x")]
    pub resolution_x: Option<String>,
    /// The vertical resolution in dpi.
    #[zvariant(rename = "resolution-y")]
    pub resolution_y: Option<String>,
    /// The resolution in lpi (lines per inch).
    #[zvariant(rename = "printer-lpi")]
    pub print_lpi: Option<String>,
    /// Basename to use for print-to-file.
    #[zvariant(rename = "output-basename")]
    pub output_basename: Option<String>,
    /// Format to use for print-to-file, one of PDF, PS, SVG
    #[zvariant(rename = "output-file-format")]
    pub output_file_format: Option<String>,
    /// The uri used for print-to file.
    #[zvariant(rename = "output-uri")]
    pub output_uri: Option<url::Url>,
}

impl Settings {
    /// Sets the orientation.
    #[must_use]
    pub fn orientation(mut self, orientation: impl Into<Option<Orientation>>) -> Self {
        self.orientation = orientation.into();
        self
    }

    /// Sets the paper name.
    #[must_use]
    pub fn paper_format<'a>(mut self, paper_format: impl Into<Option<&'a str>>) -> Self {
        self.paper_format = paper_format.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the paper width.
    #[must_use]
    pub fn paper_width<'a>(mut self, paper_width: impl Into<Option<&'a str>>) -> Self {
        self.paper_width = paper_width.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the paper height.
    #[must_use]
    pub fn paper_height<'a>(mut self, paper_height: impl Into<Option<&'a str>>) -> Self {
        self.paper_height = paper_height.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the number of copies to print.
    #[must_use]
    pub fn n_copies<'a>(mut self, n_copies: impl Into<Option<&'a str>>) -> Self {
        self.n_copies = n_copies.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the default paper source.
    #[must_use]
    pub fn default_source<'a>(mut self, default_source: impl Into<Option<&'a str>>) -> Self {
        self.default_source = default_source.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the print quality.
    #[must_use]
    pub fn quality(mut self, quality: impl Into<Option<Quality>>) -> Self {
        self.quality = quality.into();
        self
    }

    /// Sets the resolution, both resolution-x & resolution-y.
    #[must_use]
    pub fn resolution<'a>(mut self, resolution: impl Into<Option<&'a str>>) -> Self {
        self.resolution = resolution.into().map(ToOwned::to_owned);
        self
    }

    /// Sets whether to use color.
    #[must_use]
    pub fn use_color(mut self, use_color: impl Into<Option<bool>>) -> Self {
        self.use_color = use_color.into();
        self
    }

    /// Sets the duplex printing mode.
    #[must_use]
    pub fn duplex<'a>(mut self, duplex: impl Into<Option<&'a str>>) -> Self {
        self.duplex = duplex.into().map(ToOwned::to_owned);
        self
    }

    /// Whether to collate copies.
    #[must_use]
    pub fn collate<'a>(mut self, collate: impl Into<Option<&'a str>>) -> Self {
        self.collate = collate.into().map(ToOwned::to_owned);
        self
    }

    /// Sets whether to reverse the order of the printed pages.
    #[must_use]
    pub fn reverse<'a>(mut self, reverse: impl Into<Option<&'a str>>) -> Self {
        self.reverse = reverse.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the media type.
    #[must_use]
    pub fn media_type<'a>(mut self, media_type: impl Into<Option<&'a str>>) -> Self {
        self.media_type = media_type.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the dithering to use.
    #[must_use]
    pub fn dither<'a>(mut self, dither: impl Into<Option<&'a str>>) -> Self {
        self.dither = dither.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the page scale in percent.
    #[must_use]
    pub fn scale<'a>(mut self, scale: impl Into<Option<&'a str>>) -> Self {
        self.scale = scale.into().map(ToOwned::to_owned);
        self
    }

    /// Sets what pages to print, one of all, selection, current or ranges.
    #[must_use]
    pub fn print_pages<'a>(mut self, print_pages: impl Into<Option<&'a str>>) -> Self {
        self.print_pages = print_pages.into().map(ToOwned::to_owned);
        self
    }

    /// Sets a list of page ranges, formatted like this: 0-2,4,9-11.
    #[must_use]
    pub fn page_ranges<'a>(mut self, page_ranges: impl Into<Option<&'a str>>) -> Self {
        self.page_ranges = page_ranges.into().map(ToOwned::to_owned);
        self
    }

    /// Sets what pages to print, one of all, even or odd.
    #[must_use]
    pub fn page_set<'a>(mut self, page_set: impl Into<Option<&'a str>>) -> Self {
        self.page_set = page_set.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the finishings.
    #[must_use]
    pub fn finishings<'a>(mut self, finishings: impl Into<Option<&'a str>>) -> Self {
        self.finishings = finishings.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the number of pages per sheet.
    #[must_use]
    pub fn number_up<'a>(mut self, number_up: impl Into<Option<&'a str>>) -> Self {
        self.number_up = number_up.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the number up layout, one of lrtb, lrbt, rltb, rlbt, tblr, tbrl,
    /// btlr, btrl.
    #[must_use]
    pub fn number_up_layout<'a>(mut self, number_up_layout: impl Into<Option<&'a str>>) -> Self {
        self.number_up_layout = number_up_layout.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the output bin
    #[must_use]
    pub fn output_bin<'a>(mut self, output_bin: impl Into<Option<&'a str>>) -> Self {
        self.output_bin = output_bin.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the horizontal resolution in dpi.
    #[must_use]
    pub fn resolution_x<'a>(mut self, resolution_x: impl Into<Option<&'a str>>) -> Self {
        self.resolution_x = resolution_x.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the vertical resolution in dpi.
    #[must_use]
    pub fn resolution_y<'a>(mut self, resolution_y: impl Into<Option<&'a str>>) -> Self {
        self.resolution_y = resolution_y.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the resolution in lines per inch.
    #[must_use]
    pub fn print_lpi<'a>(mut self, print_lpi: impl Into<Option<&'a str>>) -> Self {
        self.print_lpi = print_lpi.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the print-to-file base name.
    #[must_use]
    pub fn output_basename<'a>(mut self, output_basename: impl Into<Option<&'a str>>) -> Self {
        self.output_basename = output_basename.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the print-to-file format, one of PS, PDF, SVG.
    #[must_use]
    pub fn output_file_format<'a>(
        mut self,
        output_file_format: impl Into<Option<&'a str>>,
    ) -> Self {
        self.output_file_format = output_file_format.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the print-to-file output uri.
    #[must_use]
    pub fn output_uri<'a>(mut self, output_uri: impl Into<Option<&'a url::Url>>) -> Self {
        self.output_uri = output_uri.into().map(ToOwned::to_owned);
        self
    }
}

#[derive(SerializeDict, DeserializeDict, Type, Debug, Default)]
/// Setup the printed pages.
#[zvariant(signature = "dict")]
pub struct PageSetup {
    /// the PPD name. It's the name to select a given driver.
    #[zvariant(rename = "PPDName")]
    pub ppdname: Option<String>,
    /// The name of the page setup.
    pub name: Option<String>,
    /// The user-visible name of the page setup.
    pub display_name: Option<String>,
    /// Paper width in millimeters.
    pub width: Option<f64>,
    /// Paper height in millimeters.
    pub height: Option<f64>,
    /// Top margin in millimeters.
    pub margin_top: Option<f64>,
    /// Bottom margin in millimeters.
    pub margin_bottom: Option<f64>,
    /// Right margin in millimeters.
    pub margin_right: Option<f64>,
    /// Left margin in millimeters.
    pub margin_left: Option<f64>,
    /// The page orientation.
    pub orientation: Option<Orientation>,
}

impl PageSetup {
    /// Sets the ppdname.
    #[must_use]
    pub fn ppdname<'a>(mut self, ppdname: impl Into<Option<&'a str>>) -> Self {
        self.ppdname = ppdname.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the name of the page setup.
    #[must_use]
    pub fn name<'a>(mut self, name: impl Into<Option<&'a str>>) -> Self {
        self.name = name.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the user visible name of the page setup.
    #[must_use]
    pub fn display_name<'a>(mut self, display_name: impl Into<Option<&'a str>>) -> Self {
        self.display_name = display_name.into().map(ToOwned::to_owned);
        self
    }

    /// Sets the orientation.
    #[must_use]
    pub fn orientation(mut self, orientation: impl Into<Option<Orientation>>) -> Self {
        self.orientation = orientation.into();
        self
    }

    /// Sets the page width.
    #[must_use]
    pub fn width(mut self, width: impl Into<Option<f64>>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the page height.
    #[must_use]
    pub fn height(mut self, height: impl Into<Option<f64>>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the page top margin.
    #[must_use]
    pub fn margin_top(mut self, margin_top: impl Into<Option<f64>>) -> Self {
        self.margin_top = margin_top.into();
        self
    }

    /// Sets the page bottom margin.
    #[must_use]
    pub fn margin_bottom(mut self, margin_bottom: impl Into<Option<f64>>) -> Self {
        self.margin_bottom = margin_bottom.into();
        self
    }

    /// Sets the page right margin.
    #[must_use]
    pub fn margin_right(mut self, margin_right: impl Into<Option<f64>>) -> Self {
        self.margin_right = margin_right.into();
        self
    }

    /// Sets the page margin left.
    #[must_use]
    pub fn margin_left(mut self, margin_left: impl Into<Option<f64>>) -> Self {
        self.margin_left = margin_left.into();
        self
    }
}

#[derive(SerializeDict, Type, Debug, Default)]
/// Specified options for a [`PrintProxy::prepare_print`] request.
#[zvariant(signature = "dict")]
struct PreparePrintOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
    /// Whether to make the dialog modal.
    modal: Option<bool>,
}

impl PreparePrintOptions {
    /// Sets whether the dialog should be a modal.
    pub fn modal(mut self, modal: impl Into<Option<bool>>) -> Self {
        self.modal = modal.into();
        self
    }
}

#[derive(SerializeDict, Type, Debug, Default)]
/// Specified options for a [`PrintProxy::print`] request.
#[zvariant(signature = "dict")]
struct PrintOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
    /// Whether to make the dialog modal.
    modal: Option<bool>,
    /// Token that was returned by a previous [`PrintProxy::prepare_print`]
    /// call.
    token: Option<u32>,
}

impl PrintOptions {
    /// A token retrieved from [`PrintProxy::prepare_print`].
    pub fn token(mut self, token: impl Into<Option<u32>>) -> Self {
        self.token = token.into();
        self
    }

    /// Sets whether the dialog should be a modal.
    pub fn modal(mut self, modal: impl Into<Option<bool>>) -> Self {
        self.modal = modal.into();
        self
    }
}

#[derive(DeserializeDict, Type, Debug)]
/// A response to a [`PrintProxy::prepare_print`] request.
#[zvariant(signature = "dict")]
pub struct PreparePrint {
    /// The printing settings.
    pub settings: Settings,
    #[zvariant(rename = "page-setup")]
    /// The printed pages setup.
    pub page_setup: PageSetup,
    /// A token to pass to the print request.
    pub token: u32,
}

/// The interface lets sandboxed applications print.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Print`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Print).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Print")]
pub struct PrintProxy<'a>(Proxy<'a>);

impl<'a> PrintProxy<'a> {
    /// Create a new instance of [`PrintProxy`].
    pub async fn new() -> Result<PrintProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Print").await?;
        Ok(Self(proxy))
    }

    /// Presents a print dialog to the user and returns print settings and page
    /// setup.
    ///
    /// # Arguments
    ///
    /// * `identifier` - Identifier for the application window.
    /// * `title` - Title for the print dialog.
    /// * `settings` - [`Settings`].
    /// * `page_setup` - [`PageSetup`].
    /// * `modal` - Whether the dialog should be a modal.
    ///
    /// # Specifications
    ///
    /// See also [`PreparePrint`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Print.PreparePrint).
    #[doc(alias = "PreparePrint")]
    #[doc(alias = "xdp_portal_prepare_print")]
    pub async fn prepare_print(
        &self,
        identifier: &WindowIdentifier,
        title: &str,
        settings: Settings,
        page_setup: PageSetup,
        modal: bool,
    ) -> Result<Request<PreparePrint>, Error> {
        let options = PreparePrintOptions::default().modal(modal);
        self.0
            .request(
                &options.handle_token,
                "PreparePrint",
                &(&identifier, title, settings, page_setup, &options),
            )
            .await
    }

    /// Asks to print a file.
    /// The file must be passed in the form of a file descriptor open for
    /// reading. This ensures that sandboxed applications only print files
    /// that they have access to.
    ///
    /// # Arguments
    ///
    /// * `identifier` - The application window identifier.
    /// * `title` - The title for the print dialog.
    /// * `fd` - File descriptor for reading the content to print.
    /// * `token` - A token returned by a call to
    ///   [`prepare_print()`][`PrintProxy::prepare_print`].
    /// * `modal` - Whether the dialog should be a modal.
    ///
    /// # Specifications
    ///
    /// See also [`Print`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-Print.Print).
    #[doc(alias = "Print")]
    #[doc(alias = "xdp_portal_print_file")]
    pub async fn print(
        &self,
        identifier: &WindowIdentifier,
        title: &str,
        fd: &impl AsRawFd,
        token: Option<u32>,
        modal: bool,
    ) -> Result<Request<()>, Error> {
        let options = PrintOptions::default()
            .token(token.unwrap_or(0))
            .modal(modal);
        self.0
            .empty_request(
                &options.handle_token,
                "Print",
                &(&identifier, title, Fd::from(fd.as_raw_fd()), &options),
            )
            .await
    }
}
