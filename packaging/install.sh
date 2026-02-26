#!/usr/bin/env bash
# Install Rust Calendar as a local desktop application on Linux.
# Usage: ./packaging/install.sh [--uninstall]
#
# Installs to ~/.local (no sudo required):
#   ~/.local/bin/rust-calendar
#   ~/.local/share/applications/rust-calendar.desktop
#   ~/.local/share/icons/hicolor/256x256/apps/rust-calendar.png

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN_SRC="$REPO_DIR/target/release/rust-calendar"
ICON_SRC="$REPO_DIR/assets/icons/663353.png"
DESKTOP_SRC="$REPO_DIR/packaging/rust-calendar.desktop"

BIN_DEST="$HOME/.local/bin/rust-calendar"
DESKTOP_DEST="$HOME/.local/share/applications/rust-calendar.desktop"
ICON_DEST="$HOME/.local/share/icons/hicolor/256x256/apps/rust-calendar.png"

uninstall() {
    echo "Uninstalling Rust Calendar..."
    rm -f "$BIN_DEST"
    rm -f "$DESKTOP_DEST"
    rm -f "$ICON_DEST"
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
    gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
    echo "Done. Rust Calendar has been removed."
    exit 0
}

if [[ "${1:-}" == "--uninstall" ]]; then
    uninstall
fi

# Pre-flight checks
if [[ ! -f "$BIN_SRC" ]]; then
    echo "Error: Release binary not found at $BIN_SRC"
    echo "Run 'cargo build --release' first."
    exit 1
fi

if [[ ! -f "$ICON_SRC" ]]; then
    echo "Warning: Icon not found at $ICON_SRC — skipping icon install."
fi

echo "Installing Rust Calendar v$(grep '^version' "$REPO_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')..."

# Create destination directories
mkdir -p "$(dirname "$BIN_DEST")"
mkdir -p "$(dirname "$DESKTOP_DEST")"
mkdir -p "$(dirname "$ICON_DEST")"

# Install binary
cp "$BIN_SRC" "$BIN_DEST"
chmod +x "$BIN_DEST"
echo "  Binary  → $BIN_DEST"

# Install icon (resize to 256x256 if possible, otherwise copy as-is)
if [[ -f "$ICON_SRC" ]]; then
    if command -v convert &>/dev/null; then
        convert "$ICON_SRC" -resize 256x256 "$ICON_DEST"
        echo "  Icon    → $ICON_DEST (resized to 256x256)"
    else
        cp "$ICON_SRC" "$ICON_DEST"
        echo "  Icon    → $ICON_DEST (original size)"
    fi
fi

# Install .desktop file (with absolute Exec path for reliability)
sed "s|^Exec=.*|Exec=$BIN_DEST|" "$DESKTOP_SRC" > "$DESKTOP_DEST"
chmod +x "$DESKTOP_DEST"
echo "  Desktop → $DESKTOP_DEST"

# Update desktop database and icon cache
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo ""
echo "Rust Calendar installed successfully!"
echo "You can launch it from the application menu or run: rust-calendar"
echo ""
echo "To uninstall: $SCRIPT_DIR/install.sh --uninstall"
