// This file contains generated code. Do not edit directly.
// To regenerate this, run 'make'.

//! Bindings to the `Damage` X11 extension.

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

pub use x11rb_protocol::protocol::damage::*;

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

pub fn create<Conn>(conn: &Conn, damage: Damage, drawable: xproto::Drawable, level: ReportLevel) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = CreateRequest {
        damage,
        drawable,
        level,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn destroy<Conn>(conn: &Conn, damage: Damage) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = DestroyRequest {
        damage,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn subtract<Conn, A, B>(conn: &Conn, damage: Damage, repair: A, parts: B) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
    A: Into<xfixes::Region>,
    B: Into<xfixes::Region>,
{
    let repair: xfixes::Region = repair.into();
    let parts: xfixes::Region = parts.into();
    let request0 = SubtractRequest {
        damage,
        repair,
        parts,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn add<Conn>(conn: &Conn, drawable: xproto::Drawable, region: xfixes::Region) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = AddRequest {
        drawable,
        region,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

/// Extension trait defining the requests of this extension.
pub trait ConnectionExt: RequestConnection {
    fn damage_query_version(&self, client_major_version: u32, client_minor_version: u32) -> Result<Cookie<'_, Self, QueryVersionReply>, ConnectionError>
    {
        query_version(self, client_major_version, client_minor_version)
    }
    fn damage_create(&self, damage: Damage, drawable: xproto::Drawable, level: ReportLevel) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        create(self, damage, drawable, level)
    }
    fn damage_destroy(&self, damage: Damage) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        destroy(self, damage)
    }
    fn damage_subtract<A, B>(&self, damage: Damage, repair: A, parts: B) -> Result<VoidCookie<'_, Self>, ConnectionError>
    where
        A: Into<xfixes::Region>,
        B: Into<xfixes::Region>,
    {
        subtract(self, damage, repair, parts)
    }
    fn damage_add(&self, drawable: xproto::Drawable, region: xfixes::Region) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        add(self, drawable, region)
    }
}

impl<C: RequestConnection + ?Sized> ConnectionExt for C {}

/// A RAII-like wrapper around a [Damage].
///
/// Instances of this struct represent a Damage that is freed in `Drop`.
///
/// Any errors during `Drop` are silently ignored. Most likely an error here means that your
/// X11 connection is broken and later requests will also fail.
#[derive(Debug)]
pub struct DamageWrapper<'c, C: RequestConnection>(&'c C, Damage);

impl<'c, C: RequestConnection> DamageWrapper<'c, C>
{
    /// Assume ownership of the given resource and destroy it in `Drop`.
    pub fn for_damage(conn: &'c C, id: Damage) -> Self {
        DamageWrapper(conn, id)
    }

    /// Get the XID of the wrapped resource
    pub fn damage(&self) -> Damage {
        self.1
    }

    /// Assume ownership of the XID of the wrapped resource
    ///
    /// This function destroys this wrapper without freeing the underlying resource.
    pub fn into_damage(self) -> Damage {
        let id = self.1;
        std::mem::forget(self);
        id
    }
}

impl<'c, C: X11Connection> DamageWrapper<'c, C>
{

    /// Create a new Damage and return a Damage wrapper and a cookie.
    ///
    /// This is a thin wrapper around [create] that allocates an id for the Damage.
    /// This function returns the resulting `DamageWrapper` that owns the created Damage and frees
    /// it in `Drop`. This also returns a `VoidCookie` that comes from the call to
    /// [create].
    ///
    /// Errors can come from the call to [X11Connection::generate_id] or [create].
    pub fn create_and_get_cookie(conn: &'c C, drawable: xproto::Drawable, level: ReportLevel) -> Result<(Self, VoidCookie<'c, C>), ReplyOrIdError>
    {
        let damage = conn.generate_id()?;
        let cookie = create(conn, damage, drawable, level)?;
        Ok((Self::for_damage(conn, damage), cookie))
    }

    /// Create a new Damage and return a Damage wrapper
    ///
    /// This is a thin wrapper around [create] that allocates an id for the Damage.
    /// This function returns the resulting `DamageWrapper` that owns the created Damage and frees
    /// it in `Drop`.
    ///
    /// Errors can come from the call to [X11Connection::generate_id] or [create].
    pub fn create(conn: &'c C, drawable: xproto::Drawable, level: ReportLevel) -> Result<Self, ReplyOrIdError>
    {
        Ok(Self::create_and_get_cookie(conn, drawable, level)?.0)
    }
}

impl<C: RequestConnection> From<&DamageWrapper<'_, C>> for Damage {
    fn from(from: &DamageWrapper<'_, C>) -> Self {
        from.1
    }
}

impl<C: RequestConnection> Drop for DamageWrapper<'_, C> {
    fn drop(&mut self) {
        let _ = destroy(self.0, self.1);
    }
}
