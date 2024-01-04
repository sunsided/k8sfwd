#!/usr/bin/env bash

set -euo pipefail

echo "Bundling Cargo.lock into cargo-sources.json"
python3 flatpak-cargo-generator.py ../Cargo.lock -o cargo-sources.json

echo "Test-installing the FlatPak"
flatpak-builder --install --force-clean --user build com.github.sunsided.k8sfwd.yaml

# echo "Test-building the FlatPak into a local repo"
# flatpak-builder --repo=repo --force-clean build com.github.sunsided.k8sfwd.yaml
# flatpak --user remote-add --no-gpg-verify k8sfwd-repo repo
# flatpak --user install k8sfwd-repo com.github.sunsided.k8sfwd

echo "Test with"
echo "flatpak run com.github.sunsided.k8sfwd"
