// Copyright 2019  Sebastian Wiesner <sebastian@swsnr.de>

// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at

// 	http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.

//! [gethostname()][ghn] for all platforms.
//!
//! ```
//! use gethostname::gethostname;
//!
//! println!("Hostname: {:?}", gethostname());
//! ```
//!
//! [ghn]: http://pubs.opengroup.org/onlinepubs/9699919799/functions/gethostname.html

#![deny(warnings, missing_docs, clippy::all)]

use std::ffi::OsString;
use std::io::Error;

/// Get the standard host name for the current machine.
///
/// On Unix simply wrap POSIX [gethostname] in a safe interface.  On Windows
/// return the DNS host name of the local computer, as returned by
/// [GetComputerNameExW] with `ComputerNamePhysicalDnsHostname` as `NameType`.
///
/// This function panics if the buffer allocated for the hostname result of the
/// operating system is too small; however we take great care to allocate a
/// buffer of sufficient size:
///
/// * On Unix we allocate the buffer using the maximum permitted hostname size,
///     as returned by [sysconf] via `sysconf(_SC_HOST_NAME_MAX)`, plus an extra
///     byte for the trailing NUL byte.  A hostname cannot exceed this limit, so
///     this function can't realistically panic.
/// * On Windows we call `GetComputerNameExW` with a NULL buffer first, which
///     makes it return the length of the current host name.  We then use this
///     length to allocate a buffer for the actual result; this leaves a tiny
///     tiny race condition in case the hostname changes to a longer name right
///     in between those two calls but that's a risk we don't consider of any
///     practical relevance.
///
/// Hence _if_ this function does panic please [report an issue][new].
///
/// [gethostname]: http://pubs.opengroup.org/onlinepubs/9699919799/functions/gethostname.html
/// [sysconf]: http://pubs.opengroup.org/onlinepubs/9699919799/functions/sysconf.html
/// [GetComputerNameExW]: https://docs.microsoft.com/en-us/windows/desktop/api/sysinfoapi/nf-sysinfoapi-getcomputernameexw
/// [new]: https://codeberg.org/flausch/gethostname.rs/issues/new
pub fn gethostname() -> OsString {
    gethostname_impl()
}

#[cfg(unix)]
#[inline]
fn gethostname_impl() -> OsString {
    use libc::{c_char, sysconf, _SC_HOST_NAME_MAX};
    use std::os::unix::ffi::OsStringExt;
    // Get the maximum size of host names on this system, and account for the
    // trailing NUL byte.
    let hostname_max = unsafe { sysconf(_SC_HOST_NAME_MAX) };
    let mut buffer = vec![0; (hostname_max as usize) + 1];
    let returncode = unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut c_char, buffer.len()) };
    if returncode != 0 {
        // There are no reasonable failures, so lets panic
        panic!(
            "gethostname failed: {}
    Please report an issue to <https://codeberg.org/flausch/gethostname.rs/issues>!",
            Error::last_os_error()
        );
    }
    // We explicitly search for the trailing NUL byte and cap at the buffer
    // length: If the buffer's too small (which shouldn't happen since we
    // explicitly use the max hostname size above but just in case) POSIX
    // doesn't specify whether there's a NUL byte at the end, so if we didn't
    // check we might read from memory that's not ours.
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    buffer.resize(end, 0);
    OsString::from_vec(buffer)
}

#[cfg(windows)]
#[inline]
fn gethostname_impl() -> OsString {
    use std::os::windows::ffi::OsStringExt;
    use winapi::ctypes::{c_ulong, wchar_t};
    use winapi::um::sysinfoapi::{ComputerNamePhysicalDnsHostname, GetComputerNameExW};

    let mut buffer_size: c_ulong = 0;

    unsafe {
        // This call always fails with ERROR_MORE_DATA, because we pass NULL to
        // get the required buffer size.
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            std::ptr::null_mut(),
            &mut buffer_size,
        )
    };

    let mut buffer = vec![0 as wchar_t; buffer_size as usize];
    let returncode = unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            buffer.as_mut_ptr() as *mut wchar_t,
            &mut buffer_size,
        )
    };
    // GetComputerNameExW returns a non-zero value on success!
    if returncode == 0 {
        panic!(
            "GetComputerNameExW failed to read hostname: {}
Please report this issue to <https://codeberg.org/flausch/gethostname.rs/issues>!",
            Error::last_os_error()
        );
    }

    let end = buffer
        .iter()
        .position(|&b| b == 0)
        .unwrap_or_else(|| buffer.len());
    OsString::from_wide(&buffer[0..end])
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::process::Command;

    #[test]
    fn gethostname_matches_system_hostname() {
        let output = Command::new("hostnamectl")
            .arg("hostname")
            .output()
            .expect("failed to get hostname");
        let hostname = String::from_utf8_lossy(&output.stdout);
        // Convert both sides to lowercase; hostnames are case-insensitive
        // anyway.
        assert_eq!(
            super::gethostname().into_string().unwrap().to_lowercase(),
            hostname.trim_end().to_lowercase()
        );
    }

    #[test]
    #[ignore]
    fn gethostname_matches_fixed_hostname() {
        assert_eq!(
            super::gethostname().into_string().unwrap().to_lowercase(),
            "hostname-for-testing"
        );
    }
}
