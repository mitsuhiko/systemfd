# systemfd

`systemfd` is the 1% of systemd that's useful for development.  It's a tiny process that
opens a bunch of sockets and passes them to another process so that that process can
then restart itself without dropping connections.  For that it uses ths systemd socket
passing protocol (`LISTEN_FDS` + `LISTEN_PID`) environment variables.  Currently this
only supports unix systems.

Teaser when combined with [catch-watch](https://github.com/passcod/cargo-watch) you can
get automatically reloading development servers:

```
$ systemfd -s http::5000 -- cargo watch -x run
```

To see how to implement a server ready for systemfd see below.

*This program was inspired by [catflap](https://github.com/passcod/catflap) but follows
systemd semantics and supports multiple sockets.*

## Installation

You can get systemfd by installing it with cargo:

```
$ cargo install systemfd
```

## Usage

`systemfd` can create one or multiple sockets as specified on the command line and then
invokes another application.  Each time you pass the `-s` (or `--socket`)
parameter a new socket is created.  The value for the parameter is a socket
specification in the format `[TYPE::]VALUE` where `TYPE` defaults to `tcp` or
`unix` depending on the value.  The following types exist:

* `tcp`: creates a tcp listener
* `http`: creates a tcp listener for http usage (prints a http url in the info output)
* `https`: creates a tcp listener for https usage (prints a https url in the info output)
* `unix`: creates a unix listener socket
* `udp`: creates a udp socket

`VALUE` depends on the socket type.  The following formats are supported:

* `<port>`: an integer port value, assumes `127.0.0.1` as host
* `<host>:<port>`: binds to a specific network interface and port
* `<unix-path>`: requests a specific unix socket

Examples:

```
$ systemfd -s http::5000 -- my-server-executable
$ systemfd -s http::5000 -s https::5443 -- my-server-executable
$ systemfd -s 5000 -- my-server-executable
$ systemfd -s udp::1567 -- my-game-server-executable
```

When `systemfd` starts it will print out the socket it created.  This can be disabled
by passing `-q`.  Additionally if a port is set to `0` a random port is picked.

## Usage with actix-web and listenfd

And here is an example [actix-web](https://actix.rs/) server that supports this
by using the [listenfd](https://github.com/mitsuhiko/rust-listenfd) crate:

```rust
use listenfd::ListenFdManager;
use actix_web::{server, App, Path};

fn index(info: Path<(String, u32)>) -> String {
   format!("Hello {}! id:{}", info.0, info.1)
}

fn main() {
    let mut manager = ListenFdManager::from_env();
    let mut server = server::new(
        || App::new()
            .resource("/{name}/{id}/index.html", |r| r.with(index)));
    server = if let Some(listener) = manager.take_tcp_listener(0)? {
        server.listener(listener)
    } else {
        server.bind("127.0.0.1:3000").unwrap()
    };
    server.run();
}
```
