#!/usr/bin/env bash
set -euo pipefail

echo "Building for Linux (GNU libc)"
# rustup target add x86_64-unknown-linux-gnu
cargo auditable build --profile=release-lto --target=x86_64-unknown-linux-gnu
ls -lah target/x86_64-unknown-linux-gnu/release-lto/k8sfwd

echo "Building for Linux (musl)"
# rustup target add x86_64-unknown-linux-musl
cargo auditable build --profile=release-lto --target=x86_64-unknown-linux-musl
ls -lah target/x86_64-unknown-linux-musl/release-lto/k8sfwd
