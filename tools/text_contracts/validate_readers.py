#!/usr/bin/env python3
"""Offline reader comparison for #666. Expected text is never learned from readers."""
import argparse
import hashlib
import importlib
import json
import math
from pathlib import Path
import re
import shutil
import subprocess
import sys

PINNED = {"pymupdf": "1.26.5", "mupdf": "1.26.10"}


def command(args):
    try:
        result = subprocess.run(args, capture_output=True, text=True, encoding="utf-8", timeout=30)
        return {"exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
    except (OSError, subprocess.TimeoutExpired, UnicodeError) as error:
        return {"exit_code": None, "stdout": "", "stderr": str(error)}


def environment():
    versions, errors = {}, []
    try:
        mupdf = importlib.import_module("pymupdf")
        versions.update(pymupdf=mupdf.version[0], mupdf=mupdf.version[1])
        for name, version in PINNED.items():
            if versions[name] != version:
                errors.append(f"{name}: expected {version}, found {versions[name]}")
    except ImportError as error:
        errors.append(f"MuPDF unavailable: {error}")
    for name, binary, flag in [("poppler", "pdftotext", "-v"), ("qpdf", "qpdf", "--version")]:
        path = shutil.which(binary)
        if path is None:
            errors.append(f"missing required validation tool: {binary}")
            continue
        result = command([path, flag])
        match = re.search(r"version\s+([\d.]+)", result["stdout"] + result["stderr"])
        if result["exit_code"] != 0 or not match:
            errors.append(f"cannot determine {name} version")
        else:
            versions[name] = match.group(1)
    return versions, errors


def classify_poppler(case, version, result):
    expected = case["expected_text"] + "\n\f"
    if result == {"exit_code": 0, "stdout": expected, "stderr": ""}:
        return "match"
    known = case.get("known_poppler")
    if (known and known["version"] == version and known.get("reason")
            and all(result.get(key) == known[key] for key in ("exit_code", "stdout", "stderr"))):
        return "known_limitation"
    return "mismatch"


def mupdf_result(path, expected, expected_origins=None):
    mupdf = importlib.import_module("pymupdf")
    mupdf.TOOLS.mupdf_warnings(reset=True)
    try:
        with mupdf.open(path) as document:
            pages = [page.get_text("text", sort=False) for page in document]
            origins = None
            if expected_origins is not None:
                origins = [{"text": chr(char[0]), "origin": list(char[2])}
                           for span in document[0].get_texttrace() for char in span["chars"]]
        warnings = mupdf.TOOLS.mupdf_warnings(reset=True)
        geometry_match = expected_origins is None or (
            len(origins) == len(expected_origins) and all(
                actual["text"] == reference["text"] and all(
                    abs(a - b) <= 0.001 for a, b in zip(actual["origin"], reference["origin"]))
                for actual, reference in zip(origins, expected_origins)))
        return {"reader": "mupdf", "pages": pages, "warnings": warnings,
                "geometry": {"actual": origins, "expected": expected_origins,
                             "match": geometry_match if expected_origins is not None else None, "tolerance_points": 0.001},
                "status": "match" if pages == [expected + "\n"] and not warnings and geometry_match else "mismatch"}
    except Exception as error:
        return {"reader": "mupdf", "status": "error", "error": str(error),
                "warnings": mupdf.TOOLS.mupdf_warnings(reset=True)}


def validate(manifest, root):
    root = root.resolve()
    versions, errors = environment()
    report = {"schema_version": 1, "versions": versions, "environment_errors": errors,
              "accepted": False, "cases": []}
    if errors:
        return report
    if manifest.get("schema_version") != 1 or not manifest.get("cases"):
        report["environment_errors"].append("empty or unsupported manifest")
        return report
    ids = [case["id"] for case in manifest["cases"]]
    if len(ids) != len(set(ids)):
        report["environment_errors"].append("duplicate case IDs")
        return report
    for case in manifest["cases"]:
        row = {"id": case["id"], "path": case["path"], "expected_text": case["expected_text"],
               "status": "fixture_error", "readers": []}
        report["cases"].append(row)
        path = (root / case["path"]).resolve()
        try:
            if not path.is_relative_to(root):
                raise ValueError("fixture path outside repository")
            if not isinstance(case["expected_text"], str) or not case.get("expectation_source"):
                raise ValueError("missing independent expectation")
            origins = case.get("expected_trace_origins")
            if origins is not None and (not isinstance(origins, list) or not origins or any(
                    not isinstance(item, dict) or not isinstance(item.get("text"), str)
                    or not isinstance(item.get("origin"), list) or len(item["origin"]) != 2
                    or any(not isinstance(value, (int, float)) or not math.isfinite(value)
                           for value in item["origin"])
                    for item in origins)):
                raise ValueError("invalid explicit geometry expectation")
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            if digest != case["sha256"]:
                raise ValueError(f"fixture hash changed: {digest}")
        except (OSError, ValueError) as error:
            row["error"] = str(error)
            continue
        row["sha256"] = digest
        row["readers"].append(mupdf_result(path, case["expected_text"], case.get("expected_trace_origins")))
        poppler = command(["pdftotext", "-raw", str(path), "-"])
        status = classify_poppler(case, versions["poppler"], poppler)
        row["readers"].append({"reader": "poppler", "status": status, **poppler,
                               "known_limit_reason": case.get("known_poppler", {}).get("reason") if status == "known_limitation" else None})
        structural = command(["qpdf", "--check", str(path)])
        row["readers"].append({"reader": "qpdf", "status": "structure_ok" if structural["exit_code"] == 0 else "error", **structural})
        row["status"] = "accepted" if all(r["status"] in {"match", "known_limitation", "structure_ok"} for r in row["readers"]) else "failed"
    report["accepted"] = all(row["status"] == "accepted" for row in report["cases"])
    report["summary"] = {status: sum(r["status"] == status for c in report["cases"] for r in c["readers"])
                         for status in ["match", "known_limitation", "mismatch", "error", "structure_ok"]}
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=Path(__file__).with_name("cases.json"))
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        manifest = json.loads(args.manifest.read_text())
        report = validate(manifest, args.root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        report = {"accepted": False, "configuration_error": str(error)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({"accepted": report["accepted"], "summary": report.get("summary"), "report": str(args.output)}))
    return 0 if report["accepted"] else 1


if __name__ == "__main__":
    sys.exit(main())
