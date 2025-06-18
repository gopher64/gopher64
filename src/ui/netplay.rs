use slint::ComponentHandle;

use crate::ui::gui::{NetplayCreate, NetplayJoin};

pub trait ServerNamesSetter {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>);
}

impl ServerNamesSetter for NetplayCreate {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
}

impl ServerNamesSetter for NetplayJoin {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
}

pub fn populate_server_names<T: ComponentHandle + ServerNamesSetter + 'static>(
    weak: slint::Weak<T>,
) {
    let task = reqwest::get("https://m64p.s3.amazonaws.com/servers.json");
    tokio::spawn(async move {
        let response = task.await;
        if let Ok(response) = response {
            let servers: std::collections::HashMap<String, String> = response.json().await.unwrap();

            weak.upgrade_in_event_loop(move |handle| {
                let server_names: slint::VecModel<slint::SharedString> = slint::VecModel::default();
                let server_urls: slint::VecModel<slint::SharedString> = slint::VecModel::default();
                for server in servers {
                    server_names.push(server.0.into());
                    server_urls.push(server.1.into());
                }

                let server_names_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
                    std::rc::Rc::new(server_names);
                let _server_urls_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
                    std::rc::Rc::new(server_urls);
                handle.set_server_names(slint::ModelRc::from(server_names_model));
            })
            .unwrap();
        }
    });
}

pub fn setup_create_window(create_window: &NetplayCreate) {
    let weak = create_window.as_weak();
    populate_server_names(weak);

    create_window.show().unwrap();
}

pub fn setup_join_window(join_window: &NetplayJoin) {
    let weak = join_window.as_weak();
    populate_server_names(weak);
    join_window.show().unwrap();
}
