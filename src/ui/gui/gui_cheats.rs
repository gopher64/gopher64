use crate::{device, ui::gui::GopherEguiApp};
use eframe::egui;

pub struct GuiCheats {
    pub cheats_db: serde_json::Value,
    pub cheats_receiver: Option<tokio::sync::mpsc::Receiver<serde_json::Value>>,
}

pub fn dialog(app: &mut GopherEguiApp, ctx: &egui::Context) {
    egui::Window::new("Manage Cheats").show(ctx, |ui| {
        if ui.button("Open ROM").clicked() {
            let select_rom = rfd::AsyncFileDialog::new()
                .set_title("Select ROM")
                .pick_file();
            let cheats_db = app.cheats.cheats_db.clone();
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            app.cheats.cheats_receiver = Some(rx);
            let gui_ctx = ctx.clone();
            tokio::spawn(async move {
                let file = select_rom.await;
                if file.is_some() {
                    let rom = device::get_rom_contents(file.unwrap().path());
                    let crc1 = u32::from_be_bytes(rom[0x10..0x10 + 4].try_into().unwrap());
                    let crc2 = u32::from_be_bytes(rom[0x14..0x14 + 4].try_into().unwrap());
                    let country_code = rom[0x3E];
                    let key = format!("{:08X}-{:08X}-C:{:02X}", crc1, crc2, country_code);
                    let cheats = cheats_db.get(key).unwrap();
                    tx.send(cheats.clone()).await.unwrap();
                    gui_ctx.request_repaint();
                }
            });
        }

        if app.cheats.cheats_receiver.is_some() {
            let cheats = app.cheats.cheats_receiver.as_mut().unwrap().try_recv();
            if cheats.is_ok() {
                let cheats = cheats.unwrap();
                for cheat in cheats.as_object().unwrap() {
                    if cheat
                        .1
                        .as_object()
                        .unwrap()
                        .get("hasOptions")
                        .unwrap()
                        .as_bool()
                        .unwrap()
                    {
                        for option in cheat
                            .1
                            .as_object()
                            .unwrap()
                            .get("options")
                            .unwrap()
                            .as_object()
                            .unwrap()
                        {
                            println!("{} option {}", cheat.0, option.0);
                        }
                    } else {
                        println!("{}", cheat.0);
                    }
                }
                app.cheats.cheats_receiver = None;
            }
        }
        if ui.button("Close").clicked() {
            app.show_cheats = false;
        }
    });
}
