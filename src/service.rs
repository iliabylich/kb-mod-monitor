use crate::{all_devices::AllDevices, inotify::INotify, keyboard::KeyboardState};
use anyhow::Result;
use pollsendd::Service;
use rustix::event::{PollFd, PollFlags};
use std::os::fd::AsRawFd;

pub struct KbModService {
    devices: AllDevices,
    kb_state: KeyboardState,
    inotify: INotify,
}

impl Service for KbModService {
    type Error = KbModServiceError;

    fn pollfds(&mut self) -> Result<impl Iterator<Item = PollFd<'_>>, Self::Error> {
        Ok(self.devices.pollfds().chain([self.inotify.pollfd()]))
    }

    fn owns_fd(&self, fd: i32) -> bool {
        self.devices.contains(fd) || self.inotify.as_raw_fd() == fd
    }

    fn on_readable(
        &mut self,
        fd: i32,
        mut broadcast: impl FnMut(&[u8]),
    ) -> Result<(), Self::Error> {
        if fd == self.inotify.as_raw_fd() {
            self.inotify.drain()?;
            let (devices, kb_state) = AllDevices::new()?;
            self.devices = devices;
            self.kb_state = kb_state;
        } else if self.devices.contains(fd) {
            let diff = self.devices.drain(fd);
            if let Some(diff_to_send) = self.kb_state.apply(diff) {
                log::trace!("Diff: {diff_to_send:?}");
                let buf = diff_to_send.serialize();
                (broadcast)(&buf);
            }
        }
        Ok(())
    }

    fn on_writable(&mut self, _fd: i32, _broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error> {
        Ok(())
    }

    fn on_poll_error(&mut self, fd: i32, revents: PollFlags) -> Result<(), Self::Error> {
        Err(anyhow::anyhow!("KbMod poll error: got revents {revents:?} for FD {fd}").into())
    }

    fn state_as_serialized_bufs(&self) -> impl Iterator<Item = Vec<u8>> {
        [self.kb_state.as_diff().serialize()].into_iter()
    }
}

impl KbModService {
    pub(crate) fn new() -> Result<Self> {
        let (devices, kb_state) = AllDevices::new()?;
        let inotify = INotify::new()?;
        Ok(Self {
            devices,
            kb_state,
            inotify,
        })
    }
}

pub struct KbModServiceError(anyhow::Error);
impl std::fmt::Display for KbModServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::fmt::Debug for KbModServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl core::error::Error for KbModServiceError {}
impl From<anyhow::Error> for KbModServiceError {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}
