use crate::{
    client::Client,
    inotify::INotify,
    keyboard::{Keyboard, diff::KeyboardStateDiff, state::KeyboardState},
    server::Server,
    xkb_state::{XkbState, callbacks::XkbCallbacks},
};
use anyhow::{Result, bail};
use evdev::KeyCode;
use rustix::event::{PollFd, PollFlags};
use std::{
    collections::HashMap,
    os::fd::{AsFd as _, AsRawFd},
};

pub struct State {
    devices: HashMap<i32, Keyboard>,
    server: Server,
    clients: HashMap<i32, Client>,
    inotify: INotify,
}

impl State {
    pub(crate) fn new() -> Result<(Self, KeyboardState)> {
        let (devices, kb_state) = keyboards()?;
        let server = Server::new()?;
        let clients = HashMap::new();
        let inotify = INotify::new()?;

        Ok((
            Self {
                devices,
                server,
                clients,
                inotify,
            },
            kb_state,
        ))
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

    pub(crate) fn add_client(&mut self, client: Client, kb_state: KeyboardState) {
        if let Err(err) = client.write(kb_state.as_diff()) {
            log::error!("{err:?}");
            return;
        }

        let fd = client.as_raw_fd();
        self.clients.insert(fd, client);
    }

    pub(crate) fn broadcast(&mut self, diff: KeyboardStateDiff) {
        log::info!("Sending {diff:?} to {} clients..", self.clients.len());

        let mut fds_to_drop = vec![];

        for (fd, client) in &self.clients {
            if let Err(err) = client.write(diff) {
                log::error!("{err:?}");
                fds_to_drop.push(*fd);
            }
        }

        for fd in fds_to_drop {
            self.clients.remove(&fd);
        }
    }

    pub(crate) fn update_devices(&mut self) -> Result<KeyboardState> {
        log::warn!("New device in /dev, refreshing device list...");
        let (devices, kb_state) = keyboards()?;
        self.devices = devices;
        Ok(kb_state)
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
    Device(&'a mut Keyboard),
    Server(&'a mut Server),
    INotify(&'a mut INotify),
}

fn keyboards() -> Result<(HashMap<i32, Keyboard>, KeyboardState)> {
    let xkb_state = XkbState::new()?;
    let xkb_callbacks = XkbCallbacks::new(&xkb_state);

    let mut keyboards = HashMap::new();
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

        let keyboard = Keyboard::new(path, dev, xkb_callbacks.clone())?;

        log::info!("Found {keyboard:?} (fd={})", keyboard.as_raw_fd());
        let led_state = keyboard.led_based_state();
        kb_state = kb_state | led_state;
        keyboards.insert(keyboard.as_raw_fd(), keyboard);
    }
    xkb_state.set_initial_state(kb_state);

    Ok((keyboards, kb_state))
}
