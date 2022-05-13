#!/bin/bash
set -e

function install_rust() {
  curl https://sh.rustup.rs -sSf --output /tmp/rustup.sh
  chmod +x /tmp/rustup.sh  
  /tmp/rustup.sh -y --profile minimal --target $RUST_TARGET
  source $HOME/.cargo/env
  cargo install cargo-get
}

. /root/env/bcm-hnd-802.11ax.sh
install_rust
aarch64-linux-gcc -v
TARGET_CC=aarch64-linux-gcc cargo build --manifest-path http_server/Cargo.toml --release --target $RUST_TARGET -vv
