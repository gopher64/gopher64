use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};

#[repr(i32)]
#[derive(Debug, PartialEq)]
enum ApplicationActivationPolicy {
    //Regular = 0,
    Accessory = 1,
    Prohibited = 2,
    //Error = -1,
}

pub struct PolicyManager {
    initial_policy: i32,
}

impl PolicyManager {
    pub fn new() -> Self {
        unsafe {
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let initial_policy: i32 = msg_send![app, activationPolicy];

            if initial_policy == ApplicationActivationPolicy::Prohibited as i32 {
                let new_pol = ApplicationActivationPolicy::Accessory as i32;
                let _: () = msg_send![app, setActivationPolicy: new_pol];
            }

            Self { initial_policy }
        }
    }
}

impl Drop for PolicyManager {
    fn drop(&mut self) {
        unsafe {
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            // Restore initial pol
            let _: () = msg_send![app, setActivationPolicy: self.initial_policy];
        }
    }
}
