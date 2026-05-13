# kb-mod-monitor

Clients can connect to `/run/kb-mod-monitor.sock`. Each state change is sent as a single ASCII character:

- `0`: Caps Lock deactivated
- `1`: Caps Lock activated
- `2`: Num Lock deactivated
- `3`: Num Lock activated

New clients receive current state immediately after connecting.

## Testing

+ `just run` builds and runs locally, requires `sudo` because your user is not in the `input` group (and if you are you should really re-consider it), and so running as `root` is a must
+ `just connect` runs a Bash script that connects to a locally running instance using `socat`
