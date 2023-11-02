// Copyright 2018-2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 or MIT license, at your option.
//
// A copy of the Apache License, Version 2.0 is included in the software as
// LICENSE-APACHE and a copy of the MIT license is included in the software
// as LICENSE-MIT. You may also obtain a copy of the Apache License, Version 2.0
// at https://www.apache.org/licenses/LICENSE-2.0 and a copy of the MIT license
// at https://opensource.org/licenses/MIT.

#![cfg_attr(not(feature = "std"), no_std)]

use core::{fmt, hash::{BuildHasherDefault, Hasher}, marker::PhantomData};

/// A `HashMap` with an integer domain, using `NoHashHasher` to perform no hashing at all.
///
/// # Examples
///
/// See [`IsEnabled`] for use with custom types.
///
/// ```
/// use nohash_hasher::IntMap;
///
/// let mut m: IntMap<u32, bool> = IntMap::default();
///
/// m.insert(0, false);
/// m.insert(1, true);
///
/// assert!(m.contains_key(&0));
/// assert!(m.contains_key(&1));
/// ```
#[cfg(feature = "std")]
pub type IntMap<K, V> = std::collections::HashMap<K, V, BuildNoHashHasher<K>>;

/// A `HashSet` of integers, using `NoHashHasher` to perform no hashing at all.
///
/// # Examples
///
/// See [`IsEnabled`] for use with custom types.
///
/// ```
/// use nohash_hasher::IntSet;
///
/// let mut m = IntSet::default();
///
/// m.insert(0u32);
/// m.insert(1u32);
///
/// assert!(m.contains(&0));
/// assert!(m.contains(&1));
/// ```
#[cfg(feature = "std")]
pub type IntSet<T> = std::collections::HashSet<T, BuildNoHashHasher<T>>;

/// An alias for `BuildHasherDefault` for use with `NoHashHasher`.
///
/// # Examples
///
/// See also [`IntMap`] and [`IntSet`] for some easier usage examples.
///
/// ```
/// use nohash_hasher::BuildNoHashHasher;
/// use std::collections::HashMap;
///
/// let mut m: HashMap::<u8, char, BuildNoHashHasher<u8>> =
///     HashMap::with_capacity_and_hasher(2, BuildNoHashHasher::default());
///
/// m.insert(0, 'a');
/// m.insert(1, 'b');
///
/// assert_eq!(Some(&'a'), m.get(&0));
/// assert_eq!(Some(&'b'), m.get(&1));
/// ```
pub type BuildNoHashHasher<T> = BuildHasherDefault<NoHashHasher<T>>;

/// For an enabled type `T`, a `NoHashHasher<T>` implements `std::hash::Hasher` and
/// uses the value set by one of the `write_{u8, u16, u32, u64, usize, i8, i16, i32,
/// i64, isize}` methods as its hash output.
///
/// `NoHashHasher` does not implement any hashing algorithm and can only be used
/// with types which can be mapped directly to a numeric value. Out of the box
/// `NoHashHasher` is enabled for `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`,
/// `i32`, `i64`, and `isize`. Types that should be used with `NoHashHasher` need
/// to implement [`IsEnabled`] and by doing so assert that their `Hash` impl invokes
/// *only one* of the `Hasher::write_{u8, u16, u32, u64, usize, i8, i16, i32, i64,
/// isize}` methods *exactly once*.
///
/// # Examples
///
/// See also [`BuildNoHashHasher`], [`IntMap`] and [`IntSet`] for some easier
/// usage examples. See [`IsEnabled`] for use with custom types.
///
/// ```
/// use nohash_hasher::NoHashHasher;
/// use std::{collections::HashMap, hash::BuildHasherDefault};
///
/// let mut m: HashMap::<u8, char, BuildHasherDefault<NoHashHasher<u8>>> =
///     HashMap::with_capacity_and_hasher(2, BuildHasherDefault::default());
///
/// m.insert(0, 'a');
/// m.insert(1, 'b');
///
/// assert_eq!(Some(&'a'), m.get(&0));
/// assert_eq!(Some(&'b'), m.get(&1));
/// ```
#[cfg(debug_assertions)]
pub struct NoHashHasher<T>(u64, bool, PhantomData<T>);

