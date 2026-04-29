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

## testing

```sh
socat -u UNIX-CONNECT:/run/caps-lock-daemon.sock - | while IFS= read -r -n1 state; do
    case "$state" in
        1) echo "activated" ;;
        0) echo "deactivated" ;;
    esac;
done
```
