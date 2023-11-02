//! Utilities for parsing X11 display strings.

#![cfg(feature = "std")]

mod connect_instruction;
pub use connect_instruction::ConnectAddress;

use alloc::string::{String, ToString};

/// A parsed X11 display string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDisplay {
    /// The hostname of the computer we nned to connect to.
    ///
    /// This is an empty string if we are connecting to the
    /// local host.
    pub host: String,
    /// The protocol we are communicating over.
    ///
    /// This is `None` if the protocol may be determined
    /// automatically.
    pub protocol: Option<String>,
    /// The index of the display we are connecting to.
    pub display: u16,
    /// The index of the screen that we are using as the
    /// default screen.
    pub screen: u16,
}

impl ParsedDisplay {
    /// Get an iterator over `ConnectAddress`es from this parsed display for connecting
    /// to the server.
    pub fn connect_instruction(&self) -> impl Iterator<Item = ConnectAddress<'_>> {
        connect_instruction::connect_addresses(self)
    }
}

/// Parse an X11 display string.
///
/// If `dpy_name` is `None`, the display is parsed from the environment variable `DISPLAY`.
pub fn parse_display(dpy_name: Option<&str>) -> Option<ParsedDisplay> {
    // If no dpy name was provided, use the env var. If no env var exists, return None.
    match dpy_name {
        Some(dpy_name) => parse_display_impl(dpy_name),
        None => parse_display_impl(&std::env::var("DISPLAY").ok()?),
    }
}

fn parse_display_impl(dpy_name: &str) -> Option<ParsedDisplay> {
    // Everything up to the last '/' is the protocol. This part is optional.
    let (protocol, remaining) = if let Some(pos) = dpy_name.rfind('/') {
        (Some(&dpy_name[..pos]), &dpy_name[pos + 1..])
    } else {
        (None, dpy_name)
    };

    // Everything up to the last ':' is the host. This part is required.
    let pos = remaining.rfind(':')?;
    let (host, remaining) = (&remaining[..pos], &remaining[pos + 1..]);

    // The remaining part is display.screen. The display is required and the screen optional.
    let (display, screen) = match remaining.find('.') {
        Some(pos) => (&remaining[..pos], &remaining[pos + 1..]),
        None => (remaining, "0"),
    };

    // Parse the display and screen number
    let (display, screen) = (display.parse().ok()?, screen.parse().ok()?);

    let host = host.to_string();
    let protocol = protocol.map(|p| p.to_string());
    Some(ParsedDisplay {
        host,
        protocol,
        display,
        screen,
    })
}

#[cfg(test)]
mod test {
    use super::{parse_display, ParsedDisplay};
    use alloc::string::ToString;

    fn do_parse_display(input: &str) -> Option<ParsedDisplay> {
        std::env::set_var("DISPLAY", input);
        let result1 = parse_display(None);

        std::env::remove_var("DISPLAY");
        let result2 = parse_display(Some(input));

        assert_eq!(result1, result2);
        result1
    }

    // The tests modify environment variables. This is process-global. Thus, the tests in this
    // module cannot be run concurrently. We achieve this by having only a single test functions
    // that calls all other functions.
    #[test]
    fn test_parsing() {
        test_missing_input();
        xcb_good_cases();
        xcb_bad_cases();
        own_good_cases();
    }

    fn test_missing_input() {
        std::env::remove_var("DISPLAY");
        assert_eq!(parse_display(None), None);
    }

    fn own_good_cases() {
        // The XCB test suite does not test protocol parsing
        for (input, output) in &[
            (
                "foo/bar:1",
                ParsedDisplay {
                    host: "bar".to_string(),
                    protocol: Some("foo".to_string()),
                    display: 1,
                    screen: 0,
                },
            ),
            (
                "foo/bar:1.2",
                ParsedDisplay {
                    host: "bar".to_string(),
                    protocol: Some("foo".to_string()),
                    display: 1,
                    screen: 2,
                },
            ),
            (
                "a:b/c/foo:bar:1.2",
                ParsedDisplay {
                    host: "foo:bar".to_string(),
                    protocol: Some("a:b/c".to_string()),
                    display: 1,
                    screen: 2,
                },
            ),
        ] {
            assert_eq!(
                do_parse_display(input).as_ref(),
                Some(output),
                "Failed parsing correctly: {}",
                input
            );
        }
    }

