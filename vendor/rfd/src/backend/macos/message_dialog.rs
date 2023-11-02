use std::ops::DerefMut;

use crate::backend::DialogFutureType;
use crate::message_dialog::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

use super::{
    modal_future::ModalFuture,
    utils::{run_on_main, FocusManager, PolicyManager},
    AsModal,
};

use super::utils::{INSWindow, NSWindow};
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use objc_foundation::{INSString, NSString};

use objc_id::Id;

#[repr(i64)]
#[derive(Debug, PartialEq)]
enum NSAlertStyle {
    Warning = 0,
    Informational = 1,
    Critical = 2,
}

#[repr(i64)]
#[derive(Debug, PartialEq)]
enum NSAlertReturn {
    FirstButton = 1000,
    SecondButton = 1001,
    ThirdButton = 1002,
}

pub struct NSAlert {
    buttons: MessageButtons,
    alert: Id<Object>,
    _focus_manager: FocusManager,
    _policy_manager: PolicyManager,
}

impl NSAlert {
    pub fn new(opt: MessageDialog) -> Self {
        let _policy_manager = PolicyManager::new();

        let alert: *mut Object = unsafe { msg_send![class!(NSAlert), new] };

        let level = match opt.level {
            MessageLevel::Info => NSAlertStyle::Informational,
            MessageLevel::Warning => NSAlertStyle::Warning,
            MessageLevel::Error => NSAlertStyle::Critical,
        };

        unsafe {
            let _: () = msg_send![alert, setAlertStyle: level as i64];
        }

        let buttons = match &opt.buttons {
            MessageButtons::Ok => vec!["OK".to_owned()],
            MessageButtons::OkCancel => vec!["OK".to_owned(), "Cancel".to_owned()],
            MessageButtons::YesNo => vec!["Yes".to_owned(), "No".to_owned()],
            MessageButtons::YesNoCancel => {
                vec!["Yes".to_owned(), "No".to_owned(), "Cancel".to_owned()]
            }
            MessageButtons::OkCustom(ok_text) => vec![ok_text.to_owned()],
            MessageButtons::OkCancelCustom(ok_text, cancel_text) => {
                vec![ok_text.to_owned(), cancel_text.to_owned()]
            }
            MessageButtons::YesNoCancelCustom(yes_text, no_text, cancel_text) => {
                vec![
                    yes_text.to_owned(),
                    no_text.to_owned(),
                    cancel_text.to_owned(),
                ]
            }
        };

        for button in buttons {
            unsafe {
                let label = NSString::from_str(&button);
                let _: () = msg_send![alert, addButtonWithTitle: label];
            }
        }

        unsafe {
            let text = NSString::from_str(&opt.title);
            let _: () = msg_send![alert, setMessageText: text];
            let text = NSString::from_str(&opt.description);
            let _: () = msg_send![alert, setInformativeText: text];
        }

        let _focus_manager = FocusManager::new();

        Self {
            alert: unsafe { Id::from_retained_ptr(alert) },
            buttons: opt.buttons,
            _focus_manager,
            _policy_manager,
        }
    }

    pub fn run(self) -> MessageDialogResult {
        let ret: i64 = unsafe { msg_send![self.alert, runModal] };
        dialog_result(&self.buttons, ret)
    }
}

fn dialog_result(buttons: &MessageButtons, ret: i64) -> MessageDialogResult {
    match buttons {
        MessageButtons::Ok if ret == NSAlertReturn::FirstButton as i64 => MessageDialogResult::Ok,
        MessageButtons::OkCancel if ret == NSAlertReturn::FirstButton as i64 => {
            MessageDialogResult::Ok
        }
        MessageButtons::OkCancel if ret == NSAlertReturn::SecondButton as i64 => {
            MessageDialogResult::Cancel
        }
        MessageButtons::YesNo if ret == NSAlertReturn::FirstButton as i64 => {
            MessageDialogResult::Yes
        }
        MessageButtons::YesNo if ret == NSAlertReturn::SecondButton as i64 => {
            MessageDialogResult::No
        }
        MessageButtons::YesNoCancel if ret == NSAlertReturn::FirstButton as i64 => {
            MessageDialogResult::Yes
        }
        MessageButtons::YesNoCancel if ret == NSAlertReturn::SecondButton as i64 => {
            MessageDialogResult::No
        }
        MessageButtons::YesNoCancel if ret == NSAlertReturn::ThirdButton as i64 => {
            MessageDialogResult::Cancel
        }
        MessageButtons::OkCustom(custom) if ret == NSAlertReturn::FirstButton as i64 => {
            MessageDialogResult::Custom(custom.to_owned())
        }
        MessageButtons::OkCancelCustom(custom, _) if ret == NSAlertReturn::FirstButton as i64 => {
            MessageDialogResult::Custom(custom.to_owned())
        }
        MessageButtons::OkCancelCustom(_, custom) if ret == NSAlertReturn::SecondButton as i64 => {
            MessageDialogResult::Custom(custom.to_owned())
        }
        MessageButtons::YesNoCancelCustom(custom, _, _)
            if ret == NSAlertReturn::FirstButton as i64 =>
        {
            MessageDialogResult::Custom(custom.to_owned())
        }
        MessageButtons::YesNoCancelCustom(_, custom, _)
            if ret == NSAlertReturn::SecondButton as i64 =>
        {
            MessageDialogResult::Custom(custom.to_owned())
        }
        MessageButtons::YesNoCancelCustom(_, _, custom)
            if ret == NSAlertReturn::ThirdButton as i64 =>
        {
            MessageDialogResult::Custom(custom.to_owned())
        }
        _ => MessageDialogResult::Cancel,
    }
}

impl AsModal for NSAlert {
    fn modal_ptr(&mut self) -> *mut Object {
        self.alert.deref_mut()
    }
}

use crate::backend::MessageDialogImpl;
impl MessageDialogImpl for MessageDialog {
    fn show(self) -> MessageDialogResult {
        objc::rc::autoreleasepool(move || run_on_main(move || NSAlert::new(self).run()))
    }
}

use crate::backend::AsyncMessageDialogImpl;

impl AsyncMessageDialogImpl for MessageDialog {
    fn show_async(self) -> DialogFutureType<MessageDialogResult> {
        let win = self.parent.as_ref().map(NSWindow::from_raw_window_handle);

        let future = ModalFuture::new(
            win,
            move || NSAlert::new(self),
            |dialog, ret| dialog_result(&dialog.buttons, ret),
        );
        Box::pin(future)
    }
}
