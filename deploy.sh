#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
# set -o xtrace

if [[ ! -v TARGET_HOST ]]; then
    readonly TARGET_HOST=pi@hostname
fi

if [[ ! -v TARGET_PATH ]]; then
    readonly TARGET_PATH=/home/pi/rust-usbtmc
fi

if [[ ! -v TARGET_ARCH ]]; then
    readonly TARGET_ARCH=armv7-unknown-linux-gnueabihf
fi

if [[ ! -v SOURCE_PATH ]]; then
    readonly SOURCE_PATH=./target/${TARGET_ARCH}/release/rust-usbtmc
fi

# Commands
# rustup target add armv7-unknown-linux-gnueabihf
# sudo apt install gcc-arm-linux-gnueabihf

# .cargo/config
# [target.armv7-unknown-linux-gnueabihf]
# linker = "arm-linux-gnueabihf-gcc"

# Run deploy

cargo build --release --target=${TARGET_ARCH}
rsync ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}
if [ $# -ne 0 ]; then
    ssh -t ${TARGET_HOST} ${TARGET_PATH} $1
else
    ssh -t ${TARGET_HOST} ${TARGET_PATH}
fi