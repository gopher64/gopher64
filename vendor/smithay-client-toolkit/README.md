[![crates.io](https://img.shields.io/crates/v/smithay-client-toolkit.svg)](https://crates.io/crates/smithay-client-toolkit)
[![docs.rs](https://docs.rs/smithay-client-toolkit/badge.svg)](https://docs.rs/smithay-client-toolkit)
[![Build Status](https://github.com/Smithay/client-toolkit/workflows/Continuous%20Integration/badge.svg)](https://github.com/Smithay/client-toolkit/actions?query=workflow%3A%22Continuous+Integration%22)

# Smithay's Client Toolkit

This crate is a toolkit for writing wayland clients in rust, on top of [wayland-client](https://crates.io/crates/wayland-client).

Currently a work in progress, it currently provides the following utilities:

- Automatic binding of general wayland globals (`wl_compositor`, `wl_shm`, etc..)
- Abstraction to create windows (aka toplevel surfaces), abstracting the interaction
  with the shell (`xdg_shell` or `wl_shell`) and the drawing of decorations
- Wrapper for `wl_keyboard` for automatic keymap interpretation using `libxkbcommon.so`.
- Utilites for creating dpi aware surfaces.

## Documentation

The documentation for the master branch is [available online](https://smithay.github.io/client-toolkit/).

The documentation for the releases can be found on [docs.rs](https://docs.rs/smithay-client-toolkit).

## Requirements

Requires at least rust 1.61 to be used and version 1.12 of the wayland system
libraries.
