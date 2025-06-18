use slint::ComponentHandle;

use crate::ui::gui::{NetplayCreate, NetplayJoin};

pub fn setup_create_window(create_window: &NetplayCreate) {
    create_window.show().unwrap();
}

pub fn setup_join_window(join_window: &NetplayJoin) {
    join_window.show().unwrap();
}
