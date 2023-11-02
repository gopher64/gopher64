use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};

mod application;
mod focus_manager;
mod policy_manager;
mod url;
mod window;

pub use application::{INSApplication, NSApplication};
pub use focus_manager::FocusManager;
pub use policy_manager::PolicyManager;
pub use url::{INSURL, NSURL};
pub use window::{INSWindow, NSWindow};

#[allow(non_upper_case_globals)]
pub const nil: *mut Object = 0 as *mut _;

pub fn is_main_thread() -> bool {
    unsafe { msg_send![class!(NSThread), isMainThread] }
}

pub fn activate_cocoa_multithreading() {
    unsafe {
        let thread: *mut Object = msg_send![class!(NSThread), new];
        let _: () = msg_send![thread, start];
    }
}

pub fn run_on_main<R: Send, F: FnOnce() -> R + Send>(run: F) -> R {
    if is_main_thread() {
        run()
    } else {
        let app = NSApplication::shared_application();
        if app.is_running() {
            let main = dispatch::Queue::main();
            main.exec_sync(run)
        } else {
            panic!("You are running RFD in NonWindowed environment, it is impossible to spawn dialog from thread different than main in this env.");
        }
    }
}
