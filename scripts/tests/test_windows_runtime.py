import importlib.util
import struct
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('windows_runtime', Path(__file__).resolve().parents[1] / 'check_windows_runtime.py')
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


def fixture(dll=b'KERNEL32.dll', delay=False):
    data = bytearray(1024)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 60, 128)
    data[128:132] = b'PE\0\0'
    struct.pack_into('<H', data, 134, 1)
    struct.pack_into('<H', data, 148, 240)
    optional = 152
    struct.pack_into('<H', data, optional, 0x20b)
    struct.pack_into('<Q', data, optional + 24, 0x140000000)
    struct.pack_into('<I', data, optional + 60, 512)
    struct.pack_into('<I', data, optional + 108, 16)
    struct.pack_into('<III', data, 392 + 12, 4096, 512, 512)
    if delay:
        struct.pack_into('<II', data, optional + 112 + 13 * 8, 4096, 64)
        struct.pack_into('<II', data, 512, 1, 4096 + 80)
    else:
        struct.pack_into('<II', data, optional + 112 + 8, 4096, 40)
        struct.pack_into('<I', data, 512 + 12, 4096 + 80)
    data[592:592 + len(dll) + 1] = dll + b'\0'
    return data


class WindowsRuntimeTests(unittest.TestCase):
    def test_normal_imports_audited(self):
        names = runtime.imported_dlls(fixture())
        self.assertEqual(names, ['kernel32.dll'])
        runtime.check(names)

    def test_delay_imports_cannot_hide_external_runtime(self):
        names = runtime.imported_dlls(fixture(b'VCRUNTIME140.dll', delay=True))
        with self.assertRaises(ValueError):
            runtime.check(names)

    def test_external_runtime_or_arbitrary_vendor_dll_rejected(self):
        for name in ('vcruntime140.dll', 'msvcp140.dll', 'vendor.dll', '../kernel32.dll', 'api-ms-win-../evil.dll'):
            with self.assertRaises(ValueError):
                runtime.check([name])

    def test_os_runtime_and_api_sets_are_allowed(self):
        runtime.check(['ucrtbase.dll', 'msvcrt.dll', 'api-ms-win-core-synch-l1-2-0.dll'])

    def test_accessibility_and_ime_dependencies_are_windows_system_libraries(self):
        # AccessKit calls Uia* from UIAutomationCore; these are not VC redistributables.
        runtime.check(['uiautomationcore.dll', 'oleacc.dll', 'msctf.dll'])

    def test_truncated_or_not_pe_fails_closed(self):
        for data in (b'not PE', fixture()[:520]):
            with self.assertRaises(ValueError):
                runtime.imported_dlls(data)

    def test_bad_rva_rejected(self):
        data = fixture()
        struct.pack_into('<I', data, 524, 0xfffffff0)
        with self.assertRaises(ValueError):
            runtime.imported_dlls(data)


if __name__ == '__main__':
    unittest.main()
