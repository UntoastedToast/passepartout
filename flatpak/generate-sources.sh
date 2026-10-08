#!/usr/bin/env bash
# Generates flatpak/cargo-sources.json from Cargo.lock for the offline Flatpak build.
set -euo pipefail

generator_commit=74697c75b630d7330e77250fc13cb5ea688d9479
script="$(mktemp --suffix=.py)"
trap 'rm -f "$script"' EXIT

cd "$(dirname "$0")/.."
curl -fsSL "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/${generator_commit}/cargo/flatpak-cargo-generator.py" -o "$script"
uv run "$script" Cargo.lock -o flatpak/cargo-sources.json
