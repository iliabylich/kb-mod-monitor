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

use anyhow::Result;
use pollsendd::Runtime;

mod all_devices;
mod args;
mod inotify;
mod key;
mod keyboard;
mod service;
mod xkb_state;

use args::{Args, RunMode};
use service::KbModService;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_target(false)
        .write_style(env_logger::WriteStyle::Always)
        .init();
    let args = Args::parse();
    log::trace!("Running with args {args:?}");

    let service = KbModService::new()?;
    let mut rt = match args.mode {
        RunMode::Systemd => Runtime::new_with_systemd_socket(service),
        RunMode::Dev => Runtime::new_with_socket_path("/run/kb-mod-monitor-dev.sock", service),
    }?;

    loop {
        rt.poll()?;
    }
}
