default:
    @just --list

connect:
    #!/usr/bin/env bash
    socat -u UNIX-CONNECT:/run/caps-lock-daemon.sock - | while IFS= read -r -n1 state; do
        case "$state" in
            1) echo "activated" ;;
            0) echo "deactivated" ;;
        esac;
    done

run log_level='trace':
    cargo build && sudo RUST_LOG={{log_level}} ./target/debug/caps-lock-daemon
