import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("omnidocbench_native_reading_order.py")
SPEC = importlib.util.spec_from_file_location("native_reading_order", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def page(
    name: str,
    source: str,
    layout: str = "single_column",
    language: str = "english",
    special_issue=None,
):
    return {
        "page_info": {
            "image_path": f"images/{name}",
            "page_attribute": {
                "data_source": source,
                "layout": layout,
                "language": language,
                "special_issue": special_issue or ["None"],
            },
        }
    }


class NativeReadingOrderAggregationTests(unittest.TestCase):
    def test_excludes_notes_and_fuzzy_scans_and_aggregates_categories(self):
        dataset = [
            page("book.jpg", "book", "double_column"),
            page("paper.jpg", "academic_literature"),
            page("scan.jpg", "note"),
            page("fuzzy.jpg", "book", special_issue=["fuzzy_scan"]),
        ]
        result = MODULE.aggregate_native_scores(
            dataset,
            {"book.jpg": 0.4, "paper.jpg": 0.2, "scan.jpg": 1.0, "fuzzy.jpg": 0.8},
        )

        self.assertEqual(result["official_scored_pages"], 4)
        self.assertEqual(result["excluded_pages"], 2)
        self.assertEqual(result["native_pages"], 2)
        self.assertAlmostEqual(result["native_edit_distance"], 0.3)
        self.assertEqual(
            result["categories"]["layout"]["double_column"],
            {"pages": 1, "edit_distance": 0.4},
        )

    def test_rejects_unknown_score_pages(self):
        with self.assertRaisesRegex(ValueError, "unknown pages"):
            MODULE.aggregate_native_scores(
                [page("known.jpg", "book")], {"missing.jpg": 0.1}
            )

    def test_rejects_non_numeric_scores(self):
        with self.assertRaisesRegex(ValueError, "invalid edit-distance score"):
            MODULE.aggregate_native_scores(
                [page("page.jpg", "book")], {"page.jpg": "0.1"}
            )

    def test_rejects_non_finite_scores(self):
        with self.assertRaisesRegex(ValueError, "invalid edit-distance score"):
            MODULE.aggregate_native_scores(
                [page("page.jpg", "book")], {"page.jpg": float("nan")}
            )

    def test_rejects_out_of_range_scores(self):
        with self.assertRaisesRegex(ValueError, "invalid edit-distance score"):
            MODULE.aggregate_native_scores(
                [page("page.jpg", "book")], {"page.jpg": 1.01}
            )

    def test_rejects_duplicate_dataset_basenames(self):
        with self.assertRaisesRegex(ValueError, "duplicate dataset page basename"):
            MODULE.aggregate_native_scores(
                [page("same.jpg", "book"), page("same.jpg", "academic_literature")],
                {"same.jpg": 0.1},
            )

    def test_rejects_an_empty_native_population(self):
        with self.assertRaisesRegex(ValueError, "no native-text pages"):
            MODULE.aggregate_native_scores(
                [page("scan.jpg", "note")], {"scan.jpg": 1.0}
            )


if __name__ == "__main__":
    unittest.main()
