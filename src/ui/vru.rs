use crate::gui;
use slint::ComponentHandle;

pub fn prompt_for_match(
    words: &[String],
    weak_vru: &slint::Weak<gui::AppWindow>,
    frame_time: f64,
) -> u16 {
    let mut dedup_words = words.to_owned();
    dedup_words.sort();
    dedup_words.dedup();

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    weak_vru
        .upgrade_in_event_loop(move |_handle| {
            let vru_dialog = gui::VruDialog::new().unwrap();
            let vru_dialog_weak = vru_dialog.as_weak();

            let tx_clicked = tx.clone();

            vru_dialog.on_vru_button_clicked(move |chosen_word| {
                tx_clicked.try_send(chosen_word.to_string()).unwrap();
                vru_dialog_weak.unwrap().window().hide().unwrap();
            });

            vru_dialog.window().on_close_requested(move || {
                tx.try_send("".to_string()).unwrap();
                slint::CloseRequestResponse::HideWindow
            });

            let words_vec = slint::VecModel::default();
            for word in dedup_words {
                words_vec.push(word.into());
            }
            let words_model: std::rc::Rc<slint::VecModel<slint::SharedString>> =
                std::rc::Rc::new(words_vec);
            vru_dialog.set_words(slint::ModelRc::from(words_model));

            vru_dialog.show().unwrap();
        })
        .unwrap();

    let mut result = rx.try_recv();
    while result.is_err() {
        result = rx.try_recv();
        std::thread::sleep(std::time::Duration::from_secs_f64(frame_time));
        unsafe { sdl3_sys::events::SDL_PumpEvents() }; // so the OS doesn't complain about the game window being frozen
    }
    for (i, v) in words.iter().enumerate() {
        if *v == *result.as_ref().unwrap() {
            return i as u16;
        }
    }
    0x7FFF
}
