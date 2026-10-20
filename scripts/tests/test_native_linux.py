"""Packaging contract tests; these are not native GUI acceptance."""
import importlib.util
import os
import struct
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('native_linux', Path(__file__).resolve().parents[1] / 'build_native_linux.py')
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)


class NativeLinuxTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.binary = self.root / 'input with spaces'
        header = bytearray(64)
        header[:7] = b'\x7fELF\x02\x01\x01'
        struct.pack_into('<HH', header, 16, 3, 62)
        self.binary.write_bytes(header)

    def test_elf_architecture_not_host_architecture(self):
        self.assertEqual(native.identity(self.binary), ('amd64', 'x86_64'))
        raw = bytearray(self.binary.read_bytes())
        struct.pack_into('<H', raw, 18, 183)
        self.binary.write_bytes(raw)
        self.assertEqual(native.identity(self.binary), ('arm64', 'aarch64'))

    def test_symlink_binary_rejected(self):
        link = self.root / 'link'
        try:
            link.symlink_to(self.binary)
        except OSError:
            self.skipTest('symlinks unavailable on this host')
        with self.assertRaises(ValueError):
            native.identity(link)

    def test_non_elf_and_truncated_binary_rejected(self):
        for raw in (b'#!/bin/sh\n', b'\x7fELF', b'x' * 64):
            self.binary.write_bytes(raw)
            with self.assertRaises(ValueError):
                native.identity(self.binary)

    def test_unknown_architecture_rejected(self):
        raw = bytearray(self.binary.read_bytes())
        struct.pack_into('<H', raw, 18, 99)
        self.binary.write_bytes(raw)
        with self.assertRaises(ValueError):
            native.identity(self.binary)

    def test_relocatable_not_executable_rejected(self):
        raw = bytearray(self.binary.read_bytes())
        struct.pack_into('<H', raw, 16, 1)
        self.binary.write_bytes(raw)
        with self.assertRaises(ValueError):
            native.identity(self.binary)

    def test_version_cannot_inject_control_or_rpm_macros(self):
        for version in ('1\nDepends: bad', '%{lua:bad}', '../1', '1.0-beta'):
            with self.assertRaises(ValueError):
                native.validate_version(version, '0.preview')
        for release in ('1\nRequires: bad', '%{lua:bad}', '../1', 'x', '1-bad'):
            with self.assertRaises(ValueError):
                native.validate_version('0.1.0', release)
        native.validate_version('0.1.0', '0.preview.abc123')

    def test_staging_has_binary_desktop_icon_license_and_no_scriptlets(self):
        tree = self.root / 'tree'
        native.stage(self.binary, tree)
        self.assertEqual((tree / 'usr/bin/codecedar').read_bytes(), self.binary.read_bytes())
        if os.name != 'nt':
            self.assertEqual((tree / 'usr/bin/codecedar').stat().st_mode & 0o777, 0o755)
        self.assertIn('Exec=codecedar', (tree / f'usr/share/applications/{native.APP_ID}.desktop').read_text())
        self.assertTrue((tree / 'usr/share/doc/codecedar/copyright').is_file())
        self.assertTrue((tree / 'usr/share/icons/hicolor/scalable/apps/codecedar.svg').is_file())
        self.assertFalse((tree / 'DEBIAN/postinst').exists())
        self.assertFalse((tree / 'home').exists())

    def test_dlopen_libraries_are_explicit_not_left_to_elf_scanner(self):
        for dependency in ('libxcursor1', 'libxkbcommon-x11-0', 'libegl1', 'libgl1', 'xdg-desktop-portal'):
            self.assertIn(dependency, native.RUNTIME['deb'])
        for dependency in ('libXcursor', 'libxkbcommon-x11', 'libglvnd-egl', 'libglvnd-glx'):
            self.assertIn(dependency, native.RUNTIME['rpm'])

    def test_deb_dependencies_include_generated_abi_floor(self):
        result = subprocess.CompletedProcess([], 0, 'shlibs:Depends=libc6 (>= 2.39), libgcc-s1 (>= 4.2)\n', '')
        with patch.object(native, 'run', return_value=result) as command:
            deps = native.deb_dependencies(self.binary, self.root)
        self.assertIn('libc6 (>= 2.39)', deps)
        self.assertIn('libxcursor1', deps)
        self.assertEqual(command.call_args.args[0][0], 'dpkg-shlibdeps')
        self.assertEqual(command.call_args.args[0][2], '-e' + str(self.binary.resolve()))

    def test_absent_generated_dependencies_fail_closed(self):
        with patch.object(native, 'run', return_value=subprocess.CompletedProcess([], 0, '', '')):
            with self.assertRaises(ValueError):
                native.deb_dependencies(self.binary, self.root)

    def test_abi_scanner_error_not_ignored(self):
        with patch.object(native, 'run', side_effect=subprocess.CalledProcessError(1, ['dpkg-shlibdeps'])):
            with self.assertRaises(subprocess.CalledProcessError):
                native.deb_dependencies(self.binary, self.root)


if __name__ == '__main__':
    unittest.main()
