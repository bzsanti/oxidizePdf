import importlib.util
import hashlib
import json
import argparse
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).with_name("omnidocbench_gate.py")
SPEC = importlib.util.spec_from_file_location("omnidocbench_gate", MODULE_PATH)
assert SPEC and SPEC.loader
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


def page(name, source="book", special=None, page_no=1):
    return {
        "layout_dets": [{"category_type": "text_block", "text": "text"}],
        "page_info": {
            "image_path": f"images/{name}",
            "page_no": page_no,
            "page_attribute": {
                "data_source": source,
                "special_issue": special or ["None"],
                "layout": "single_column",
                "language": "english",
            },
        }
    }


def sealed_summary(identity=None, dataset_count=1, similarity=0.5):
    value = {
        "identity": identity or GATE.identity_fixture(),
        "population": {"scored_pages_sha256": "pages"},
        "provenance": {"worktree_clean": True},
        "artifacts": {
            "dataset_sha256": "dataset",
            "scores_sha256": "scores",
            "predictions_sha256": "predictions",
        },
        "counts": {
            "dataset": dataset_count,
            "official_text_scorable": dataset_count,
            "included_native": dataset_count,
            "excluded_native": 0,
        },
        "metrics": {
            "official_global_text_similarity": similarity,
            "native_text_similarity": similarity,
        },
    }
    value["summary_sha256"] = GATE.canonical_hash(value)
    return value


