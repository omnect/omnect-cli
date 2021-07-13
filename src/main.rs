use std::process;

fn main() {
    if let Err(e) = ics_dm_cli::run() {
        eprintln!("Application error: {}", e);

        process::exit(1);
    }
}
