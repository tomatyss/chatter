# Installation

You can install Chatter either with Homebrew or by building from source. Homebrew is the fastest path for macOS users, while the source build works on every platform supported by Rust.

## Homebrew

```bash
brew tap tomatyss/chatter
brew install chatter
```

Upgrades follow the usual `brew update && brew upgrade chatter` flow.

## From Source

```bash
git clone https://github.com/tomatyss/chatter.git
cd chatter
cargo build --release
sudo cp target/release/chatter /usr/local/bin/
```

You need the Rust toolchain (via [rustup](https://rustup.rs/)) and a C toolchain for compiling the dependency chain. Once the binary is copied into your `$PATH`, run `chatter --help` to confirm the installation.
