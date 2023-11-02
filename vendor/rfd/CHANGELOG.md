# Change Log

## 0.12.1
- Fix `FileHandle::inner` (under feature `file-handle-inner`) on wasm

## 0.12.0
- Add title support for WASM (#132) 
- Add Create folder button to `pick_folder` on macOS (#127)
- Add support for Yes/No/Cancel buttons (#123)
- Change a string method signatures #117 
- WASM `save_file` (#134)
- Update `gtk-sys` to `0.18` (#143)
- Update `ashpd` to `0.6` (#133)
- Replace windows with `windows-sys` (#118) 
- Make zenity related deps optional (#141)

## 0.11.3
- Zenity message dialogs for xdg portal backend 

## 0.10.1
- Update `gtk-sys` to `0.16` and `windows-rs` to `0.44`

## 0.10.0
- fix(FileDialog::set_directory): fallback to default if path is empty

## 0.9.0
- feat: customize button text, Close #74
- feat: Add support for selecting multiple folders, fixes #73

## 0.8.4
- XDG: decode URI before converting to PathBuf #70 
  
## 0.8.3
- Windows-rs update 0.37

## 0.8.2
- Windows-rs update 0.35

## 0.8.1
- Macos parent for sync FileDialog (#58)
- Windows-rs update 0.33

## 0.8.0
- `parent` feature was removed, it is always on now
- New feature `xdg-portal` 
- Now you have to choose one of the features `gtk3` or `xdg-portal`, gtk is on by default
- `window` crate got updated to 0.32

## 0.7.0
- Safe Rust XDG Desktop Portal support

## 0.6.3

- Update `windows` crate to 0.30.

## 0.6.2
- Strip Win32 namespaces from directory paths 

## 0.6.0
- FreeBSD support
- Port to windows-rs
- Update RawWindowHandle to 0.4

## 0.4.4

- Fix `set_directory` on some windows setups (#22)
- Implement `set_file_name` on MacOS (#21)

## 0.4.3

- `set_parent` support for `MessageDialog` on windows

## 0.4.2

- GTK save dialog now sets current_name correctly (#18)

## 0.4.1

- Update gtk

## 0.4.0

- **[Breaking]** Fix misspeled `OkCancel` in `MessageButtons` (#12)
- `set_parent` support for Windows (#14)
