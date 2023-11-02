[![crates.io](http://meritbadge.herokuapp.com/smithay-clipboard)](https://crates.io/crates/smithay-clipboard)
[![Build Status](https://travis-ci.org/Smithay/smithay-clipboard.svg?branch=master)](https://travis-ci.org/Smithay/smithay-clipboard)


# Smithay Clipboard

This crate provides access to the Wayland clipboard for applications
already using some sort of GUI toolkit or a windowing library, like
[winit](https://github.com/rust-windowing/winit), since you should
have some surface around to receive keyboard/pointer events.

If you want to access clipboard from the CLI or to write clipboard manager,
this is not what you're looking for.

## Documentation

The documentation for the master branch is [available online](https://smithay.github.io/smithay-clipboard/).

The documentation for the releases can be found on [docs.rs](https://docs.rs/smithay-clipboard).
