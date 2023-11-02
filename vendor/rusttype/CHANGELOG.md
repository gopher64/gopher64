## Unreleased
* Improve lifetime flexibility for `Font::glyphs_for` & `Font::layout`.
* Update owned_ttf_parser -> `0.15`.
* Update ab_glyph_rasterizer => `0.1.5`.
* Update crossbeam-queue -> `0.8`.
* Update crossbeam-utils -> `0.8`.
* Update num_cpus => `1.13`.
* Update approx => `0.5`.

## 0.9.2
* Update ttf-parser -> `0.6`.
* Use more flexible lifetime bounds for `Font::layout`.

## 0.9.1
* Use crate owned_ttf_parser to provide `OwnedFont` eliminating direct unsafe usage in rusttype.
* Remove unused legacy trait `BoundingBox`.
* Add `ScaledGlyph::build_outline` & `PositionedGlyph::build_outline` methods.

## 0.9.0
* Major rework to use crates **ttf-parser** & **ab_glyph_rasterizer** to respectively read and render OpenType .oft format fonts.
* Remove dependencies **approx**, **stb_truetype** & **ordered-float** along with in-crate rasterization code.
* Strip back some non-vital API functionality.
  - Remove support for `.standalone()` variants which are sparsely used.
  - Remove some functions that didn't immediately translate to ttf-parser. Please raise issues to re-add any you relied on via the new stack.

## 0.8.3
* Remove arrayvec dependency.
* Add `Default` implementations for geometry structs.

## 0.8.2
* Update crossbeam-utils -> `0.7`.
* Update libm -> `0.2.1`.

## 0.8.1
* Update arrayvec -> `0.5`.

## 0.8
* Support no-std usage by disabling the new default feature `std` and using new features `libm-math` and `has-atomics`. The gpu_cache module/feature requires std.

## 0.7.9
* Use semver trick to re-expect rusttype `0.8` with default-features on.

## 0.7.8
_yanked_

## 0.7.7
* gpu_cache: Add `CacheBuilder::align_4x4` method which forces texture updates to align to 4x4 pixel boxes.
* gpu_cache: Disable multithread code and remove dependencies on wasm32.

## 0.7.6
* `GlyphIter` and `LayoutIter` provide the lifetime of the font data.

## 0.7.5
* gpu_cache: `Cache::cache_queued` now returns `CachedBy` for successes which can allow callers to tell that the texture cache has been re-ordered.

## 0.7.4
* Add fn `PositionedGlyph::set_position`
* gpu_cache: Update crossbeam-deque -> `0.7`, use `Injector` for minor rasterization performance boost.

## 0.7.3
* gpu_cache: Update crossbeam-utils -> `0.6`.

## 0.7.2
* Update ordered-float -> `1`.

## 0.7.1
* Fix `PositionedGlyph::pixel_bounding_box()` size inconsistencies at different positions with identical sub-pixel positions.

## 0.7

* Rework `CacheBuilder` to use methods to allow non-breaking additions
  in future. New style is `Cache::builder().dimensions(512, 512).build()`.
* Support multithreaded rasterization in the gpu_cache. This yields
  significant improvements in worst case performance when more than 1
  CPU core is available. _Thrashing, resizing & population benchmarks
  are ~3x faster on a 4-core Haswell._
  Multithreading is enabled by default in environments with more than
  a single core, but can be explicitly disabled using
  `Cache::builder().multithread(false)`.
* Remove all deprecated API.
* Add `Debug` implementations for `Font`, `Glyph`, `ScaledGlyph` &
  `PositionedGlyph`
* Add and improve documentation + examples.

## 0.6.5

* Re-export rusttype `0.7` non-breaking main API, while keeping the current
  version of the gpu_cache module.

## 0.6.4

* Add `CacheBuilder::rebuild` & `Cache::to_builder` methods.
* gpu_cache: Only rasterize & upload after queue has successfully fit in cache
  producing a 1.16-1.29x speedup in resizing & thrashing benchmarks.

## 0.6.3

* Documentation clarifications
* Avoid depending on unused dependency default-features

## 0.6.2

* Add `From<&AsRef<[u8]>> for SharedBytes`.
* Optimise `gpu_cache` hashing to improve benchmark performance by ~30%.

## 0.6.1

* Optimise rasterizer removing internal hashing. Improves draw benchmark
  performance by 11-91%.

## 0.6

* Rework gpu_cache data structures allowing constant time hash lookup
  of matching cached glyph textures. Improve performance by ~60-200%.
* Deprecate `gpu_cache::Cache::new` in favour of `gpu_cache::CacheBuilder`.
* Deprecate `gpu_cache::Cache::set_scale_tolerance` &
  `gpu_cache::Cache::set_position_tolerance`. These are now equivalent to
  recreating the cache as they invalidate the cache keys.
* gpu_cache `scale_tolerance` & `position_tolerance` now have subtly different
  meanings but guarantee their error in all cases, where previously the
  worst case was double the set tolerance.

## 0.5.2

* Add gpu cache glyph padding option to fix texture bleeding from other
  glyphs when using interpolated texture coordinates near edges. Use
  `CacheBuilder` to construct a `Cache` that makes use of padding.
* Inlining performance improvements.

## 0.5.1

* Fix tree removal on row clear (gpu_cache).

## 0.5

* Let functions like `Font::glyph` and `Font::pair_kerning` work with both
  characters and glyph ids by having them accept any type that implements the
  new `IntoGlyphId` trait. This replaces the `CodepointOrGlyph` enum, which
  didn't seem widely used.
* Make `Font::glyph` always return a `Glyph`, not `Option<Glyph>`. Passing a
  `char` the font doesn't cover returns a `.notdef` glyph (id 0), as it did
  before. Passing an invalid glyph id now panics, like a bad array index: glyph
  ids should only be used to index the font they were looked up for.
* Introduce `rusttype::Error`, which implements `std::error::Error`, `Debug` and
  `Display`, and can be converted to `std::io::Error`.
* Use `Result<_, rusttype::Error>` to report failures in FontCollection, Font
  and associated iterators.
* Add `Font::from_bytes` method similar to `FontCollection::from_bytes` for 1
  font collections.
* Improve gpu_cache performance ~2-6%

## 0.4.3

* Improve gpu_cache performance ~6-17%

## 0.4.2

* Allow users to get font names from `Font`. (#86)

## 0.4

* Add more debugging features
* Add support for unscaled fonts
* Improve performance
* Make gpu_cache optional

## 0.3

* Transfer to redox-os organization, merge a number of pull requests

## 0.2.1

* Made the API more convenient (courtesy of @mitchmindtree, @I1048576).
* Fixes for the examples (@I1048576)
* Removed the dependency on ndarray (@I1048576)

## 0.2

* Initial GPU caching implementation.
* Made font data management more flexible.
* Made the interface for font scales simpler.

## 0.1.2

Fixed issue #8

## 0.1.1

Fixed issue #7

## 0.1

Initial release
