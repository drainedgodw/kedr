#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
command -v python3 >/dev/null 2>&1 || { printf '%s\n' 'Python 3 is required (Arch: sudo pacman -S python).'; exit 1; }
exec python3 "$ROOT/installer.py" --bundle "$ROOT" "$@"
