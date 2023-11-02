owned_ttf_parser
[![crates.io](https://img.shields.io/crates/v/owned_ttf_parser.svg)](https://crates.io/crates/owned_ttf_parser)
[![Documentation](https://docs.rs/owned_ttf_parser/badge.svg)](https://docs.rs/owned_ttf_parser)
================
[ttf-parser](https://github.com/RazrFalcon/ttf-parser) plus support for owned data.

Provides `OwnedFace`, `AsFaceRef` and re-exports `ttf_parser::*`.

## Example
```rust
use owned_ttf_parser::{AsFaceRef, OwnedFace, Face};

let owned_face = OwnedFace::from_vec(owned_face_data, 0).unwrap();
let face_ref: &Face<'_> = owned_face.as_face_ref();

assert_eq!(face_ref.ascender(), 2254);
```

## no_std
no_std environments are supported using `alloc`.
```toml
owned_ttf_parser = { default-features = false }
```

## Minimum supported rust compiler
All crates maintained with [latest stable rust](https://gist.github.com/alexheretic/d1e98d8433b602e57f5d0a9637927e0c).
