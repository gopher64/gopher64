#![deny(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::Parser;

#[tokio::main(worker_threads = 4)]
async fn main() -> std::io::Result<()> {
    let args = gopher64::Args::parse();
    gopher64::run(args, std::env::args().count()).await
}
