#!/usr/bin/env python3
"""Build an ad-hoc signed macOS .app/.dmg; not notarized or Developer-ID signed."""
import argparse
import base64
import json
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def dependencies(output):
    found = []
    for line in output.splitlines():
        if not line.startswith((' ', '\t')):
            continue  # otool's filename/architecture header
        name = line.strip().split(' (compatibility version', 1)[0]
        if not name.startswith(('/usr/lib/', '/System/Library/')):
            raise ValueError(f'Unbundled non-system dylib: {name}')
        found.append(name)
    if not found:
        raise ValueError('No Mach-O dependencies found; refusing unaudited bundle')
    return found


def minimum_os(output):
    versions = []
    for block in re.split(r'(?m)^Load command \d+\s*$', output):
        if re.search(r'(?m)^\s*cmd LC_BUILD_VERSION\s*$', block):
            versions += re.findall(r'(?m)^\s*minos (\d+\.\d+(?:\.\d+)?)\s*$', block)
        elif re.search(r'(?m)^\s*cmd LC_VERSION_MIN_MACOSX\s*$', block):
            versions += re.findall(r'(?m)^\s*version (\d+\.\d+(?:\.\d+)?)\s*$', block)
    if not versions:
        raise ValueError('Cannot determine minimum macOS version from Mach-O load commands')
    return max(versions, key=lambda v: tuple(int(x) for x in v.split('.')))


def stage(binary, destination, icon, version, min_os):
    if not re.fullmatch(r'\d+\.\d+(?:\.\d+)?', version):
        raise ValueError('Invalid app version')
    if destination.exists():
        raise ValueError('App destination already exists; refusing to overwrite')
    contents = destination / 'Contents'
    (contents / 'MacOS').mkdir(parents=True)
    (contents / 'Resources').mkdir()
    shutil.copyfile(binary, contents / 'MacOS/codecedar')
    (contents / 'MacOS/codecedar').chmod(0o755)
    shutil.copyfile(icon, contents / 'Resources/codecedar.icns')
    shutil.copyfile(ROOT / 'LICENSE', contents / 'Resources/LICENSE')
    shutil.copyfile(ROOT / 'docs/suite/INSTALL.md', contents / 'Resources/INSTALL.md')
    info = {
        'CFBundleDevelopmentRegion': 'en',
        'CFBundleExecutable': 'codecedar',
        'CFBundleIdentifier': 'io.github.drainedgodw.CodeCedar',
        'CFBundleName': 'CodeCedar',
        'CFBundleDisplayName': 'CodeCedar',
        'CFBundlePackageType': 'APPL',
        'CFBundleShortVersionString': version,
        'CFBundleVersion': version,
        'CFBundleIconFile': 'codecedar.icns',
        'LSMinimumSystemVersion': min_os,
        'NSHighResolutionCapable': True,
        'NSHumanReadableCopyright': 'Kedr contributors. MIT License.',
    }
    with (contents / 'Info.plist').open('wb') as stream:
        plistlib.dump(info, stream)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--version', default='0.1.0')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('Native macOS tools are required; cross-staging is not macOS acceptance')
    if args.binary.is_symlink() or not args.binary.is_file():
        parser.error('A regular native binary is required')
    def run(cmd, **kwargs):
        return subprocess.run([str(x) for x in cmd], check=True, text=True, **kwargs)
    linked = dependencies(run(['otool', '-L', args.binary], capture_output=True).stdout)
    min_os = minimum_os(run(['otool', '-l', args.binary], capture_output=True).stdout)
    args.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='kedr-app-') as temp:
        work = Path(temp)
        icons = json.loads((ROOT / 'packaging/assets/codecedar.json').read_text())
        source = work / 'source.png'
        source.write_bytes(base64.b64decode(icons['codecedar-512.png'], validate=True))
        iconset = work / 'codecedar.iconset'
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                name = f'icon_{size}x{size}' + ('@2x' if scale == 2 else '') + '.png'
                run(['sips', '-z', size * scale, size * scale, source, '--out', iconset / name], stdout=subprocess.DEVNULL)
        icon = work / 'codecedar.icns'
        run(['iconutil', '-c', 'icns', iconset, '-o', icon])
        image = work / 'image'
        image.mkdir()
        app = image / 'CodeCedar.app'
        stage(args.binary, app, icon, args.version, min_os)
        run(['codesign', '--force', '--sign', '-', app])
        run(['codesign', '--verify', '--deep', '--strict', app])
        run([app / 'Contents/MacOS/codecedar', '--help'])
        (image / 'Applications').symlink_to('/Applications')
        artifact = args.out.resolve() / 'CodeCedar-macOS.dmg'
        if artifact.exists():
            raise ValueError('DMG destination exists; refusing overwrite')
        run(['hdiutil', 'create', '-volname', 'CodeCedar Preview', '-srcfolder', image,
             '-format', 'UDZO', artifact])
        run(['hdiutil', 'verify', artifact])
        report = {'artifact': artifact.name, 'minimum_macos': min_os,
                  'system_dylibs': linked, 'signing': 'ad-hoc, not Developer ID/notarized',
                  'acceptance': 'bundle signature, executable help, disk-image integrity; not GUI acceptance'}
        (args.out / 'macos-package-audit.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
