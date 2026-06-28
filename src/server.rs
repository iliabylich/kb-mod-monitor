use crate::args::Args;
use anyhow::{Context as _, Result};
use std::os::fd::{AsFd, AsRawFd, OwnedFd};

pub struct Server {
    fd: OwnedFd,
}

impl Server {
    pub(crate) fn new(args: &Args) -> Result<Self> {
        let fd = args.get_server_fd()?;
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
