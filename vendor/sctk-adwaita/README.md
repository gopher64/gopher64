# Adwaita-like SCTK Frame

|   |   |
|---|---|
|![active](https://i.imgur.com/WdO8e0i.png)|![hover](https://i.imgur.com/TkUq2WF.png)|
![inactive](https://i.imgur.com/MTFdSjK.png)|

### Dark mode:
![image](https://user-images.githubusercontent.com/20758186/169424673-3b9fa022-f112-4928-8360-305a714ba979.png)

## Title text: ab_glyph
By default title text is drawn with _ab_glyph_ crate. This can be disabled by disabling default features.

## Title text: crossfont
Alternatively title text may be drawn with _crossfont_ crate. This adds a requirement on _freetype_.

```toml
sctk-adwaita = { default-features = false, features = ["crossfont"] }
```
