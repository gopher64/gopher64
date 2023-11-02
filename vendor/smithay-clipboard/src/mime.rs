/// List of allowed mimes.
static ALLOWED_MIME_TYPES: [&str; 2] = ["text/plain;charset=utf-8", "UTF8_STRING"];

/// Mime type supported by clipboard.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum MimeType {
    /// text/plain;charset=utf-8 mime type.
    ///
    /// The primary mime type used by most clients
    TextPlainUtf8 = 0,
    /// UTF8_STRING mime type.
    ///
    /// Some X11 clients are using only this mime type, so we
    /// should have it as a fallback just in case.
    Utf8String = 1,
}

impl MimeType {
    /// Find first allowed mime type among the `offered_mime_types`.
    ///
    /// `find_allowed()` searches for mime type clipboard supports, if we have a match,
    /// returns `Some(MimeType)`, otherwise `None`.
    pub fn find_allowed(offered_mime_types: &[String]) -> Option<Self> {
        for offered_mime_type in offered_mime_types.iter() {
            if offered_mime_type == ALLOWED_MIME_TYPES[Self::TextPlainUtf8 as usize] {
                return Some(Self::TextPlainUtf8);
            } else if offered_mime_type == ALLOWED_MIME_TYPES[Self::Utf8String as usize] {
                return Some(Self::Utf8String);
            }
        }

        None
    }
}

impl ToString for MimeType {
    fn to_string(&self) -> String {
        String::from(ALLOWED_MIME_TYPES[*self as usize])
    }
}

/// Normalize CR and CRLF into LF.
///
/// 'text' mime types require CRLF line ending according to
/// RFC-2046, however the platform line terminator and what applications
/// expect is LF.
pub fn normalize_to_lf(text: String) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
