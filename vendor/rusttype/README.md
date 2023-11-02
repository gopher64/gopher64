# RustType
[![crates.io](https://img.shields.io/crates/v/rusttype.svg)](https://crates.io/crates/rusttype)
[![docs.rs](https://docs.rs/rusttype/badge.svg)](https://docs.rs/rusttype)

RustType is a pure Rust alternative to libraries like FreeType.

The current capabilities of RustType:

* Reading OpenType formatted fonts and font collections. This includes `*.ttf`
  as well as `*.otf` font files.
* Retrieving glyph shapes and commonly used properties for a font and its glyphs.
* Laying out glyphs horizontally using horizontal and vertical metrics, and
  glyph-pair-specific kerning.
* Rasterising glyphs with sub-pixel positioning using an accurate analytical
  algorithm (not based on sampling).
* Managing a font cache on the GPU with the `gpu_cache` module. This keeps
  recently used glyph renderings in a dynamic cache in GPU memory to minimise
  texture uploads per-frame. It also allows you keep the draw call count for
  text very low, as all glyphs are kept in one GPU texture.

Notable things that RustType does not support *yet*:

* Font hinting.
* Ligatures of any kind.
* Some less common TrueType sub-formats.
* Right-to-left and vertical text layout.

## Testing & examples
Heavier examples, tests & benchmarks are in the `./dev` directory. This avoids dev-dependency feature bleed.

Run all tests with `cargo test --all --all-features`.

Run examples with `cargo run --example <NAME> -p dev`

## Getting Started

To hit the ground running with RustType, look at `dev/examples/ascii.rs`
supplied with the crate. It demonstrates loading a font file, rasterising an
arbitrary string, and displaying the result as ASCII art. If you prefer to just
look at the documentation, the entry point for loading fonts is `Font`,
from which you can access individual fonts, then their glyphs.

## Future Plans

The initial motivation for the project was to provide easy-to-use font rendering for games.
There are numerous avenues for improving RustType. Ideas:

* Support for some common forms of ligatures.
* And, eventually, support for embedded right-to-left Unicode text.

If you think you could help with achieving any of these goals, feel free to open
a tracking issue for discussing them.

## Minimum supported rust compiler
This crate is maintained with [latest stable rust](https://gist.github.com/alexheretic/d1e98d8433b602e57f5d0a9637927e0c).

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

### See Also

- [glyph_brush](https://github.com/alexheretic/glyph-brush) - can cache vertex generation & provides more complex layouts.
