use crate::device;
use crate::ui;
use eframe::egui;

#[derive(Default)]
pub struct GopherEguiApp {
    configure_profile: bool,
    profile_name: String,
}

impl eframe::App for GopherEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.configure_profile {
            egui::Window::new("Configure Input Profile")
                // .open(&mut self.configure_profile)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let name_label = ui.label("Profile Name: ");
                        ui.text_edit_singleline(&mut self.profile_name)
                            .labelled_by(name_label.id);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Configure Profile").clicked() {
                            let profile_name = self.profile_name.clone();
                            execute(async {
                                let mut device = device::Device::new();
                                ui::input::configure_input_profile(&mut device.ui, profile_name);
                            });
                            self.configure_profile = false
                        };
                        if ui.button("Close").clicked() {
                            self.configure_profile = false
                        };
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.configure_profile);

            if ui.button("Open ROM").clicked() {
                // Spawn dialog on main thread
                let task = rfd::AsyncFileDialog::new().pick_file();

                execute(async {
                    let file = task.await;

                    if let Some(file) = file {
                        let running_file = dirs::cache_dir()
                            .unwrap()
                            .join("gopher64")
                            .join("game_running");
                        if running_file.exists() {
                            return;
                        }
                        let _ = std::fs::File::create(running_file.clone());
                        let mut device = device::Device::new();
                        device::run_game(std::path::Path::new(file.path()), &mut device, false);
                        let _ = std::fs::remove_file(running_file.clone());
                    }
                });
            }

            if ui.button("Configure Input Profile").clicked() {
                let running_file = dirs::cache_dir()
                    .unwrap()
                    .join("gopher64")
                    .join("game_running");
                if running_file.exists() {
                    return;
                }
                self.configure_profile = true;
            }
        });
    }
}

fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}
