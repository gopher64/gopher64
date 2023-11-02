//! Start a screencast session and get the PipeWire remote of it.
//!
//! # Examples
//!
//! How to create a screen cast session & start it.
//! The portal is currently useless without PipeWire & Rust support.
//!
//! ```rust,no_run
//! use ashpd::{
//!     desktop::screencast::{CursorMode, PersistMode, Screencast, SourceType},
//!     WindowIdentifier,
//! };
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = Screencast::new().await?;
//!     let session = proxy.create_session().await?;
//!     proxy
//!         .select_sources(
//!             &session,
//!             CursorMode::Metadata,
//!             SourceType::Monitor | SourceType::Window,
//!             true,
//!             None,
//!             PersistMode::DoNot,
//!         )
//!         .await?;
//!
//!     let response = proxy
//!         .start(&session, &WindowIdentifier::default())
//!         .await?
//!         .response()?;
//!     response.streams().iter().for_each(|stream| {
//!         println!("node id: {}", stream.pipe_wire_node_id());
//!         println!("size: {:?}", stream.size());
//!         println!("position: {:?}", stream.position());
//!     });
//!     Ok(())
//! }
//! ```

use std::{
    collections::HashMap,
    fmt::Debug,
    os::unix::prelude::{IntoRawFd, RawFd},
};

use enumflags2::{bitflags, BitFlags};
use futures_util::TryFutureExt;
use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};
use zbus::zvariant::{DeserializeDict, OwnedFd, SerializeDict, Type, Value};

use super::{HandleToken, Request, Session};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[bitflags]
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Copy, Clone, Debug, Type)]
#[repr(u32)]
#[doc(alias = "XdpOutputType")]
/// A bit flag for the available sources to record.
pub enum SourceType {
    #[doc(alias = "XDP_OUTPUT_MONITOR")]
    /// A monitor.
    Monitor,
    #[doc(alias = "XDP_OUTPUT_WINDOW")]
    /// A specific window
    Window,
    #[doc(alias = "XDP_OUTPUT_VIRTUAL")]
    /// Virtual
    Virtual,
}

#[bitflags]
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Debug, Copy, Clone, Type)]
#[repr(u32)]
#[doc(alias = "XdpCursorMode")]
/// A bit flag for the possible cursor modes.
pub enum CursorMode {
    #[doc(alias = "XDP_CURSOR_MODE_HIDDEN")]
    /// The cursor is not part of the screen cast stream.
    Hidden,
    #[doc(alias = "XDP_CURSOR_MODE_EMBEDDED")]
    /// The cursor is embedded as part of the stream buffers.
    Embedded,
    #[doc(alias = "XDP_CURSOR_MODE_METADATA")]
    /// The cursor is not part of the screen cast stream, but sent as PipeWire
    /// stream metadata.
    Metadata,
}

#[derive(Default, Serialize_repr, PartialEq, Eq, Debug, Copy, Clone, Type)]
#[doc(alias = "XdpPersistMode")]
#[repr(u32)]
/// Persistence mode for a screencast session.
pub enum PersistMode {
    #[doc(alias = "XDP_PERSIST_MODE_NONE")]
    #[default]
    /// Do not persist.
    DoNot = 0,
    #[doc(alias = "XDP_PERSIST_MODE_TRANSIENT")]
    /// Persist while the application is running.
    Application = 1,
    #[doc(alias = "XDP_PERSIST_MODE_PERSISTENT")]
    /// Persist until explicitly revoked.
    ExplicitlyRevoked = 2,
}

#[derive(SerializeDict, Type, Debug, Default)]
/// Specified options for a [`Screencast::create_session`] request.
#[zvariant(signature = "dict")]
struct CreateSessionOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
    /// A string that will be used as the last element of the session handle.
    session_handle_token: HandleToken,
}

#[derive(SerializeDict, Type, Debug, Default)]
/// Specified options for a [`Screencast::select_sources`] request.
#[zvariant(signature = "dict")]
struct SelectSourcesOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
    /// What types of content to record.
    types: Option<BitFlags<SourceType>>,
    /// Whether to allow selecting multiple sources.
    multiple: Option<bool>,
    /// Determines how the cursor will be drawn in the screen cast stream.
    cursor_mode: Option<CursorMode>,
    restore_token: Option<String>,
    persist_mode: Option<PersistMode>,
}

impl SelectSourcesOptions {
    /// Sets whether to allow selecting multiple sources.
    #[must_use]
    pub fn multiple(mut self, multiple: impl Into<Option<bool>>) -> Self {
        self.multiple = multiple.into();
        self
    }

