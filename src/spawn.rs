use std::ffi::OsString;
use std::process::Command;

use anyhow::Error;

use crate::fd::{Fd, RawFd};

#[cfg(unix)]
mod imp {
    use super::*;
    use nix::unistd::getpid;
    use std::os::unix::process::CommandExt;

    pub fn spawn(
        raw_fds: Vec<(Fd, RawFd)>,
        cmdline: &[&OsString],
        no_pid: bool,
    ) -> Result<(), Error> {
        let mut cmd = Command::new(cmdline[0]);
        cmd.args(&cmdline[1..]);

        if !raw_fds.is_empty() {
            cmd.env("LISTEN_FDS", raw_fds.len().to_string());
            let (_, rawfd) = raw_fds.first().unwrap();
            cmd.env("LISTEN_FDS_FIRST_FD", rawfd.to_string());
            if !no_pid {
                cmd.env("LISTEN_PID", getpid().to_string());
            }
        }
        anyhow::bail!("failed to spawn: {}", cmd.exec());
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::io::{Read, Write};
    use std::mem;
    use std::net::{TcpListener, TcpStream};
    use std::slice;
    use std::thread;

    use anyhow::anyhow;
    use uuid::Uuid;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::winsock2::{WSADuplicateSocketW, SOCKET, WSAPROTOCOL_INFOW};

    use crate::utils::QuietExit;

    fn share_sockets(
        mut sock: TcpStream,
        ref_secret: &Uuid,
        raw_fds: &[(Fd, RawFd)],
    ) -> Result<(), Error> {
        let mut data = Vec::new();
        sock.read_to_end(&mut data)?;
        let out = String::from_utf8(data)?;
        let mut pieces = out.split('|');

        let secret: Uuid = pieces
            .next()
            .and_then(|x| x.parse().ok())
            .ok_or_else(|| anyhow!("invalid secret"))?;
        if &secret != ref_secret {
            return Err(anyhow!("invalid secret"));
        }
        let pid: DWORD = pieces
            .next()
            .and_then(|x| x.parse().ok())
            .ok_or_else(|| anyhow!("invalid or missing pid"))?;

        for &(_, raw_fd) in raw_fds {
            let mut proto_info: WSAPROTOCOL_INFOW = unsafe { mem::zeroed() };
            unsafe {
                let rv = WSADuplicateSocketW(raw_fd as SOCKET, pid, &mut proto_info);
                if rv != 0 {
                    return Err(anyhow!("socket duplicate failed with {}", rv));
                }
            }
            let bytes: *const u8 = &proto_info as *const WSAPROTOCOL_INFOW as *const _;
            sock.write_all(unsafe {
                slice::from_raw_parts(bytes, mem::size_of::<WSAPROTOCOL_INFOW>())
            })?;
        }

        Ok(())
    }

    pub fn spawn(
        raw_fds: Vec<(Fd, RawFd)>,
        cmdline: &[&OsString],
        _no_pid: bool,
    ) -> Result<(), Error> {
        let mut cmd = Command::new(cmdline[0]);
        cmd.args(&cmdline[1..]);

        let secret: Uuid = Uuid::new_v4();
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let sockserver_addr = listener.local_addr()?;

        cmd.env("SYSTEMFD_SOCKET_SERVER", sockserver_addr.to_string());
        cmd.env("SYSTEMFD_SOCKET_SECRET", secret.to_string());

        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                share_sockets(stream, &secret, &raw_fds).unwrap();
            }
        });

        let mut child = cmd.spawn()?;
        let status = child.wait()?;

        Err(QuietExit(status.code().unwrap()).into())
    }
}

pub use self::imp::*;
