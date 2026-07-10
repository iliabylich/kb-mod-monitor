#![forbid(unsafe_code)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::os::fd::{AsFd as _, AsRawFd as _};

use all_devices::AllDevices;
use anyhow::{Result, bail};
use args::Args;
use inotify::INotify;
use keyboard::diff::KeyboardStateDiff;
use rustix::event::PollFlags;
use server::Server;

mod all_devices;
mod args;
mod client;
mod inotify;
mod key;
mod keyboard;
mod server;
mod xkb_state;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_target(false)
        .write_style(env_logger::WriteStyle::Always)
        .init();
    let args = Args::parse();
    log::trace!("Running with args {args:?}");

    let mut devices;
    let mut kb_state;
    (devices, kb_state) = AllDevices::new()?;
    let mut server = Server::new(&args)?;
    let inotify = INotify::new()?;

    loop {
        let (server_events, keyboard_events) = poll(&devices, &server, &inotify)?;
        let mut diff = KeyboardStateDiff::empty();

        match keyboard_events {
            KeyboardEvents::DevicesChangedOnDiskEvent => {
                inotify.drain()?;
                (devices, kb_state) = AllDevices::new()?;
            }
            KeyboardEvents::Updates(events) => {
                for event in events {
                    match event {
                        KeyboardEvent::Changed(fd) => {
                            diff = KeyboardStateDiff::merge(diff, devices.drain(fd));
                        }
                        KeyboardEvent::Removed(fd) => devices.remove(fd),
                    }
                }
            }
            KeyboardEvents::None => {}
        }

        if let Some(diff_to_send) = kb_state.apply(diff) {
            server.broadcast(diff_to_send);
        }

        for event in server_events {
            match event {
                ServerEvent::NewClient => server.accept()?,
                ServerEvent::Request(fd) => server.reply(fd, kb_state.as_diff()),
                ServerEvent::Disconnected(fd) => server.remove_client(fd),
            }
        }
    }
}

fn poll(
    devices: &AllDevices,
    server: &Server,
    inotify: &INotify,
) -> Result<(Vec<ServerEvent>, KeyboardEvents)> {
    let mut pollfds = std::iter::empty()
        .chain(devices.pollfds())
        .chain(server.pollfds())
        .chain(inotify.pollfds())
        .collect::<Vec<_>>();
    rustix::event::poll(&mut pollfds, None)?;
    let fd_to_revents = pollfds
        .into_iter()
        .map(|pollfd| (pollfd.as_fd().as_raw_fd(), pollfd.revents()))
        .collect::<Vec<_>>();
    let mut server_events = vec![];
    let mut keyboard_events = KeyboardEvents::new();

    for (fd, revents) in fd_to_revents {
        let Some(revents) = REvents::new(fd, revents) else {
            continue;
        };
        let kind = FdKind::classify(fd, inotify, devices, server)?;

        match (kind, revents) {
            (FdKind::INotify, REvents::Readable) => keyboard_events.devices_changed_on_disk(),
            (FdKind::INotify, REvents::Err) => bail!("inotify has crashed"),

            (FdKind::Device, REvents::Readable) => keyboard_events.keyboard_state_changed(fd),
            (FdKind::Device, REvents::Err) => keyboard_events.keyboard_removed(fd),

            (FdKind::Server, REvents::Readable) => server_events.push(ServerEvent::NewClient),
            (FdKind::Server, REvents::Err) => bail!("server has crashed"),

            (FdKind::Client, REvents::Readable) => server_events.push(ServerEvent::Request(fd)),
            (FdKind::Client, REvents::Err) => server_events.push(ServerEvent::Disconnected(fd)),
        }
    }

    Ok((server_events, keyboard_events))
}

enum REvents {
    Readable,
    Err,
}
impl REvents {
    fn new(fd: i32, revents: PollFlags) -> Option<Self> {
        if revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL) {
            log::error!("FD {fd} returned revents {revents:?}");
            Some(Self::Err)
        } else if revents.contains(PollFlags::IN) {
            Some(Self::Readable)
        } else {
            None
        }
    }
}

enum FdKind {
    INotify,
    Device,
    Server,
    Client,
}
impl FdKind {
    fn classify(fd: i32, inotify: &INotify, devices: &AllDevices, server: &Server) -> Result<Self> {
        if fd == inotify.as_raw_fd() {
            Ok(Self::INotify)
        } else if devices.contains(fd) {
            Ok(Self::Device)
        } else if fd == server.as_raw_fd() {
            Ok(Self::Server)
        } else if server.contains_client(fd) {
            Ok(Self::Client)
        } else {
            bail!("Unknown FD {fd}")
        }
    }
}

enum ServerEvent {
    NewClient,
    Request(i32),
    Disconnected(i32),
}

enum KeyboardEvent {
    Changed(i32),
    Removed(i32),
}

enum KeyboardEvents {
    DevicesChangedOnDiskEvent,
    Updates(Vec<KeyboardEvent>),
    None,
}
impl KeyboardEvents {
    const fn new() -> Self {
        Self::None
    }

    fn devices_changed_on_disk(&mut self) {
        *self = Self::DevicesChangedOnDiskEvent;
    }

    fn push_event(&mut self, e: KeyboardEvent) {
        match self {
            Self::DevicesChangedOnDiskEvent => {}
            Self::Updates(events) => events.push(e),
            Self::None => *self = Self::Updates(vec![e]),
        }
    }

    fn keyboard_state_changed(&mut self, fd: i32) {
        self.push_event(KeyboardEvent::Changed(fd));
    }

    fn keyboard_removed(&mut self, fd: i32) {
        self.push_event(KeyboardEvent::Removed(fd));
    }
}
