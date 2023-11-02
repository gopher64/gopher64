// This file contains generated code. Do not edit directly.
// To regenerate this, run 'make'.

//! Bindings to the `Shm` X11 extension.

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
use super::xproto;

pub use x11rb_protocol::protocol::shm::*;

/// Get the major opcode of this extension
fn major_opcode<Conn: RequestConnection + ?Sized>(conn: &Conn) -> Result<u8, ConnectionError> {
    let info = conn.extension_information(X11_EXTENSION_NAME)?;
    let info = info.ok_or(ConnectionError::UnsupportedExtension)?;
    Ok(info.major_opcode)
}

pub fn query_version<Conn>(conn: &Conn) -> Result<Cookie<'_, Conn, QueryVersionReply>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = QueryVersionRequest;
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_with_reply(&slices, fds)
}

pub fn attach<Conn>(conn: &Conn, shmseg: Seg, shmid: u32, read_only: bool) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = AttachRequest {
        shmseg,
        shmid,
        read_only,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn detach<Conn>(conn: &Conn, shmseg: Seg) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = DetachRequest {
        shmseg,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn put_image<Conn>(conn: &Conn, drawable: xproto::Drawable, gc: xproto::Gcontext, total_width: u16, total_height: u16, src_x: u16, src_y: u16, src_width: u16, src_height: u16, dst_x: i16, dst_y: i16, depth: u8, format: u8, send_event: bool, shmseg: Seg, offset: u32) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = PutImageRequest {
        drawable,
        gc,
        total_width,
        total_height,
        src_x,
        src_y,
        src_width,
        src_height,
        dst_x,
        dst_y,
        depth,
        format,
        send_event,
        shmseg,
        offset,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn get_image<Conn>(conn: &Conn, drawable: xproto::Drawable, x: i16, y: i16, width: u16, height: u16, plane_mask: u32, format: u8, shmseg: Seg, offset: u32) -> Result<Cookie<'_, Conn, GetImageReply>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = GetImageRequest {
        drawable,
        x,
        y,
        width,
        height,
        plane_mask,
        format,
        shmseg,
        offset,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_with_reply(&slices, fds)
}

pub fn create_pixmap<Conn>(conn: &Conn, pid: xproto::Pixmap, drawable: xproto::Drawable, width: u16, height: u16, depth: u8, shmseg: Seg, offset: u32) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = CreatePixmapRequest {
        pid,
        drawable,
        width,
        height,
        depth,
        shmseg,
        offset,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn attach_fd<Conn, A>(conn: &Conn, shmseg: Seg, shm_fd: A, read_only: bool) -> Result<VoidCookie<'_, Conn>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
    A: Into<RawFdContainer>,
{
    let shm_fd: RawFdContainer = shm_fd.into();
    let request0 = AttachFdRequest {
        shmseg,
        shm_fd,
        read_only,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_without_reply(&slices, fds)
}

pub fn create_segment<Conn>(conn: &Conn, shmseg: Seg, size: u32, read_only: bool) -> Result<CookieWithFds<'_, Conn, CreateSegmentReply>, ConnectionError>
where
    Conn: RequestConnection + ?Sized,
{
    let request0 = CreateSegmentRequest {
        shmseg,
        size,
        read_only,
    };
    let (bytes, fds) = request0.serialize(major_opcode(conn)?);
    let slices = bytes.iter().map(|b| IoSlice::new(&*b)).collect::<Vec<_>>();
    conn.send_request_with_reply_with_fds(&slices, fds)
}

/// Extension trait defining the requests of this extension.
pub trait ConnectionExt: RequestConnection {
    fn shm_query_version(&self) -> Result<Cookie<'_, Self, QueryVersionReply>, ConnectionError>
    {
        query_version(self)
    }
    fn shm_attach(&self, shmseg: Seg, shmid: u32, read_only: bool) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        attach(self, shmseg, shmid, read_only)
    }
    fn shm_detach(&self, shmseg: Seg) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        detach(self, shmseg)
    }
    fn shm_put_image(&self, drawable: xproto::Drawable, gc: xproto::Gcontext, total_width: u16, total_height: u16, src_x: u16, src_y: u16, src_width: u16, src_height: u16, dst_x: i16, dst_y: i16, depth: u8, format: u8, send_event: bool, shmseg: Seg, offset: u32) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        put_image(self, drawable, gc, total_width, total_height, src_x, src_y, src_width, src_height, dst_x, dst_y, depth, format, send_event, shmseg, offset)
    }
    fn shm_get_image(&self, drawable: xproto::Drawable, x: i16, y: i16, width: u16, height: u16, plane_mask: u32, format: u8, shmseg: Seg, offset: u32) -> Result<Cookie<'_, Self, GetImageReply>, ConnectionError>
    {
        get_image(self, drawable, x, y, width, height, plane_mask, format, shmseg, offset)
    }
    fn shm_create_pixmap(&self, pid: xproto::Pixmap, drawable: xproto::Drawable, width: u16, height: u16, depth: u8, shmseg: Seg, offset: u32) -> Result<VoidCookie<'_, Self>, ConnectionError>
    {
        create_pixmap(self, pid, drawable, width, height, depth, shmseg, offset)
    }
    fn shm_attach_fd<A>(&self, shmseg: Seg, shm_fd: A, read_only: bool) -> Result<VoidCookie<'_, Self>, ConnectionError>
    where
        A: Into<RawFdContainer>,
    {
        attach_fd(self, shmseg, shm_fd, read_only)
    }
    fn shm_create_segment(&self, shmseg: Seg, size: u32, read_only: bool) -> Result<CookieWithFds<'_, Self, CreateSegmentReply>, ConnectionError>
    {
        create_segment(self, shmseg, size, read_only)
    }
}

impl<C: RequestConnection + ?Sized> ConnectionExt for C {}

/// A RAII-like wrapper around a [Seg].
///
/// Instances of this struct represent a Seg that is freed in `Drop`.
///
/// Any errors during `Drop` are silently ignored. Most likely an error here means that your
/// X11 connection is broken and later requests will also fail.
#[derive(Debug)]
pub struct SegWrapper<'c, C: RequestConnection>(&'c C, Seg);

