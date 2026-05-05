use anyhow::{Result, ensure};
use rustix::net::SendFlags;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};

pub struct Client {
    fd: OwnedFd,
}

impl Client {
    pub(crate) const fn new(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub(crate) fn write(&self, value: bool) -> Result<()> {
        let buf = &[if value { b'1' } else { b'0' }];
        let bytes_written = rustix::net::send(&self.fd, buf, SendFlags::NOSIGNAL)?;
        ensure!(bytes_written == 1);
        Ok(())
    }
}

impl AsFd for Client {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Client {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_raw_fd()
    }
}
