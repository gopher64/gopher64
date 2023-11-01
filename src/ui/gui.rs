use crate::device;
use eframe::egui;

#[derive(Default)]
pub struct GopherEguiApp {}

impl eframe::App for GopherEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
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

            ui.button("Configure Input Profile").clicked();
        });
    }
}

fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}
