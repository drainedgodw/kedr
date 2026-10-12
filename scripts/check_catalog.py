#!/usr/bin/env python3
"""Validate the public catalog and shortcut table against actual source data."""
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
root = Path(__file__).resolve().parents[1]
catalog = json.loads((root / 'products/catalog.json').read_text(encoding="utf-8"))['products']
assert len({p['slug'] for p in catalog}) == len(catalog)
assert len({p['monogram'] for p in catalog}) == len(catalog)
assert [p['slug'] for p in catalog if p['status'] == 'preview'] == ['codecedar']
for product in catalog:
    guide = root / product['guide']
    assert guide.is_file(), guide
    svg = ET.parse(root / product['logo']).getroot()
    assert svg.get('viewBox') == '0 0 128 128'
    assert svg.find('{http://www.w3.org/2000/svg}title') is not None
    if product['status'] == 'planned':
        assert 'There is no' in guide.read_text(encoding="utf-8"), guide
        assert 'no installation command yet' in guide.read_text(encoding="utf-8"), guide
for path in [root / 'README.md', *root.joinpath('products').glob('*.md'), *root.joinpath('docs/suite').glob('*.md')]:
    for link in re.findall(r'\]\(([^)]+)\)', path.read_text(encoding="utf-8")):
        if ':' in link or link.startswith('#'):
            continue
        assert (path.parent / link.split('#')[0]).exists(), (path, link)
actual = subprocess.check_output(['cargo', 'run', '--locked', '-q', '-p', 'kedr-ui', '--example', 'keymap'], cwd=root).decode()
assert actual.replace('\r\n', '\n') == root.joinpath('docs/suite/KEYMAP.md').read_text(encoding="utf-8").replace('\r\n', '\n'), 'Regenerate KEYMAP.md from the registry'
print(f'{len(catalog)} product entries, logos, guides and generated keymap validated')
