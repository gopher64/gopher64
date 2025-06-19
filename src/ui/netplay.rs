use slint::ComponentHandle;

use crate::ui::gui::{NetplayCreate, NetplayJoin};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message;

pub trait ServerNamesSetter {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>);
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>);
}

impl ServerNamesSetter for NetplayCreate {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>) {
        self.set_server_urls(urls);
    }
}

impl ServerNamesSetter for NetplayJoin {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>) {
        self.set_server_urls(urls);
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
                let server_urls_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
                    std::rc::Rc::new(server_urls);
                handle.set_server_names(slint::ModelRc::from(server_names_model));
                handle.set_server_urls(slint::ModelRc::from(server_urls_model));
            })
            .unwrap();
        }
    });
}

pub fn setup_create_window(create_window: &NetplayCreate) {
    let weak = create_window.as_weak();
    populate_server_names(weak);

    create_window.on_get_ping(move |server_url| {
        println!("pinging {server_url}");
        tokio::spawn(async move {
            if let Ok((mut sock, _response)) =
                tokio_tungstenite::connect_async(server_url.to_string()).await
            {
                sock.send(Message::Ping(Vec::new().into())).await.unwrap();
                let start = std::time::Instant::now();

                if let Some(Ok(_response)) = sock.next().await {
                    println!("Time elapsed in response is: {:?}", start.elapsed());
                }
            }
        });
    });

    create_window.show().unwrap();
}

pub fn setup_join_window(join_window: &NetplayJoin) {
    let weak = join_window.as_weak();
    populate_server_names(weak);
    join_window.show().unwrap();
}
