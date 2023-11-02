mod file_dialog;
mod message_dialog;

mod modal_future;

mod utils;

use objc::runtime::Object;
trait AsModal {
    fn modal_ptr(&mut self) -> *mut Object;
}
