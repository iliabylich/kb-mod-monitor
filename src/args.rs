#[derive(Debug)]
pub struct Args {
    pub(crate) mode: RunMode,
}

#[derive(Debug)]
pub enum RunMode {
    Systemd,
    Dev,
}

const USAGE: &str = "
Usage: kb-mod-monitor [--systemd|--dev]

--systemd: runs daemon under systemd and expects it to pass a socket via sd_listen_fds()
    --dev: starts listening on /run/kb-mod-monitor-dev.sock
";
fn print_usage_and_exit() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(1);
}

impl Args {
    pub(crate) fn parse() -> Self {
        let mut mode = None;

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--systemd" => mode = Some(RunMode::Systemd),
                "--dev" => mode = Some(RunMode::Dev),
                other => {
                    eprintln!("Unknown argument {other:?}");
                    print_usage_and_exit();
                }
            }
        }

        let mode = mode.unwrap_or_else(|| {
            eprintln!("Either --systemd or --dev is required");
            print_usage_and_exit()
        });

        Self { mode }
    }
}
