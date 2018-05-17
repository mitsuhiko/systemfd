use std::net::{Ipv4Addr, SocketAddr};
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::str::FromStr;

use failure::{err_msg, Error};
use libc::close;
use nix::sys::socket;
use regex::Regex;

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
    pub fn new_listener(s: &str) -> Result<Fd, Error> {
        if let Ok(port) = s.parse() {
            Ok(Fd::TcpListener(SocketAddr::new(
                Ipv4Addr::new(127, 0, 0, 1).into(),
                port,
            )))
        } else if let Ok(socket_addr) = s.parse() {
            Ok(Fd::TcpListener(socket_addr))
        } else if s.contains("/") {
            Fd::new_unix_listener(s)
        } else {
            Err(err_msg(format!(
                "unsupported specification '{}'. Please provide \
                 an explicit socket type",
                s
            )))
        }
    }

    /// Creates a new tcp listener from a string.
    pub fn new_tcp_listener(s: &str) -> Result<Fd, Error> {
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
    pub fn new_http_listener(s: &str, secure: bool) -> Result<Fd, Error> {
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
    pub fn new_unix_listener(s: &str) -> Result<Fd, Error> {
        Ok(Fd::UnixListener(PathBuf::from(s)))
    }

    /// Creates a new udp socket from a string.
    pub fn new_udp_socket(s: &str) -> Result<Fd, Error> {
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
            Fd::HttpListener(..) => true,
            Fd::UdpSocket(..) => true,
            _ => false,
        }
    }

    fn sock_info(
        &self,
    ) -> Result<(socket::SockAddr, socket::AddressFamily, socket::SockType), Error> {
        Ok(match self {
            Fd::TcpListener(addr) => (
                socket::SockAddr::new_inet(socket::InetAddr::from_std(addr)),
                if addr.is_ipv4() {
                    socket::AddressFamily::Inet
                } else {
                    socket::AddressFamily::Inet6
                },
                socket::SockType::Stream,
            ),
            Fd::HttpListener(addr, _secure) => (
                socket::SockAddr::new_inet(socket::InetAddr::from_std(addr)),
                if addr.is_ipv4() {
                    socket::AddressFamily::Inet
                } else {
                    socket::AddressFamily::Inet6
                },
                socket::SockType::Stream,
            ),
            Fd::UdpSocket(addr) => (
                socket::SockAddr::new_inet(socket::InetAddr::from_std(addr)),
                if addr.is_ipv4() {
                    socket::AddressFamily::Inet
                } else {
                    socket::AddressFamily::Inet6
                },
                socket::SockType::Datagram,
            ),
            Fd::UnixListener(path) => (
                socket::SockAddr::new_unix(path)?,
                socket::AddressFamily::Unix,
                socket::SockType::Stream,
            ),
        })
    }

    /// Creates a raw fd from the fd spec.
    pub fn create_raw_fd(&self) -> Result<RawFd, Error> {
        let (addr, fam, ty) = self.sock_info()?;
        let sock = socket::socket(fam, ty, socket::SockFlag::empty(), None)?;

        let rv = socket::bind(sock, &addr).map_err(From::from).and_then(|_| {
            if self.should_listen() {
                socket::listen(sock, 1)?;
            }
            Ok(())
        });

        if rv.is_err() {
            unsafe { close(sock) };
        }

        rv.map(|_| sock)
    }

    pub fn describe_raw_fd(&self, raw_fd: RawFd) -> Result<String, Error> {
        let addr = socket::getsockname(raw_fd)?;
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Fd, Error> {
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
            Some(ty) => Err(err_msg(format!("unknown socket type '{}'", ty))),
            None => Fd::new_listener(val),
        }
    }
}
