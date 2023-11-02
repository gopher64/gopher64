// This file contains generated code. Do not edit directly.
// To regenerate this, run 'make'.

//! Bindings to the `Composite` X11 extension.

#![allow(clippy::too_many_arguments)]

#[allow(unused_imports)]
use std::borrow::Cow;
#[allow(unused_imports)]
use std::convert::TryInto;
#[allow(unused_imports)]
use crate::utils::RawFdContainer;
#[allow(unused_imports)]
use crate::x11_utils::{Request, RequestHeader, Serialize, TryParse, TryParseFd};
use std::io::IoSlice;
use crate::connection::RequestConnection;
#[allow(unused_imports)]
use crate::connection::Connection as X11Connection;
#[allow(unused_imports)]
use crate::cookie::{Cookie, CookieWithFds, VoidCookie};
use crate::errors::ConnectionError;
#[allow(unused_imports)]
use crate::errors::ReplyOrIdError;
#[allow(unused_imports)]
use super::xfixes;
#[allow(unused_imports)]
use super::xproto;

pub use x11rb_protocol::protocol::composite::*;

/// Get the major opcode of this extension
fn major_opcode<Conn: RequestConnection + ?Sized>(conn: &Conn) -> Result<u8, ConnectionError> {
    let info = conn.extension_information(X11_EXTENSION_NAME)?;
    let info = info.ok_or(ConnectionError::UnsupportedExtension)?;
    Ok(info.major_opcode)
}

pub fn query_version<Conn>(conn: &Conn, client_major_version: u32, client_minor_version: u32) -> Result<Cookie<'_, Conn, QueryVersionReply>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = QueryVersionRequest {
        client_major_version,
        client_minor_version,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_with_reply(&slices, fds)
}

pub fn redirect_window<Conn>(conn: &Conn, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = RedirectWindowRequest {
        window,
        update,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn redirect_subwindows<Conn>(conn: &Conn, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = RedirectSubwindowsRequest {
        window,
        update,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn unredirect_window<Conn>(conn: &Conn, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = UnredirectWindowRequest {
        window,
        update,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn unredirect_subwindows<Conn>(conn: &Conn, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = UnredirectSubwindowsRequest {
        window,
        update,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn create_region_from_border_clip<Conn>(conn: &Conn, region: xfixes::Region, window: xproto::Window) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = CreateRegionFromBorderClipRequest {
        region,
        window,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn name_window_pixmap<Conn>(conn: &Conn, window: xproto::Window, pixmap: xproto::Pixmap) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = NameWindowPixmapRequest {
        window,
        pixmap,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn get_overlay_window<Conn>(conn: &Conn, window: xproto::Window) -> Result<Cookie<'_, Conn, GetOverlayWindowReply>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = GetOverlayWindowRequest {
        window,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_with_reply(&slices, fds)
}

pub fn release_overlay_window<Conn>(conn: &Conn, window: xproto::Window) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = ReleaseOverlayWindowRequest {
        window,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

/// Extension trait defining the requests of this extension.
pub trait ConnectionExt: RequestConnection {
    fn composite_query_version(&self, client_major_version: u32, client_minor_version: u32) -> Result<Cookie<'_, Self, QueryVersionReply>, ConnectionError>
    {
        query_version(self, client_major_version, client_minor_version)
    }
    fn composite_redirect_window(&self, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        redirect_window(self, window, update)
    }
    fn composite_redirect_subwindows(&self, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        redirect_subwindows(self, window, update)
    }
    fn composite_unredirect_window(&self, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        unredirect_window(self, window, update)
    }
    fn composite_unredirect_subwindows(&self, window: xproto::Window, update: Redirect) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        unredirect_subwindows(self, window, update)
    }
    fn composite_create_region_from_border_clip(&self, region: xfixes::Region, window: xproto::Window) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        create_region_from_border_clip(self, region, window)
    }
    fn composite_name_window_pixmap(&self, window: xproto::Window, pixmap: xproto::Pixmap) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        name_window_pixmap(self, window, pixmap)
    }
    fn composite_get_overlay_window(&self, window: xproto::Window) -> Result<Cookie<'_, Self, GetOverlayWindowReply>, ConnectionError>
    {
        get_overlay_window(self, window)
    }
    fn composite_release_overlay_window(&self, window: xproto::Window) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        release_overlay_window(self, window)
    }
}

impl<C: RequestConnection + ?Sized> ConnectionExt for C {}
