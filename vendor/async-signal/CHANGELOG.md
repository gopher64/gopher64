# Version 0.2.4

- Add `LICENSE` files to this crate. (#23)

# Version 0.2.3

- Remove the `signalfd` backend, as it offered little to no advantages over the pipe-based backend and it didn't catch signals sometimes. (#20)

# Version 0.2.2

- Fix build error on Android. (#18)

# Version 0.2.1

- Add support for the signalfd mechanism on Linux. (#5)
- Add support for Windows. (#6)
- Bump MSRV to 1.63. (#8)

# Version 0.2.0

- Initial release