impl<'c, C: RequestConnection> SegWrapper<'c, C>
{
    /// Assume ownership of the given resource and destroy it in `Drop`.
    pub fn for_seg(conn: &'c C, id: Seg) -> Self {
        SegWrapper(conn, id)
    }

    /// Get the XID of the wrapped resource
    pub fn seg(&self) -> Seg {
        self.1
    }

    /// Assume ownership of the XID of the wrapped resource
    ///
    /// This function destroys this wrapper without freeing the underlying resource.
    pub fn into_seg(self) -> Seg {
        let id = self.1;
        std::mem::forget(self);
        id
    }
}

impl<'c, C: X11Connection> SegWrapper<'c, C>
{

    /// Create a new Seg and return a Seg wrapper and a cookie.
    ///
    /// This is a thin wrapper around [attach] that allocates an id for the Seg.
    /// This function returns the resulting `SegWrapper` that owns the created Seg and frees
    /// it in `Drop`. This also returns a `VoidCookie` that comes from the call to
    /// [attach].
    ///
    /// Errors can come from the call to [X11Connection::generate_id] or [attach].
    pub fn attach_and_get_cookie(conn: &'c C, shmid: u32, read_only: bool) -> Result<(Self, VoidCookie<'c, C>), ReplyOrIdError>
    {
        let shmseg = conn.generate_id()?;
        let cookie = attach(conn, shmseg, shmid, read_only)?;
        Ok((Self::for_seg(conn, shmseg), cookie))
    }

    /// Create a new Seg and return a Seg wrapper
    ///
    /// This is a thin wrapper around [attach] that allocates an id for the Seg.
    /// This function returns the resulting `SegWrapper` that owns the created Seg and frees
    /// it in `Drop`.
    ///
    /// Errors can come from the call to [X11Connection::generate_id] or [attach].
    pub fn attach(conn: &'c C, shmid: u32, read_only: bool) -> Result<Self, ReplyOrIdError>
    {
        Ok(Self::attach_and_get_cookie(conn, shmid, read_only)?.0)
    }

    /// Create a new Seg and return a Seg wrapper and a cookie.
    ///
    /// This is a thin wrapper around [attach_fd] that allocates an id for the Seg.
    /// This function returns the resulting `SegWrapper` that owns the created Seg and frees
    /// it in `Drop`. This also returns a `VoidCookie` that comes from the call to
    /// [attach_fd].
    ///
    /// Errors can come from the call to [X11Connection::generate_id] or [attach_fd].
    pub fn attach_fd_and_get_cookie<A>(conn: &'c C, shm_fd: A, read_only: bool) -> Result<(Self, VoidCookie<'c, C>), ReplyOrIdError>
    where
        A: Into<RawFdContainer>,
    {
        let shmseg = conn.generate_id()?;
        let cookie = attach_fd(conn, shmseg, shm_fd, read_only)?;
        Ok((Self::for_seg(conn, shmseg), cookie))
    }

    /// Create a new Seg and return a Seg wrapper
    ///
    /// This is a thin wrapper around [attach_fd] that allocates an id for the Seg.
    /// This function returns the resulting `SegWrapper` that owns the created Seg and frees
    /// it in `Drop`.
    ///
    /// Errors can come from the call to [X11Connection::generate_id] or [attach_fd].
    pub fn attach_fd<A>(conn: &'c C, shm_fd: A, read_only: bool) -> Result<Self, ReplyOrIdError>
    where
        A: Into<RawFdContainer>,
    {
        Ok(Self::attach_fd_and_get_cookie(conn, shm_fd, read_only)?.0)
    }
}

impl<C: RequestConnection> From<&SegWrapper<'_, C>> for Seg {
    fn from(from: &SegWrapper<'_, C>) -> Self {
        from.1
    }
}

impl<C: RequestConnection> Drop for SegWrapper<'_, C> {
    fn drop(&mut self) {
        let _ = detach(self.0, self.1);
    }
}
