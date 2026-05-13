default:
    @just --list

connect:
    #!/usr/bin/env bash
    socat -u UNIX-CONNECT:/run/kb-mod-monitor.sock - | while IFS= read -r -n1 state; do
        case "$state" in
            0) echo "caps lock deactivated" ;;
            1) echo "caps lock activated" ;;
            2) echo "num lock deactivated" ;;
            3) echo "num lock activated" ;;
        esac;
    done

run log_level='trace':
    cargo build && sudo RUST_LOG={{log_level}} ./target/debug/kb-mod-monitor
