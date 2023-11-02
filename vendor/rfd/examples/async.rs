fn main() {
    // Spawn dialog on main thread
    let task = rfd::AsyncFileDialog::new().pick_file();

    // Await somewhere else
    execute(async {
        let file = task.await;

        if let Some(file) = file {
            // If you are on native platform you can just get the path
            #[cfg(not(target_arch = "wasm32"))]
            println!("{:?}", file.path());

            // If you care about wasm support you just read() the file
            file.read().await;
        }
    });

    loop {}
}

use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
