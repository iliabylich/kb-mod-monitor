use crate::{
    client::Client, device::Device, inotify::INotify, server::Server, xkb_state::XkbState,
};
use anyhow::{Result, bail};
use rustix::event::{PollFd, PollFlags};
use std::{
    collections::HashMap,
    os::fd::{AsFd as _, AsRawFd},
    rc::Rc,
};

pub struct State {
    devices: HashMap<i32, Device>,
    server: Server,
    clients: HashMap<i32, Client>,
    inotify: INotify,
    caps_lock_is_active: bool,
}

impl State {
    pub(crate) fn new() -> Result<Self> {
        let (devices, caps_lock_is_active) = devices()?;
        let server = Server::new()?;
        let clients = HashMap::new();
        let inotify = INotify::new()?;

        Ok(Self {
            devices,
            server,
            clients,
            inotify,
            caps_lock_is_active,
        })
    }

    fn pollfds(&self) -> Vec<PollFd<'_>> {
        self.devices
            .values()
            .map(|device| PollFd::new(device, PollFlags::IN))
            .chain([
                PollFd::new(&self.server, PollFlags::IN),
                PollFd::new(&self.inotify, PollFlags::IN),
            ])
            .collect()
    }

    fn poll0(&self) -> Result<Vec<(i32, PollFlags)>> {
        let mut pollfds = self.pollfds();
        rustix::event::poll(&mut pollfds, None)?;
        Ok(pollfds
            .into_iter()
            .map(|pollfd| (pollfd.as_fd().as_raw_fd(), pollfd.revents()))
            .collect())
    }

    pub(crate) fn poll_readable(&mut self) -> Result<Vec<i32>> {
        let mut fds = vec![];
        for (fd, revents) in self.poll0()? {
            match (Op::from(revents), self.classify_fd_kind(fd)?) {
                (Op::Success, _) => {
                    fds.push(fd);
                }
                (Op::Failure, AppFdKind::Device) => {
                    log::error!("Device {fd} died: {revents:?}");
                    self.remove_device_by_fd(fd);
                }
                (Op::Failure, AppFdKind::Server) => {
                    bail!("server died: {revents:?}");
                }
                (Op::Failure, AppFdKind::Client) => {
                    log::error!("Client {fd} died: {revents:?}");
                    self.clients.remove(&fd);
                }
                (Op::Failure, AppFdKind::INotify) => {
                    bail!("inotify died: {revents:?}");
                }
                (Op::Skip, _) => {}
            }
        }
        Ok(fds)
    }

    fn classify_fd_kind(&self, fd: i32) -> Result<AppFdKind> {
        if self.server.as_raw_fd() == fd {
            Ok(AppFdKind::Server)
        } else if self.inotify.as_raw_fd() == fd {
            Ok(AppFdKind::INotify)
        } else if self.devices.contains_key(&fd) {
            Ok(AppFdKind::Device)
        } else if self.clients.contains_key(&fd) {
            Ok(AppFdKind::Client)
        } else {
            bail!("unknown fd {fd}")
        }
    }

    pub(crate) fn classify_fd_mut(&mut self, fd: i32) -> Result<AppFd<'_>> {
        if let Some(device) = self.devices.get_mut(&fd) {
            Ok(AppFd::Device(device))
        } else if self.server.as_raw_fd() == fd {
            Ok(AppFd::Server(&mut self.server))
        } else if self.inotify.as_raw_fd() == fd {
            Ok(AppFd::INotify(&mut self.inotify))
        } else {
            bail!("unknown FD {fd}")
        }
    }

    pub(crate) fn remove_device_by_fd(&mut self, fd: i32) {
        self.devices.remove(&fd);
    }

    pub(crate) fn add_client(&mut self, client: Client) {
        if let Err(err) = client.write(self.caps_lock_is_active) {
            log::error!("{err:?}");
            return;
        }

        let fd = client.as_raw_fd();
        self.clients.insert(fd, client);
    }

    pub(crate) fn broadcast(&mut self, value: bool) {
        log::info!("Sending {value} to {} clients..", self.clients.len());

        let mut fds_to_drop = vec![];

        for (fd, client) in &self.clients {
            if let Err(err) = client.write(value) {
                log::error!("{err:?}");
                fds_to_drop.push(*fd);
            }
        }

        for fd in fds_to_drop {
            self.clients.remove(&fd);
        }
    }

    pub(crate) const fn set_caps_lock_is_active(&mut self, value: bool) -> Option<bool> {
        if self.caps_lock_is_active == value {
            None
        } else {
            self.caps_lock_is_active = value;
            Some(value)
        }
    }

    pub(crate) fn update_devices(&mut self) -> Result<()> {
        log::warn!("Updating input devices...");
        let (devices, caps_lock_is_active) = devices()?;
        self.devices = devices;
        self.caps_lock_is_active = caps_lock_is_active;
        Ok(())
    }
}

enum AppFdKind {
    Device,
    Server,
    Client,
    INotify,
}

enum Op {
    Success,
    Failure,
    Skip,
}
impl From<PollFlags> for Op {
    fn from(revents: PollFlags) -> Self {
        if revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL) {
            Self::Failure
        } else if revents.contains(PollFlags::IN) {
            Self::Success
        } else {
            Self::Skip
        }
    }
}

pub enum AppFd<'a> {
    Device(&'a mut Device),
    Server(&'a mut Server),
    INotify(&'a mut INotify),
}

fn devices() -> Result<(HashMap<i32, Device>, bool)> {
    let xkb_state = XkbState::new()?;
    let on_press = xkb_state.on_caps_lock_pressed();
    let on_release = xkb_state.on_caps_lock_released();

    let mut devices = HashMap::new();
    let mut caps_lock_is_active = false;
    for (path, dev) in evdev::enumerate() {
        let on_press = Rc::clone(&on_press);
        let on_release = Rc::clone(&on_release);

        let Some(device) = Device::new(path, dev, on_press, on_release)? else {
            continue;
        };

        log::trace!("Found {device:?} (fd={})", device.as_raw_fd());
        caps_lock_is_active |= device.caps_lock_led_is_active()?;
        devices.insert(device.as_raw_fd(), device);
    }
    xkb_state.set_initial_caps_lock_state(caps_lock_is_active)?;

    Ok((devices, caps_lock_is_active))
}
