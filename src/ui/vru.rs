use eframe::egui;

use crate::ui;

pub fn prompt_for_match(
    words: &Vec<String>,
    window_notifier: &std::sync::mpsc::Sender<Vec<String>>,
    word_index_notifier: &std::sync::mpsc::Receiver<String>,
    gui_ctx: &egui::Context,
) -> u16 {
    let mut dedup_words = words.clone();
    dedup_words.sort();
    dedup_words.dedup();
    window_notifier.send(dedup_words).unwrap();
    gui_ctx.request_repaint(); // this is so the window pops up right away
    let mut result = word_index_notifier.try_recv();
    while !result.is_ok() {
        result = word_index_notifier.try_recv();
        std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / 60.0));
        ui::video::update_screen(); // so the OS doesn't complain about the game window being frozen
    }
    for (i, v) in words.iter().enumerate() {
        if *v == *result.as_ref().unwrap() {
            return i as u16;
        }
    }
    0x7FFF
}
