test:
    cargo test
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

# Run the release build, forwarding each argument unchanged.
[positional-arguments]
run *args:
    cargo run --release -- "$@"

# Build the release binary and create a new local config.
init:
    @if [ -e config.toml ] || [ -L config.toml ]; then echo 'error: config.toml already exists' >&2; exit 1; fi
    cargo build --release
    (set -C; cat config_example.toml > config.toml)
