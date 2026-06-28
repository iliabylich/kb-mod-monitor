# kb-mod-monitor

Clients can connect to `/run/kb-mod-monitor-{dev,systemd}.sock`. Each state change is sent as a single ASCII character:

- `0`: Caps Lock deactivated
- `1`: Caps Lock activated
- `2`: Num Lock deactivated
- `3`: Num Lock activated

New clients receive current state immediately after connecting.

## systemd socket activation

When `--systemd` is passed the daemon expects socket to be passed from systemd using [sd_listen_fds](https://www.freedesktop.org/software/systemd/man/latest/sd_listen_fds.html) function, the socket path in this case is `/run/kb-mod-monitor-systemd.sock`.

When `--dev` is passed the daemon starts in the "owned" mode and listens on `/run/kb-mod-monitor-dev.sock`.

These two options co-exist to let both development and release builds run in parallel on the same machine.

## Testing

+ `just run` builds and runs locally, requires `sudo` because your user is not in the `input` group (and if you are you should really re-consider it), and so running as `root` is a must
+ `just connect /path/to/socket` runs a Bash script that connects to a locally running instance using `socat` (that is either a dev build or the one managed by systemd)
