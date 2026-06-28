# kb-mod-monitor

Clients can connect to `/run/kb-mod-monitor-{dev,systemd}.sock`. Each state change is sent as a single ASCII character:

- `0`: Caps Lock deactivated
- `1`: Caps Lock activated
- `2`: Num Lock deactivated
- `3`: Num Lock activated

New clients DON'T receive current state after connecting, but if you send a single byte the daemon will write both current Caps Lock and Num Lock state as events (i.e. then you can expect two bytes to be sent back).

## systemd socket activation

When `--systemd` is passed the daemon expects socket to be passed from systemd using [sd_listen_fds](https://www.freedesktop.org/software/systemd/man/latest/sd_listen_fds.html) function, the socket path in this case is `/run/kb-mod-monitor-systemd.sock`.

When `--dev` is passed the daemon starts in the "owned" mode and listens on `/run/kb-mod-monitor-dev.sock`.

These two options co-exist to let both development and release builds run in parallel on the same machine.

## Testing

+ `just run` builds and runs locally in `--dev` mode, requires `sudo` because your user is not in the `input` group (and if you are you should really re-consider it), and so running as `root` is a must
+ `just connect /path/to/socket` runs an example that connects to a locally running instance

Alternatively the following here's a quick demo using `socat`:

```bash
socat -u UNIX-CONNECT:/path/to/socket - | while IFS= read -r -n1 state; do
    case "$state" in
        0) echo "caps lock deactivated" ;;
        1) echo "caps lock activated" ;;
        2) echo "num lock deactivated" ;;
        3) echo "num lock activated" ;;
    esac;
done
```
