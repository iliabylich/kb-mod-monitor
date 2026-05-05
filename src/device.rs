use anyhow::{Context, Result};
use evdev::{EventType, InputEvent, KeyCode, LedCode};
use std::{
    io::ErrorKind,
    os::fd::{AsFd, AsRawFd},
    path::PathBuf,
    rc::Rc,
};

pub struct Device {
    name: String,
    dev: evdev::Device,
    path: PathBuf,
    on_press: Rc<dyn Fn() -> bool>,
    on_release: Rc<dyn Fn() -> bool>,
}

impl Device {
    pub(crate) fn new(
        path: PathBuf,
        dev: evdev::Device,
        on_press: Rc<dyn Fn() -> bool>,
        on_release: Rc<dyn Fn() -> bool>,
    ) -> Result<Option<Self>> {
        if !has_caps_lock(&dev) {
            return Ok(None);
        }

        let name = dev.name().unwrap_or("<unnamed>").to_string();

        dev.set_nonblocking(true).with_context(|| {
            format!(
                "failed to set device with name={name:?} path={}",
                path.display()
            )
        })?;

        Ok(Some(Self {
            name,
            dev,
            path,
            on_press,
            on_release,
        }))
    }

    fn fetch_events(&mut self) -> Result<Option<Vec<InputEvent>>> {
        match self.dev.fetch_events() {
            Ok(events) => Ok(Some(events.collect())),
            Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub(crate) fn drain(&mut self) -> Result<Option<bool>> {
        let mut caps_lock_activated = None;

        while let Some(events) = self.fetch_events()? {
            for event in events {
                if event.event_type() == EventType::KEY
                    && event.code() == KeyCode::KEY_CAPSLOCK.code()
                {
                    match event.value() {
                        0 => {
                            // release
                            log::trace!("caps lock released on {:?}", self.name);
                            caps_lock_activated = Some((self.on_release)());
                        }
                        1 => {
                            // press
                            log::trace!("caps lock pressed on {:?}", self.name);
                            caps_lock_activated = Some((self.on_press)());
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(caps_lock_activated)
    }

    pub(crate) fn caps_lock_led_is_active(&self) -> Result<bool> {
        let is_active = self.dev.get_led_state()?.contains(LedCode::LED_CAPSL);
        if is_active {
            log::trace!("LED is active on {self:?}");
        }
        Ok(is_active)
    }
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Device[{:?} at {:?}]", self.name, self.path)
    }
}

impl AsFd for Device {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.dev.as_fd()
    }
}

impl AsRawFd for Device {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.dev.as_raw_fd()
    }
}

fn has_caps_lock(dev: &evdev::Device) -> bool {
    dev.supported_events().contains(EventType::KEY)
        && dev
            .supported_keys()
            .is_some_and(|keys| keys.contains(KeyCode::KEY_CAPSLOCK))
}
