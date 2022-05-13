#!/bin/bash
set -e

function install_rust() {
  curl https://sh.rustup.rs -sSf --output /tmp/rustup.sh
  chmod +x /tmp/rustup.sh  
  /tmp/rustup.sh -y --profile minimal --target $RUST_TARGET
  source $HOME/.cargo/env
}

#. /root/env/bcm-hnd-802.11ax.sh
ln -sf /root/am-toolchains/brcm-arm-hnd /opt/toolchains
export LD_LIBRARY_PATH=/opt/toolchains/crosstools-arm-gcc-5.3-linux-4.1-glibc-2.22-binutils-2.25/usr/lib:$LD_LIBRARY
export TOOLCHAIN_BASE=/opt/toolchains
export PATH=/opt/toolchains/crosstools-arm-gcc-5.3-linux-4.1-glibc-2.22-binutils-2.25/usr/bin:$PATH
export PATH=/opt/toolchains/crosstools-aarch64-gcc-5.3-linux-4.1-glibc-2.22-binutils-2.25/usr/bin:$PATH

install_rust
aarch64-linux-gcc -v
TARGET_CC=aarch64-linux-gcc cargo build --manifest-path http_server/Cargo.toml --release --target $RUST_TARGET -vv
