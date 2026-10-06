"""Independent adversarial contracts for the versioned content measurement."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "content_differential", ROOT / "tools/text_contracts/content_differential.py")
DIFF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DIFF)


class ContentMetrics(unittest.TestCase):
    def test_substitution_is_distinct_from_missing_and_added_content(self):
        counts = DIFF.measure("abc", "axc")["counts"]
        self.assertEqual((counts["matched"], counts["substitutions"],
                          counts["deletions"], counts["insertions"]), (2, 1, 0, 0))
        self.assertEqual(DIFF.measure("abc", "ac")["counts"]["deletions"], 1)
        self.assertEqual(DIFF.measure("ac", "abc")["counts"]["insertions"], 1)

    def test_empty_successful_extraction_counts_all_reference_content_as_missing(self):
        counts = DIFF.measure("A😀é", "")["counts"]
        self.assertEqual(counts["reference_scalars"], 3)
        self.assertEqual(counts["deletions"], 3)
        self.assertEqual(counts["multiset_deficit"], 3)
        self.assertEqual(counts["candidate_scalars"], 0)

    def test_whitespace_is_separate_and_unicode_is_not_normalized(self):
        counts = DIFF.measure("a b\n", "ab")["counts"]
        self.assertEqual(counts["matched"], 2)
        self.assertEqual(counts["reference_whitespace"], 2)
        self.assertEqual(counts["candidate_whitespace"], 0)
        counts = DIFF.measure("é", "e\u0301")["counts"]
        self.assertEqual((counts["substitutions"], counts["insertions"]), (1, 1))

    def test_case_punctuation_and_replacement_characters_remain_observable(self):
        counts = DIFF.measure("A!", "a\ufffd")["counts"]
        self.assertEqual(counts["substitutions"], 2)
        self.assertEqual(counts["candidate_replacements"], 1)
        self.assertEqual(counts["reference_replacements"], 0)

    def test_reordering_does_not_become_multiset_content_loss(self):
        counts = DIFF.measure("abcXYZ", "XYZabc")["counts"]
        self.assertEqual((counts["multiset_deficit"], counts["multiset_excess"]), (0, 0))
        self.assertEqual((counts["deletions"], counts["insertions"]), (3, 3))

    def test_repeated_characters_are_not_discarded_as_alignment_junk(self):
        counts = DIFF.measure("a" * 300, "b" + "a" * 299)["counts"]
        self.assertEqual(counts["matched"], 299)
        self.assertEqual(counts["reference_scalars"], 300)

    def test_blank_reference_has_no_invented_rate_denominator(self):
        pages = DIFF.compare_pages([{"text": "extra"}], [""])
        summary = DIFF.summarize([{"status": "complete", "pages": pages}])
        self.assertEqual(summary["comparable_page_totals"]["insertions"], 5)
        self.assertIsNone(summary["rates_over_comparable_reference"]["insertions"])

    def test_page_failure_is_not_empty_success(self):
        pages = DIFF.compare_pages([{"error": "bad stream"}, {"text": ""}], ["abc", "xyz"])
        self.assertEqual([p["status"] for p in pages],
                         ["extraction_error", "compared"])
        summary = DIFF.summarize([{"status": "partial", "pages": pages},
                                  {"status": "excluded", "error": "timeout"}])
        self.assertEqual(summary["files"], 2)
        self.assertEqual(summary["page_statuses"]["compared"], 1)
        self.assertEqual(summary["comparable_page_totals"]["reference_scalars"], 3)
        self.assertEqual(summary["comparable_page_totals"]["deletions"], 3)

    def test_page_count_mismatch_cannot_shift_the_remaining_comparisons(self):
        pages = DIFF.compare_pages([{"text": "first"}, {"text": "second"}],
                                   ["inserted", "first", "second"])
        self.assertEqual([p["status"] for p in pages], ["page_count_mismatch"] * 3)
        self.assertTrue(all("counts" not in p for p in pages))

    def test_oversized_page_is_explicitly_excluded_from_comparable_population(self):
        pages = DIFF.compare_pages([{"text": "ok"}], ["a" * (DIFF.MAX_SCALARS + 1)])
        self.assertEqual(pages[0]["status"], "measurement_limit")
        self.assertNotIn("counts", pages[0])

    def test_poppler_page_delimiters_preserve_blank_pages(self):
        self.assertEqual(DIFF.split_poppler_pages("one\n\f\fthree\n\f"),
                         ["one\n", "", "three\n"])
        with self.assertRaisesRegex(ValueError, "terminal"):
            DIFF.split_poppler_pages("unfinished")

    def test_export_rejects_partial_duplicate_and_ambiguous_records(self):
        header = {"kind": "document", "pages": 2}
        page = {"kind": "page", "page": 0, "text": "abc"}
        for records in [[header, page], [header, page, page],
                        [{"kind": "document", "pages": 1}, {**page, "error": "bad"}]]:
            with self.assertRaises(ValueError):
                DIFF.validate_export("\n".join(map(json.dumps, records)))
        self.assertEqual(DIFF.validate_export('\n'.join(map(json.dumps, [
            {"kind": "document", "pages": 1}, page]))), [page])

    def test_population_hash_and_path_are_enforced(self):
        # Keep disposable files inside the project, never alter corpus inputs.
        work = ROOT / "target/content-differential-tests"
        work.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=work) as name:
            root = Path(name)
            pdf = root / "one.pdf"
            pdf.write_bytes(b"original")
            entry = DIFF.freeze(root)["files"][0]
            self.assertEqual(DIFF.checked_path(root, entry), pdf)
            pdf.write_bytes(b"mutated")
            with self.assertRaisesRegex(ValueError, "hash changed"):
                DIFF.checked_path(root, entry)
            for path in ["../escape.pdf", "/outside.pdf"]:
                with self.assertRaisesRegex(ValueError, "escapes"):
                    DIFF.checked_path(root, {"path": path, "sha256": "ignored"})


if __name__ == "__main__":
    unittest.main()
