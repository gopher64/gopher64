//! Trivial forwarding impls on `Ivar`.
//!
//! Kept here to keep `ivar.rs` free from this boilerplate.
//!
//! `#[inline]` is used where the standard library `Box` uses it.

#![forbid(unsafe_code)]

// use alloc::borrow;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;
use core::future::Future;
use core::hash;
use core::iter::FusedIterator;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use std::error::Error;
use std::io;

use super::{Ivar, IvarType};

impl<T: IvarType> PartialEq for Ivar<T>
where
    <Self as Deref>::Target: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (**self).eq(&**other)
    }

    #[inline]
    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &Self) -> bool {
        (**self).ne(&**other)
    }
}

impl<T: IvarType> Eq for Ivar<T> where <Self as Deref>::Target: Eq {}

impl<T: IvarType> PartialOrd for Ivar<T>
where
    <Self as Deref>::Target: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        (**self).lt(&**other)
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        (**self).le(&**other)
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        (**self).ge(&**other)
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        (**self).gt(&**other)
    }
}

impl<T: IvarType> Ord for Ivar<T>
where
    <Self as Deref>::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: IvarType> hash::Hash for Ivar<T>
where
    <Self as Deref>::Target: hash::Hash,
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: IvarType> hash::Hasher for Ivar<T>
where
    <Self as Deref>::Target: hash::Hasher,
{
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i)
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i)
    }
    fn write_u128(&mut self, i: u128) {
        (**self).write_u128(i)
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i)
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i)
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i)
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i)
    }
    fn write_i64(&mut self, i: i64) {
        (**self).write_i64(i)
    }
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i)
    }
}

impl<T: IvarType> fmt::Display for Ivar<T>
where
    <Self as Deref>::Target: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: IvarType> fmt::Debug for Ivar<T>
where
    <Self as Deref>::Target: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<I: IvarType> Iterator for Ivar<I>
where
    <Self as Deref>::Target: Iterator,
{
    type Item = <<Self as Deref>::Target as Iterator>::Item;
    fn next(&mut self) -> Option<<<Self as Deref>::Target as Iterator>::Item> {
        (**self).next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
}

impl<I: IvarType> DoubleEndedIterator for Ivar<I>
where
    <Self as Deref>::Target: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<<<Self as Deref>::Target as Iterator>::Item> {
        (**self).next_back()
    }
    fn nth_back(&mut self, n: usize) -> Option<<<Self as Deref>::Target as Iterator>::Item> {
        (**self).nth_back(n)
    }
}

impl<I: IvarType> ExactSizeIterator for Ivar<I>
where
    <Self as Deref>::Target: ExactSizeIterator,
{
    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<I: IvarType> FusedIterator for Ivar<I> where <Self as Deref>::Target: FusedIterator {}

// impl<T: IvarType> borrow::Borrow<<Self as Deref>::Target> for Ivar<T> {
//     fn borrow(&self) -> &<Self as Deref>::Target {
//         Deref::deref(self)
//     }
// }
//
// impl<T: IvarType> borrow::BorrowMut<<Self as Deref>::Target> for Ivar<T> {
//     fn borrow_mut(&mut self) -> &mut <Self as Deref>::Target {
//         DerefMut::deref_mut(self)
//     }
// }

impl<T: IvarType> AsRef<<Self as Deref>::Target> for Ivar<T> {
    fn as_ref(&self) -> &<Self as Deref>::Target {
        Deref::deref(self)
    }
}

impl<T: IvarType> AsMut<<Self as Deref>::Target> for Ivar<T> {
    fn as_mut(&mut self) -> &mut <Self as Deref>::Target {
        DerefMut::deref_mut(self)
    }
}

impl<T: IvarType> Error for Ivar<T>
where
    <Self as Deref>::Target: Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}

impl<T: IvarType> io::Read for Ivar<T>
where
    <Self as Deref>::Target: io::Read,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (**self).read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        (**self).read_vectored(bufs)
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        (**self).read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        (**self).read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        (**self).read_exact(buf)
    }
}

impl<T: IvarType> io::Write for Ivar<T>
where
    <Self as Deref>::Target: io::Write,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (**self).write(buf)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        (**self).write_vectored(bufs)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        (**self).flush()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        (**self).write_all(buf)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        (**self).write_fmt(fmt)
    }
}

impl<T: IvarType> io::Seek for Ivar<T>
where
    <Self as Deref>::Target: io::Seek,
{
    #[inline]
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        (**self).seek(pos)
    }

    #[inline]
    fn stream_position(&mut self) -> io::Result<u64> {
        (**self).stream_position()
    }
}

impl<T: IvarType> io::BufRead for Ivar<T>
where
    <Self as Deref>::Target: io::BufRead,
{
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        (**self).fill_buf()
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }

    #[inline]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        (**self).read_until(byte, buf)
    }

    #[inline]
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        (**self).read_line(buf)
    }
}

impl<T: IvarType> Future for Ivar<T>
where
    <Self as Deref>::Target: Future + Unpin,
{
    type Output = <<Self as Deref>::Target as Future>::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        <<Self as Deref>::Target as Future>::poll(Pin::new(&mut *self), cx)
    }
}

// TODO: impl Fn traits, CoerceUnsized, Stream and so on when stabilized
