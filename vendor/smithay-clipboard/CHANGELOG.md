# Change Log

## Unreleased

## 0.6.6 -- 2022-06-20

- Update SCTK to 0.16

## 0.6.5 -- 2021-10-31

- Update SCTK to 0.15, updating wayland-rs to `0.29`

## 0.6.4 -- 2021-06-25

- Update SCTK to 0.14, significantly reducing the depdendency tree

## 0.6.3 -- 2021-02-04

- Consecutive clipboard stores dropped until the application is refocused

## 0.6.2 -- 2020-12-17

- Segfault when dropping clipboard in multithreaded context while main queue is still running

## 0.6.1 -- 2020-10-13

- Crash when failing to write to a clipboard

## 0.6.0 -- 2020-10-03

- Updated smithay-client-toolkit to 0.12
- **Breaking** `Clipboard::new` is now marked with `unsafe`

## 0.5.2 -- 2020-08-30

- Fixed clipboard crashing, when seat has neither keyboard nor pointer focus
- Advertise UTF8_STRING mimetype
- Fixed crash when writing data to the server fails
- Fixed fd leaking from keymap updates

## 0.5.1 -- 2020-07-10

- Fixed clipboard not working, when seat had empty name

# 0.5.0 -- 2020-05-20

- Minimal rust version was bumped to 1.41.0
- Add support for `UTF8_STRING` mime type
- **Breaking** Clipboard now works only with extern display
- **Breaking** Clipboard now works only with last observed seats, instead of optionally accepting seat names

## 0.4.0 -- 2020-03-09

- Fix crash when receiving non-utf8 data
- **Breaking** `load` and `load_primary` now return `Result<String>` to indicate errors
- Fix clipboard dying after TTY switch

## 0.3.7 -- 2020-02-27

- Only bind seat with version up to 6, as version 7 is not yet supported by SCTK
  for loading keymaps

## 0.3.6 -- 2019-11-21

- Perform loaded data normalization for text/plain;charset=utf-8 mime type
- Fix clipboard throttling

## 0.3.5 -- 2019-09-3

- Fix primary selection storing, when releasing button outside of the surface

## 0.3.4 -- 2019-08-14

- Add fallback to gtk primary selection, when zwp primary selection is not available

## 0.3.3 -- 2019-06-14

- Update nix version to 0.14.1

## 0.3.2 -- 2019-06-13

- Update smithay-client-toolkit version to 0.6.1

## 0.3.1 -- 2019-06-08

- Fix primary clipboard storing

## 0.3.0 -- 2019-06-07

- Add support for primary selection through `store_primary()` and `load_primary()`

## 0.2.1 -- 2019-04-27

- Remove dbg! macro from code

## 0.2.0 -- 2019-04-27

- `Clipboard::store()` and `Clipboard::load()` now take a `Option<String>` for the seat name, if
no seat name is provided then the name of the last seat to generate an event will be used instead

## 0.1.1 -- 2019-04-24

- Do a sync roundtrip to register avaliable seats on clipboard creation
- Collect serials from key and pointer events
- Return an empty string for load requests when no seats are avaliable

## 0.1.0 -- 2019-02-14

Initial version, including:

- `WaylandClipboard` with `new_threaded()` and `new_threaded_from_external()`
- multi seat support
