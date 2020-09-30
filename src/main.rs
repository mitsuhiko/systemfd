extern crate clap;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate console;
extern crate libc;

#[cfg(unix)]
extern crate nix;
#[cfg(windows)]
extern crate socket2;
#[cfg(windows)]
extern crate uuid;
#[cfg(windows)]
extern crate winapi;

mod cli;
mod fd;
mod spawn;
mod utils;

use std::env;
use std::process;

fn main() {
    let want_bt = match env::var("RUST_BACKTRACE").as_ref().map(|x| x.as_str()) {
        Ok("1") | Ok("full") => true,
        _ => false,
    };

    match cli::execute() {
        Ok(()) => {}
        Err(err) => {
            if let Some(&utils::QuietExit(code)) = err.downcast_ref() {
                process::exit(code);
            }
            println!("error: {}", err);
            for cause in err.iter_chain().skip(1) {
                println!("  caused by: {}", cause);
            }
            if want_bt {
                let bt = err.backtrace();
                println!();
                println!("{}", bt);
            } else if cfg!(debug_assertions) {
                println!();
                println!("hint: you can set RUST_BACKTRACE=1 to get the entire backtrace.");
            }
        }
    }
}
