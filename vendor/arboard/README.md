# Arboard (Arthur's Clipboard)

[![Latest version](https://img.shields.io/crates/v/arboard?color=mediumvioletred)](https://crates.io/crates/arboard)
[![Documentation](https://docs.rs/arboard/badge.svg)](https://docs.rs/arboard)

## General

This is a cross-platform library for interacting with the clipboard. It allows
to copy and paste both text and image data in a platform independent way on
Linux, Mac, and Windows.

## GNU/Linux

The GNU/Linux implementation uses the X protocol by default for managing the
clipboard but *fear not*  because Wayland works with the X11 protocol just as
well. Furthermore this implementation uses the Clipboard selection (as opposed
to the primary selection) and it sends the data to the clipboard manager when
the application exits so that the data placed onto the clipboard with your
application remains to be available after exiting.

There's also an optional wayland data control backend through the
`wl-clipboard-rs` crate. This can be enabled using the `wayland-data-control`
feature. When enabled this will be prioritized over the X11 backend, but if the
initialization fails, the implementation falls back to using the X11 protocol
automatically. Note that in my tests the wayland backend did not keep the
clipboard contents after the process exited. (Although neither did the X11
backend on my Wayland setup).

## Example

```rust
use arboard::Clipboard;

fn main() {
    let mut clipboard = Clipboard::new().unwrap();
    println!("Clipboard text was: {}", clipboard.get_text().unwrap());

    let the_string = "Hello, world!";
    clipboard.set_text(the_string).unwrap();
    println!("But now the clipboard text should be: \"{}\"", the_string);
}
```

## Yet another clipboard crate

This is a fork of `rust-clipboard`. The reason for forking instead of making a
PR is that `rust-clipboard` is not being maintained any more. Furthermore note
that the API of this crate is considerably different from that of
`rust-clipboard`. There are already a ton of clipboard crates out there which
is a bit unfortunate; I don't know why this is happening but while it is, we
might as well just start naming the clipboard crates after ourselves. This one
is arboard which stands for Artur's clipboard.
