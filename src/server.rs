use anyhow::{Context as _, Result, bail};
use rustix::{
    fs::Mode,
    net::{AddressFamily, SocketAddrUnix, SocketType},
};
use std::{
    io::ErrorKind,
    os::fd::{AsFd, AsRawFd, OwnedFd},
};

pub struct Server {
    fd: OwnedFd,
}

const SOCKET_PATH: &str = "/run/kb-mod-monitor.sock";
const SOMAXCONN: i32 = 4096;

impl Server {
    pub(crate) fn new() -> Result<Self> {
        let addr = SocketAddrUnix::new(SOCKET_PATH).context("failed to create sockaddr_un")?;
        ensure_no_other_process_running(&addr)?;

        let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)
            .context("socket() failed")?;

        match rustix::fs::unlink(SOCKET_PATH) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(anyhow::anyhow!(err))
                    .with_context(|| format!("failed to unlink {SOCKET_PATH}"));
            }
        }
        rustix::net::bind(&fd, &addr)
            .with_context(|| format!("failed to bind() at {SOCKET_PATH}"))?;
        rustix::fs::chmod(SOCKET_PATH, Mode::from_raw_mode(0o666))
            .with_context(|| format!("failed to chmod(666) {SOCKET_PATH}"))?;
        rustix::net::listen(&fd, SOMAXCONN)
            .with_context(|| format!("failed to listen() {SOCKET_PATH}"))?;

        Ok(Self { fd })
    }

    pub(crate) fn accept(&self) -> Result<OwnedFd> {
        log::trace!("Accepting a new client");
        rustix::net::accept(&self.fd).context("failed to accept()")
    }
}

impl AsFd for Server {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Server {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_raw_fd()
    }
}

fn ensure_no_other_process_running(addr: &SocketAddrUnix) -> Result<()> {
    let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)
        .context("socket() failed")?;

    if rustix::net::connect(&fd, addr).is_ok() {
        bail!("other process is running on the same UNIX socket {SOCKET_PATH}")
    }

    Ok(())
}
