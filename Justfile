default:
    @just --list

connect path:
    cargo run --example cli -- {{path}}

dev log_level='trace':
    cargo build && sudo RUST_LOG={{log_level}} ./target/debug/kb-mod-monitor --dev
