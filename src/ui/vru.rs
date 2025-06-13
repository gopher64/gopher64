use crate::ui;

pub fn prompt_for_match(
    words: &[String],
    window_notifier: &tokio::sync::mpsc::Sender<Vec<String>>,
    word_index_receiver: &mut tokio::sync::mpsc::Receiver<String>,
) -> u16 {
    let mut dedup_words = words.to_owned();
    dedup_words.sort();
    dedup_words.dedup();
    window_notifier.try_send(dedup_words).unwrap();
    let mut result = word_index_receiver.try_recv();
    while result.is_err() {
        result = word_index_receiver.try_recv();
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
