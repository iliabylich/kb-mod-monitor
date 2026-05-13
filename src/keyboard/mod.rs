use crate::{
    key::Key,
    xkb_state::{Dir, callbacks::XkbCallbacks},
};
use anyhow::{Context, Result};
use diff::KeyboardStateDiff;
use evdev::{EventType, InputEvent, LedCode};
use state::KeyboardState;
use std::{
    io::ErrorKind,
    os::fd::{AsFd, AsRawFd},
    path::PathBuf,
};

pub mod diff;
pub mod state;

pub struct Keyboard {
    name: String,
    dev: evdev::Device,
    path: PathBuf,
    xkb_callbacks: XkbCallbacks,
}

impl Keyboard {
    pub(crate) fn new(
        path: PathBuf,
        dev: evdev::Device,
        xkb_callbacks: XkbCallbacks,
    ) -> Result<Self> {
        let name = dev.name().unwrap_or("<unnamed>").to_string();

        dev.set_nonblocking(true).with_context(|| {
            format!(
                "failed to set device with name={name:?} path={}",
                path.display()
            )
        })?;

        Ok(Self {
            name,
            dev,
            path,
            xkb_callbacks,
        })
    }

    fn fetch_events(&mut self) -> Result<Option<Vec<InputEvent>>> {
        match self.dev.fetch_events() {
            Ok(events) => Ok(Some(events.collect())),
            Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub(crate) fn drain(&mut self) -> Result<KeyboardStateDiff> {
        let mut diff = KeyboardStateDiff::empty();

        while let Some(events) = self.fetch_events()? {
            for event in events {
                if event.event_type() != EventType::KEY {
                    continue;
                }

                let Some(key) = Key::from_evdev_keycode(event.code()) else {
                    continue;
                };

                match event.value() {
                    0 => {
                        // release
                        log::trace!("{key:?} released on {:?}", self.name);
                        diff.set(key, Some(self.xkb_callbacks.dispatch(key, Dir::Up)));
                    }
                    1 => {
                        // press
                        log::trace!("{key:?} pressed on {:?}", self.name);
                        diff.set(key, Some(self.xkb_callbacks.dispatch(key, Dir::Down)));
                    }
                    _ => {}
                }
            }
        }

        Ok(diff)
    }

    pub(crate) fn led_based_state(&self) -> KeyboardState {
        let led_state = match self.dev.get_led_state() {
            Ok(led_state) => led_state,
            Err(err) => {
                log::error!("Failed to get initial LED state for {self:?}: {err:?}");
                return KeyboardState::empty();
            }
        };

        let mut state = KeyboardState::empty();
        state.set(Key::CapsLock, led_state.contains(LedCode::LED_CAPSL));
        state.set(Key::NumLock, led_state.contains(LedCode::LED_NUML));
        state
    }
}

impl std::fmt::Debug for Keyboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Device[{:?} at {:?}]", self.name, self.path)
    }
}

impl AsFd for Keyboard {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.dev.as_fd()
    }
}

impl AsRawFd for Keyboard {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.dev.as_raw_fd()
    }
}
