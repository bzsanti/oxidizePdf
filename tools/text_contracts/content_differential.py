#!/usr/bin/env python3
"""Versioned per-page content signal; Poppler is a comparator, not ground truth.

Linux measurement runner (GNU timeout + resource limits), Python stdlib only.
Content = Unicode scalars excluding isspace(); case, punctuation, combining
marks and U+FFFD are preserved. SequenceMatcher, autojunk=False, gives anchored
edits, NOT minimum Levenshtein distance. Reorders can appear as deletions and
insertions. Multiset deficits are also reported as an order-independent lower
bound, not interpreted as proven missing PDF glyphs. Whitespace counts stay
separate. No historical baseline is read, written or recalibrated.
"""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from difflib import SequenceMatcher
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

SCHEMA = "anchored-content-v1"
MAX_SCALARS = 100_000
MAX_OUTPUT = 64 * 1024 * 1024


def sha256(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def measure(reference, candidate):
    ref = "".join(c for c in reference if not c.isspace())
    got = "".join(c for c in candidate if not c.isspace())
    if max(len(ref), len(got)) > MAX_SCALARS:
        raise ValueError("page exceeds 100000 content scalars")
    counts = dict(reference_scalars=len(ref), candidate_scalars=len(got),
                  reference_whitespace=len(reference) - len(ref),
                  candidate_whitespace=len(candidate) - len(got),
                  reference_replacements=ref.count("\ufffd"),
                  candidate_replacements=got.count("\ufffd"),
                  matched=0, substitutions=0, deletions=0, insertions=0,
                  multiset_deficit=sum((Counter(ref) - Counter(got)).values()),
                  multiset_excess=sum((Counter(got) - Counter(ref)).values()))
    diagnostics = []
    for tag, a, b, c, d in SequenceMatcher(None, ref, got, autojunk=False).get_opcodes():
        if tag == "equal":
            counts["matched"] += b - a
            continue
        substitutions = min(b - a, d - c) if tag == "replace" else 0
        counts["substitutions"] += substitutions
        counts["deletions"] += b - a - substitutions
        counts["insertions"] += d - c - substitutions
        if len(diagnostics) < 32:
            diagnostics.append(dict(kind=tag, reference_range=[a, b],
                                    candidate_range=[c, d],
                                    reference_preview=ref[a:min(b, a + 48)],
                                    candidate_preview=got[c:min(d, c + 48)]))
    assert counts["matched"] + counts["substitutions"] + counts["deletions"] == len(ref)
    assert counts["matched"] + counts["substitutions"] + counts["insertions"] == len(got)
    return {"counts": counts, "edits_preview": diagnostics}


def split_poppler_pages(text):
    # pdftotext emits one structural form-feed after every page, including blank
    # pages. Reject missing termination rather than miscounting or trimming text.
    if not text.endswith("\f"):
        raise ValueError("Poppler output lacks terminal page delimiter")
    return text[:-1].split("\f")


def validate_export(text):
    records = [json.loads(line) for line in text.splitlines()]
    if not records or records[0].get("kind") != "document":
        raise ValueError("missing document record")
    count = records[0]["pages"]
    if type(count) is not int or not 0 <= count <= 10_000 or len(records) != count + 1:
        raise ValueError("incomplete page export")
    pages = records[1:]
    for index, record in enumerate(pages):
        if record.get("kind") != "page" or record.get("page") != index:
            raise ValueError("duplicate or out-of-order page")
        if ("text" in record) == ("error" in record):
            raise ValueError("page requires exactly one of text/error")
        value = record.get("text", record.get("error"))
        if not isinstance(value, str):
            raise ValueError("page payload is not text")
    return pages


def compare_pages(pages, reference):
    if len(pages) != len(reference):
        # A missing/extra delimiter may be anywhere, not merely at the end.
        # There is no independent page identity with which to realign safely.
        return [dict(page=index, status="page_count_mismatch",
                     error=f"candidate pages={len(pages)}, reference pages={len(reference)}")
                for index in range(max(len(pages), len(reference)))]
    results = []
    for index in range(max(len(pages), len(reference))):
        row = {"page": index}
        if "error" in pages[index]:
            row.update(status="extraction_error", error=pages[index]["error"])
        else:
            try:
                row.update(status="compared", **measure(reference[index], pages[index]["text"]))
            except ValueError as error:
                row.update(status="measurement_limit", error=str(error))
        results.append(row)
    return results


def summarize(rows):
    totals = Counter()
    statuses = Counter(row["status"] for row in rows)
    page_statuses = Counter()
    for row in rows:
        for page in row.get("pages", []):
            page_statuses[page["status"]] += 1
            if page["status"] == "compared":
                totals.update(page["counts"])
    denominator = totals["reference_scalars"]
    return dict(files=len(rows), file_statuses=dict(statuses),
                page_statuses=dict(page_statuses), comparable_page_totals=dict(totals),
                rates_over_comparable_reference={
                    key: totals[key] / denominator if denominator else None
                    for key in ("substitutions", "deletions", "insertions", "multiset_deficit")})


def freeze(root):
    files = sorted(p for p in root.rglob("*.pdf") if p.is_file())
    if not files:
        raise ValueError("empty PDF population")
    entries = []
    for path in files:
        path.resolve().relative_to(root.resolve())
        entries.append(dict(path=path.relative_to(root).as_posix(), sha256=sha256(path)))
    return dict(schema=SCHEMA, files=entries)


def checked_path(root, entry):
    relative = Path(entry["path"])
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError("manifest path escapes corpus")
    path = (root / relative).resolve()
    path.relative_to(root.resolve())
    if sha256(path) != entry["sha256"]:
        raise ValueError("input hash changed")
    return path


def command_text(args, work, timeout=20):
    # File-backed stdout/stderr avoids unbounded communicate() allocation. The
    # worker's RLIMIT_FSIZE caps each file; outer timeout kills its process group.
    with tempfile.TemporaryFile(dir=work) as out, tempfile.TemporaryFile(dir=work) as err:
        result = subprocess.run(args, stdout=out, stderr=err, timeout=timeout, check=False)
        err.seek(0)
        diagnostic = err.read(4096).decode("utf-8", errors="replace")
        if result.returncode:
            raise ValueError(f"command exit {result.returncode}: {diagnostic}")
        out.seek(0)
        data = out.read(MAX_OUTPUT + 1)
        if len(data) > MAX_OUTPUT:
            raise ValueError("output exceeds 64 MiB")
        return data.decode("utf-8"), diagnostic


def worker(pdf, probe, work):
    import resource
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_OUTPUT, MAX_OUTPUT))
    resource.setrlimit(resource.RLIMIT_AS, (2 * 1024**3, 2 * 1024**3))
    if pdf.stat().st_size > 128 * 1024**2:
        raise ValueError("PDF exceeds 128 MiB input limit")
    ours, ours_warnings = command_text([str(probe), str(pdf)], work)
    reference, warnings = command_text(
        ["pdftotext", "-enc", "UTF-8", "-eol", "unix", str(pdf), "-"], work)
    pages = compare_pages(validate_export(ours), split_poppler_pages(reference))
    return dict(status="complete" if all(p["status"] == "compared" for p in pages) else "partial",
                pages=pages, candidate_warnings=ours_warnings, reference_warnings=warnings)


