use anyhow::{Context as _, Result};
use rustix::net::{AddressFamily, SocketAddrUnix, SocketType};

const USAGE: &str = "
Usage: cli /path/to/socket [--query-at-start]
";
fn print_usage_and_exit() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(1);
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| {
        eprintln!("no path given");
        print_usage_and_exit()
    });
    let query_at_start = args.next().is_some_and(|arg| arg == "--query-at-start");

    let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)
        .context("socket() failed")?;

    let addr = SocketAddrUnix::new(path).context("can't build UNIX socket address")?;
    rustix::net::connect(&fd, &addr).context("connect() failed")?;

    if query_at_start {
        rustix::io::write(&fd, b"1").context("write() failed")?;
    }

    loop {
        let mut buf = [0; 2];
        let len = rustix::io::read(&fd, &mut buf)?;
        let buf = &buf[..len];

        for byte in buf {
            println!(
                "{}",
                match &byte {
                    b'0' => "CapsLock activated",
                    b'1' => "CapsLock deactivated",
                    b'2' => "NumLock activated",
                    b'3' => "NumLock deactivated",
                    _ => panic!("Unknown byte received: {byte}"),
                }
            );
        }
    }
}