    // Based on libxcb's test suite; (C) 2001-2006 Bart Massey, Jamey Sharp, and Josh Triplett
    fn xcb_good_cases() {
        for (input, output) in &[
            // unix
            (
                ":0",
                ParsedDisplay {
                    host: "".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                ":1",
                ParsedDisplay {
                    host: "".to_string(),
                    protocol: None,
                    display: 1,
                    screen: 0,
                },
            ),
            (
                ":0.1",
                ParsedDisplay {
                    host: "".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
            // ip
            (
                "x.org:0",
                ParsedDisplay {
                    host: "x.org".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "expo:0",
                ParsedDisplay {
                    host: "expo".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "bigmachine:1",
                ParsedDisplay {
                    host: "bigmachine".to_string(),
                    protocol: None,
                    display: 1,
                    screen: 0,
                },
            ),
            (
                "hydra:0.1",
                ParsedDisplay {
                    host: "hydra".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
            // ipv4
            (
                "198.112.45.11:0",
                ParsedDisplay {
                    host: "198.112.45.11".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "198.112.45.11:0.1",
                ParsedDisplay {
                    host: "198.112.45.11".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
            // ipv6
            (
                ":::0",
                ParsedDisplay {
                    host: "::".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "1:::0",
                ParsedDisplay {
                    host: "1::".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "::1:0",
                ParsedDisplay {
                    host: "::1".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "::1:0.1",
                ParsedDisplay {
                    host: "::1".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
            (
                "::127.0.0.1:0",
                ParsedDisplay {
                    host: "::127.0.0.1".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "::ffff:127.0.0.1:0",
                ParsedDisplay {
                    host: "::ffff:127.0.0.1".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "2002:83fc:3052::1:0",
                ParsedDisplay {
                    host: "2002:83fc:3052::1".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "2002:83fc:3052::1:0.1",
                ParsedDisplay {
                    host: "2002:83fc:3052::1".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
            (
                "[::]:0",
                ParsedDisplay {
                    host: "[::]".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "[1::]:0",
                ParsedDisplay {
                    host: "[1::]".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "[::1]:0",
                ParsedDisplay {
                    host: "[::1]".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "[::1]:0.1",
                ParsedDisplay {
                    host: "[::1]".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
            (
                "[::127.0.0.1]:0",
                ParsedDisplay {
                    host: "[::127.0.0.1]".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "[2002:83fc:d052::1]:0",
                ParsedDisplay {
                    host: "[2002:83fc:d052::1]".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "[2002:83fc:d052::1]:0.1",
                ParsedDisplay {
                    host: "[2002:83fc:d052::1]".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
            // decnet
            (
                "myws::0",
                ParsedDisplay {
                    host: "myws:".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "big::0",
                ParsedDisplay {
                    host: "big:".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 0,
                },
            ),
            (
                "hydra::0.1",
                ParsedDisplay {
                    host: "hydra:".to_string(),
                    protocol: None,
                    display: 0,
                    screen: 1,
                },
            ),
        ] {
            assert_eq!(
                do_parse_display(input).as_ref(),
                Some(output),
                "Failed parsing correctly: {}",
                input
            );
        }
    }

    // Based on libxcb's test suite; (C) 2001-2006 Bart Massey, Jamey Sharp, and Josh Triplett
    fn xcb_bad_cases() {
        for input in &[
            "",
            ":",
            "::",
            ":::",
            ":.",
            ":a",
            ":a.",
            ":0.",
            ":.a",
            ":.0",
            ":0.a",
            ":0.0.",
            "127.0.0.1",
            "127.0.0.1:",
            "127.0.0.1::",
            "::127.0.0.1",
            "::127.0.0.1:",
            "::127.0.0.1::",
            "::ffff:127.0.0.1",
            "::ffff:127.0.0.1:",
            "::ffff:127.0.0.1::",
            "localhost",
            "localhost:",
            "localhost::",
        ] {
            assert_eq!(
                do_parse_display(input),
                None,
                "Unexpectedly parsed: {}",
                input
            );
        }
    }
}
