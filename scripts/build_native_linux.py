#!/usr/bin/env python3
"""Build real distro packages, not a root-running download/bootstrap script.

Build on the target distribution. dpkg-shlibdeps/rpm augment the explicit
runtime list with ELF library/ABI requirements. No supplied binary is executed.
"""
import argparse
import json
import os
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = 'codecedar'
APP_ID = 'io.github.drainedgodw.CodeCedar'
# Libraries opened at runtime are invisible to ELF automatic dependency scanners.
RUNTIME = {
    'deb': ['libx11-6', 'libxcursor1', 'libxi6', 'libxrandr2', 'libxkbcommon0',
            'libxkbcommon-x11-0', 'libwayland-client0', 'libegl1', 'libgl1',
            'xdg-desktop-portal'],
    'rpm': ['libX11', 'libXcursor', 'libXi', 'libXrandr', 'libxkbcommon',
            'libxkbcommon-x11', 'libwayland-client.so.0()(64bit)', 'libglvnd-egl',
            'libglvnd-glx', 'xdg-desktop-portal'],
}
RECOMMENDS = {'deb': ['xdg-desktop-portal-gtk'], 'rpm': ['xdg-desktop-portal-gtk']}
ARCHES = {62: ('amd64', 'x86_64'), 183: ('arm64', 'aarch64')}


def run(args, **kwargs):
    return subprocess.run([str(x) for x in args], check=True, text=True, **kwargs)


def identity(binary):
    binary = Path(binary)
    if binary.is_symlink() or not binary.is_file():
        raise ValueError('Input must be a regular ELF binary, not a symlink')
    with binary.open('rb') as stream:
        header = stream.read(64)
    if len(header) != 64 or header[:7] != b'\x7fELF\x02\x01\x01':
        raise ValueError('Only 64-bit little-endian Linux ELF binaries are supported')
    kind, machine = struct.unpack_from('<HH', header, 16)
    if kind not in (2, 3) or machine not in ARCHES:
        raise ValueError('Unsupported ELF type or architecture')
    return ARCHES[machine]


def validate_version(version, release):
    if not re.fullmatch(r'[0-9]+(?:\.[0-9]+){1,3}', version):
        raise ValueError('Version must contain two to four numeric components')
    if not re.fullmatch(r'[0-9][A-Za-z0-9.]*', release):
        raise ValueError('Release must start with a digit and use only letters, digits or dot')


def stage(binary, destination):
    identity(binary)
    for source, target, mode in [
        (binary, 'usr/bin/codecedar', 0o755),
        (ROOT / 'packaging/linux/codecedar.desktop', f'usr/share/applications/{APP_ID}.desktop', 0o644),
        (ROOT / 'assets/brand/codecedar.svg', 'usr/share/icons/hicolor/scalable/apps/codecedar.svg', 0o644),
        (ROOT / 'LICENSE', 'usr/share/doc/codecedar/copyright', 0o644),
        (ROOT / 'docs/suite/INSTALL.md', 'usr/share/doc/codecedar/INSTALL.md', 0o644),
    ]:
        target = destination / target
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        target.chmod(mode)


def deb_dependencies(binary, work):
    # A minimal Debian source control file gives dpkg-shlibdeps package context.
    (work / 'debian').mkdir()
    (work / 'debian/control').write_text(
        'Source: codecedar\nSection: devel\nPriority: optional\n'
        'Maintainer: Kedr contributors <noreply@github.com>\n\n'
        'Package: codecedar\nArchitecture: any\nDescription: CodeCedar preview\n', encoding='utf-8')
    result = run(['dpkg-shlibdeps', '-O', '-e' + str(binary.resolve())],
                 cwd=work, capture_output=True)
    lines = [line.removeprefix('shlibs:Depends=') for line in result.stdout.splitlines()
             if line.startswith('shlibs:Depends=')]
    if len(lines) != 1 or not lines[0].strip():
        raise ValueError('dpkg-shlibdeps returned no ABI dependencies; refusing incomplete package')
    return ', '.join([lines[0].strip(), *RUNTIME['deb']])


