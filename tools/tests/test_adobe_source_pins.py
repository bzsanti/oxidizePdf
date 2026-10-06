"""Reject altered upstream sources before publishing tables or provenance."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

TOOLS = Path(__file__).resolve().parents[1]
ROOT = TOOLS.parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, TOOLS / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class AdobeSourcePins(unittest.TestCase):
    def setUp(self):
        scratch = ROOT / 'target' / 'adobe-pin-tests'
        scratch.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'pdf2unicode').mkdir()

    def test_product_rejects_modified_source_before_touching_output(self):
        module = load('generate_adobe_pdf_cid_tables')
        (self.root / 'pdf2unicode' / 'Adobe-CNS1-UCS2').write_text(
            '1 beginbfchar\n<0001> <0041>\nendbfchar\n')
        output = self.root / 'output.rs'
        output.write_text('previous artifact')
        with self.assertRaisesRegex(ValueError, 'source hash mismatch'):
            module.generate(self.root, output)
        self.assertEqual(output.read_text(), 'previous artifact')
        self.assertFalse(output.with_suffix('.provenance.json').exists())

    def test_oracle_rejects_modified_source_before_creating_output(self):
        module = load('generate_text_cjk_oracles')
        (self.root / 'pdf2unicode' / 'Adobe-GB1-UCS2').write_text(
            '1 beginbfchar\n<0001> <0041>\nendbfchar\n')
        output = self.root / 'oracles'
        with self.assertRaisesRegex(ValueError, 'source hash mismatch'):
            module.generate(self.root, self.root, output)
        self.assertFalse(output.exists())

    def test_pin_inventory_matches_frozen_independent_provenance(self):
        pins = json.loads((TOOLS / 'adobe_cjk_source_pins.json').read_text())
        frozen = json.loads((ROOT / 'oxidize-pdf-core/tests/fixtures/text_contracts/cjk/provenance.json').read_text())
        self.assertEqual(pins['unicode_revision'], frozen['unicode_revision'])
        self.assertEqual(pins['cmap_revision'], frozen['cmap_revision'])
        extended = json.loads((ROOT / 'oxidize-pdf-core/tests/fixtures/text_contracts/kr_extended/provenance.json').read_text())
        self.assertEqual(pins['unicode_revision'], extended['unicode_revision'])
        self.assertEqual(pins['cmap_revision'], extended['cmap_revision'])
        expected = {key: value['sha256'] for key, value in frozen['sources'].items()}
        expected.update({f'cmap-resources/Adobe-KR-9/CMap/{name}': digest
                         for name, digest in extended['sources_sha256'].items()})
        self.assertEqual(pins['sources'], expected)
