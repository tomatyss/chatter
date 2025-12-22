# Development Guide

Chatter is a Rust 2021 project organised as a single binary crate. Key directories:

- `src/cli` — argument parsing and command wiring using `clap`
- `src/chat` — interactive chat runtime and terminal presentation using `crossterm` and `ratatui`
- `src/api` — provider-specific HTTP clients
- `src/agent` — tool definitions and execution
- `src/config` — persistent configuration management
- `src/templates` — reusable output templates

## Building and Testing

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

Use `cargo build --release` for production builds. The `build.sh` script wraps a release build plus Homebrew packaging steps.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement the change and any relevant tests
4. Run the command suite above
5. Open a pull request with a summary of the change
