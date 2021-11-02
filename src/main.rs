use env_logger::{Builder, Env};
use log::error;
use std::process;

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    if let Err(e) = ics_dm_cli::run() {
        error!("Application error: {}", e);

        process::exit(1);
    }
}
