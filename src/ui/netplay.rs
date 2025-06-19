use slint::{ComponentHandle, Model};

use crate::ui::gui::{NetplayCreate, NetplayJoin};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message;

trait NetplayPages {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>);
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>);
    fn set_ping(&self, ping: slint::SharedString);
}

impl NetplayPages for NetplayCreate {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>) {
        self.set_server_urls(urls);
    }
    fn set_ping(&self, ping: slint::SharedString) {
        self.set_ping(ping);
    }
}

impl NetplayPages for NetplayJoin {
    fn set_server_names(&self, names: slint::ModelRc<slint::SharedString>) {
        self.set_server_names(names);
    }
    fn set_server_urls(&self, urls: slint::ModelRc<slint::SharedString>) {
        self.set_server_urls(urls);
    }
    fn set_ping(&self, ping: slint::SharedString) {
        self.set_ping(ping);
    }
}

fn populate_server_names<T: ComponentHandle + NetplayPages + 'static>(weak: slint::Weak<T>) {
    let task = reqwest::get("https://m64p.s3.amazonaws.com/servers.json");
    tokio::spawn(async move {
        let response = task.await;
        if let Ok(response) = response {
            let servers: std::collections::HashMap<String, String> = response.json().await.unwrap();

            let weak2 = weak.clone();
            weak.upgrade_in_event_loop(move |handle| {
                let server_names: slint::VecModel<slint::SharedString> = slint::VecModel::default();
                let server_urls: slint::VecModel<slint::SharedString> = slint::VecModel::default();
                for server in servers {
                    server_names.push(server.0.into());
                    server_urls.push(server.1.into());
                }
                update_ping(weak2, server_urls.row_data(0).unwrap().into());
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

fn update_ping<T: ComponentHandle + NetplayPages + 'static>(
    weak: slint::Weak<T>,
    server_url: String,
) {
    weak.upgrade_in_event_loop(move |handle| {
        handle.set_ping("Ping: Unknown".into());
    })
    .unwrap();
    tokio::spawn(async move {
        if let Ok(Ok((mut sock, _response))) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            tokio_tungstenite::connect_async(server_url),
        )
        .await
        {
            sock.send(Message::Ping(Vec::new().into())).await.unwrap();
            let start = std::time::Instant::now();

            if let Some(Ok(_response)) = sock.next().await {
                let elapsed = start.elapsed();
                weak.upgrade_in_event_loop(move |handle| {
                    handle.set_ping(format!("Ping: {:.0} ms", elapsed.as_millis()).into());
                })
                .unwrap();
            }
            sock.close(None).await.unwrap();
        }
    });
}

pub fn setup_create_window(create_window: &NetplayCreate) {
    let weak = create_window.as_weak();
    populate_server_names(weak);
    let weak2 = create_window.as_weak();
    create_window.on_get_ping(move |server_url| {
        update_ping(weak2.clone(), server_url.to_string());
    });

    create_window.show().unwrap();
}

pub fn setup_join_window(join_window: &NetplayJoin) {
    let weak = join_window.as_weak();
    populate_server_names(weak);
    let weak2 = join_window.as_weak();
    join_window.on_get_ping(move |server_url| {
        update_ping(weak2.clone(), server_url.to_string());
    });

    join_window.show().unwrap();
}
