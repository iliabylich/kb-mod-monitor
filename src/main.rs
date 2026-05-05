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

use anyhow::Result;
use state::{AppFd, State};

use crate::client::Client;

mod client;
mod device;
mod inotify;
mod server;
mod state;
mod xkb_state;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_target(false)
        .write_style(env_logger::WriteStyle::Always)
        .init();

    let mut state = State::new()?;

    loop {
        let fds = state.poll_readable()?;
        let mut caps_lock_activated = None;

        for fd in fds {
            match state.classify_fd_mut(fd)? {
                AppFd::Device(device) => match device.drain() {
                    Ok(Some(value)) => {
                        log::trace!("{device:?} - {value}");
                        caps_lock_activated = Some(value);
                    }
                    Ok(None) => {}
                    Err(err) => {
                        log::error!("{err:?}");
                        state.remove_device_by_fd(fd);
                    }
                },

                AppFd::Server(server) => {
                    let fd = server.accept()?;
                    state.add_client(Client::new(fd));
                }

                AppFd::INotify(inotify) => {
                    inotify.drain()?;
                    state.update_devices()?;
                }
            }
        }

        let Some(caps_lock_activated) = caps_lock_activated else {
            continue;
        };
        log::trace!("=== Drained everyone, activated: {caps_lock_activated}");

        let Some(changed_to) = state.set_caps_lock_is_active(caps_lock_activated) else {
            continue;
        };
        state.broadcast(changed_to);
    }
}
