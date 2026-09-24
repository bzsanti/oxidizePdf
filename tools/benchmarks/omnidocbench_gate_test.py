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
            "evaluation_run_sha256": "run",
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


class ExportEquivalenceTests(unittest.TestCase):
    def test_compares_bytes_and_keeps_empty_predictions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "left", root / "right"
            left.mkdir(); right.mkdir()
            for path in (left, right):
                (path / "page.md").write_bytes(b"Alpha Beta")
                (path / "empty.md").write_bytes(b"")
            report = {"serialization": GATE.serialization_contract("rust-split-whitespace-v1"), "extraction_config": {"api": "test"}, "counts": {"written": 2, "failed": 0}, "failures": []}
            result = GATE.verify_exports(left, report, right, report)
            self.assertEqual(result["predictions"], 2)
            (right / "page.md").write_bytes(b"Alpha Beta\n")
            with self.assertRaisesRegex(ValueError, "bytes.*page.md"):
                GATE.verify_exports(left, report, right, report)
            (right / "page.md").write_bytes(b"Alpha Beta")
            (right / "empty.md").unlink()
            with self.assertRaisesRegex(ValueError, "population"):
                GATE.verify_exports(left, report, right, report)


class IdentityAndHashTests(unittest.TestCase):
    def test_compare_rejects_different_serialization(self):
        baseline = sealed_summary(GATE.identity_fixture(serialization=GATE.serialization_contract("rust-split-whitespace-v1")))
        candidate = sealed_summary(GATE.identity_fixture(serialization=GATE.serialization_contract("preserve-text-v1")))
        with self.assertRaisesRegex(ValueError, "serialization"):
            GATE.compare_summaries(baseline, candidate)

    def test_compare_rejects_missing_serialization_on_both_sides(self):
        identity = GATE.identity_fixture()
        identity.pop("serialization", None)
        with self.assertRaisesRegex(ValueError, "serialization"):
            GATE.compare_summaries(sealed_summary(identity), sealed_summary(identity))

    def test_compare_rejects_unknown_or_misdeclared_serialization(self):
        for serialization in ({"id": "future-v9"}, {"id": "rust-split-whitespace-v1", "encoding": "UTF-16", "appended_newline": False}):
            with self.subTest(serialization=serialization):
                identity = GATE.identity_fixture(serialization=serialization)
                with self.assertRaisesRegex(ValueError, "serialization"):
                    GATE.compare_summaries(sealed_summary(identity), sealed_summary(identity))

    def test_compare_accepts_conformant_exporters_with_distinct_provenance(self):
        candidate = sealed_summary(GATE.identity_fixture(exporter_sha256="c" * 64, source_lock_sha256="d" * 64), similarity=0.75)
        result = GATE.compare_summaries(sealed_summary(), candidate)
        self.assertEqual(result["official_global_text_similarity_delta"], 0.25)
        self.assertEqual(result["candidate_summary_sha256"], candidate["summary_sha256"])

    def test_compare_rejects_missing_provenance_on_both_sides(self):
        identity = GATE.identity_fixture()
        del identity["exporter_sha256"]
        with self.assertRaisesRegex(ValueError, "exporter_sha256"):
            GATE.compare_summaries(sealed_summary(identity), sealed_summary(identity))

    def test_compare_rejects_different_evaluation_config(self):
        candidate = sealed_summary(GATE.identity_fixture(evaluator_config_sha256="different"))
        with self.assertRaisesRegex(ValueError, "evaluator_config"):
            GATE.compare_summaries(sealed_summary(), candidate)

    def test_comparison_requires_evaluation_run_provenance(self):
        summary = sealed_summary()
        del summary["artifacts"]["evaluation_run_sha256"]
        summary["summary_sha256"] = GATE.canonical_hash({k:v for k,v in summary.items() if k != "summary_sha256"})
        with self.assertRaisesRegex(ValueError, "artifact identity"):
            GATE.compare_summaries(summary, summary)

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


