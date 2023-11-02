# NoHashHasher

For an enabled type `T`, a `NoHashHasher<T>` implements `std::hash::Hasher` and
uses the value set by one of the `write_{u8, u16, u32, u64, usize, i8, i16, i32,
i64, isize}` methods as its hash output.

`NoHashHasher` does not implement any hashing algorithm and can only be used
with types which can be mapped directly to a numeric value. Out of the box
`NoHashHasher` is enabled for `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`,
`i32`, `i64`, and `isize`. Types that should be used with `NoHashHasher` need
to implement [`IsEnabled`] and by doing so assert that their `Hash` impl invokes
*only one* of the `Hasher::write_{u8, u16, u32, u64, usize, i8, i16, i32, i64,
isize}` methods *exactly once*.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
