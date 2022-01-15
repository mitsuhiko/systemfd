mod cli;
mod fd;
mod spawn;
mod utils;

use std::process;

fn main() {
    match cli::execute() {
        Ok(()) => {}
        Err(err) => {
            if let Some(&utils::QuietExit(code)) = err.downcast_ref() {
                process::exit(code);
            }
            println!("error: {}", err);
            for cause in err.chain().skip(1) {
                println!("  caused by: {}", cause);
            }
        }
    }
}
