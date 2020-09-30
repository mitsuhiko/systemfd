use std::fs::File;
use std::io::{self, Write};

use clap::{App, AppSettings, Arg};
use console::{set_colors_enabled, Style};
use failure::{err_msg, Error};

use fd::Fd;
use spawn;

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
             processed and then passed to others.  On windows a custom protocol \
             is used.  When paired with cargo-watch and the listenfd crate, \
             automatic reloading servers can be used during development.",
        ).arg(
            Arg::with_name("color")
                .long("color")
                .value_name("WHEN")
                .default_value("auto")
                .possible_values(&["auto", "always", "never"])
                .help("Controls the color output"),
        ).arg(
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
        ).arg(Arg::with_name("no_pid").long("no-pid").help(
            "When this is set the LISTEN_PID environment variable is not \
             emitted.  This is supported by some systems such as the listenfd \
             crate to skip the pid check.  This is necessary for proxying \
             through to other processe like cargo-watch which would break \
             the pid check.  This has no effect on windows.",
        )).arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Suppress all systemfd output."),
        ).arg(
            Arg::with_name("command")
                .multiple(true)
                .last(true)
                .required(true)
                .help("The command that should be run"),
        )
        .arg(
            Arg::with_name("write_file")
                .short("w")
                .long("write-file")
                .value_name("FILENAME")
                .help(
                    "When this is set, the description of the sockets will be written \
                     to tne named file. Each socket will be on a separate line. This \
                     may be useful, for example, when specifying a TCP port number of \
                     \"0\" which will cause the kernel to pick an unused port at random.",
                ),
        )
}

pub fn execute() -> Result<(), Error> {
    let app = make_app();
    let matches = app.get_matches();
    let quiet = matches.is_present("quiet");

    let prefix_style = Style::new().dim().bold();
    let log_style = Style::new().cyan();
    match matches.value_of("color") {
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
    if let Some(values) = matches.values_of("socket") {
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
            log!("socket {}", fd.describe_raw_fd(raw_fd)?);
        }
    }

    if let Some(filename) = matches.value_of("write_file") {
        let mut output = File::create(filename)
            .map_err(|error| err_msg(format!("Could not create {:?}: {}", filename, error)))?;
        for &(ref fd, raw_fd) in &raw_fds {
            write!(
                output,
                "{}",
                fd.describe_raw_fd(raw_fd).map_err(|error| err_msg(format!(
                    "Could not write to {:?}: {}",
                    filename, error
                )))?
            )?;
        }
    }

    let cmdline: Vec<_> = matches.values_of("command").unwrap().collect();
    spawn::spawn(raw_fds, &cmdline, matches.is_present("no_pid"))
}
