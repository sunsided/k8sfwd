#!/usr/bin/env bash

set -euo pipefail

echo "Bundling Cargo.lock into cargo-sources.json"
python3 flatpak-cargo-generator.py ../Cargo.lock -o cargo-sources.json

echo "Test-installing the FlatPak"
flatpak-builder --install --force-clean --user build com.github.sunsided.k8sfwd.yaml

echo "Test with"
echo "flatpak run com.github.sunsided.k8sfwd"
