#!/usr/bin/env python3
"""Fail packaging if a PE binary needs an unshipped non-Windows DLL.

Reads normal AND delay imports without executing the PE or requiring dumpbin.
This checks runtime dependency closure, not Authenticode or GUI compatibility.
"""
import argparse
import json
import re
import struct
from pathlib import Path

SYSTEM = set('''advapi32 avrt bcrypt bcryptprimitives cabinet cfgmgr32 comctl32
comdlg32 crypt32 cryptbase d3d11 d3d12 d3dcompiler_47 dcomp dbgcore dbghelp dnsapi
dwmapi dwrite dxgi gdi32 imm32 iphlpapi kernel32 ktmw32 msimg32 msvcrt mswsock
ncrypt netapi32 normaliz ntdll ole32 oleaut32 opengl32 powrprof propsys psapi
rpcrt4 secur32 setupapi shcore shell32 shlwapi ucrtbase user32 userenv uxtheme
version windowscodecs winhttp wininet winmm winspool wintrust winusb ws2_32
wtsapi32 rstrtmgr'''.split())


def imported_dlls(data):
    def block(offset, size):
        if offset < 0 or offset + size > len(data):
            raise ValueError('Truncated PE structure')
        return data[offset:offset + size]
    def number(offset, fmt):
        return struct.unpack(fmt, block(offset, struct.calcsize(fmt)))[0]
    if block(0, 2) != b'MZ':
        raise ValueError('Missing DOS executable header')
    pe = number(60, '<I')
    if block(pe, 4) != b'PE\0\0':
        raise ValueError('Missing PE signature')
    count = number(pe + 6, '<H')
    size = number(pe + 20, '<H')
    optional = pe + 24
    magic = number(optional, '<H')
    if magic == 0x20B:
        directory, count_offset = 112, 108
        image_base = number(optional + 24, '<Q')
    elif magic == 0x10B:
        directory, count_offset = 96, 92
        image_base = number(optional + 28, '<I')
    else:
        raise ValueError('Unsupported PE optional header')
    if not 1 <= count <= 96 or size < directory + 14 * 8:
        raise ValueError('Invalid PE section/data-directory counts')
    directories = number(optional + count_offset, '<I')
    if directories < 14:
        raise ValueError('PE lacks required import/delay-import audit directories')
    section_table = optional + size
    sections = []
    for index in range(count):
        start = section_table + index * 40
        va = number(start + 12, '<I')
        raw_size = number(start + 16, '<I')
        raw = number(start + 20, '<I')
        block(raw, raw_size)
        sections.append((va, raw, raw_size))
    headers = number(optional + 60, '<I')
    def offset(rva, length):
        if 0 <= rva and rva + length <= headers:
            block(rva, length)
            return rva
        for va, raw, extent in sections:
            if va <= rva and rva + length <= va + extent:
                return raw + rva - va
        raise ValueError('Import RVA outside file-backed sections')
    def name(rva):
        result = bytearray()
        for index in range(260):
            char = data[offset(rva + index, 1)]
            if char == 0:
                if not result:
                    raise ValueError('Empty DLL name')
                return result.decode('ascii').lower()
            result.append(char)
        raise ValueError('Unterminated DLL name')
    names = set()
    for index, width in ((1, 20), (13, 32)):
        rva = number(optional + directory + index * 8, '<I')
        extent = number(optional + directory + index * 8 + 4, '<I')
        if rva == 0 and extent == 0:
            continue
        if rva == 0 or extent < width or extent // width > 2048:
            raise ValueError('Invalid import directory bounds')
        terminated = False
        for cursor in range(0, extent - width + 1, width):
            record = block(offset(rva + cursor, width), width)
            if not any(record):
                terminated = True
                break
            fields = struct.unpack('<' + 'I' * (width // 4), record)
            if index == 1:
                dll_rva = fields[3]
            else:
                if fields[0] not in (0, 1):
                    raise ValueError('Unsupported delay import attributes')
                dll_rva = fields[1] if fields[0] == 1 else fields[1] - image_base
            names.add(name(dll_rva))
        if not terminated:
            raise ValueError('Unterminated import descriptor directory')
    if not names:
        raise ValueError('No DLL imports found; refusing unaudited executable')
    return sorted(names)


def check(names):
    unshipped = [n for n in names if not (
        re.fullmatch(r'[a-z0-9_.-]+\.dll', n) and (n[:-4] in SYSTEM or n.startswith(('api-ms-win-', 'ext-ms-win-'))))]
    if unshipped:
        raise ValueError('Unshipped non-system DLLs (bundle or statically link): ' + ', '.join(unshipped))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    names = imported_dlls(args.binary.read_bytes())
    check(names)
    print(json.dumps({'dlls': names, 'runtime_check': 'WINDOWS_SYSTEM_DLLS_ONLY',
                      'gui_acceptance': 'NOT_PERFORMED_BY_IMPORT_CHECK'}))


if __name__ == '__main__':
    main()
