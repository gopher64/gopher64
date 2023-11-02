ab_glyph
[![crates.io](https://img.shields.io/crates/v/ab_glyph.svg)](https://crates.io/crates/ab_glyph)
[![Documentation](https://docs.rs/ab_glyph/badge.svg)](https://docs.rs/ab_glyph)
========
Fast API for loading, scaling, positioning and rasterizing OpenType font glyphs.

```rust
use ab_glyph::{FontRef, Font, Glyph, point};

let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;

// Get a glyph for 'q' with a scale & position.
let q_glyph: Glyph = font.glyph_id('q').with_scale_and_position(24.0, point(100.0, 0.0));

// Draw it.
if let Some(q) = font.outline_glyph(q_glyph) {
    q.draw(|x, y, c| { /* draw pixel `(x, y)` with coverage: `c` */ });
}
```

## no_std
no_std environments are supported using `alloc` & [`libm`](https://github.com/rust-lang/libm).
```toml
ab_glyph = { default-features = false, features = ["libm"] }
```

## Comparison with [`rusttype`](https://gitlab.redox-os.org/redox-os/rusttype)
ab_glyph is a rewrite of rusttype made after I added .otf support for the latter and saw some performance issue's
with the rusttype API.

ab_glyph is a more focussed API concentrating on high performance for both .ttf & .otf fonts.

When laying out glyphs into paragraph, ab_glyph is faster than rusttype using .ttf fonts &
**much** faster for .otf fonts.

```
group                               ab-glyph                    rusttype 0.9
-----                               --------                    ------------
layout_a_sentence (exo2-ttf)        1.00     11.1±0.08µs        1.56     17.3±0.14µs
layout_a_sentence (exo2-otf)        1.00     11.1±0.12µs        8.85     98.1±1.17µs
```
_Note: Numbers from May-2020 benchmarks, ab-glyph performance is also expected to have improved since then_.
