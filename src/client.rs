use crate::{key::Key, keyboard::diff::KeyboardStateDiff};
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

    pub(crate) fn write(&self, diff: KeyboardStateDiff) -> Result<()> {
        let mut buf = vec![];
        if let Some(v) = diff.get(Key::CapsLock) {
            buf.push(if v { b'1' } else { b'0' });
        }
        if let Some(v) = diff.get(Key::NumLock) {
            buf.push(if v { b'3' } else { b'2' });
        }
        let bytes_written = rustix::net::send(&self.fd, &buf, SendFlags::NOSIGNAL)?;
        ensure!(bytes_written == buf.len());
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