    /// Sets how the cursor will be drawn on the screen cast stream.
    #[must_use]
    pub fn cursor_mode(mut self, cursor_mode: impl Into<Option<CursorMode>>) -> Self {
        self.cursor_mode = cursor_mode.into();
        self
    }

    /// Sets the types of content to record.
    #[must_use]
    pub fn types(mut self, types: impl Into<Option<BitFlags<SourceType>>>) -> Self {
        self.types = types.into();
        self
    }

    #[must_use]
    pub fn persist_mode(mut self, persist_mode: impl Into<Option<PersistMode>>) -> Self {
        self.persist_mode = persist_mode.into();
        self
    }

    #[must_use]
    pub fn restore_token<'a>(mut self, token: impl Into<Option<&'a str>>) -> Self {
        self.restore_token = token.into().map(ToOwned::to_owned);
        self
    }
}

#[derive(SerializeDict, Type, Debug, Default)]
/// Specified options for a [`Screencast::start`] request.
#[zvariant(signature = "dict")]
struct StartCastOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
}

#[derive(DeserializeDict, Type, Debug)]
/// A response to a [`Screencast::create_session`] request.
#[zvariant(signature = "dict")]
struct CreateSession {
    // TODO: investigate why this doesn't return an ObjectPath
    // replace with an ObjectPath once https://github.com/flatpak/xdg-desktop-portal/pull/609's merged
    /// A string that will be used as the last element of the session handle.
    session_handle: String,
}

#[derive(DeserializeDict, Type)]
/// A response to a [`Screencast::start`] request.
#[zvariant(signature = "dict")]
pub struct Streams {
    streams: Vec<Stream>,
    restore_token: Option<String>,
}

impl Streams {
    /// The session restore token.
    pub fn restore_token(&self) -> Option<&str> {
        self.restore_token.as_deref()
    }

    /// The list of streams.
    pub fn streams(&self) -> &[Stream] {
        &self.streams
    }
}

impl Debug for Streams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Streams")
            .field(&self.restore_token)
            .field(&self.streams)
            .finish()
    }
}

#[derive(Clone, Deserialize, Type)]
/// A PipeWire stream.
pub struct Stream(u32, StreamProperties);

impl Stream {
    /// The PipeWire stream Node ID
    pub fn pipe_wire_node_id(&self) -> u32 {
        self.0
    }

    /// A tuple consisting of the position (x, y) in the compositor coordinate
    /// space.
    ///
    /// **Note** the position may not be equivalent to a position in a pixel
    /// coordinate space. Only available for monitor streams.
    pub fn position(&self) -> Option<(i32, i32)> {
        self.1.position
    }

    /// A tuple consisting of (width, height).
    /// The size represents the size of the stream as it is displayed in the
    /// compositor coordinate space.
    ///
    /// **Note** the size may not be equivalent to a size in a pixel coordinate
    /// space. The size may differ from the size of the stream.
    pub fn size(&self) -> Option<(i32, i32)> {
        self.1.size
    }

    /// The source type of the stream.
    pub fn source_type(&self) -> Option<SourceType> {
        self.1.source_type
    }

    /// The stream identifier.
    pub fn id(&self) -> Option<&str> {
        self.1.id.as_deref()
    }

    // TODO Added in version 5 of the interface.
    /// The stream mapping id.
    pub fn mapping_id(&self) -> Option<&str> {
        self.1.mapping_id.as_deref()
    }
}

impl Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("pipewire_node_id", &self.pipe_wire_node_id())
            .field("position", &self.position())
            .field("size", &self.size())
            .field("source_type", &self.source_type())
            .field("id", &self.id())
            .finish()
    }
}
#[derive(Clone, DeserializeDict, Type, Debug)]
/// The stream properties.
#[zvariant(signature = "dict")]
struct StreamProperties {
    id: Option<String>,
    position: Option<(i32, i32)>,
    size: Option<(i32, i32)>,
    source_type: Option<SourceType>,
    mapping_id: Option<String>,
}

/// The interface lets sandboxed applications create screen cast sessions.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.ScreenCast`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.ScreenCast).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.ScreenCast")]
pub struct Screencast<'a>(Proxy<'a>);

