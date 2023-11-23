use env_logger::{Builder, Env};
use log::error;
use std::process;

fn main() {
    if cfg!(debug_assertions) {
        Builder::from_env(Env::default().default_filter_or("debug,bollard::read=info")).init();
    } else {
        Builder::from_env(Env::default().default_filter_or("info")).init();
    }
    if let Err(e) = omnect_cli::run() {
        error!("Application error: {e:#?}");

        process::exit(1);
    }
}
