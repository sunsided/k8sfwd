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

# TODO: Add OSX support
# echo "Building for Apple Darwin"
# # rustup target add x86_64-apple-darwin
# cargo auditable build --profile=${PROFILE} --target=x86_64-apple-darwin
# ls -lah target/x86_64-apple-darwin/${PROFILE}/k8sfwd