class JsonAndPopulationTests(unittest.TestCase):
    def test_rejects_duplicate_json_keys(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "scores.json"
            path.write_text('{"a.jpg": 0.1, "a.jpg": 0.2}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
                GATE.load_json(path)

    def test_rejects_missing_unknown_and_duplicate_dataset_pages(self):
        with self.assertRaisesRegex(ValueError, "missing scores"):
            GATE.summarize_scores([page("a.jpg"), page("b.jpg")], {"a.jpg": 0.1})
        with self.assertRaisesRegex(ValueError, "unknown pages"):
            GATE.summarize_scores([page("a.jpg")], {"a.jpg": 0.1, "x.jpg": 0.2})
        with self.assertRaisesRegex(ValueError, "duplicate dataset page"):
            GATE.summarize_scores([page("a.jpg"), page("a.jpg")], {"a.jpg": 0.1})

    def test_rejects_invalid_scores(self):
        for value in (float("nan"), float("inf"), -0.1, 1.1, True, "0.1"):
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, "invalid score"):
                GATE.summarize_scores([page("a.jpg")], {"a.jpg": value})

    def test_rejects_malformed_dataset_and_score_shapes(self):
        with self.assertRaisesRegex(ValueError, "dataset JSON must be an array"):
            GATE.summarize_scores({}, {})
        with self.assertRaisesRegex(ValueError, "scores JSON must be an object"):
            GATE.summarize_scores([page("a.jpg")], [])
        malformed = page("a.jpg")
        malformed["page_info"]["page_attribute"]["special_issue"] = "fuzzy_scan"
        with self.assertRaisesRegex(ValueError, "array of strings"):
            GATE.summarize_scores([malformed], {"a.jpg": 0.1})

    def test_code_text_caption_is_officially_scorable(self):
        entry = page("caption.jpg")
        entry["layout_dets"] = [{"category_type": "code_txt_caption", "text": "caption"}]
        result = GATE.summarize_scores([entry], {"caption.jpg": 0.25})
        self.assertEqual(result["counts"]["official_text_scorable"], 1)

    def test_accepts_evaluator_score_for_known_page_furniture_only_page(self):
        entry = page("header.jpg")
        entry["layout_dets"] = [{"category_type": "header", "text": "running title"}]
        result = GATE.summarize_scores([entry], {"header.jpg": 0.25})
        self.assertEqual(result["counts"]["official_text_scorable"], 0)
        self.assertEqual(result["counts"]["scored"], 1)

    def test_reports_distinct_global_and_native_populations(self):
        dataset = [
            page("native.jpg"),
            page("note.jpg", source="note"),
            page("fuzzy.jpg", special=["fuzzy_scan"]),
        ]
        result = GATE.summarize_scores(
            dataset, {"native.jpg": 0.2, "note.jpg": 1.0, "fuzzy.jpg": 0.8}, failed_pages=1
        )
        self.assertEqual(result["counts"], {
            "dataset": 3, "official_text_scorable": 3,
            "official_text_unscored": 0, "scored": 3, "included_native": 1,
            "excluded_native": 2, "missing": 0, "duplicate": 0, "failed": 1,
        })
        self.assertAlmostEqual(result["metrics"]["official_global_text_similarity"], 1 - 2 / 3)
        self.assertAlmostEqual(result["metrics"]["native_text_similarity"], 0.8)


class IdentityAndHashTests(unittest.TestCase):
    def test_canonical_hash_ignores_mapping_order(self):
        self.assertEqual(GATE.canonical_hash({"b": 2, "a": 1}), GATE.canonical_hash({"a": 1, "b": 2}))

    def test_compare_rejects_incompatible_identity(self):
        baseline = sealed_summary(GATE.identity_fixture(dataset_revision="old"))
        candidate = sealed_summary(GATE.identity_fixture(dataset_revision="new"))
        with self.assertRaisesRegex(ValueError, "incompatible.*dataset_revision"):
            GATE.compare_summaries(baseline, candidate)

    def test_summary_hash_is_repeatable(self):
        dataset = [page("b.jpg"), page("a.jpg")]
        first = GATE.summarize_scores(dataset, {"a.jpg": 0.1, "b.jpg": 0.2})
        second = GATE.summarize_scores(list(reversed(dataset)), {"b.jpg": 0.2, "a.jpg": 0.1})
        self.assertEqual(GATE.canonical_hash(first), GATE.canonical_hash(second))

    def test_dirty_checkout_requires_explicit_provenance(self):
        with mock.patch.object(GATE, "git_output", side_effect=["abc123", " M file"]):
            with self.assertRaisesRegex(ValueError, "dirty worktree"):
                GATE.git_provenance(False)

    def test_dirty_provenance_has_diff_hash(self):
        with mock.patch.object(GATE, "git_output", side_effect=["abc123", " M file", "diff", "file\0"]):
            result = GATE.git_provenance(True)
        self.assertEqual(result["git_sha"], "abc123")
        self.assertFalse(result["worktree_clean"])
        self.assertRegex(result["dirty_state_sha256"], r"^[0-9a-f]{64}$")

    def test_verified_revision_uses_real_temporary_repository(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.com"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.name", "Test"], check=True)
            (root / "tracked").write_text("content", encoding="utf-8")
            subprocess.run(["git", "-C", str(root), "add", "tracked"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "--quiet", "-m", "fixture"], check=True)
            revision = GATE.git_output("-C", str(root), "rev-parse", "HEAD")
            self.assertEqual(GATE.verified_revision(root, revision, "fixture")["git_sha"], revision)
            (root / "tracked").write_text("changed", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "dirty worktree"):
                GATE.verified_revision(root, revision, "fixture")

    def test_verified_revision_accepts_only_matching_materialized_lfs_objects(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.com"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.name", "Test"], check=True)
            content = b"materialized lfs payload"
            pointer = (
                "version https://git-lfs.github.com/spec/v1\n"
                f"oid sha256:{hashlib.sha256(content).hexdigest()}\n"
                f"size {len(content)}\n"
            )
            (root / "asset.bin").write_text(pointer, encoding="ascii")
            subprocess.run(["git", "-C", str(root), "add", "asset.bin"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "--quiet", "-m", "fixture"], check=True)
            revision = GATE.git_output("-C", str(root), "rev-parse", "HEAD")

            (root / "asset.bin").write_bytes(content)
            provenance = GATE.verified_revision(root, revision, "fixture")
            self.assertTrue(provenance["worktree_clean"])
            self.assertEqual(provenance["verified_materialized_lfs_files"], 1)

            (root / "asset.bin").write_bytes(content + b"tampered")
            with self.assertRaisesRegex(ValueError, "dirty worktree"):
                GATE.verified_revision(root, revision, "fixture")

    def test_population_count_mismatch_is_not_comparable(self):
        baseline = sealed_summary(dataset_count=2)
        candidate = sealed_summary(dataset_count=3)
        with self.assertRaisesRegex(ValueError, "population counts"):
            GATE.compare_summaries(baseline, candidate)

    def test_scored_page_population_mismatch_is_not_comparable(self):
        baseline = sealed_summary()
        candidate = sealed_summary()
        candidate["population"]["scored_pages_sha256"] = "different-pages"
        candidate["summary_sha256"] = GATE.canonical_hash(
            {key: value for key, value in candidate.items() if key != "summary_sha256"}
        )
        with self.assertRaisesRegex(ValueError, "scored-page population"):
            GATE.compare_summaries(baseline, candidate)

    def test_compare_rejects_tampered_or_dirty_summaries(self):
        baseline = sealed_summary()
        candidate = sealed_summary(similarity=0.6)
        candidate["metrics"]["native_text_similarity"] = 0.9
        with self.assertRaisesRegex(ValueError, "summary hash"):
            GATE.compare_summaries(baseline, candidate)
        candidate = sealed_summary(similarity=0.6)
        candidate["provenance"]["worktree_clean"] = False
        candidate["summary_sha256"] = GATE.canonical_hash(
            {key: value for key, value in candidate.items() if key != "summary_sha256"}
        )
        with self.assertRaisesRegex(ValueError, "clean worktree"):
            GATE.compare_summaries(baseline, candidate)


