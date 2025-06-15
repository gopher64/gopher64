slint::include_modules!();

fn about_window(app: &AppWindow) {
    app.on_wiki_button_clicked(move || {
        println!("Wiki button clicked");
    });
    app.on_discord_button_clicked(move || {
        println!("Discord button clicked");
    });
}

pub fn app_window() {
    let app = AppWindow::new().unwrap();
    about_window(&app);
    app.run().unwrap();
}
