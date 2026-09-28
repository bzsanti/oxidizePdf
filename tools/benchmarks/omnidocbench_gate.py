#!/usr/bin/env python3
"""Reproducible prediction and score gate for OmniDocBench native PDF text."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "oxidize-pdf-omnidocbench-gate/v2"
POPULATION_VERSION = "native-text-v1"
PROTOCOL = "OmniDocBench/end2end_eval/quick_match/text"
EXCLUDED_DATA_SOURCES = frozenset({"note"})
EXCLUDED_SPECIAL_ISSUES = frozenset({"fuzzy_scan"})
OFFICIAL_TEXT_CATEGORIES = frozenset(
    {"text_block", "title", "code_txt", "code_txt_caption", "reference"}
)
IDENTITY_FIELDS = (
    "schema_version",
    "dataset_revision",
    "evaluator_revision",
    "protocol",
    "ocr_enabled",
    "population_version",
    "extraction_config",
    "serialization",
    "serialization_vectors_sha256",
    "evaluator_config_sha256",
    "exporter_sha256",
    "source_lock_sha256",
    "rustc_version",
    "cargo_version",
)


SERIALIZATION_IDS = ("rust-split-whitespace-v1", "preserve-text-v1")
SERIALIZATION_VECTORS = Path(__file__).resolve().parents[2] / "oxidize-pdf-core/examples/support/omnidocbench-serialization-v1.json"
EVALUATOR_CONFIG = Path(__file__).with_name("fixtures") / "omnidocbench-v1-official.yaml"
# These describe implementations, not the semantics being compared. They remain
# mandatory provenance in each sealed identity.
IMPLEMENTATION_FIELDS = ("exporter_sha256", "source_lock_sha256", "rustc_version", "cargo_version")
COMPATIBILITY_FIELDS = tuple(field for field in IDENTITY_FIELDS if field not in IMPLEMENTATION_FIELDS)


def serialization_contract(name: str) -> dict[str, Any]:
    if name not in SERIALIZATION_IDS:
        raise ValueError(f"unknown serialization contract: {name!r}")
    return {"id": name, "encoding": "UTF-8", "appended_newline": False}


def validate_identity(identity: dict[str, Any]) -> None:
    missing = [field for field in IDENTITY_FIELDS if field not in identity or identity[field] is None]
    if missing:
        raise ValueError(f"manifest identity is incomplete: {', '.join(missing)}")
    for field in IDENTITY_FIELDS:
        value = identity[field]
        if field.endswith("_sha256"):
            if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{64}", value):
                raise ValueError(f"invalid {field}: expected SHA-256")
        elif field not in ("serialization", "extraction_config", "ocr_enabled"):
            if not isinstance(value, str) or not value.strip():
                raise ValueError(f"invalid {field}: expected nonempty string")
    if not isinstance(identity["extraction_config"], dict) or not identity["extraction_config"]:
        raise ValueError("invalid extraction_config")
    if identity["schema_version"] != SCHEMA_VERSION:
        raise ValueError("unsupported schema_version; legacy manifests need explicit migration evidence")
    config = identity["serialization"]
    if not isinstance(config, dict) or config != serialization_contract(config.get("id")):
        raise ValueError("invalid serialization configuration")
    if identity["serialization_vectors_sha256"] != file_hash(SERIALIZATION_VECTORS):
        raise ValueError("unknown serialization conformance vectors")
    if identity["ocr_enabled"] is not False or identity["protocol"] != PROTOCOL:
        raise ValueError("manifest does not describe the supported OCR-free protocol")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object)
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON in {path}: {error}") from error


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def canonical_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def file_hash(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_hash(directory: Path, suffix: str = ".md") -> str:
    entries = []
    for path in sorted(directory.rglob(f"*{suffix}"), key=lambda item: item.relative_to(directory).as_posix()):
        entries.append({"path": path.relative_to(directory).as_posix(), "sha256": file_hash(path)})
    return canonical_hash(entries)


def git_output(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, stdout=subprocess.PIPE, text=True
    ).stdout.rstrip("\n")


def _matches_lfs_pointer(root: Path, relative: str) -> bool:
    path = root / relative
    if not path.is_file():
        return False
    pointer = subprocess.run(
        ["git", "-C", str(root), "show", f"HEAD:{relative}"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    lines = pointer.decode("ascii", errors="replace").splitlines()
    if len(lines) != 3 or lines[0] != "version https://git-lfs.github.com/spec/v1":
        return False
    if not lines[1].startswith("oid sha256:") or not lines[2].startswith("size "):
        return False
    expected_hash = lines[1].removeprefix("oid sha256:")
    try:
        expected_size = int(lines[2].removeprefix("size "))
    except ValueError:
        return False
    return path.stat().st_size == expected_size and file_hash(path) == expected_hash


def git_provenance(allow_dirty: bool, source_root: Path | None = None) -> dict[str, Any]:
    prefix = ("-C", str(source_root.resolve())) if source_root else ()
    root = source_root.resolve() if source_root else Path.cwd()
    sha = git_output(*prefix, "rev-parse", "HEAD")
    raw_status = git_output(
        *prefix, "status", "--porcelain=v1", "-z", "--untracked-files=all"
    )
    entries = [entry for entry in raw_status.split("\0") if entry]
    materialized_lfs = [
        entry[3:]
        for entry in entries
        if entry.startswith(" M ") and _matches_lfs_pointer(root, entry[3:])
    ]
    remaining = [entry for entry in entries if entry[3:] not in materialized_lfs]
    status = "\0".join(remaining)
    if status and not allow_dirty:
        raise ValueError("dirty worktree cannot be labelled as a clean commit; pass --allow-dirty")
    result: dict[str, Any] = {"git_sha": sha, "worktree_clean": not bool(status)}
    if materialized_lfs:
        result["verified_materialized_lfs_files"] = len(materialized_lfs)
    if status:
        diff = git_output(*prefix, "diff", "--binary", "HEAD")
        untracked = git_output(*prefix, "ls-files", "--others", "--exclude-standard", "-z")
        dirty = hashlib.sha256()
        dirty.update(status.encode())
        dirty.update(b"\0")
        dirty.update(diff.encode())
        dirty.update(b"\0")
        dirty.update(untracked.encode())
        for name in sorted(filter(None, untracked.split("\0"))):
            path = (source_root / name) if source_root else Path(name)
            if path.is_file():
                dirty.update(name.encode())
                dirty.update(bytes.fromhex(file_hash(path)))
        result["dirty_state_sha256"] = dirty.hexdigest()
    return result


def verified_revision(root: Path, expected: str, label: str) -> dict[str, Any]:
    provenance = git_provenance(False, root)
    if provenance["git_sha"] != expected:
        raise ValueError(
            f"{label} revision mismatch: expected {expected}, found {provenance['git_sha']}"
        )
    return provenance


def command_version(*command: str) -> str:
    return subprocess.run(
        list(command), check=True, stdout=subprocess.PIPE, text=True
    ).stdout.strip()


def identity_fixture(**overrides: Any) -> dict[str, Any]:
    identity = {
        "schema_version": SCHEMA_VERSION,
        "dataset_revision": "dataset",
        "evaluator_revision": "evaluator",
        "protocol": PROTOCOL,
        "ocr_enabled": False,
        "population_version": POPULATION_VERSION,
        "extraction_config": {
            "api": "PlainTextExtractor::preserve_layout",
            "preserve_layout": True,
            "line_break_mode": "PreserveAll",
            "space_threshold": 0.3,
            "tj_space_threshold": 0.2,
            "newline_threshold": 10.0,
        },
        "serialization": serialization_contract("rust-split-whitespace-v1"),
        "serialization_vectors_sha256": file_hash(SERIALIZATION_VECTORS),
        "evaluator_config_sha256": file_hash(EVALUATOR_CONFIG),
        "exporter_sha256": "a" * 64,
        "source_lock_sha256": "b" * 64,
        "rustc_version": "rustc",
        "cargo_version": "cargo",
    }
    identity.update(overrides)
    return identity


def _dataset_metadata(dataset: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    if not isinstance(dataset, list):
        raise ValueError("dataset JSON must be an array")
    metadata: dict[str, dict[str, Any]] = {}
    for entry in dataset:
        if not isinstance(entry, dict):
            raise ValueError("each dataset page must be an object")
        try:
            info = entry["page_info"]
            name = Path(info["image_path"]).name
            attributes = info["page_attribute"]
        except (KeyError, TypeError) as error:
            raise ValueError(f"malformed dataset page: {error}") from error
        if name in metadata:
            raise ValueError(f"duplicate dataset page: {name}")
        if not isinstance(attributes, dict):
            raise ValueError(f"page attributes must be an object: {name}")
        special = attributes.get("special_issue", [])
        if not isinstance(special, list) or not all(isinstance(item, str) for item in special):
            raise ValueError(f"special_issue must be an array of strings: {name}")
        layout = entry.get("layout_dets", [])
        if not isinstance(layout, list) or not all(isinstance(item, dict) for item in layout):
            raise ValueError(f"layout_dets must be an array of objects: {name}")
        scorable = any(
            item.get("category_type") in OFFICIAL_TEXT_CATEGORIES
            and str(item.get("text", "")).strip()
            and not item.get("ignore", False)
            for item in layout
        )
        metadata[name] = {"attributes": attributes, "text_scorable": scorable}
    if not metadata:
        raise ValueError("dataset population is empty")
    return metadata


def summarize_scores(
    dataset: list[dict[str, Any]], scores: dict[str, Any], failed_pages: int = 0
) -> dict[str, Any]:
    if not isinstance(scores, dict):
        raise ValueError("scores JSON must be an object")
    metadata = _dataset_metadata(dataset)
    expected = {name for name, item in metadata.items() if item["text_scorable"]}
    unknown = sorted(set(scores) - set(metadata))
    missing = sorted(expected - set(scores))
    if unknown:
        raise ValueError(f"scores reference {len(unknown)} unknown pages; first: {unknown[0]}")
    if missing:
        raise ValueError(f"missing scores for {len(missing)} pages; first: {missing[0]}")

    native: list[float] = []
    global_values: list[float] = []
    for name in sorted(scores):
        score = scores[name]
        if (
            not isinstance(score, (int, float))
            or isinstance(score, bool)
            or not math.isfinite(score)
            or not 0.0 <= score <= 1.0
        ):
            raise ValueError(f"invalid score for {name}: {score!r}")
        value = float(score)
        global_values.append(value)
        attrs = metadata[name]["attributes"]
        special = set(attrs.get("special_issue", []))
        if attrs.get("data_source") not in EXCLUDED_DATA_SOURCES and not special.intersection(EXCLUDED_SPECIAL_ISSUES):
            native.append(value)
    if not native:
        raise ValueError("native-text population is empty")

    global_edit = math.fsum(global_values) / len(global_values)
    native_edit = math.fsum(native) / len(native)
    return {
        "population": {
            "version": POPULATION_VERSION,
            "excluded_data_sources": sorted(EXCLUDED_DATA_SOURCES),
            "excluded_special_issues": sorted(EXCLUDED_SPECIAL_ISSUES),
            "scored_pages_sha256": canonical_hash(sorted(scores)),
        },
        "counts": {
            "dataset": len(metadata),
            "official_text_scorable": len(expected),
            "official_text_unscored": len(metadata) - len(expected),
            "scored": len(scores),
            "included_native": len(native),
            "excluded_native": len(global_values) - len(native),
            "missing": 0,
            "duplicate": 0,
            "failed": failed_pages,
        },
        "metrics": {
            "official_global_text_edit_distance": global_edit,
            "official_global_text_similarity": 1.0 - global_edit,
            "native_text_edit_distance": native_edit,
            "native_text_similarity": 1.0 - native_edit,
        },
    }


def compare_summaries(baseline: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
    for label, summary in (("baseline", baseline), ("candidate", candidate)):
        recorded_hash = summary.get("summary_sha256")
        unhashed = {key: value for key, value in summary.items() if key != "summary_sha256"}
        if recorded_hash != canonical_hash(unhashed):
            raise ValueError(f"{label} summary hash is invalid")
        if summary.get("provenance", {}).get("worktree_clean") is not True:
            raise ValueError(f"{label} summary is not from a clean worktree")
        artifacts = summary.get("artifacts", {})
        if not all(artifacts.get(name) for name in ("dataset_sha256", "scores_sha256", "predictions_sha256", "evaluation_run_sha256")):
            raise ValueError(f"{label} artifact identity is incomplete")
        validate_identity(summary.get("identity", {}))
    if baseline["artifacts"]["dataset_sha256"] != candidate["artifacts"]["dataset_sha256"]:
        raise ValueError("incompatible dataset artifact hash")
    left = baseline.get("identity", {})
    right = candidate.get("identity", {})
    mismatches = [field for field in COMPATIBILITY_FIELDS if left.get(field) != right.get(field)]
    if mismatches:
        raise ValueError(f"incompatible benchmark identity: {', '.join(mismatches)}")
    population_counts = ("dataset", "official_text_scorable", "included_native", "excluded_native")
    count_mismatches = [
        name
        for name in population_counts
        if baseline.get("counts", {}).get(name) != candidate.get("counts", {}).get(name)
    ]
    if count_mismatches:
        raise ValueError(f"incompatible benchmark population counts: {', '.join(count_mismatches)}")
    left_pages = baseline.get("population", {}).get("scored_pages_sha256")
    right_pages = candidate.get("population", {}).get("scored_pages_sha256")
    if not left_pages or left_pages != right_pages:
        raise ValueError("incompatible scored-page population")
    before = baseline["metrics"]
    after = candidate["metrics"]
    return {
        "identity_sha256": canonical_hash({key: left[key] for key in COMPATIBILITY_FIELDS}),
        "baseline_summary_sha256": baseline["summary_sha256"],
        "candidate_summary_sha256": candidate["summary_sha256"],
        "official_global_text_similarity_delta": after["official_global_text_similarity"] - before["official_global_text_similarity"],
        "native_text_similarity_delta": after["native_text_similarity"] - before["native_text_similarity"],
    }


def verify_exports(left: Path, left_report: dict[str, Any], right: Path, right_report: dict[str, Any]) -> dict[str, Any]:
    for report in (left_report, right_report):
        config = report.get("serialization", {})
        if config != serialization_contract(config.get("id")):
            raise ValueError("invalid serialization configuration")
    for field in ("serialization", "extraction_config"):
        if left_report.get(field) is None or left_report.get(field) != right_report.get(field):
            raise ValueError(f"incompatible export {field}")
    names = [{p.relative_to(root).as_posix() for p in root.rglob("*.md")} for root in (left, right)]
    if not names[0] or names[0] != names[1]:
        raise ValueError("incompatible prediction population")
    failures = []
    for report in (left_report, right_report):
        count = report.get("counts", {}).get("written", report.get("evaluated"))
        failed = report.get("counts", {}).get("failed", report.get("extraction_errors"))
        records = report.get("failures")
        if count != len(names[0]) or not isinstance(records, list) or failed != len(records):
            raise ValueError("incompatible reported prediction population or failures")
        records = sorted((item["prediction_name"], item["error"]) for item in records)
        if len({name for name, _ in records}) != len(records) or any(name not in names[0] for name, _ in records):
            raise ValueError("invalid extraction failure population")
        failures.append(records)
    if failures[0] != failures[1]:
        raise ValueError("incompatible extraction failures")
    for name in sorted(names[0]):
        if (left / name).read_bytes() != (right / name).read_bytes():
            raise ValueError(f"different prediction bytes: {name}")
    return {"predictions": len(names[0]), "failures": len(failures[0]), "serialization": left_report["serialization"], "left_sha256": tree_hash(left), "right_sha256": tree_hash(right)}


def verify_exports_command(args: argparse.Namespace) -> None:
    _write_json(args.output, verify_exports(args.left, load_json(args.left_report), args.right, load_json(args.right_report)))


def source_pdf_identity(entry: dict[str, Any]) -> tuple[str, int]:
    info = entry["page_info"]
    image_name = Path(info["image_path"]).name
    page_no = info["page_no"]
    if not isinstance(page_no, int) or isinstance(page_no, bool) or page_no < 1:
        raise ValueError(f"invalid page_no for {image_name}: {page_no!r}")
    stem = Path(image_name).stem
    suffix = f"_{page_no}"
    if not stem.endswith(suffix):
        raise ValueError(f"image name does not end in _<page_no>: {image_name}")
    source = stem[: -len(suffix)]
    if not source.lower().endswith(".pdf"):
        source += ".pdf"
    return source, page_no - 1


def resolve_source_pdf(
    entry: dict[str, Any], pdfs: dict[str, Path]
) -> tuple[Path, int]:
    image_name = Path(entry["page_info"]["image_path"]).name
    split_name = f"{Path(image_name).stem}.pdf"
    split_pdf = pdfs.get(split_name.casefold())
    if split_pdf is not None:
        return split_pdf, 0
    source_name, page_index = source_pdf_identity(entry)
    source = pdfs.get(source_name.casefold())
    if source is None:
        raise ValueError(
            f"missing source PDF: tried {split_name} and {source_name}"
        )
    return source, page_index


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_json(value))


def _pdf_index(root: Path) -> dict[str, Path]:
    result: dict[str, Path] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix.lower() != ".pdf":
            continue
        key = path.name.casefold()
        if key in result:
            raise ValueError(f"duplicate source PDF basename: {path.name}")
        result[key] = path.resolve()
    return result


def render_evaluation_config(template: str, dataset: Path, predictions: Path) -> str:
    replacements = {"/dataset/OmniDocBench.json": str(dataset.resolve()), "/predictions": str(predictions.resolve())}
    return re.sub(r"/dataset/OmniDocBench\.json|/predictions", lambda match: json.dumps(replacements[match[0]]), template)


def exporter_bundle_hash(examples: Path) -> str:
    return canonical_hash({"exporter": file_hash(examples / "omnidocbench_export.rs"), "serialization": file_hash(examples / "support/omnidocbench_serialization.rs")})


def export_predictions(args: argparse.Namespace) -> None:
    serialization = serialization_contract(args.serialization)
    dataset = load_json(args.dataset)
    _dataset_metadata(dataset)
    dataset_root = args.dataset_root.resolve()
    evaluator_root = args.evaluator_root.resolve()
    try:
        args.dataset.resolve().relative_to(dataset_root)
    except ValueError as error:
        raise ValueError("--dataset must be inside --dataset-root") from error
    dataset_provenance = verified_revision(
        dataset_root, args.dataset_revision, "dataset"
    )
    evaluator_provenance = verified_revision(
        evaluator_root, args.evaluator_revision, "evaluator"
    )
    pdfs = _pdf_index(args.pdf_root)
    jobs = []
    for entry in sorted(dataset, key=lambda item: Path(item["page_info"]["image_path"]).name):
        source, page_index = resolve_source_pdf(entry, pdfs)
        image_name = Path(entry["page_info"]["image_path"]).name
        jobs.append({"prediction_name": str(Path(image_name).with_suffix(".md")), "pdf_path": str(source), "page_index": page_index})

    source_root = args.source_root.resolve()
    provenance = git_provenance(args.allow_dirty, source_root)
    provenance["dataset_repository"] = dataset_provenance
    provenance["evaluator_repository"] = evaluator_provenance
    exporter = Path(__file__).resolve().parents[2] / "oxidize-pdf-core/examples/omnidocbench_export.rs"
    source_lock = source_root / "Cargo.lock"
    if not source_lock.is_file():
        raise ValueError(f"source checkout has no Cargo.lock: {source_root}")
    with tempfile.TemporaryDirectory(prefix="oxidize-omnidocbench-") as directory:
        harness = Path(directory)
        archived_source = harness / "source"
        jobs_path = harness / "jobs.json"
        report_path = harness / "export-report.json"
        core_path = archived_source / "oxidize-pdf-core"
        if not (source_root / "oxidize-pdf-core/Cargo.toml").is_file():
            raise ValueError(f"source checkout has no oxidize-pdf-core crate: {source_root}")
        archive_path = harness / "source.tar"
        subprocess.run(
            ["git", "-C", str(source_root), "archive", "--format=tar", f"--output={archive_path}", "HEAD"],
            check=True,
        )
        archived_source.mkdir()
        with tarfile.open(archive_path) as archive:
            archive.extractall(archived_source, filter="data")
        examples = core_path / "examples"
        examples.mkdir(exist_ok=True)
        (examples / "omnidocbench_export.rs").write_bytes(exporter.read_bytes())
        support = examples / "support"
        support.mkdir(exist_ok=True)
        for name in ("omnidocbench_serialization.rs", "omnidocbench-serialization-v1.json"):
            (support / name).write_bytes((exporter.parent / "support" / name).read_bytes())
        exporter_sha256 = exporter_bundle_hash(examples)
        vectors_sha256 = file_hash(support / "omnidocbench-serialization-v1.json")
        config_template = EVALUATOR_CONFIG.read_bytes()
        _write_json(jobs_path, jobs)
        command_env = os.environ.copy()
        command_env.setdefault(
            "CARGO_TARGET_DIR",
            str(Path(__file__).resolve().parents[2] / "target/omnidocbench-gate"),
        )
        subprocess.run(
            ["cargo", "run", "--locked", "--offline", "--quiet", "--release", "--manifest-path", str(archived_source / "Cargo.toml"), "-p", "oxidize-pdf", "--example", "omnidocbench_export", "--", str(jobs_path), str(args.predictions), str(report_path), "--serialization", args.serialization],
            check=True,
            env=command_env,
        )
        report = load_json(report_path)
        if report.get("serialization") != serialization:
            raise ValueError("exporter serialization differs from requested contract")
        archived_lock_sha256 = file_hash(archived_source / "Cargo.lock")
    identity = identity_fixture(
        dataset_revision=args.dataset_revision,
        evaluator_revision=args.evaluator_revision,
        extraction_config=report["extraction_config"],
        serialization=report["serialization"],
        exporter_sha256=exporter_sha256,
        serialization_vectors_sha256=vectors_sha256,
        evaluator_config_sha256=hashlib.sha256(config_template).hexdigest(),
        source_lock_sha256=archived_lock_sha256,
        rustc_version=command_version("rustc", "-Vv"),
        cargo_version=command_version("cargo", "-V"),
    )
    config_path = args.manifest.with_suffix(".evaluation.yaml")
    config_path.parent.mkdir(parents=True, exist_ok=True)
    configuration = render_evaluation_config(config_template.decode("utf-8"), args.dataset, args.predictions)
    config_path.write_text(configuration, encoding="utf-8")
    manifest = {
        "evaluation_config": {"path": str(config_path.resolve()), "sha256": file_hash(config_path)},
        "prediction_hashes": {p.name: file_hash(p) for p in sorted(args.predictions.glob("*.md"))},
        "identity": identity,
        "provenance": provenance,
        "dataset_sha256": file_hash(args.dataset),
        "predictions_sha256": tree_hash(args.predictions),
        "counts": report["counts"],
        "failures": report["failures"],
    }
    _write_json(args.manifest, manifest)


def evaluate_command(args: argparse.Namespace) -> None:
    manifest = load_json(args.manifest)
    validate_identity(manifest.get("identity", {}))
    verified_revision(args.evaluator_root, manifest["identity"]["evaluator_revision"], "evaluator")
    # A fresh directory prevents stale evaluator files from being blessed.
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    predictions = output / "predictions"
    import shutil
    shutil.copytree(args.predictions, predictions)
    dataset = output / "OmniDocBench.json"
    shutil.copyfile(args.dataset, dataset)
    if tree_hash(predictions) != manifest.get("predictions_sha256") or file_hash(dataset) != manifest.get("dataset_sha256"):
        raise ValueError("evaluation input hashes do not match export manifest")
    template = EVALUATOR_CONFIG.read_bytes()
    if hashlib.sha256(template).hexdigest() != manifest["identity"]["evaluator_config_sha256"]:
        raise ValueError("evaluation template hash does not match export manifest")
    configuration = output / "evaluation.yaml"
    configuration.write_text(render_evaluation_config(template.decode("utf-8"), dataset, predictions), encoding="utf-8")
    (output / "result").mkdir()
    command = [str(args.python.absolute()), str(args.evaluator_root.resolve() / "pdf_validation.py"), "--config", str(configuration)]
    before = {"dataset_sha256": file_hash(dataset), "predictions_sha256": tree_hash(predictions), "evaluation_config_sha256": file_hash(configuration)}
    subprocess.run(command, cwd=output, check=True)
    after = {"dataset_sha256": file_hash(dataset), "predictions_sha256": tree_hash(predictions), "evaluation_config_sha256": file_hash(configuration)}
    if after != before:
        raise ValueError("evaluation inputs changed during execution")
    verified_revision(args.evaluator_root, manifest["identity"]["evaluator_revision"], "evaluator")
    scores = output / "result/predictions_quick_match_text_block_per_page_edit.json"
    summarize_scores(load_json(dataset), load_json(scores), manifest["counts"]["failed"])
    run = {**before, "schema_version": "omnidocbench-evaluation/v1", "export_manifest_sha256": file_hash(args.manifest), "identity": manifest["identity"], "command": command, "scores_sha256": file_hash(scores), "evaluation_config_path": str(configuration)}
    _write_json(output / "evaluation-run.json", run)


def verify_evaluation_run(path: Path, manifest_path: Path, scores_path: Path, identity: dict[str, Any]) -> dict[str, Any]:
    run = load_json(path)
    manifest = load_json(manifest_path)
    expected = {"schema_version": "omnidocbench-evaluation/v1", "export_manifest_sha256": file_hash(manifest_path), "identity": identity, "scores_sha256": file_hash(scores_path), "dataset_sha256": manifest["dataset_sha256"], "predictions_sha256": manifest["predictions_sha256"]}
    for field, value in expected.items():
        if run.get(field) != value:
            raise ValueError(f"evaluation run {field} mismatch")
    if file_hash(Path(run["evaluation_config_path"])) != run.get("evaluation_config_sha256"):
        raise ValueError("evaluation run configuration hash mismatch")
    return run


def summarize_command(args: argparse.Namespace) -> None:
    dataset = load_json(args.dataset)
    scores = load_json(args.scores)
    manifest = load_json(args.manifest)
    dataset_sha256 = file_hash(args.dataset)
    if manifest.get("dataset_sha256") != dataset_sha256:
        raise ValueError("manifest dataset hash does not match --dataset")
    dataset_pages = len(_dataset_metadata(dataset))
    export_counts = manifest.get("counts", {})
    if export_counts.get("attempted") != dataset_pages or export_counts.get("written") != dataset_pages:
        raise ValueError("incomplete prediction manifest")
    if export_counts.get("failed") != len(manifest.get("failures", [])):
        raise ValueError("prediction failure count does not match failure records")
    identity = manifest.get("identity", {})
    validate_identity(identity)
    evaluation = manifest.get("evaluation_config", {})
    if not evaluation.get("path") or file_hash(Path(evaluation["path"])) != evaluation.get("sha256"):
        raise ValueError("evaluation configuration hash does not match manifest")
    verified_revision(args.evaluator_root, identity["evaluator_revision"], "evaluator")
    predictions_sha256 = tree_hash(args.predictions)
    if manifest.get("predictions_sha256") != predictions_sha256:
        raise ValueError("prediction tree hash does not match manifest")
    prediction_count = sum(1 for path in args.predictions.rglob("*.md") if path.is_file())
    expected_predictions = {str(Path(entry["page_info"]["image_path"]).with_suffix(".md").name) for entry in dataset}
    actual_predictions = {p.name for p in args.predictions.glob("*.md")}
    if prediction_count != dataset_pages or actual_predictions != expected_predictions:
        raise ValueError("prediction directory is incomplete or has unexpected pages")
    if manifest.get("prediction_hashes") != {p.name: file_hash(p) for p in sorted(args.predictions.glob("*.md"))}:
        raise ValueError("per-page prediction hashes do not match manifest")
    run = verify_evaluation_run(args.evaluation_run, args.manifest, args.scores, identity)
    result = summarize_scores(dataset, scores, manifest["counts"]["failed"])
    result["identity"] = manifest["identity"]
    result["provenance"] = manifest["provenance"]
    result["artifacts"] = {
        "dataset_sha256": dataset_sha256,
        "scores_sha256": canonical_hash(scores),
        "predictions_sha256": predictions_sha256,
        "evaluation_config_sha256": run["evaluation_config_sha256"],
        "evaluation_run_sha256": file_hash(args.evaluation_run),
    }
    result["summary_sha256"] = canonical_hash(result)
    _write_json(args.output, result)


def compare_command(args: argparse.Namespace) -> None:
    result = compare_summaries(load_json(args.baseline), load_json(args.candidate))
    _write_json(args.output, result)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    export = commands.add_parser("export", help="generate deterministic Markdown predictions")
    export.add_argument("--dataset", type=Path, required=True)
    export.add_argument("--dataset-root", type=Path, required=True, help="clean Git checkout containing the dataset")
    export.add_argument("--evaluator-root", type=Path, required=True, help="clean pinned evaluator Git checkout")
    export.add_argument("--pdf-root", type=Path, required=True)
    export.add_argument("--source-root", type=Path, default=Path.cwd(), help="clean oxidize-pdf checkout to evaluate")
    export.add_argument("--predictions", type=Path, required=True)
    export.add_argument("--manifest", type=Path, required=True)
    export.add_argument("--dataset-revision", required=True)
    export.add_argument("--evaluator-revision", required=True)
    export.add_argument("--allow-dirty", action="store_true")
    export.add_argument("--serialization", choices=SERIALIZATION_IDS, default=SERIALIZATION_IDS[0])
    export.set_defaults(handler=export_predictions)
    evaluate = commands.add_parser("evaluate", help="run the pinned evaluator and bind its scores to snapshot inputs")
    for name in ("dataset", "predictions", "manifest", "evaluator-root", "python", "output"):
        evaluate.add_argument("--" + name, type=Path, required=True)
    evaluate.set_defaults(handler=evaluate_command)
    summary = commands.add_parser("summarize", help="validate official per-page text scores")
    summary.add_argument("--dataset", type=Path, required=True)
    summary.add_argument("--evaluation-run", type=Path, required=True)
    summary.add_argument("--scores", type=Path, required=True)
    summary.add_argument("--predictions", type=Path, required=True)
    summary.add_argument("--evaluator-root", type=Path, required=True)
    summary.add_argument("--manifest", type=Path, required=True)
    summary.add_argument("--output", type=Path, required=True)
    summary.set_defaults(handler=summarize_command)
    compare = commands.add_parser("compare", help="compare compatible summaries")
    compare.add_argument("--baseline", type=Path, required=True)
    compare.add_argument("--candidate", type=Path, required=True)
    compare.add_argument("--output", type=Path, required=True)
    compare.set_defaults(handler=compare_command)
    verify = commands.add_parser("verify-exports", help="verify exact bytes, contract and failure population between exporters")
    verify.add_argument("--left", type=Path, required=True)
    verify.add_argument("--left-report", type=Path, required=True)
    verify.add_argument("--right", type=Path, required=True)
    verify.add_argument("--right-report", type=Path, required=True)
    verify.add_argument("--output", type=Path, required=True)
    verify.set_defaults(handler=verify_exports_command)
    return parser


def main() -> None:
    args = build_parser().parse_args()
    try:
        args.handler(args)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error


if __name__ == "__main__":
    main()
