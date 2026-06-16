#![deny(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::Parser;

fn main() -> std::io::Result<()> {
    let (close_tx, handle) = gopher64::create_runtime();
    let _guard = handle.enter();

    let args = gopher64::Args::parse();
    let result = gopher64::run(args, std::env::args().count());
    close_tx.send(()).unwrap();
    result
}
