//! System configuration.
use std::process::Command;

/// Query system to see if dark theming should be preferred.
pub(crate) fn prefer_dark() -> bool {
    // outputs something like: `variant       variant          uint32 1`
    let stdout = Command::new("dbus-send")
        .arg("--reply-timeout=100")
        .arg("--print-reply=literal")
        .arg("--dest=org.freedesktop.portal.Desktop")
        .arg("/org/freedesktop/portal/desktop")
        .arg("org.freedesktop.portal.Settings.Read")
        .arg("string:org.freedesktop.appearance")
        .arg("string:color-scheme")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok());

    if matches!(stdout, Some(ref s) if s.is_empty()) {
        log::error!("XDG Settings Portal did not return response in time: timeout: 100ms, key: color-scheme");
    }

    matches!(stdout, Some(s) if s.trim().ends_with("uint32 1"))
}
