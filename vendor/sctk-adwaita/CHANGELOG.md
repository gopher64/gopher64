## 0.5.4
- Timeout dbus call to settings portal (100ms)

## 0.5.3
- `ab_glyph` titles will read the system title font using memory mapped buffers instead of reading to heap.
  Lowers RAM usage.
- Improve titlebar-font config parsing to correctly handle more font names.

## 0.5.2
- `ab_glyph` & `crossfont` titles will use gnome "titlebar-font" config if available.
- `ab_glyph` titles are now more consistent with `crossfont` titles both using system sans
  if no better font config is available.
- Rounded corners are now disabled on maximized and tiled windows.
- Double click interval is now 400ms (as previous 1s interval was caused by bug fixed in 0.5.1)

## 0.5.1
- Use dbus org.freedesktop.portal.Settings to automatically choose light or dark theming.
- Double click detection fix.
- Apply button click on release instead of press.

## 0.5.0
- `title` feature got removed
- `ab_glyph` default feature got added (for `ab_glyph` based title rendering)
- `crossfont` feature got added (for `crossfont` based title rendering)
    - Can be enable like this: 
        ```toml
        sctk-adwaita = { default-features = false, features = ["crossfont"] }
        ```
