use std::ffi::OsString;
use std::io::{self, Write};

use anyhow::Error;
use clap::{builder::Command, Arg};
use clap::{value_parser, ArgAction};
use console::{set_colors_enabled, Style};

use crate::fd::Fd;
use crate::spawn;

fn make_app() -> Command {
    Command::new("systemfd")
        .version(env!("CARGO_PKG_VERSION"))
        .max_term_width(79)
        .about(
            "\nsystemfd is a helper application that is particularly useful for \
             Rust developers during development.  It implements the systemd \
             socket passing protocol which permits a socket to be opened from a \
             processed and then passed to others.  On windows a custom protocol \
             is used.  When paired with cargo-watch and the listenfd crate, \
             automatic reloading servers can be used during development.",
        )
        .arg(
            Arg::new("color")
                .long("color")
                .value_name("WHEN")
                .default_value("auto")
                .value_parser(["auto", "always", "never"])
                .help("Controls the color output"),
        )
        .arg(
            Arg::new("socket")
                .short('s')
                .long("socket")
                .num_args(1..)
                .number_of_values(1)
                .value_name("TYPE::SPEC")
                .help(
                    "This parameter can be supplied multiple times.  Each time it's \
                     specified a new socket of a certain specification is created.\n\
                     In the simplest situation just a port number is given in which \
                     case a TCP listening socket at that port is created.  To also \
                     force a certain network interface the format can be given as \
                     HOST:PORT.  Additionally a type prefix in the form TYPE::SPEC \
                     can be given which picks a different socket type.\n\n\
                     The following socket types exist: tcp, http, https, unix, udp.\n\n\
                     The http/https sockets are just aliases to tcp that render \
                     different help output.",
                ),
        )
        .arg(
            Arg::new("backlog")
                .short('b')
                .long("backlog")
                .default_value("1")
                .value_parser(value_parser!(i32).range(1..))
                .value_name("LISTEN-QUEUE")
                .help(
                    "The length of the socket backlog queue in any listen call which \
                     must be a positive integer greater than or equal to 1.  The OS may \
                     silently cap this value to a lower setting.",
                ),
        )
        .arg(
            Arg::new("no_pid")
                .long("no-pid")
                .action(ArgAction::SetTrue)
                .help(
                    "When this is set the LISTEN_PID environment variable is not \
             emitted.  This is supported by some systems such as the listenfd \
             crate to skip the pid check.  This is necessary for proxying \
             through to other processes like cargo-watch which would break \
             the pid check.  This has no effect on windows.",
                ),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .action(ArgAction::SetTrue)
                .help("Suppress all systemfd output."),
        )
        .arg(
            Arg::new("command")
                .last(true)
                .num_args(1..)
                .value_parser(value_parser!(OsString))
                .required(true)
                .help("The command that should be run"),
        )
}

pub fn execute() -> Result<(), Error> {
    let app = make_app();
    let matches = app.get_matches();
    let quiet = matches.get_flag("quiet");

    let prefix_style = Style::new().dim().bold();
    let log_style = Style::new().cyan();
    match matches
        .get_one::<String>("color")
        .as_ref()
        .map(|x| x.as_str())
    {
        Some("always") => set_colors_enabled(true),
        Some("never") => set_colors_enabled(false),
        _ => {}
    }

    macro_rules! log {
        ($($arg:expr),*) => {
            if !quiet {
                writeln!(
                    &mut io::stderr(),
                    "{} {}",
                    prefix_style.apply_to("~>"),
                    log_style.apply_to(format_args!($($arg),*))
                ).ok();
            }
        }
    }

    let mut fds: Vec<Fd> = Vec::new();
    if let Some(values) = matches.get_many::<String>("socket") {
        for socket in values {
            fds.push(socket.parse()?);
        }
    }

    let mut raw_fds = vec![];
    if fds.is_empty() {
        log!("warning: no sockets created");
    } else {
        for fd in fds {
            let raw_fd = fd.create_raw_fd(*matches.get_one("backlog").expect("default value"))?;
            raw_fds.push((fd, raw_fd));
        }
    }

    if !quiet {
        for &(ref fd, raw_fd) in &raw_fds {
            log!("socket {} -> fd #{}", fd.describe_raw_fd(raw_fd)?, raw_fd);
        }
    }

    let cmdline: Vec<_> = matches.get_many::<OsString>("command").unwrap().collect();
    spawn::spawn(raw_fds, &cmdline, matches.get_flag("no_pid"))
}
