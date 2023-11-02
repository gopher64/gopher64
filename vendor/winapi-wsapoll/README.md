# Safe bindings to `WSAPoll`

[![Workflow Status](https://github.com/psychon/winapi-wsapoll/workflows/ci/badge.svg)](https://github.com/psychon/winapi-wsapoll/actions)
[![Crate](https://img.shields.io/crates/v/winapi-wsapoll.svg)](https://crates.io/crates/winapi-wsapoll)
[![API](https://docs.rs/winapi-wsapoll/badge.svg)](https://docs.rs/winapi-wsapoll)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.34+-lightgray.svg)
[![License](https://img.shields.io/crates/l/winapi-wsapoll.svg)](https://github.com/psychon/winapi-wsapoll#license)

You want to `#![forbid(unsafe_code)]` in your crate? But you also need access to
`WSAPoll()`? Then this crate is for you! It exports a safe `wsa_poll()` function
that you can use.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
