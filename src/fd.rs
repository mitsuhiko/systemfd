use std::fmt::Display;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::bail;
use lazy_static::lazy_static;
use regex::Regex;

#[cfg(unix)]
pub use std::os::unix::io::RawFd;
#[cfg(windows)]
pub use std::os::windows::io::RawSocket as RawFd;

lazy_static! {
    static ref SPLIT_PREFIX: Regex = Regex::new(r"^([a-zA-Z]+)::(.+)$").unwrap();
}

#[derive(Debug)]
pub enum Fd {
    HttpListener(SocketAddr, bool),
    TcpListener(SocketAddr),
    UnixListener(PathBuf),
    UdpSocket(SocketAddr),
}

impl Fd {
    /// Creates a new listener from a string.
    pub fn new_listener(s: &str) -> Result<Fd, anyhow::Error> {
        if let Ok(port) = s.parse() {
            Ok(Fd::TcpListener(SocketAddr::new(
                Ipv4Addr::new(127, 0, 0, 1).into(),
                port,
            )))
        } else if let Ok(socket_addr) = s.parse() {
            Ok(Fd::TcpListener(socket_addr))
        } else if s.contains('/') {
            Fd::new_unix_listener(s)
        } else {
            bail!(
                "unsupported specification '{}'. Please provide \
                 an explicit socket type",
                s
            )
        }
    }

    /// Creates a new tcp listener from a string.
    pub fn new_tcp_listener(s: &str) -> Result<Fd, anyhow::Error> {
        if let Ok(port) = s.parse() {
            Ok(Fd::TcpListener(SocketAddr::new(
                Ipv4Addr::new(127, 0, 0, 1).into(),
                port,
            )))
        } else {
            Ok(Fd::TcpListener(s.parse()?))
        }
    }

    /// Creates a new http listener from a string.
    pub fn new_http_listener(s: &str, secure: bool) -> Result<Fd, anyhow::Error> {
        if let Ok(port) = s.parse() {
            Ok(Fd::HttpListener(
                SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), port),
                secure,
            ))
        } else {
            Ok(Fd::HttpListener(s.parse()?, secure))
        }
    }

    /// Creates a new unix listener from a string.
    pub fn new_unix_listener(s: &str) -> Result<Fd, anyhow::Error> {
        Ok(Fd::UnixListener(PathBuf::from(s)))
    }

    /// Creates a new udp socket from a string.
    pub fn new_udp_socket(s: &str) -> Result<Fd, anyhow::Error> {
        if let Ok(port) = s.parse() {
            Ok(Fd::UdpSocket(SocketAddr::new(
                Ipv4Addr::new(127, 0, 0, 1).into(),
                port,
            )))
        } else {
            Ok(Fd::UdpSocket(s.parse()?))
        }
    }

    fn should_listen(&self) -> bool {
        match self {
            Fd::TcpListener(..) => true,
            Fd::UnixListener(..) => true,
            Fd::HttpListener(..) => true,
            Fd::UdpSocket(..) => false,
        }
    }

    /// Creates a raw fd from the fd spec.
    pub fn create_raw_fd(&self, listen_backlog: i32, reuse: bool) -> Result<RawFd, anyhow::Error> {
        create_raw_fd(self, listen_backlog, reuse)
    }

    pub fn describe_raw_fd(&self, raw_fd: RawFd) -> Result<String, anyhow::Error> {
        let addr = describe_addr(raw_fd)?;
        Ok(match self {
            Fd::TcpListener(..) => format!("{} (tcp listener)", addr),
            Fd::HttpListener(_addr, secure) => {
                format!("{}://{}/", if *secure { "https" } else { "http" }, addr)
            }
            Fd::UnixListener(..) => format!("{} (unix listener)", addr),
            Fd::UdpSocket(..) => format!("{} (udp)", addr),
        })
    }
}

impl FromStr for Fd {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Fd, anyhow::Error> {
        let (ty, val) = if let Some(caps) = SPLIT_PREFIX.captures(s) {
            (
                Some(caps.get(1).unwrap().as_str()),
                caps.get(2).unwrap().as_str(),
            )
        } else {
            (None, s)
        };

        match ty {
            Some("tcp") => Fd::new_tcp_listener(val),
            Some("http") => Fd::new_http_listener(val, false),
            Some("https") => Fd::new_http_listener(val, true),
            Some("unix") => Fd::new_unix_listener(val),
            Some("udp") => Fd::new_udp_socket(val),
            Some(ty) => bail!("unknown socket type '{}'", ty),
            None => Fd::new_listener(val),
        }
    }
}

#[cfg(unix)]
mod imp {
    use super::*;
    use anyhow::Error;
    use libc::close;
    use nix::sys::socket;
    use nix::sys::socket::setsockopt;
    use nix::sys::socket::sockopt::ReuseAddr;
    use nix::sys::socket::sockopt::ReusePort;
    use nix::sys::socket::AddressFamily;

