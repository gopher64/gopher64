#![deny(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::Parser;

fn main() -> std::io::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to build runtime");
        tx.send(rt.handle().clone()).unwrap();
        rt.block_on(close_rx).unwrap();
    });

    let _guard = rx.recv().unwrap().enter();

    let args = gopher64::Args::parse();
    let result = gopher64::run(args, std::env::args().count());
    close_tx.send(()).unwrap();
    result
}