#[cfg(not(debug_assertions))]
pub struct NoHashHasher<T>(u64, PhantomData<T>);

impl<T> fmt::Debug for NoHashHasher<T> {
    #[cfg(debug_assertions)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("NoHashHasher").field(&self.0).field(&self.1).finish()
    }

    #[cfg(not(debug_assertions))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("NoHashHasher").field(&self.0).finish()
    }
}

impl<T> Default for NoHashHasher<T> {
    #[cfg(debug_assertions)]
    fn default() -> Self {
        NoHashHasher(0, false, PhantomData)
    }

    #[cfg(not(debug_assertions))]
    fn default() -> Self {
        NoHashHasher(0, PhantomData)
    }
}

impl<T> Clone for NoHashHasher<T> {
    #[cfg(debug_assertions)]
    fn clone(&self) -> Self {
        NoHashHasher(self.0, self.1, self.2)
    }

    #[cfg(not(debug_assertions))]
    fn clone(&self) -> Self {
        NoHashHasher(self.0, self.1)
    }
}

impl<T> Copy for NoHashHasher<T> {}

/// Types which are safe to use with `NoHashHasher`.
///
/// This marker trait is an option for types to enable themselves for use
/// with `NoHashHasher`. In order to be safe, the `Hash` impl needs to
/// satisfy the following constraint:
///
/// > **One of the `Hasher::write_{u8,u16,u32,u64,usize,i8,i16,i32,i64,isize}`
/// methods is invoked exactly once.**
///
/// The best way to ensure this is to write a custom `Hash` impl even when
/// deriving `Hash` for a simple newtype of a single type which itself
/// implements `IsEnabled` may work as well.
///
/// # Example
///
/// ```
/// #[derive(PartialEq, Eq)]
/// struct SomeType(u32);
///
/// impl std::hash::Hash for SomeType {
///     fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
///         hasher.write_u32(self.0)
///     }
/// }
///
/// impl nohash_hasher::IsEnabled for SomeType {}
///
/// let mut m = nohash_hasher::IntMap::default();
///
/// m.insert(SomeType(1), 't');
/// m.insert(SomeType(0), 'f');
///
/// assert_eq!(Some(&'t'), m.get(&SomeType(1)));
/// assert_eq!(Some(&'f'), m.get(&SomeType(0)));
/// ```
pub trait IsEnabled {}

impl IsEnabled for u8 {}
impl IsEnabled for u16 {}
impl IsEnabled for u32 {}
impl IsEnabled for u64 {}
impl IsEnabled for usize {}
impl IsEnabled for i8 {}
impl IsEnabled for i16 {}
impl IsEnabled for i32 {}
impl IsEnabled for i64 {}
impl IsEnabled for isize {}

#[cfg(not(debug_assertions))]
impl<T: IsEnabled> Hasher for NoHashHasher<T> {
    fn write(&mut self, _: &[u8]) {
        panic!("Invalid use of NoHashHasher")
    }

    fn write_u8(&mut self, n: u8)       { self.0 = u64::from(n) }
    fn write_u16(&mut self, n: u16)     { self.0 = u64::from(n) }
    fn write_u32(&mut self, n: u32)     { self.0 = u64::from(n) }
    fn write_u64(&mut self, n: u64)     { self.0 = n }
    fn write_usize(&mut self, n: usize) { self.0 = n as u64 }

    fn write_i8(&mut self, n: i8)       { self.0 = n as u64 }
    fn write_i16(&mut self, n: i16)     { self.0 = n as u64 }
    fn write_i32(&mut self, n: i32)     { self.0 = n as u64 }
    fn write_i64(&mut self, n: i64)     { self.0 = n as u64 }
    fn write_isize(&mut self, n: isize) { self.0 = n as u64 }