impl<'a> Screencast<'a> {
    /// Create a new instance of [`Screencast`].
    pub async fn new() -> Result<Screencast<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.ScreenCast").await?;
        Ok(Self(proxy))
    }

    /// Create a screen cast session.
    ///
    /// # Specifications
    ///
    /// See also [`CreateSession`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-ScreenCast.CreateSession).
    #[doc(alias = "CreateSession")]
    #[doc(alias = "xdp_portal_create_screencast_session")]
    pub async fn create_session(&self) -> Result<Session<'a>, Error> {
        let options = CreateSessionOptions::default();
        let (request, proxy) = futures_util::try_join!(
            self.0
                .request::<CreateSession>(&options.handle_token, "CreateSession", &options)
                .into_future(),
            Session::from_unique_name(&options.session_handle_token).into_future(),
        )?;
        assert_eq!(proxy.path().as_str(), &request.response()?.session_handle);
        Ok(proxy)
    }

    /// Open a file descriptor to the PipeWire remote where the screen cast
    /// streams are available.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`Session`], created with
    ///   [`create_session()`][`Screencast::create_session`].
    ///
    /// # Returns
    ///
    /// File descriptor of an open PipeWire remote.
    ///
    /// # Specifications
    ///
    /// See also [`OpenPipeWireRemote`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-ScreenCast.OpenPipeWireRemote).
    #[doc(alias = "OpenPipeWireRemote")]
    pub async fn open_pipe_wire_remote(&self, session: &Session<'_>) -> Result<RawFd, Error> {
        // `options` parameter doesn't seems to be used yet
        // see https://github.com/flatpak/xdg-desktop-portal/blob/master/src/screen-cast.c#L812
        let options: HashMap<&str, Value<'_>> = HashMap::new();
        let fd = self
            .0
            .call::<OwnedFd>("OpenPipeWireRemote", &(session, options))
            .await?;
        Ok(fd.into_raw_fd())
    }

    /// Configure what the screen cast session should record.
    /// This method must be called before starting the session.
    ///
    /// Passing invalid input to this method will cause the session to be
    /// closed. An application may only attempt to select sources once per
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`Session`], created with
    ///   [`create_session()`][`Screencast::create_session`].
    /// * `cursor_mode` - Sets how the cursor will be drawn on the screen cast
    ///   stream.
    /// * `types` - Sets the types of content to record.
    /// * `multiple`- Sets whether to allow selecting multiple sources.
    ///
    /// # Specifications
    ///
    /// See also [`SelectSources`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-ScreenCast.SelectSources).
    #[doc(alias = "SelectSources")]
    pub async fn select_sources(
        &self,
        session: &Session<'_>,
        cursor_mode: CursorMode,
        types: BitFlags<SourceType>,
        multiple: bool,
        restore_token: Option<&str>,
        persist_mode: PersistMode,
    ) -> Result<Request<()>, Error> {
        let options = SelectSourcesOptions::default()
            .cursor_mode(cursor_mode)
            .multiple(multiple)
            .types(types)
            .persist_mode(persist_mode)
            .restore_token(restore_token);
        self.0
            .empty_request(&options.handle_token, "SelectSources", &(session, &options))
            .await
    }

    /// Start the screen cast session.
    ///
    /// This will typically result the portal presenting a dialog letting the
    /// user do the selection set up by `select_sources`.
    ///
    /// An application can only attempt start a session once.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`Session`], created with
    ///   [`create_session()`][`Screencast::create_session`].
    /// * `identifier` - Identifier for the application window.
    ///
    /// # Return
    ///
    /// A list of [`Stream`] and an optional restore token.
    ///
    /// # Specifications
    ///
    /// See also [`Start`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-ScreenCast.Start).
    #[doc(alias = "Start")]
    pub async fn start(
        &self,
        session: &Session<'_>,
        identifier: &WindowIdentifier,
    ) -> Result<Request<Streams>, Error> {
        let options = StartCastOptions::default();
        self.0
            .request(
                &options.handle_token,
                "Start",
                &(session, &identifier, &options),
            )
            .await
    }

    /// Available cursor mode.
    ///
    /// # Specifications
    ///
    /// See also [`AvailableCursorModes`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-property-org-freedesktop-portal-ScreenCast.AvailableCursorModes).
    #[doc(alias = "AvailableCursorModes")]
    pub async fn available_cursor_modes(&self) -> Result<BitFlags<CursorMode>, Error> {
        self.0.property_versioned("AvailableCursorModes", 2).await
    }

    /// Available source types.
    ///
    /// # Specifications
    ///
    /// See also [`AvailableSourceTypes`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-property-org-freedesktop-portal-ScreenCast.AvailableSourceTypes).
    #[doc(alias = "AvailableSourceTypes")]
    pub async fn available_source_types(&self) -> Result<BitFlags<SourceType>, Error> {
        self.0.property("AvailableSourceTypes").await
    }
}
