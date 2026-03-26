#!/usr/bin/env bash

set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
    cat <<'EOF'
Install the Linux build dependencies for Rust Calendar on Debian/Ubuntu/Mint.

Usage:
  ./scripts/install-linux-build-deps.sh
  ./scripts/install-linux-build-deps.sh --dry-run

This installs the top-level packages needed for local builds and SHIP verification.
APT resolves the remaining GTK, GLib, Cairo, Pango, GDK Pixbuf, X11, and tray-related
development packages automatically.
EOF
    exit 0
fi

if [[ ! -r /etc/os-release ]]; then
    echo "Error: Cannot determine Linux distribution from /etc/os-release."
    exit 1
fi

. /etc/os-release

if [[ "${ID:-}" != "ubuntu" && "${ID:-}" != "linuxmint" && "${ID_LIKE:-}" != *"ubuntu"* && "${ID_LIKE:-}" != *"debian"* ]]; then
    echo "Error: This helper currently supports Debian/Ubuntu/Mint style systems only."
    echo "Install the equivalent packages for your distribution manually."
    exit 1
fi

packages=(
    build-essential
    libglib2.0-dev
    libgtk-3-dev
    libssl-dev
    libayatana-appindicator3-dev
    libxdo-dev
)

if [[ "${1:-}" == "--dry-run" ]]; then
    installer=(apt-get)
    apt_args=(-s install)
    echo "Running in dry-run mode."
else
    installer=(sudo apt-get)
    apt_args=(install -y)
    echo "Updating apt package lists..."
    sudo apt-get update
fi

echo "Installing Rust Calendar Linux build dependencies..."
echo "Packages: ${packages[*]}"

"${installer[@]}" "${apt_args[@]}" "${packages[@]}"

cat <<'EOF'

Done.

Validate the permanent setup with:
  cargo fmt -- --check
  cargo clippy -- -D warnings
  cargo test
  cargo build
EOF