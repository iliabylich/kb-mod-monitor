# caps-lock-daemon

```sh
just setup
just build
sudo just run
```

Clients can connect to `/run/caps-lock-daemon.sock`. Each state change is sent as a single byte:

- `1`: Caps Lock activated
- `0`: Caps Lock deactivated

New clients receive the current state immediately after connecting.
