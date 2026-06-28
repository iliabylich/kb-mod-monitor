use anyhow::{Context as _, Result, bail};
use rustix::{
    fs::Mode,
    net::{AddressFamily, SocketAddrUnix, SocketType},
};
use std::{io::ErrorKind, os::fd::OwnedFd};

#[derive(Debug)]
pub struct Args {
    mode: RunMode,
}

#[derive(Debug)]
pub enum RunMode {
    Systemd,
    Dev,
}

const USAGE: &str = "
Usage: kb-mod-monitor [--systemd|--dev]

--systemd: runs daemon under systemd and expects it to pass a socket via sd_listen_fds()
    --dev: starts listening on /run/kb-mod-monitor-dev.sock
";
fn print_usage_and_exit() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(1);
}

impl Args {
    pub(crate) fn parse() -> Self {
        let mut mode = None;

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--systemd" => mode = Some(RunMode::Systemd),
                "--dev" => mode = Some(RunMode::Dev),
                other => {
                    eprintln!("Unknown argument {other:?}");
                    print_usage_and_exit();
                }
            }
        }

        let mode = mode.unwrap_or_else(|| {
            eprintln!("Either --systemd or --dev is required");
            print_usage_and_exit()
        });

        Self { mode }
    }

    pub(crate) fn get_server_fd(&self) -> Result<OwnedFd> {
        match self.mode {
            RunMode::Systemd => systemd_socket(),
            RunMode::Dev => dev_socket(),
        }
    }
}

fn systemd_socket() -> Result<OwnedFd> {
    let fds = sd_listen_fds::get().context("sd_listen_fds() failed")?;
    let (_name, fd) = fds
        .into_iter()
        .next()
        .context("sd_listen_fds() returned an empty list of FDs")?;
    log::trace!("Listening on a systemd socket");
    Ok(fd.into_std())
}

fn dev_socket() -> Result<OwnedFd> {
    const SOCKET_PATH: &str = "/run/kb-mod-monitor-dev.sock";
    const SOMAXCONN: i32 = 4096;

    let addr = SocketAddrUnix::new(SOCKET_PATH).context("failed to create sockaddr_un")?;

    {
        let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)
            .context("socket() failed")?;
        if rustix::net::connect(&fd, &addr).is_ok() {
            bail!("other process is running on the same UNIX socket {SOCKET_PATH}")
        }
    }

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
    rustix::net::bind(&fd, &addr).with_context(|| format!("failed to bind() at {SOCKET_PATH}"))?;
    rustix::fs::chmod(SOCKET_PATH, Mode::from_raw_mode(0o666))
        .with_context(|| format!("failed to chmod(666) {SOCKET_PATH}"))?;
    rustix::net::listen(&fd, SOMAXCONN)
        .with_context(|| format!("failed to listen() {SOCKET_PATH}"))?;

    log::trace!("Listening on {SOCKET_PATH}");

    Ok(fd)
}
