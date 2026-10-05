"""Adversarial checks of the independent source parser, without PDF product code."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('cjk_oracles', Path(__file__).parents[1] / 'generate_text_cjk_oracles.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class SourceParserTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def source(self, name, text):
        path = self.root / name
        path.write_text(text)
        return path

    def test_cid_ranges_keep_byte_length_and_increment_destination(self):
        path = self.source('sample', '2 begincidrange\n<00FE> <0100> 17\n<00000041> <00000041> 29\nendcidrange')
        self.assertEqual(module.entries(path, 'cid'), {b'\x00\xfe': 17, b'\x00\xff': 18, b'\x01\x00': 19, b'\x00\x00\x00A': 29})

    def test_vertical_child_overrides_parent_and_keeps_inherited_codes(self):
        self.source('H', '1 begincidrange\n<41> <43> 10\nendcidrange')
        child = self.source('V', '/H usecmap\n1 begincidchar\n<42> 90\nendcidchar')
        self.assertEqual(module.entries(child, 'cid'), {b'A': 10, b'B': 90, b'C': 12})

    def test_utf16_sequences_surrogates_and_ranges_are_not_normalized(self):
        path = self.source('unicode', '2 beginbfchar\n<0001> <00410301>\n<0002> <D840DC00DB40DD00>\nendbfchar\n1 beginbfrange\n<0003> <0004> <D83DDE00>\nendbfrange')
        self.assertEqual(module.entries(path, 'bf'), {b'\x00\x01': 'A\u0301', b'\x00\x02': '\U00020000\U000e0100', b'\x00\x03': '😀', b'\x00\x04': '😁'})

    def test_cycles_are_rejected(self):
        path = self.source('A', '/B usecmap\n')
        self.source('B', '/A usecmap\n')
        with self.assertRaisesRegex(ValueError, 'cyclic'):
            module.entries(path, 'cid')

    def test_missing_rows_and_unsupported_syntax_fail_closed(self):
        for text in ['2 begincidchar\n<41> 17\nendcidchar', '1 beginbfrange\n<01> <02> [<0041> <0042>]\nendbfrange']:
            path = self.source('broken', text)
            with self.assertRaises(ValueError):
                module.entries(path, 'cid' if 'cid' in text else 'bf')

if __name__ == '__main__':
    unittest.main()