def run_entry(entry, root, probe, work):
    row = dict(path=entry["path"], sha256=entry["sha256"])
    try:
        pdf = checked_path(root, entry)
    except (OSError, ValueError) as error:
        return dict(row, status="invalid_input", error=str(error))
    try:
        # GNU timeout bounds the aligner too, killing the entire worker group.
        text, _ = command_text(
            ["timeout", "-k", "1", "45", sys.executable, str(Path(__file__).resolve()),
             "worker", str(pdf), "--probe", str(probe), "--work", str(work)], work, timeout=50)
        row.update(json.loads(text))
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        row.update(status="excluded", error=str(error))
        row.pop("pages", None)
    try:
        checked_path(root, entry)
    except (OSError, ValueError) as error:
        row.update(status="invalid_input", error=str(error))
        row.pop("pages", None)
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["freeze", "run", "worker"])
    parser.add_argument("path", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--probe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--work", type=Path)
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()
    if args.action == "freeze":
        if args.manifest is None:
            parser.error("freeze needs --manifest")
        with args.manifest.open("x") as stream:
            json.dump(freeze(args.path), stream, indent=2)
            stream.write("\n")
    elif args.action == "worker":
        print(json.dumps(worker(args.path, args.probe, args.work), ensure_ascii=False))
    else:
        if any(value is None for value in (args.manifest, args.probe, args.output, args.work)):
            parser.error("run needs --manifest, --probe, --output and --work")
        if not 1 <= args.jobs <= 16:
            parser.error("--jobs must be in 1..16")
        manifest = json.loads(args.manifest.read_text())
        if manifest["schema"] != SCHEMA or not manifest["files"]:
            raise ValueError("unsupported or empty manifest")
        paths = [e["path"] for e in manifest["files"]]
        if len(set(paths)) != len(paths):
            raise ValueError("duplicate manifest paths")
        args.work.mkdir(parents=True, exist_ok=True)
        probe = args.probe.resolve()
        work = args.work.resolve()
        if args.output.with_suffix(".summary.json").exists():
            raise FileExistsError("summary evidence already exists")
        probe_hash = sha256(probe)
        runner_hash = sha256(Path(__file__))
        manifest_hash = sha256(args.manifest)
        # Refuse overwriting evidence or measuring a missing comparator.
        version = subprocess.run(["pdftotext", "-v"], capture_output=True, check=True)
        rows = []
        with args.output.open("x") as output, ThreadPoolExecutor(max_workers=args.jobs) as pool:
            for row in pool.map(lambda e: run_entry(e, args.path, probe, work), manifest["files"]):
                rows.append(row)
                output.write(json.dumps(row, ensure_ascii=False) + "\n")
                output.flush()
                if len(rows) % 100 == 0:
                    print(f"measured {len(rows)}/{len(paths)}", flush=True)
        invalid = any(row["status"] == "invalid_input" for row in rows)
        invalid |= probe_hash != sha256(probe) or runner_hash != sha256(Path(__file__))
        invalid |= manifest_hash != sha256(args.manifest)
        summary = dict(schema=SCHEMA, manifest_sha256=manifest_hash,
                       probe_sha256=probe_hash, runner_sha256=runner_hash,
                       inputs_unchanged=not invalid,
                       reference_version=(version.stdout + version.stderr).decode("utf-8"),
                       limits=dict(worker_seconds=45, command_seconds=20, content_scalars=MAX_SCALARS),
                       **summarize(rows))
        with args.output.with_suffix(".summary.json").open("x") as stream:
            json.dump(summary, stream, indent=2)
            stream.write("\n")
        print(json.dumps(summary, indent=2))
        if invalid or not summary["page_statuses"].get("compared"):
            raise SystemExit("measurement invalid: changed inputs or no comparable pages")


if __name__ == "__main__":
    main()
