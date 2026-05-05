use anyhow::{Context as _, Result};
use rustix::fs::inotify::{CreateFlags, WatchFlags};
use std::{
    io::ErrorKind,
    os::fd::{AsFd, AsRawFd, OwnedFd},
};

pub struct INotify {
    fd: OwnedFd,
}

impl INotify {
    pub(crate) fn new() -> Result<Self> {
        let fd = rustix::fs::inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK)
            .context("inotify_init() failed")?;

        rustix::fs::inotify::add_watch(
            &fd,
            "/dev/input",
            WatchFlags::CREATE
                | WatchFlags::DELETE
                | WatchFlags::ATTRIB
                | WatchFlags::MOVED_FROM
                | WatchFlags::MOVED_TO,
        )?;

        Ok(Self { fd })
    }

    pub(crate) fn drain(&self) -> Result<()> {
        let mut buf = [0; 4_096];

        loop {
            match rustix::io::read(&self.fd, &mut buf) {
                Ok(_) => {}
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(err.into()),
            }
        }

        Ok(())
    }
}

impl AsFd for INotify {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for INotify {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.as_fd().as_raw_fd()
    }
}
