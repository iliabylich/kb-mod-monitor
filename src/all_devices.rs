use crate::{
    keyboard::{Keyboard, diff::KeyboardStateDiff, state::KeyboardState},
    xkb_state::XkbState,
};
use anyhow::Result;
use evdev::KeyCode;
use rustix::event::{PollFd, PollFlags};
use std::{collections::HashMap, os::fd::AsRawFd as _};

pub struct AllDevices {
    fd_to_keyboard: HashMap<i32, Keyboard>,
}

impl AllDevices {
    pub(crate) fn new() -> Result<(Self, KeyboardState)> {
        let xkb_state = XkbState::new()?;
        let xkb_callback = xkb_state.callback();

        let mut fd_to_keyboard = HashMap::new();
        let mut kb_state = KeyboardState::empty();
        for (path, dev) in evdev::enumerate() {
            let is_keyboard = dev.supported_keys().is_some_and(|keys| {
                keys.contains(KeyCode::KEY_SPACE)
                    && keys.contains(KeyCode::KEY_A)
                    && keys.contains(KeyCode::KEY_Z)
            });

            if !is_keyboard {
                let name = dev.name().unwrap_or("<unnamed>");
                log::trace!(
                    "Skipping non-keyboard device {name:?} at {}",
                    path.display()
                );
                continue;
            }

            let keyboard = Keyboard::new(path, dev, xkb_callback.clone())?;

            log::info!("Found {keyboard:?} (fd={})", keyboard.as_raw_fd());
            let led_state = keyboard.led_based_state();
            kb_state = kb_state.or(led_state);
            fd_to_keyboard.insert(keyboard.as_raw_fd(), keyboard);
        }
        xkb_state.set_initial_state(kb_state);

        Ok((Self { fd_to_keyboard }, kb_state))
    }

    pub(crate) fn pollfds(&self) -> impl Iterator<Item = PollFd<'_>> {
        self.fd_to_keyboard
            .values()
            .map(|keyboard| PollFd::new(keyboard, PollFlags::IN))
    }

    pub(crate) fn remove(&mut self, fd: i32) {
        self.fd_to_keyboard.remove(&fd);
    }

    pub(crate) fn contains(&self, fd: i32) -> bool {
        self.fd_to_keyboard.contains_key(&fd)
    }

    pub(crate) fn drain(&mut self, fd: i32) -> KeyboardStateDiff {
        let Some(keyboard) = self.fd_to_keyboard.get_mut(&fd) else {
            return KeyboardStateDiff::empty();
        };

        match keyboard.drain() {
            Ok(diff) => diff,
            Err(err) => {
                log::error!("{err:?}");
                self.remove(fd);
                KeyboardStateDiff::empty()
            }
        }
    }
}
