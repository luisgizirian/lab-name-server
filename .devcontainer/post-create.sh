#!/usr/bin/env bash
set -euo pipefail

# Update apt and install helpful tools
sudo apt-get update -y
sudo apt-get install -y --no-install-recommends dnsutils iproute2 libcap2-bin curl ca-certificates gdb

# Pre-fetch Rust deps for faster first build
if [ -f Cargo.toml ]; then
  cargo fetch || true
fi

# Optional: Setup cargo tools
cargo install cargo-watch --locked || true

# Create a default config if not present
if [ ! -f config.yaml ] && [ -f config.sample.yaml ]; then
  cp config.sample.yaml config.yaml
fi

# Allow binding to privileged ports for built binary path under workspace when run inside container
BIN_PATH="/workspaces/lab-name-server/target/debug/lab-name-server"
if [ -f "$BIN_PATH" ]; then
  sudo setcap 'cap_net_bind_service=+ep' "$BIN_PATH" || true
fi
