# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased

* Replaced `no_std` feature with `std` feature, selected by default
    * `no_std` environments are now built by setting `default-features = false`
* Added new `sdl` feature for building SDL2 without bundling
    * Included by `bundled`, so only one or the other needs to be selected

## 0.3.35

* Better fix for macOS builds

## 0.3.34

* Pin SDL2 for macOS compatibility

## 0.3.33

* Support multi-character SDL text input

## 0.3.32

* Change Rust edition from 2015 to 2018
* Change `Color` from `repr(packed)` to `repr(transparent)`
* Fix compiling against wayland > 1.20.0

## 0.3.31

* Web support

## 0.3.28

* Add sdl2 bundled and static feature
* Add HiDPi support for sdl2 sys
* Add DropEvent (file | text) for sdl2 sys
* Add raw-window-handle implementation for sdl2
* Add TextInputEvent
