# caps-lock-daemon

Clients can connect to `/run/caps-lock-daemon.sock`. Each state change is sent as a single byte:

- `1`: Caps Lock activated
- `0`: Caps Lock deactivated

New clients receive the current state immediately after connecting.

## Testing

+ `just run` builds and runs a daemon locally, requires sude because your user is not in the `input` group (and if you are you should really re-consider it), and so running as `root` is a must
+ `just connect` connects to a locally running instance using `socat`