def build_deb(binary, output, version, release, work):
    arch = identity(binary)[0]
    dependencies = deb_dependencies(binary, work)
    tree = work / 'tree'
    stage(binary, tree)
    control = tree / 'DEBIAN'
    control.mkdir()
    size = sum(p.stat().st_size for p in (tree / 'usr').rglob('*') if p.is_file())
    (control / 'control').write_text(
        f'Package: {PACKAGE}\nVersion: {version}-{release}\nArchitecture: {arch}\n'
        'Maintainer: Kedr contributors <noreply@github.com>\nSection: devel\nPriority: optional\n'
        f'Installed-Size: {(size + 1023) // 1024}\nDepends: {dependencies}\n'
        f'Recommends: {", ".join(RECOMMENDS["deb"])}\n'
        'Homepage: https://github.com/drainedgodw/kedr\n'
        'Description: CodeCedar native code editor (unsigned preview)\n'
        ' One implemented editor, not the planned Kedr suite. Rust toolchains and\n'
        ' rust-analyzer are separate optional development tools; see INSTALL.md.\n', encoding='utf-8')
    artifact = output / f'{PACKAGE}_{version}-{release}_{arch}.deb'
    run(['dpkg-deb', '--root-owner-group', '--build', tree, artifact])
    return artifact


def build_rpm(binary, output, version, release, work):
    arch = identity(binary)[1]
    tree = work / 'tree'
    stage(binary, tree)
    top = work / 'rpmbuild'
    for folder in ('BUILD', 'BUILDROOT', 'RPMS', 'SOURCES', 'SPECS', 'SRPMS'):
        (top / folder).mkdir(parents=True)
    with tarfile.open(top / 'SOURCES/payload.tar', 'w') as archive:
        archive.add(tree / 'usr', arcname='usr')
    spec = top / 'SPECS/codecedar.spec'
    spec.write_text(f'''%global debug_package %{{nil}}
Name: {PACKAGE}
Version: {version}
Release: {release}
Summary: CodeCedar native code editor (unsigned preview)
License: MIT
URL: https://github.com/drainedgodw/kedr
Source0: payload.tar
Requires: {', '.join(RUNTIME['rpm'])}
Recommends: {', '.join(RECOMMENDS['rpm'])}

%description
One implemented editor, not the planned Kedr suite. Rust toolchains and
rust-analyzer are separate development tools; see the installed INSTALL.md.

%prep
%build
%install
mkdir -p "%{{buildroot}}"
tar -xf "%{{SOURCE0}}" -C "%{{buildroot}}"

%files
/usr/bin/codecedar
/usr/share/applications/{APP_ID}.desktop
/usr/share/icons/hicolor/scalable/apps/codecedar.svg
%doc /usr/share/doc/codecedar/INSTALL.md
%license /usr/share/doc/codecedar/copyright
''', encoding='utf-8')
    # Automatic ELF dependencies remain enabled. Never use --nodeps.
    run(['rpmbuild', '-bb', '--target', arch, '--define', f'_topdir {top}', spec])
    packages = list((top / 'RPMS').rglob('*.rpm'))
    if len(packages) != 1:
        raise ValueError('Expected exactly one application RPM')
    artifact = output / packages[0].name
    shutil.copyfile(packages[0], artifact)
    return artifact


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--format', choices=['deb', 'rpm'], required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--version', default='0.1.0')
    parser.add_argument('--release', default='0.preview')
    args = parser.parse_args()
    validate_version(args.version, args.release)
    identity(args.binary)
    tools = ['dpkg-deb', 'dpkg-shlibdeps'] if args.format == 'deb' else ['rpmbuild']
    for tool in tools:
        if not shutil.which(tool):
            parser.error(f'{tool} is required; build on the target distribution with its packaging tools')
    args.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='kedr-package-') as temp:
        artifact = (build_deb if args.format == 'deb' else build_rpm)(
            args.binary.resolve(), args.out.resolve(), args.version, args.release, Path(temp))
    print(json.dumps({'artifact': str(artifact), 'format': args.format,
                      'status': 'BUILT_NOT_INSTALLED_OR_GUI_TESTED'}))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'Package build refused: {error}') from error
