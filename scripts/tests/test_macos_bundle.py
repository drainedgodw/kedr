"""Cross-platform bundle contracts, not macOS execution evidence."""
import importlib.util
import plistlib
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('macos_app', Path(__file__).resolve().parents[1] / 'build_macos_app.py')
app = importlib.util.module_from_spec(spec)
spec.loader.exec_module(app)


class MacOSBundleTests(unittest.TestCase):
    def test_only_system_dependencies_accepted(self):
        text = 'binary:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)\n'
        self.assertEqual(app.dependencies(text), ['/usr/lib/libSystem.B.dylib'])

    def test_unbundled_homebrew_relative_or_rpath_library_rejected(self):
        for name in ('/opt/homebrew/lib/libfoo.dylib', '@rpath/foo.dylib', '@loader_path/foo.dylib', '/tmp/foo.dylib'):
            with self.assertRaises(ValueError):
                app.dependencies(f'binary:\n\t{name} (compatibility version 1.0.0)\n')

    def test_empty_dependency_audit_fails_closed(self):
        with self.assertRaises(ValueError):
            app.dependencies('not a Mach-O file')

    def test_minimum_os_comes_from_binary_not_a_marketing_claim(self):
        self.assertEqual(app.minimum_os('Load command 1\n cmd LC_BUILD_VERSION\n minos 11.0\n tool LD\n version 1200.0\nLoad command 2\n cmd LC_BUILD_VERSION\n minos 12.3\n'), '12.3')
        self.assertEqual(app.minimum_os('Load command 1\n cmd LC_VERSION_MIN_MACOSX\n version 10.15\n'), '10.15')
        with self.assertRaises(ValueError):
            app.minimum_os('no load commands')

    def test_stage_is_finder_bundle_with_executable_icon_and_license(self):
        with tempfile.TemporaryDirectory() as temp:
            p = Path(temp)
            binary, icon = p / 'input', p / 'icon'
            binary.write_bytes(b'fixture, not a native executable')
            icon.write_bytes(b'icon fixture')
            destination = p / 'CodeCedar.app'
            app.stage(binary, destination, icon, '0.1.0', '11.0')
            with (destination / 'Contents/Info.plist').open('rb') as stream:
                info = plistlib.load(stream)
            self.assertEqual(info['CFBundleExecutable'], 'codecedar')
            self.assertEqual(info['CFBundlePackageType'], 'APPL')
            self.assertEqual(info['LSMinimumSystemVersion'], '11.0')
            self.assertTrue((destination / 'Contents/Resources/LICENSE').is_file())
            self.assertEqual((destination / 'Contents/MacOS/codecedar').read_bytes(), binary.read_bytes())
            with self.assertRaises(ValueError):
                app.stage(binary, destination, icon, '0.1.0', '11.0')


if __name__ == '__main__':
    unittest.main()
