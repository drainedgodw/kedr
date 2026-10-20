#!/usr/bin/env bash
# DESTRUCTIVE package install/remove acceptance, only in a disposable CI container.
set -euo pipefail
[[ "${KEDR_DISPOSABLE_PACKAGE_TEST:-}" == 1 ]] || { echo 'Refusing: disposable-container opt-in is required' >&2; exit 2; }
[[ $# == 2 ]] || { echo 'Usage: test_native_linux.sh deb|rpm /absolute/package' >&2; exit 2; }
format=$1
artifact=$(realpath "$2")
case "$format" in
  deb)
    apt-get update
    apt-get install -y "$artifact"
    dpkg-query -W -f='${Status}\n${Depends}\n' codecedar
    ;;
  rpm)
    dnf install -y "$artifact"
    rpm -q --requires codecedar
    ;;
  *) echo 'Unknown package format' >&2; exit 2 ;;
esac
codecedar --help
if ldd /usr/bin/codecedar | grep -q 'not found'; then echo 'Unresolved linked runtime library' >&2; exit 1; fi
# Install test equipment AFTER installation/linked dependency acceptance.
if [[ $format == deb ]]; then
  apt-get install -y python3 desktop-file-utils xvfb xauth libgl1-mesa-dri
else
  dnf install -y python3 desktop-file-utils xorg-x11-server-Xvfb xorg-x11-xauth mesa-dri-drivers shadow-utils util-linux
fi
desktop-file-validate /usr/share/applications/io.github.drainedgodw.CodeCedar.desktop
test -f /usr/share/icons/hicolor/scalable/apps/codecedar.svg
useradd -m kedrtest
runuser -u kedrtest -- mkdir -p /home/kedrtest/.config/kedr
printf 'keep this unrelated user file\n' > /home/kedrtest/.config/kedr/acceptance-sentinel
# Run the installed binary, not target/release; real native render, no project code.
runuser -u kedrtest -- env HOME=/home/kedrtest LIBGL_ALWAYS_SOFTWARE=1 \
  xvfb-run -a codecedar --capture /home/kedrtest/package-smoke.png --state welcome --size 900x640
python3 - <<'PY'
from pathlib import Path
p = Path('/home/kedrtest/package-smoke.png')
assert p.read_bytes()[:8] == b'\x89PNG\r\n\x1a\n' and p.stat().st_size > 4096
PY
if [[ $format == deb ]]; then
  apt-get install --reinstall -y "$artifact"
  apt-get remove -y codecedar
else
  dnf reinstall -y "$artifact"
  dnf remove -y codecedar
fi
test ! -e /usr/bin/codecedar
test ! -e /usr/share/applications/io.github.drainedgodw.CodeCedar.desktop
test ! -e /usr/share/icons/hicolor/scalable/apps/codecedar.svg
grep -q 'keep this unrelated user file' /home/kedrtest/.config/kedr/acceptance-sentinel
echo 'PASS: native dependency install, linked libraries, help, X11 render, reinstall, uninstall, user-file preservation'