    fn finish(&self) -> u64 { self.0 }
}

#[cfg(debug_assertions)]
impl<T: IsEnabled> Hasher for NoHashHasher<T> {
    fn write(&mut self, _: &[u8]) {
        panic!("Invalid use of NoHashHasher")
    }

    fn write_u8(&mut self, n: u8) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = u64::from(n);
        self.1 = true
    }

    fn write_u16(&mut self, n: u16) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = u64::from(n);
        self.1 = true
    }

    fn write_u32(&mut self, n: u32) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = u64::from(n);
        self.1 = true
    }

    fn write_u64(&mut self, n: u64) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = n;
        self.1 = true
    }

    fn write_usize(&mut self, n: usize) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = n as u64;
        self.1 = true
    }

    fn write_i8(&mut self, n: i8) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = n as u64;
        self.1 = true
    }

    fn write_i16(&mut self, n: i16) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = n as u64;
        self.1 = true
    }

    fn write_i32(&mut self, n: i32) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = n as u64;
        self.1 = true
    }

    fn write_i64(&mut self, n: i64) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = n as u64;
        self.1 = true
    }

    fn write_isize(&mut self, n: isize) {
        assert!(!self.1, "NoHashHasher: second write attempt detected.");
        self.0 = n as u64;
        self.1 = true
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok() {
        let mut h1 = NoHashHasher::<u8>::default();
        h1.write_u8(42);
        assert_eq!(42, h1.finish());

        let mut h2 = NoHashHasher::<u16>::default();
        h2.write_u16(42);
        assert_eq!(42, h2.finish());

        let mut h3 = NoHashHasher::<u32>::default();
        h3.write_u32(42);
        assert_eq!(42, h3.finish());

        let mut h4 = NoHashHasher::<u64>::default();
        h4.write_u64(42);
        assert_eq!(42, h4.finish());

        let mut h5 = NoHashHasher::<usize>::default();
        h5.write_usize(42);
        assert_eq!(42, h5.finish());

        let mut h6 = NoHashHasher::<i8>::default();
        h6.write_i8(42);
        assert_eq!(42, h6.finish());

        let mut h7 = NoHashHasher::<i16>::default();
        h7.write_i16(42);
        assert_eq!(42, h7.finish());

        let mut h8 = NoHashHasher::<i32>::default();
        h8.write_i32(42);
        assert_eq!(42, h8.finish());

        let mut h9 = NoHashHasher::<i64>::default();
        h9.write_i64(42);
        assert_eq!(42, h9.finish());

        let mut h10 = NoHashHasher::<isize>::default();
        h10.write_isize(42);
        assert_eq!(42, h10.finish())
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn u8_double_usage() {
        let mut h = NoHashHasher::<u8>::default();
        h.write_u8(42);
        h.write_u8(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn u16_double_usage() {
        let mut h = NoHashHasher::<u16>::default();
        h.write_u16(42);
        h.write_u16(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn u32_double_usage() {
        let mut h = NoHashHasher::<u32>::default();
        h.write_u32(42);
        h.write_u32(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn u64_double_usage() {
        let mut h = NoHashHasher::<u64>::default();
        h.write_u64(42);
        h.write_u64(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn usize_double_usage() {
        let mut h = NoHashHasher::<usize>::default();
        h.write_usize(42);
        h.write_usize(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn i8_double_usage() {
        let mut h = NoHashHasher::<i8>::default();
        h.write_i8(42);
        h.write_i8(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn i16_double_usage() {
        let mut h = NoHashHasher::<i16>::default();
        h.write_i16(42);
        h.write_i16(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn i32_double_usage() {
        let mut h = NoHashHasher::<i32>::default();
        h.write_i32(42);
        h.write_i32(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn i64_double_usage() {
        let mut h = NoHashHasher::<i64>::default();
        h.write_i64(42);
        h.write_i64(43);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn isize_double_usage() {
        let mut h = NoHashHasher::<isize>::default();
        h.write_isize(42);
        h.write_isize(43);
    }
}

