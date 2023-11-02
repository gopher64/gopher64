//! System font configuration.
use crate::title::font_preference::FontPreference;
use std::process::Command;

/// Query system for which font to use for window titles.
pub(crate) fn titlebar_font() -> Option<FontPreference> {
    // outputs something like: `'Cantarell Bold 12'`
    let stdout = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.wm.preferences", "titlebar-font"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())?;

    FontPreference::from_name_style_size(
        stdout
            .trim()
            .trim_end_matches('\'')
            .trim_start_matches('\''),
    )
}
