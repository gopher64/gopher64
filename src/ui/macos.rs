//! macOS-specific integration.

use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter, NSString,
};

// Apple Event codes (FourCharCode), see AppleEvents.h / AEDataModel.h.
const K_CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt"); // kCoreEventClass
const K_AE_OPEN_DOCUMENTS: u32 = u32::from_be_bytes(*b"odoc"); // kAEOpenDocuments
const KEY_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----"); // keyDirectObject

struct Ivars {
    on_open: Box<dyn Fn(PathBuf)>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "Gopher64OpenDocumentsHandler"]
    #[ivars = Ivars]
    struct OpenDocumentsHandler;

    impl OpenDocumentsHandler {
        // AppKit installs its own Apple Event handlers during `finishLaunching`
        // (inside the event loop), which would replace ours if we registered it
        // earlier. "Will finish launching" is the point Apple recommends for
        // installing custom handlers. winit owns the app delegate, so we listen
        // for the notification instead.
        #[unsafe(method(applicationWillFinishLaunching:))]
        fn application_will_finish_launching(&self, _notification: &NSNotification) {
            let manager = NSAppleEventManager::sharedAppleEventManager();
            unsafe {
                manager.setEventHandler_andSelector_forEventClass_andEventID(
                    self,
                    sel!(handleOpenDocuments:withReplyEvent:),
                    K_CORE_EVENT_CLASS,
                    K_AE_OPEN_DOCUMENTS,
                );
            }
        }

        #[unsafe(method(handleOpenDocuments:withReplyEvent:))]
        fn handle_open_documents(
            &self,
            event: &NSAppleEventDescriptor,
            _reply: &NSAppleEventDescriptor,
        ) {
            let Some(list) = event.paramDescriptorForKeyword(KEY_DIRECT_OBJECT) else {
                return;
            };
            // Only the first file is used, Gopher64 runs one ROM at a time.
            let Some(url) = list
                .descriptorAtIndex(1)
                .and_then(|item| item.fileURLValue())
            else {
                return;
            };
            let Some(path) = url.to_file_path() else {
                return;
            };
            (self.ivars().on_open)(path);
        }
    }
);

/// Handles ROMs opened via Finder (double-click, "Open With") or `open -a Gopher64 <rom>`.
///
/// macOS delivers these as an "open documents" Apple Event. Files opened this way
/// get a sandbox extension, so the ROM can be read without extra permissions.
/// Must be called on the main thread before the event loop starts.
pub fn register_open_documents_handler(on_open: impl Fn(PathBuf) + 'static) {
    let mtm = MainThreadMarker::new().expect("must be called on the main thread");
    let handler = OpenDocumentsHandler::alloc(mtm).set_ivars(Ivars {
        on_open: Box::new(on_open),
    });
    let handler: Retained<OpenDocumentsHandler> = unsafe { msg_send![super(handler), init] };

    let center = NSNotificationCenter::defaultCenter();
    let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
    unsafe {
        center.addObserver_selector_name_object(
            &handler,
            sel!(applicationWillFinishLaunching:),
            Some(&name),
            None,
        );
    }

    // Neither NSNotificationCenter nor NSAppleEventManager retain the handler,
    // so keep it alive for the lifetime of the app.
    std::mem::forget(handler);
}
