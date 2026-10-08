"""Independently validate #690 artifacts without recording document contents."""
import argparse
import json
from pathlib import Path
import subprocess


def run(args):
    return subprocess.run(args, capture_output=True, check=False)


def page_text(path, page):
    result = run(["pdftotext", "-f", str(page), "-l", str(page), str(path), "-"])
    assert result.returncode == 0, result.stderr.decode(errors="replace")
    return result.stdout


def page_pixels(path, page):
    result = run(["pdftoppm", "-r", "72", "-singlefile", "-f", str(page), "-l", str(page), str(path)])
    assert result.returncode == 0, result.stderr.decode(errors="replace")
    return result.stdout


def compare(source, prepared, parts):
    assert prepared.read_bytes().startswith(source.read_bytes())
    count = 0
    for part, first, last in zip(parts, [1, 4, 7], [3, 6, 12]):
        assert part.read_bytes().startswith(source.read_bytes())
        for page in range(first, last + 1):
            local = page - first + 1
            text = page_text(source, page)
            pixels = page_pixels(source, page)
            assert text == page_text(prepared, page) == page_text(part, local), f"text page {page}"
            assert pixels == page_pixels(prepared, page) == page_pixels(part, local), f"pixels page {page}"
            count += 1
    return {"pages": count, "text_identical": True, "pixels_identical_72dpi": True, "prefixes_preserved": True}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("synthetic", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--original", type=Path)
    args = parser.parse_args()
    files = sorted(p for p in args.synthetic.glob("*.pdf") if not p.name.endswith("-source.pdf"))
    assert files, "No synthetic artifacts"
    if args.original:
        files += [args.original / "prepared.pdf"] + [args.original / f"part{i}.pdf" for i in range(3)]
    checks = []
    for path in files:
        result = run(["qpdf", "--check", str(path)])
        checks.append({"file": str(path), "exit": result.returncode, "stderr": result.stderr.decode(errors="replace")})
    comparisons = {}
    for kind in ["direct", "prepared"]:
        comparisons[kind] = compare(args.synthetic / "nested-source.pdf", args.synthetic / "nested-prepared.pdf", [args.synthetic / f"nested-{kind}-part{i}.pdf" for i in range(3)])
    if args.original:
        comparisons["original"] = compare(args.original / "source.pdf", args.original / "prepared.pdf", [args.original / f"part{i}.pdf" for i in range(3)])
    result = {"qpdf_version": run(["qpdf", "--version"]).stdout.decode().splitlines()[0], "poppler_version": run(["pdftotext", "-v"]).stderr.decode().splitlines()[0], "qpdf": checks, "comparisons": comparisons}
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    assert all(item["exit"] == 0 for item in checks), "qpdf warnings or errors"
    print(json.dumps({"qpdf_clean": len(checks), "comparisons": comparisons}))


if __name__ == "__main__":
    main()
