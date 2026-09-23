#!/usr/bin/env python3
"""Reproducible prediction and score gate for OmniDocBench native PDF text."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "oxidize-pdf-omnidocbench-gate/v1"
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
    "exporter_sha256",
    "source_lock_sha256",
    "rustc_version",
    "cargo_version",
)


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
        "exporter_sha256": "exporter",
        "source_lock_sha256": "lock",
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
        if not all(artifacts.get(name) for name in ("dataset_sha256", "scores_sha256", "predictions_sha256")):
            raise ValueError(f"{label} artifact identity is incomplete")
    left = baseline.get("identity", {})
    right = candidate.get("identity", {})
    mismatches = [field for field in IDENTITY_FIELDS if left.get(field) != right.get(field)]
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
        "identity_sha256": canonical_hash(left),
        "official_global_text_similarity_delta": after["official_global_text_similarity"] - before["official_global_text_similarity"],
        "native_text_similarity_delta": after["native_text_similarity"] - before["native_text_similarity"],
    }


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


def export_predictions(args: argparse.Namespace) -> None:
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
        _write_json(jobs_path, jobs)
        command_env = os.environ.copy()
        command_env.setdefault(
            "CARGO_TARGET_DIR",
            str(Path(__file__).resolve().parents[2] / "target/omnidocbench-gate"),
        )
        subprocess.run(
            ["cargo", "run", "--locked", "--offline", "--quiet", "--release", "--manifest-path", str(archived_source / "Cargo.toml"), "-p", "oxidize-pdf", "--example", "omnidocbench_export", "--", str(jobs_path), str(args.predictions), str(report_path)],
            check=True,
            env=command_env,
        )
        report = load_json(report_path)
        archived_lock_sha256 = file_hash(archived_source / "Cargo.lock")
    identity = identity_fixture(
        dataset_revision=args.dataset_revision,
        evaluator_revision=args.evaluator_revision,
        extraction_config=report["extraction_config"],
        exporter_sha256=file_hash(exporter),
        source_lock_sha256=archived_lock_sha256,
        rustc_version=command_version("rustc", "-Vv"),
        cargo_version=command_version("cargo", "-V"),
    )
    manifest = {
        "identity": identity,
        "provenance": provenance,
        "dataset_sha256": file_hash(args.dataset),
        "predictions_sha256": tree_hash(args.predictions),
        "counts": report["counts"],
        "failures": report["failures"],
    }
    _write_json(args.manifest, manifest)


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
    missing_identity = [field for field in IDENTITY_FIELDS if field not in identity]
    if missing_identity:
        raise ValueError(f"manifest identity is incomplete: {', '.join(missing_identity)}")
    if identity["ocr_enabled"] is not False or identity["protocol"] != PROTOCOL:
        raise ValueError("manifest does not describe the supported OCR-free protocol")
    verified_revision(args.evaluator_root, identity["evaluator_revision"], "evaluator")
    predictions_sha256 = tree_hash(args.predictions)
    if manifest.get("predictions_sha256") != predictions_sha256:
        raise ValueError("prediction tree hash does not match manifest")
    prediction_count = sum(1 for path in args.predictions.rglob("*.md") if path.is_file())
    if prediction_count != dataset_pages:
        raise ValueError("prediction directory is incomplete")
    result = summarize_scores(dataset, scores, manifest["counts"]["failed"])
    result["identity"] = manifest["identity"]
    result["provenance"] = manifest["provenance"]
    result["artifacts"] = {
        "dataset_sha256": dataset_sha256,
        "scores_sha256": canonical_hash(scores),
        "predictions_sha256": predictions_sha256,
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
    export.set_defaults(handler=export_predictions)
    summary = commands.add_parser("summarize", help="validate official per-page text scores")
    summary.add_argument("--dataset", type=Path, required=True)
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
