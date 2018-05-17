use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::process::Command;

use clap::{App, AppSettings, Arg};
use console::{set_colors_enabled, Style};
use failure::Error;
use nix::unistd::getpid;

use fd::Fd;

fn make_app() -> App<'static, 'static> {
    App::new("systemfd")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::ColorNever)
        .max_term_width(79)
        .usage("[OPTIONS] -- <command>...")
        .about(
            "\nsystemfd is a helper application that is particularly useful for \
             Rust developers during development.  It implements the systemd \
             socket passing protocol which permits a socket to be opened from a \
             processed and then passed to others.  When paired with cargo-watch \
             for instance automatic reloading servers can be used during \
             development.\n\n\
             To consume such a socket the listenfd crate can be used.",
        )
        .arg(
            Arg::with_name("color")
                .long("color")
                .value_name("WHEN")
                .default_value("auto")
                .possible_values(&["auto", "always", "never"])
                .help("Controls the color output"),
        )
        .arg(
            Arg::with_name("socket")
                .short("s")
                .long("socket")
                .multiple(true)
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
            Arg::with_name("method")
                .short("m")
                .long("method")
                .value_name("METHOD")
                .default_value("systemd")
                .possible_values(&["systemd"])
                .help(
                    "The file descriptor passing method that should be used.  The \
                     default and currently only supported is to use the systemd \
                     protocol.",
                ),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Suppress all systemfd output."),
        )
        .arg(
            Arg::with_name("command")
                .multiple(true)
                .last(true)
                .required(true)
                .help("The command that should be run"),
        )
}

pub fn execute() -> Result<(), Error> {
    let app = make_app();
    let args = app.get_matches();
    let quiet = args.is_present("quiet");

    let prefix_style = Style::new().dim().bold();
    let log_style = Style::new().cyan();
    match args.value_of("color") {
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
    if let Some(values) = args.values_of("socket") {
        for socket in values {
            fds.push(socket.parse()?);
        }
    }

    let mut raw_fds = vec![];
    if fds.is_empty() {
        log!("warning: no sockets created");
    } else {
        for fd in fds {
            let raw_fd = fd.create_raw_fd()?;
            raw_fds.push((fd, raw_fd));
        }
    }

    if !quiet {
        for &(ref fd, raw_fd) in &raw_fds {
            log!("fd {}: {}", raw_fd, fd.describe_raw_fd(raw_fd)?);
        }
    }

    let cmdline: Vec<_> = args.values_of("command").unwrap().collect();
    let mut cmd = Command::new(&cmdline[0]);
    cmd.args(&cmdline[1..]);

    if !raw_fds.is_empty() {
        cmd.env("LISTEN_FDS", raw_fds.len().to_string());
        cmd.env("LISTEN_PID", getpid().to_string());
    }

    cmd.exec();

    Ok(())
}
