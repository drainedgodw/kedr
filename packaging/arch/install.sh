#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
command -v pacman >/dev/null 2>&1 && command -v makepkg >/dev/null 2>&1 || { echo 'This route requires Arch Linux, base-devel and makepkg.'; exit 1; }
[ "$(id -u)" -ne 0 ] || { echo 'Run as your regular user, not root. makepkg will request privileges for pacman.'; exit 1; }
printf '%s\n' '1. CodeCedar — available preview' 'Other Kedr products are planned; no installable packages exist.' '0. Cancel'
printf '%s' 'Choose [1/0]: '; read -r choice
[ "$choice" = 1 ] || exit 0
printf '%s\n' 'Review PKGBUILD first. This builds from the repository and installs a local package with pacman.'
printf '%s' 'Continue? [y/N] '; read -r answer
[ "$answer" = y ] || [ "$answer" = Y ] || exit 0
cd "$ROOT"
exec makepkg -si
