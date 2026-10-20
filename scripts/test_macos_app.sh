#!/usr/bin/env bash
# Only uses an owned temporary directory; does not modify /Applications or user data.
set -euo pipefail
[[ $# == 1 ]] || { echo 'Usage: test_macos_app.sh CodeCedar-macOS.dmg' >&2; exit 2; }
work=$(mktemp -d)
mounted=0
cleanup() {
  if [[ $mounted == 1 ]]; then hdiutil detach "$work/mounted"; fi
  rm -rf "$work"
}
trap cleanup EXIT
mkdir -p "$work/mounted" "$work/Applications"
hdiutil attach -readonly -nobrowse -mountpoint "$work/mounted" "$1"
mounted=1
ditto "$work/mounted/CodeCedar.app" "$work/Applications/CodeCedar.app"
codesign --verify --deep --strict "$work/Applications/CodeCedar.app"
"$work/Applications/CodeCedar.app/Contents/MacOS/codecedar" --help
# A copy/replace/remove smoke, not Gatekeeper, interactive Finder, or a signed updater.
printf 'keep unrelated file\n' > "$work/Applications/unrelated.txt"
rm -rf "$work/Applications/CodeCedar.app"
ditto "$work/mounted/CodeCedar.app" "$work/Applications/CodeCedar.app"
codesign --verify --deep --strict "$work/Applications/CodeCedar.app"
rm -rf "$work/Applications/CodeCedar.app"
test ! -e "$work/Applications/CodeCedar.app"
grep -q 'keep unrelated file' "$work/Applications/unrelated.txt"
echo 'PASS: mounted DMG, copied app, signature, help, replacement/removal, unrelated file preserved'
