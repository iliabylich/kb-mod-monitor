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
use keyboard::diff::KeyboardStateDiff;
use state::{AppFd, State};

use crate::client::Client;

mod client;
mod inotify;
mod key;
mod keyboard;
mod server;
mod state;
mod xkb_state;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_target(false)
        .write_style(env_logger::WriteStyle::Always)
        .init();

    let (mut state, mut kb_state) = State::new()?;

    loop {
        let fds = state.poll_readable()?;
        let mut diff = KeyboardStateDiff::empty();

        for fd in fds {
            match state.classify_fd_mut(fd)? {
                AppFd::Device(device) => match device.drain() {
                    Ok(subdiff) => {
                        diff = subdiff.or(diff);
                    }
                    Err(err) => {
                        log::error!("{err:?}");
                        state.remove_device_by_fd(fd);
                    }
                },

                AppFd::Server(server) => {
                    let fd = server.accept()?;
                    state.add_client(Client::new(fd), kb_state);
                }

                AppFd::INotify(inotify) => {
                    inotify.drain()?;
                    state.update_devices()?;
                }
            }
        }

        if let Some(diff_to_send) = kb_state.apply(diff) {
            state.broadcast(diff_to_send);
        }
    }
}
