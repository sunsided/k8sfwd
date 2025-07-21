#!/usr/bin/env bash
set -euo pipefail

PACKAGE_VERSION=$(sed -n 's/^version *= *"\(.*\)"/\1/p' "Cargo.toml")
echo "Cross-building for version ${PACKAGE_VERSION}"

PROFILE=release-lto

echo "Building for Linux (GNU libc)"
LINUX_FLAVOR=linux-gnu
TARGET=x86_64-unknown-${LINUX_FLAVOR}
# rustup target add x86_64-unknown-linux-gnu
cargo auditable build --profile=${PROFILE} --target=${TARGET}
ls -lah target/${TARGET}/${PROFILE}/k8sfwd
gzip --keep -c target/${TARGET}/${PROFILE}/k8sfwd > "k8sfwd-${PACKAGE_VERSION}-${LINUX_FLAVOR}.gz"

echo "Building for Linux (musl)"
LINUX_FLAVOR=linux-musl
TARGET=x86_64-unknown-${LINUX_FLAVOR}
# rustup target add x86_64-unknown-linux-musl
cargo auditable build --profile=${PROFILE} --target=${TARGET}
ls -lah target/${TARGET}/${PROFILE}/k8sfwd
gzip --keep -c target/${TARGET}/${PROFILE}/k8sfwd > "k8sfwd-${PACKAGE_VERSION}-${LINUX_FLAVOR}.gz"

exit 0

echo "Building for Apple Darwin"
FLAVOR=apple-darwin
TARGET=x86_64-${FLAVOR}
sudo apt install -y gcc-multilib g++-multilib
rustup target add x86_64-apple-darwin
export CC=o64-clang
export CXX=o64-clang++
cargo auditable build --profile=${PROFILE} --target=${TARGET}
ls -lah target/${TARGET}/${PROFILE}/k8sfwd
gzip --keep -c target/${TARGET}/${PROFILE}/k8sfwd > "k8sfwd-${PACKAGE_VERSION}-${FLAVOR}.gz"

