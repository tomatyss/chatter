# Installation

You can install Chatter either with Homebrew or by building from source. Homebrew is the fastest path for macOS users, while the source build works on every platform supported by Rust.

## Debian / Ubuntu (APT)

An official APT repository is published at `https://tomatyss.github.io/chatter/apt`. Import the signing key, add the source, and install:

``` bash
curl -fsSL https://tomatyss.github.io/chatter/apt/KEY.gpg | \
  sudo gpg --dearmor -o /usr/share/keyrings/chatter-archive-keyring.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/chatter-archive-keyring.gpg] \
  https://tomatyss.github.io/chatter/apt stable main" | \
  sudo tee /etc/apt/sources.list.d/chatter.list
sudo apt update
sudo apt install chatter
```

The repository currently ships 64-bit (amd64) builds created by the automated release workflow. If you are working from a fork, replace the base URL with your own GitHub Pages endpoint.

## Homebrew

``` bash
brew tap tomatyss/chatter
brew install chatter
```

Upgrades follow the usual `brew update && brew upgrade chatter` flow.

## From Source

``` bash
git clone https://github.com/tomatyss/chatter.git
cd chatter
cargo build --release
sudo cp target/release/chatter /usr/local/bin/
```

You need the Rust toolchain (via [rustup](https://rustup.rs/)) and a C toolchain for compiling the dependency chain. Once the binary is copied into your `$PATH`, run `chatter --help` to confirm the installation.