class ReviewRegressionTests(unittest.TestCase):
    def test_required_provenance_rejects_empty_and_wrong_types(self):
        for field in GATE.IMPLEMENTATION_FIELDS + ("evaluator_config_sha256",):
            for value in ("", " ", False, 7, [], {}):
                with self.subTest(field=field, value=value), self.assertRaisesRegex(ValueError, field):
                    GATE.validate_identity(GATE.identity_fixture(**{field: value}))
        for field in ("exporter_sha256", "source_lock_sha256", "evaluator_config_sha256"):
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, field):
                GATE.validate_identity(GATE.identity_fixture(**{field: "not-a-hash"}))

    def test_yaml_paths_are_replaced_once_and_remain_json_scalars(self):
        dataset = Path('/predictions-dataset/with "quotes"/OmniDocBench.json')
        predictions = Path('/dataset/OmniDocBench.json/predictions')
        rendered = GATE.render_evaluation_config('data: /dataset/OmniDocBench.json\nprediction: /predictions\n', dataset, predictions)
        self.assertEqual(json.loads(rendered.splitlines()[0].split(': ', 1)[1]), str(dataset))
        self.assertEqual(json.loads(rendered.splitlines()[1].split(': ', 1)[1]), str(predictions))


    def test_evaluator_failure_or_changed_inputs_never_emit_completion(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dataset = root / "dataset.json"
            GATE._write_json(dataset, [page("a.jpg")])
            predictions = root / "source-predictions"
            predictions.mkdir()
            (predictions / "a.md").write_text("Alpha Beta")
            manifest = root / "export.json"
            GATE._write_json(manifest, {"identity": GATE.identity_fixture(), "predictions_sha256": GATE.tree_hash(predictions), "dataset_sha256": GATE.file_hash(dataset), "counts": {"failed": 0}})
            for mode in ("failure", "changed-input"):
                output = root / mode
                args = argparse.Namespace(manifest=manifest, dataset=dataset, predictions=predictions, evaluator_root=root, python=Path("/usr/bin/python3"), output=output)
                def evaluator(command, cwd, check):
                    if mode == "failure":
                        raise subprocess.CalledProcessError(1, command)
                    (cwd / "predictions/a.md").write_text("changed")
                with mock.patch.object(GATE, "verified_revision"), mock.patch.object(GATE.subprocess, "run", side_effect=evaluator), self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    GATE.evaluate_command(args)
                self.assertFalse((output / "evaluation-run.json").exists())
    def test_export_hash_uses_copied_sources_when_live_files_change(self):
        repository = MODULE_PATH.parents[2]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake_gate = root / "tools/benchmarks/omnidocbench_gate.py"
            fake_gate.parent.mkdir(parents=True)
            examples = root / "oxidize-pdf-core/examples"
            shutil.copytree(repository / "oxidize-pdf-core/examples/support", examples / "support")
            exporter = examples / "omnidocbench_export.rs"
            shutil.copyfile(repository / "oxidize-pdf-core/examples/omnidocbench_export.rs", exporter)
            expected = GATE.exporter_bundle_hash(examples)
            dataset = root / "dataset.json"
            GATE._write_json(dataset, [page("source.pdf_1.jpg")])
            (root / "source.pdf").write_bytes(b"fixture")
            predictions = root / "predictions"
            args = argparse.Namespace(dataset=dataset,dataset_root=root,evaluator_root=repository,pdf_root=root,source_root=repository,predictions=predictions,manifest=root / "manifest.json",dataset_revision="dataset",evaluator_revision="evaluator",allow_dirty=True,serialization="rust-split-whitespace-v1")
            real_run = subprocess.run
            def command(cmd, **kwargs):
                if cmd[0] != "cargo":
                    return real_run(cmd, **kwargs)
                exporter.write_text("changed after snapshot")
                predictions.mkdir()
                (predictions / "source.pdf_1.md").write_text("fixture")
                report_path = Path(cmd[cmd.index("--") + 3])
                GATE._write_json(report_path, {"serialization":GATE.serialization_contract(args.serialization),"extraction_config":{"api":"fixture"},"counts":{"attempted":1,"written":1,"failed":0},"failures":[]})
            with mock.patch.object(GATE, "__file__", str(fake_gate)), mock.patch.object(GATE, "verified_revision", return_value={}), mock.patch.object(GATE, "git_provenance", return_value={}), mock.patch.object(GATE, "command_version", return_value="fixture-version"), mock.patch.object(GATE.subprocess, "run", side_effect=command):
                GATE.export_predictions(args)
            recorded = GATE.load_json(args.manifest)
            self.assertEqual(recorded["identity"]["exporter_sha256"], expected)
            self.assertNotEqual(recorded["identity"]["exporter_sha256"], GATE.exporter_bundle_hash(examples))


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
                serialization="rust-split-whitespace-v1",
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
            run_dir = root / "evaluation"
            evaluator_args = argparse.Namespace(dataset=dataset, predictions=predictions, manifest=manifest, evaluator_root=repository, python=Path("/usr/bin/python3"), output=run_dir)
            def fake_evaluator(command, cwd, check):
                # Test double for the external metric engine; production runner owns
                # the fresh directory, snapshots, invocation and completion record.
                (cwd / "result/predictions_quick_match_text_block_per_page_edit.json").write_bytes(scores.read_bytes())
            with mock.patch.object(GATE, "verified_revision"), mock.patch.object(GATE.subprocess, "run", side_effect=fake_evaluator):
                GATE.evaluate_command(evaluator_args)
            evaluation_run = run_dir / "evaluation-run.json"
            scores = run_dir / "result/predictions_quick_match_text_block_per_page_edit.json"
            summary = root / "summary.json"
            summary_args = argparse.Namespace(
                dataset=dataset,
                scores=scores,
                evaluation_run=evaluation_run,
                predictions=predictions,
                evaluator_root=repository,
                manifest=manifest,
                output=summary,
            )
            with mock.patch.object(GATE, "verified_revision", return_value={"git_sha": "verified", "worktree_clean": True}):
                GATE.summarize_command(summary_args)
            self.assertEqual(GATE.load_json(summary)["metrics"]["official_global_text_similarity"], 0.75)
            original_scores = scores.read_bytes()
            scores.write_text('{"source.pdf_1.jpg":0.9}')
            with mock.patch.object(GATE, "verified_revision"), self.assertRaisesRegex(ValueError, "evaluation run scores_sha256"):
                GATE.summarize_command(summary_args)
            scores.write_bytes(original_scores)
            run = GATE.load_json(evaluation_run)
            run["identity"]["serialization"] = GATE.serialization_contract("preserve-text-v1")
            GATE._write_json(evaluation_run, run)
            with mock.patch.object(GATE, "verified_revision"), self.assertRaisesRegex(ValueError, "evaluation run identity"):
                GATE.summarize_command(summary_args)
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
