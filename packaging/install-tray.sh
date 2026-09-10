#!/bin/bash
# Install the fido2-manage tray daemon autostart.
#
# Usage:
#   ./packaging/install-tray.sh systemd     # systemd --user service (default)
#   ./packaging/install-tray.sh autostart   # XDG autostart .desktop
set -euo pipefail

MODE="${1:-systemd}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$MODE" in
  systemd)
    dest="$HOME/.config/systemd/user"
    mkdir -p "$dest"
    cp "$SCRIPT_DIR/fido2-manage-tray.service" "$dest/"
    systemctl --user daemon-reload
    systemctl --user enable --now fido2-manage-tray.service
    echo "Installed and started systemd user service: fido2-manage-tray.service"
    echo "Status: systemctl --user status fido2-manage-tray.service"
    ;;
  autostart)
    dest="$HOME/.config/autostart"
    mkdir -p "$dest"
    cp "$SCRIPT_DIR/fido2-manage-tray.desktop" "$dest/"
    echo "Installed XDG autostart entry. It will start at your next login."
    echo "To start now: python3 \"$HOME/fido2-manage/fido2_tray.py\" &"
    ;;
  *)
    echo "Unknown mode '$MODE'. Use 'systemd' or 'autostart'." >&2
    exit 1
    ;;
esac
