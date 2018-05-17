extern crate clap;
extern crate failure;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate console;
extern crate libc;
extern crate nix;

mod cli;
mod fd;

use std::env;

fn main() {
    let want_bt = match env::var("RUST_BACKTRACE").as_ref().map(|x| x.as_str()) {
        Ok("1") | Ok("full") => true,
        _ => false,
    };

    match cli::execute() {
        Ok(()) => {}
        Err(err) => {
            println!("error: {}", err);
            for cause in err.causes().skip(1) {
                println!("  caused by: {}", cause);
            }
            if want_bt {
                let bt = err.backtrace();
                println!("");
                println!("{}", bt);
            } else if cfg!(debug_assertions) {
                println!("");
                println!("hint: you can set RUST_BACKTRACE=1 to get the entire backtrace.");
            }
        }
    }
}
