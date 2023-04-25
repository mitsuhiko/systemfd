# systemfd

[![Build Status](https://github.com/mitsuhiko/systemfd/workflows/Tests/badge.svg?branch=master)](https://github.com/mitsuhiko/systemfd/actions?query=workflow%3ATests)
[![Crates.io](https://img.shields.io/crates/d/systemfd.svg)](https://crates.io/crates/systemfd)
[![License](https://img.shields.io/github/license/mitsuhiko/systemfd)](https://github.com/mitsuhiko/systemfd/blob/master/LICENSE)
[![rustc 1.46.0](https://img.shields.io/badge/rust-1.46%2B-orange.svg)](https://img.shields.io/badge/rust-1.46%2B-orange.svg)
[![Documentation](https://docs.rs/systemfd/badge.svg)](https://docs.rs/systemfd)

`systemfd` is the 1% of systemd that's useful for development.  It's a tiny process that
opens a bunch of sockets and passes them to another process so that that process can
then restart itself without dropping connections.  For that it uses the systemd socket
passing protocol (`LISTEN_FDS` + `LISTEN_PID`) environment variables on macOS and Linux
and a custom protocol on Windows.  Both are supported by the
[listenfd](https://github.com/mitsuhiko/listenfd) crate.

Teaser when combined with [cargo-watch](https://github.com/passcod/cargo-watch) you can
get automatically reloading development servers:

```
$ systemfd --no-pid -s http::5000 -- cargo watch -x run
```

The `--no-pid` flag disables passing the `LISTEN_PID` variable on unix (it has no effect
on Windows).  This makes `listenfd` skip the pid check which would fail with
`cargo watch` otherwise.  To see how to implement a server ready for systemfd
see below.

*This program was inspired by [catflap](https://github.com/passcod/catflap) but follows
systemd semantics and supports multiple sockets.*

## Installation

You can get systemfd by installing it with cargo:

```
$ cargo install systemfd
```

## Usage

`systemfd` creates one or more sockets as specified on the command line and then
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

## Examples

```
$ systemfd -s http::5000 -- my-server-executable
$ systemfd -s http::5000 -s https::5443 -- my-server-executable
$ systemfd -s 5000 -- my-server-executable
$ systemfd -s udp::1567 -- my-game-server-executable
```

When `systemfd` starts it will print out the socket it created.  This can be disabled
by passing `-q`.  Additionally if a port is set to `0` a random port is picked.

## Windows Protocol

On Windows, passing of sockets is significantly more complex than on Unix.  To
achieve this, this utility implements a custom socket passing system that is also
implemented by the listenfd crate.  When the sockets are created, an additional
local RPC server is spawned that gives out duplicates of the sockets to other
processes.  The RPC server uses TCP and is communicated to the child with the
`SYSTEMFD_SOCKET_SERVER` environment variable.  The RPC calls themselves are
protected with a `SYSTEMFD_SOCKET_SECRET` secret key.

The only understood command is `SECRET|PID` with secret and the processes' PID
inserted.  The server replies with N `WSAPROTOCOL_INFOW` structures.  The client
is expected to count the number of bytes and act accordingly.

This protocol is currently somewhat of a hack and might change.  It only
exists to support the `listenfd` crate.

## License and Links

- [Documentation](https://docs.rs/systemfd/)
- [Issue Tracker](https://github.com/mitsuhiko/systemfd/issues)
- [Examples](https://github.com/mitsuhiko/systemfd/tree/main/examples)
- License: [Apache-2.0](https://github.com/mitsuhiko/systemfd/blob/main/LICENSE)
