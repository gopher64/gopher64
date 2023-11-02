# 0.19.0
* Update _ttf-parser_ to `0.19.0`, [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0190---2023-04-17).

# 0.18.1
* Make `PreParsedSubtables` `face` field public. This allows referencing and unwrapping the underlying face.

# 0.18.0
* Update _ttf-parser_ to `0.18.0`, [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0180---2022-12-25).

# 0.17.1
* Add `PreParsedSubtables::glyph_variation_index`.

# 0.17.0
* Update _ttf-parser_ to `0.17.0`, [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0170---2022-09-28).

# 0.16.0
* Update _ttf-parser_ to `0.16.0`, [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0160---2022-09-18).

# 0.15.2
* Add `FaceMut::set_variation` trait abstraction for calling mutable `Face::set_variation` via `OwnedFace`.
* Use edition 2021.

# 0.15.1
* Add `OwnedFace::as_slice`, `OwnedFace::into_vec`.

# 0.15.0
* Update _ttf-parser_ to `0.15.0`, add `apple-layout` feature [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0150---2022-02-20).

# 0.14.0
* Update _ttf-parser_ to `0.14.0`, [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0140---2021-12-28).

# 0.13.2
* Add `PreParsedSubtables` struct allowing pre-parsing of cmap & kern face subtables at initialization
  time for re-use. This allows much faster `glyph_index` & `glyphs_hor_kerning` avoiding the need
  to parse subtables inside each call.
* Update _ttf-parser_ to `0.13.2`, add `gvar-alloc` feature [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0132---2021-10-28).

# 0.13.1
* Update _ttf-parser_ to `0.13.1` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0131---2021-10-27).

# 0.13.0
* Update _ttf-parser_ to `0.13` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0130---2021-10-24).

# 0.12.1
* Update _ttf-parser_ to `0.12.3` to ensure consistent glyph bounding box behaviour.

# 0.12
* Update _ttf-parser_ to `0.12` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0120---2021-02-14).

# 0.11
* Update _ttf-parser_ to `0.11` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0110---2021-02-04).

# 0.10
* Update _ttf-parser_ to `0.10` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#0100---2021-01-16).
* Add `variable-fonts` features, alongside existing `std` feature (both default) inline with upstream.

# 0.9
* Update _ttf-parser_ to `0.9` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#090---2020-12-05).

# 0.8
* Update _ttf-parser_ to `0.8` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#080---2020-07-21).
* `OwnedFace::from_vec` now returns a `Result`.

# 0.7
* Update _ttf-parser_ to `0.7` [changelog](https://github.com/RazrFalcon/ttf-parser/blob/master/CHANGELOG.md#070---2020-07-16).
* Update `*Font` -> `*Face` to reflect the _ttf-parser_ API changes. 
  ```rust
  // 0.6
  let owned_font = OwnedFont::from_vec(owned_font_data, 0)?;

  // 0.7
  let owned_face = OwnedFace::from_vec(owned_font_data, 0)?;
  ```

# 0.6
* Update _ttf-parser_ to `0.6`.

# 0.5.1
* Support no_std.

# 0.5
* Implement crate supporting _ttf-parser_ `0.5`.
