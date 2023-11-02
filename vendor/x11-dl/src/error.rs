// x11-rs: Rust bindings for X11 libraries
// The X11 libraries are available under the MIT license.
// These bindings are public domain.

use std::error::Error;
use std::fmt::{Display, Formatter};

//
// OpenError

//

#[derive(Clone, Debug)]
pub struct OpenError {
    kind: OpenErrorKind,
    detail: String,
}

impl OpenError {
    pub fn detail(&self) -> &str {
        self.detail.as_ref()
    }

    pub fn kind(&self) -> OpenErrorKind {
        self.kind
    }

    pub fn new(kind: OpenErrorKind, detail: String) -> OpenError {
        OpenError { kind, detail }
    }
}

impl Display for OpenError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), ::std::fmt::Error> {
        //try!(f.write_str(self.kind.as_str())); TEST Erle July 2020 (on dev branch)
        f.write_str(self.kind.as_str())?;
        if !self.detail.is_empty() {
            f.write_str(" (")?;
            f.write_str(self.detail.as_ref())?;
            f.write_str(")")?;
        }
        Ok(())
    }
}

impl Error for OpenError {
    fn description(&self) -> &str {
        self.kind.as_str()
    }
}

//
// OpenErrorKind
//

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OpenErrorKind {
    Library,
    Symbol,
}

impl OpenErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OpenErrorKind::Library => "opening library failed",
            OpenErrorKind::Symbol => "loading symbol failed",
        }
    }
}
