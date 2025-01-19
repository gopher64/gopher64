use crate::device;
use crate::ui::gui::GopherEguiApp;
use eframe::egui;

pub struct Netplay {
    pub session_name: String,
    pub password: String,
    pub player_name: String,
    pub server: (String, String),
    pub servers: std::collections::HashMap<String, String>,
    pub server_receiver:
        Option<tokio::sync::mpsc::Receiver<std::collections::HashMap<String, String>>>,
    pub broadcast_socket: Option<std::net::UdpSocket>,
    pub broadcast_timer: Option<std::time::Instant>,
}

pub fn netplay_create(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Create Netplay Session").show(ctx, |ui| {
        egui::Grid::new("button_grid").show(ui, |ui| {
            let profile_name_label = ui.label("Profile Name:");
            let mut size = ui.spacing().interact_size;
            size.x = 200.0;
            ui.add_sized(size, |ui: &mut egui::Ui| {
                ui.text_edit_singleline(&mut app.netplay.session_name)
                    .labelled_by(profile_name_label.id)
            });

            ui.end_row();

            let password_label = ui.label("Password (Optional):");

            ui.text_edit_singleline(&mut app.netplay.password)
                .labelled_by(password_label.id);

            ui.end_row();

            ui.label("ROM");
            if ui.button("Open ROM").clicked() {
                // Spawn dialog on main thread
                let task = rfd::AsyncFileDialog::new().pick_file();
                tokio::spawn(async {
                    let file = task.await;

                    if let Some(file) = file {
                        let _rom_contents = device::get_rom_contents(file.path());
                    }
                });
            }

            ui.end_row();

            let player_name_label = ui.label("Player Name:");

            ui.text_edit_singleline(&mut app.netplay.player_name)
                .labelled_by(player_name_label.id);

            ui.end_row();

            ui.label("Server:");

            if app.netplay.servers.is_empty() {
                if app.netplay.broadcast_socket.is_none() {
                    app.netplay.broadcast_socket = Some(
                        std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
                            .expect("couldn't bind to address"),
                    );
                    let socket = app.netplay.broadcast_socket.as_ref().unwrap();
                    socket
                        .set_broadcast(true)
                        .expect("set_broadcast call failed");
                    socket
                        .set_nonblocking(true)
                        .expect("could not set up socket");
                    let data: [u8; 1] = [1];
                    socket
                        .send_to(&data, (std::net::Ipv4Addr::BROADCAST, 45000))
                        .expect("couldn't send data");
                    app.netplay.broadcast_timer =
                        Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
                }
                if app.netplay.server_receiver.is_none() {
                    let (tx, rx) = tokio::sync::mpsc::channel(1);
                    app.netplay.server_receiver = Some(rx);

                    tokio::spawn(async move {
                        if let Ok(response) =
                            reqwest::get("https://m64p.s3.amazonaws.com/servers.json").await
                        {
                            if let Ok(servers) = response
                                .json::<std::collections::HashMap<String, String>>()
                                .await
                            {
                                let _ = tx.send(servers).await;
                            }
                        }
                    });
                }
            }
            if app.netplay.broadcast_timer.is_some()
                && std::time::Instant::now() > app.netplay.broadcast_timer.unwrap()
            {
                app.netplay.broadcast_timer = None;
            }
            if app.netplay.broadcast_socket.is_some() && app.netplay.broadcast_timer.is_some() {
                let mut buffer = [0; 1024];
                let result = app
                    .netplay
                    .broadcast_socket
                    .as_ref()
                    .unwrap()
                    .recv_from(&mut buffer);
                if result.is_ok() {
                    let (amt, _src) = result.unwrap();
                    let data: std::collections::HashMap<String, String> =
                        serde_json::from_slice(&buffer[..amt]).unwrap();
                    for server in data.iter() {
                        let (server_name, server_ip) = server;
                        app.netplay
                            .servers
                            .insert(server_name.to_string(), server_ip.to_string());
                        app.netplay.server = (server.0.clone(), server.1.clone());
                    }
                    app.netplay.broadcast_socket = None;
                }
            }
            if app.netplay.server_receiver.is_some() {
                let result = app.netplay.server_receiver.as_mut().unwrap().try_recv();
                if result.is_ok() {
                    app.netplay.servers.extend(result.unwrap());
                    app.netplay.server_receiver = None;
                    if app.netplay.server.0.is_empty() {
                        let first_server = app.netplay.servers.iter().next().unwrap();
                        app.netplay.server = (first_server.0.clone(), first_server.1.clone());
                    }
                }
            }

            egui::ComboBox::from_id_salt("server-combobox")
                .selected_text(app.netplay.server.0.to_string())
                .show_ui(ui, |ui| {
                    for server in app.netplay.servers.iter() {
                        ui.selectable_value(
                            &mut app.netplay.server,
                            (server.0.clone(), server.1.clone()),
                            server.0,
                        );
                    }
                });

            ui.end_row();

            if ui.button("Create Session").clicked() {
                app.netplay_create = false;
                app.netplay_wait = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    app.netplay_create = false
                };
            })
        });
    });
}

pub fn netplay_join(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Join Netplay Session").show(ctx, |ui| {
        if ui.button("Close").clicked() {
            app.netplay_join = false
        };
    });
}

pub fn netplay_wait(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Pending Netplay Session").show(ctx, |ui| {
        if ui.button("Close").clicked() {
            app.netplay_wait = false
        };
    });
}
