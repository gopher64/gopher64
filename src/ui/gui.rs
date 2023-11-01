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
                        let mut device = device::Device::new();
                        device::run_game(std::path::Path::new(file.path()), &mut device, false);
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
