use crate::{Browser, BrowserOptions, Error, ErrorKind, Result, TargetType};
use objc::{class, msg_send, runtime::Object, sel, sel_impl};

/// Deal with opening of browsers on iOS
pub(super) fn open_browser_internal(
    _browser: Browser,
    target: &TargetType,
    options: &BrowserOptions,
) -> Result<()> {
    // ensure we're opening only http/https urls, failing otherwise
    let url = target.get_http_url()?;

    // always return true for a dry run
    if options.dry_run {
        return Ok(());
    }

    unsafe {
        let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
        if app.is_null() {
            return Err(Error::new(
                ErrorKind::Other,
                "UIApplication is null, can't open url",
            ));
        }
        let url_cstr = std::ffi::CString::new(url).unwrap();
        // Create ns string class from our string
        let url_string: *mut Object = msg_send![class!(NSString), stringWithUTF8String: url_cstr];
        // Create NSURL object with given string
        let url_object: *mut Object = msg_send![class!(NSURL), URLWithString: url_string];
        // No completion handler
        let null_ptr = std::ptr::null_mut::<Object>();
        // Open url
        let () = msg_send![app, openURL: url_object options: {} completionHandler: null_ptr];
        Ok(())
    }
}
