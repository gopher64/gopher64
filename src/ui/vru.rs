use crate::retroachievements;
use crate::ui::gui;
use slint::ComponentHandle;

pub fn prompt_for_match(words: &[String], frame_time: f64) -> u16 {
    let mut dedup_words = words.to_owned();
    dedup_words.sort();
    dedup_words.dedup();

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let vru_dialog = gui::VruDialog::new().unwrap();
    let vru_dialog_weak = vru_dialog.as_weak();

    vru_dialog.on_vru_button_clicked(move |chosen_word| {
        tx.try_send(chosen_word.to_string()).unwrap();
        vru_dialog_weak.unwrap().window().hide().unwrap();
    });

    vru_dialog.set_words(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(
            dedup_words
                .iter()
                .map(|x| x.into())
                .collect::<Vec<slint::SharedString>>(),
        ),
    )));

    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs_f64(frame_time),
        move || {
            unsafe { sdl3_sys::events::SDL_PumpEvents() }; // so the OS doesn't complain about the game window being frozen
            retroachievements::do_idle();
        },
    );

    vru_dialog.run().unwrap();

    if let Ok(result) = rx.try_recv() {
        for (i, v) in words.iter().enumerate() {
            if *v == result {
                return i as u16;
            }
        }
    }
    0x7FFF
}
