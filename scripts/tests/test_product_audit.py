"""Negative tests prevent logos/catalog entries from masquerading as products."""
import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('audit_products', ROOT / 'scripts/audit_products.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class ProductAuditTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for folder in ('products', 'assets/brand', 'apps/studio'):
            shutil.copytree(ROOT / folder, self.root / folder, ignore=shutil.ignore_patterns('target'))
        self.catalog = json.loads((self.root / 'products/catalog.json').read_text(encoding='utf-8'))

    def run_inventory(self):
        (self.root / 'products/catalog.json').write_text(json.dumps(self.catalog), encoding='utf-8')
        return audit.inventory(self.root)

    def test_every_catalog_product_has_a_separate_honest_verdict(self):
        r = self.run_inventory()
        self.assertEqual(r['totals'], dict(products=33, runtime_available=1, not_implemented=32))
        self.assertEqual(len({p['slug'] for p in r['products']}), 33)
        self.assertTrue(all(p['runtime'] != 'PASS' for p in r['products']))

    def test_unknown_status_rejected(self):
        self.catalog['products'][1]['status'] = 'shipping'
        with self.assertRaises(ValueError): self.run_inventory()

    def test_preview_requires_actual_runtime(self):
        self.catalog['products'][1]['status'] = 'preview'
        with self.assertRaises(ValueError): self.run_inventory()

    def test_runtime_package_must_match_manifest(self):
        self.catalog['products'][0]['implementation']['package'] = 'fiction'
        with self.assertRaises(ValueError): self.run_inventory()

    def test_planned_cannot_claim_implementation(self):
        self.catalog['products'][1]['implementation'] = self.catalog['products'][0]['implementation']
        with self.assertRaises(ValueError): self.run_inventory()

    def test_missing_runtime_entry_point_rejected(self):
        (self.root / 'apps/studio/src/main.rs').unlink()
        with self.assertRaises(ValueError): self.run_inventory()

    def test_duplicate_slug_rejected(self):
        self.catalog['products'][1]['slug'] = 'codecedar'
        with self.assertRaises(ValueError): self.run_inventory()

    def test_duplicate_monogram_rejected(self):
        self.catalog['products'][1]['monogram'] = self.catalog['products'][0]['monogram']
        with self.assertRaises(ValueError): self.run_inventory()

    def test_implementation_evidence_cannot_be_omitted(self):
        del self.catalog['products'][1]['implementation']
        with self.assertRaises(ValueError): self.run_inventory()

    def test_path_traversal_rejected(self):
        self.catalog['products'][1]['guide'] = '../outside.md'
        with self.assertRaises(ValueError): self.run_inventory()

    def test_nonmatching_guide_rejected(self):
        self.catalog['products'][1]['guide'] = 'products/pycedar.md'
        with self.assertRaises(ValueError): self.run_inventory()

    def test_missing_capability_targets_rejected(self):
        self.catalog['products'][1]['targets'] = []
        with self.assertRaises(ValueError): self.run_inventory()

    def test_missing_logo_rejected(self):
        self.catalog['products'][1]['logo'] = 'assets/brand/absent.svg'
        with self.assertRaises(ValueError): self.run_inventory()

    def test_rebranding_one_binary_does_not_count_as_two_products(self):
        self.catalog['products'][1]['status'] = 'preview'
        self.catalog['products'][1]['implementation'] = self.catalog['products'][0]['implementation']
        with self.assertRaises(ValueError): self.run_inventory()
