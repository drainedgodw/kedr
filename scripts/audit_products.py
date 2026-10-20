#!/usr/bin/env python3
"""Inventory ALL catalog products. Structural PASS is never runtime acceptance."""
import argparse
import json
import re
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def repository_file(root, name):
    if not isinstance(name, str) or not name or '\\' in name:
        raise ValueError(f'Invalid repository path: {name!r}')
    path = Path(name)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError(f'Unsafe repository path: {name}')
    target = (root / path).resolve()
    if not target.is_relative_to(root.resolve()) or not target.is_file():
        raise ValueError(f'Missing or escaping repository file: {name}')
    return target


def inventory(root=ROOT):
    catalog = json.loads((root / 'products/catalog.json').read_text(encoding='utf-8'))
    products = catalog.get('products')
    if not isinstance(products, list) or not products:
        raise ValueError('Product catalog must be a nonempty list')
    rows, slugs, monograms, runtimes = [], set(), set(), set()
    for p in products:
        slug, name, monogram = p['slug'], p['name'], p['monogram']
        if not isinstance(slug, str) or not re.fullmatch('[a-z][a-z0-9-]*', slug) or slug in slugs:
            raise ValueError(f'Invalid or duplicate product slug: {slug}')
        if not isinstance(name, str) or not name.strip():
            raise ValueError(f'Missing product name: {slug}')
        if not isinstance(monogram, str) or not monogram or monogram in monograms:
            raise ValueError(f'Invalid or duplicate monogram: {slug}')
        slugs.add(slug)
        monograms.add(monogram)
        status = p.get('status')
        if status not in ('planned', 'preview'):
            raise ValueError(f'Unknown product status: {slug}: {status}')
        guide = repository_file(root, p['guide']).read_text(encoding='utf-8')
        svg = ET.parse(repository_file(root, p['logo'])).getroot()
        title = svg.find('{http://www.w3.org/2000/svg}title')
        if svg.get('viewBox') != '0 0 128 128' or title is None or not (title.text or '').strip():
            raise ValueError(f'Missing SVG accessibility title or viewBox: {slug}')
        if name not in guide:
            raise ValueError(f'Guide does not identify product: {slug}')
        if not isinstance(p.get('targets'), list) or not p['targets'] or any(not isinstance(t, str) or not t.strip() for t in p['targets']):
            raise ValueError(f'Missing capability targets: {slug}')
        if 'implementation' not in p:
            raise ValueError(f'Implementation evidence must be explicit: {slug}')
        implementation = p['implementation']
        if status == 'planned':
            if implementation is not None:
                raise ValueError(f'Planned product cannot claim runtime evidence: {slug}')
            if 'There is no' not in guide or 'no installation command yet' not in guide:
                raise ValueError(f'Planned guide must disclose missing runtime: {slug}')
            runtime = 'NOT_IMPLEMENTED'
        else:
            if not isinstance(implementation, dict) or not implementation.get('package'):
                raise ValueError(f'Preview needs a real package and manifest: {slug}')
            manifest = repository_file(root, implementation['manifest'])
            data = tomllib.loads(manifest.read_text(encoding='utf-8'))
            if data.get('package', {}).get('name') != implementation['package']:
                raise ValueError(f'Runtime package mismatch: {slug}')
            if not (manifest.parent / 'src/main.rs').is_file():
                raise ValueError(f'No runtime entry point: {slug}')
            identity = (manifest.resolve(), implementation['package'])
            if identity in runtimes:
                raise ValueError(f'Multiple product names claim the same runtime: {slug}')
            runtimes.add(identity)
            runtime = 'AVAILABLE_NOT_EXECUTED_BY_INVENTORY'
        rows.append(dict(slug=slug, name=name, kind=p['kind'], status=status,
                         structure='PASS', runtime=runtime, implementation=implementation,
                         unimplemented_targets=p['targets'] if status == 'planned' else [],
                         reason='No application/backend, runtime tests or installer' if status == 'planned'
                         else 'Use workspace, live-adapter, native and installer acceptance separately'))
    return dict(schema=1, scope='Structural inventory only; NOT an execution or feature-parity test',
                products=rows, totals=dict(products=len(rows),
                runtime_available=sum(r['status'] == 'preview' for r in rows),
                not_implemented=sum(r['status'] == 'planned' for r in rows)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    report = inventory()
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    t = report['totals']
    print(f"{t['products']} catalog products: {t['runtime_available']} runtime available, "
          f"{t['not_implemented']} NOT IMPLEMENTED. Structural validation only.")


if __name__ == '__main__':
    main()