class PdfResolutionTests(unittest.TestCase):
    def test_prefers_page_specific_pdf_when_dataset_stores_split_pages(self):
        entry = page("source.pdf_7.jpg", page_no=7)
        split_pdf = Path("/dataset/ori_pdfs/source.pdf_7.pdf")
        combined_pdf = Path("/dataset/ori_pdfs/source.pdf")
        pdfs = {
            split_pdf.name.casefold(): split_pdf,
            combined_pdf.name.casefold(): combined_pdf,
        }

        self.assertEqual(GATE.resolve_source_pdf(entry, pdfs), (split_pdf, 0))

    def test_resolves_pdf_suffix_and_one_based_page(self):
        self.assertEqual(
            GATE.source_pdf_identity(page("source.pdf_7.jpg", page_no=7)),
            ("source.pdf", 6),
        )

    def test_resolves_source_without_pdf_in_image_stem(self):
        self.assertEqual(
            GATE.source_pdf_identity(page("slides_455.jpg", page_no=455)),
            ("slides.pdf", 454),
        )

    def test_exporter_writes_prediction_and_manifest(self):
        repository = MODULE_PATH.parents[2]
        fixture = repository / "oxidize-pdf-core/tests/fixtures/issue_498_actual_text_diagnostic.pdf"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pdf_root = root / "pdfs"
            pdf_root.mkdir()
            shutil.copy2(fixture, pdf_root / "source.pdf")
            dataset = root / "dataset.json"
            dataset.write_text(json.dumps([page("source.pdf_1.jpg")]), encoding="utf-8")
            predictions = root / "predictions"
            manifest = root / "manifest.json"
            args = argparse.Namespace(
                dataset=dataset,
                dataset_root=root,
                evaluator_root=repository,
                pdf_root=pdf_root,
                source_root=repository,
                predictions=predictions,
                manifest=manifest,
                dataset_revision="dataset-rev",
                evaluator_revision="evaluator-rev",
                allow_dirty=True,
            )
            with mock.patch.object(GATE, "verified_revision", return_value={"git_sha": "verified", "worktree_clean": True}), mock.patch.object(
                GATE, "git_provenance", return_value={"git_sha": "candidate", "worktree_clean": True}
            ):
                GATE.export_predictions(args)
            recorded = GATE.load_json(manifest)
            self.assertTrue((predictions / "source.pdf_1.md").is_file())
            self.assertEqual(recorded["counts"], {"attempted": 1, "failed": 0, "written": 1})
            self.assertEqual(recorded["predictions_sha256"], GATE.tree_hash(predictions))

            scores = root / "scores.json"
            scores.write_text('{"source.pdf_1.jpg":0.25}', encoding="utf-8")
            summary = root / "summary.json"
            summary_args = argparse.Namespace(
                dataset=dataset,
                scores=scores,
                predictions=predictions,
                evaluator_root=repository,
                manifest=manifest,
                output=summary,
            )
            with mock.patch.object(GATE, "verified_revision", return_value={"git_sha": "verified", "worktree_clean": True}):
                GATE.summarize_command(summary_args)
            self.assertEqual(GATE.load_json(summary)["metrics"]["official_global_text_similarity"], 0.75)
            (predictions / "source.pdf_1.md").write_text("tampered", encoding="utf-8")
            with mock.patch.object(GATE, "verified_revision", return_value={"git_sha": "verified", "worktree_clean": True}), self.assertRaisesRegex(ValueError, "prediction tree hash"):
                GATE.summarize_command(summary_args)

            second_predictions = root / "predictions-second"
            second_manifest = root / "manifest-second.json"
            args.predictions = second_predictions
            args.manifest = second_manifest
            with mock.patch.object(GATE, "verified_revision", return_value={"git_sha": "verified", "worktree_clean": True}), mock.patch.object(
                GATE, "git_provenance", return_value={"git_sha": "candidate", "worktree_clean": True}
            ):
                GATE.export_predictions(args)
            repeated = GATE.load_json(second_manifest)
            self.assertEqual(recorded["predictions_sha256"], repeated["predictions_sha256"])


if __name__ == "__main__":
    unittest.main()