    pub fn create_raw_fd(fd: &Fd, listen_backlog: i32, reuse: bool) -> Result<RawFd, Error> {
        let (addr, fam, ty) = sock_info(fd)?;
        let sock = socket::socket(fam, ty, socket::SockFlag::empty(), None)?;

        if reuse {
            setsockopt(sock, ReuseAddr, &true)?;

            // port reuse will only work on inet sockets.  On new linux kernels
            // in particular it will fail with an error if attempted on unix sockets
            // or others.
            if matches!(fam, AddressFamily::Inet | AddressFamily::Inet6) {
                setsockopt(sock, ReusePort, &true)?;
            }
        }

        // kill stale unix sockets if they are there
        if let Fd::UnixListener(ref path) = fd {
            std::fs::remove_file(path).ok();
        }

        let rv = socket::bind(sock, &*addr)
            .map_err(From::from)
            .and_then(|_| {
                if fd.should_listen() {
                    socket::listen(sock, listen_backlog as usize)?;
                }
                Ok(())
            });

        if rv.is_err() {
            unsafe { close(sock) };
        }

        rv.map(|_| sock)
    }

    pub fn describe_addr(raw_fd: RawFd) -> Result<impl Display, Error> {
        Ok(socket::getsockname::<socket::SockaddrStorage>(raw_fd)?)
    }

    fn sock_info(
        fd: &Fd,
    ) -> Result<
        (
            Box<dyn socket::SockaddrLike>,
            socket::AddressFamily,
            socket::SockType,
        ),
        Error,
    > {
        Ok(match *fd {
            Fd::TcpListener(addr) => (
                Box::new(socket::SockaddrStorage::from(addr)),
                if addr.is_ipv4() {
                    socket::AddressFamily::Inet
                } else {
                    socket::AddressFamily::Inet6
                },
                socket::SockType::Stream,
            ),
            Fd::HttpListener(addr, _secure) => (
                Box::new(socket::SockaddrStorage::from(addr)),
                if addr.is_ipv4() {
                    socket::AddressFamily::Inet
                } else {
                    socket::AddressFamily::Inet6
                },
                socket::SockType::Stream,
            ),
            Fd::UdpSocket(addr) => (
                Box::new(socket::SockaddrStorage::from(addr)),
                if addr.is_ipv4() {
                    socket::AddressFamily::Inet
                } else {
                    socket::AddressFamily::Inet6
                },
                socket::SockType::Datagram,
            ),
            Fd::UnixListener(ref path) => (
                Box::new(socket::UnixAddr::new(path)?),
                socket::AddressFamily::Unix,
                socket::SockType::Stream,
            ),
        })
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::mem::ManuallyDrop;
    use std::os::windows::io::{FromRawSocket, IntoRawSocket};

    use anyhow::{bail, Error};

    pub fn create_raw_fd(fd: &Fd, listen_backlog: i32, reuse: bool) -> Result<RawFd, Error> {
        let (addr, dom, ty) = sock_info(fd)?;
        let sock = socket2::Socket::new(dom, ty, None)?;

        if reuse {
            sock.set_reuse_address(true);
        }

        sock.bind(&addr)?;
        if fd.should_listen() {
            sock.listen(listen_backlog as _)?;
        }

        Ok(sock.into_raw_socket())
    }

    pub fn describe_addr(raw_fd: RawFd) -> Result<impl Display, Error> {
        let sock = ManuallyDrop::new(unsafe { socket2::Socket::from_raw_socket(raw_fd) });
        let local_addr = sock.local_addr()?;
        let rv = local_addr.as_socket().unwrap();
        Ok(rv)
    }

    fn sock_info(fd: &Fd) -> Result<(socket2::SockAddr, socket2::Domain, socket2::Type), Error> {
        Ok(match *fd {
            Fd::TcpListener(addr) => (
                addr.into(),
                if addr.is_ipv4() {
                    socket2::Domain::IPV4
                } else {
                    socket2::Domain::IPV6
                },
                socket2::Type::STREAM,
            ),
            Fd::HttpListener(addr, _secure) => (
                addr.into(),
                if addr.is_ipv4() {
                    socket2::Domain::IPV4
                } else {
                    socket2::Domain::IPV6
                },
                socket2::Type::STREAM,
            ),
            Fd::UdpSocket(addr) => (
                addr.into(),
                if addr.is_ipv4() {
                    socket2::Domain::IPV4
                } else {
                    socket2::Domain::IPV6
                },
                socket2::Type::DGRAM,
            ),
            Fd::UnixListener(..) => {
                bail!("Cannot use unix sockets on windows");
            }
        })
    }
}

use self::imp::*;
