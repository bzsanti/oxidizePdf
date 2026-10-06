"""Consolidate completed T4 checks; refuse unfinished or changed candidates."""
from pathlib import Path
import hashlib
import json
import re

p = Path(__file__).resolve().parent
assert json.loads((p / "stream-final-aggregate-exit.json").read_text())["exit"] == 0
assert json.loads((p / "stream-final-run-exit.json").read_text())["exit"] == 0
hashes = json.loads((p / "stream-final-hashes.json").read_text())
changed = [name for name, digest in hashes.items() if hashlib.sha256(Path(name).read_bytes()).hexdigest() != digest]
assert not changed, changed
log = (p / "stream-final-aggregate.log").read_text()
results = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", log)
assert len(results) == 62, len(results)
counts = [sum(int(row[i]) for row in results) for i in range(3)]
assert counts[1] == 0
names = ["flat_extraction_does_not_fuse_more_words_than_poppler", "flat_extraction_does_not_transpose_more_words_than_poppler", "reading_order_option_does_not_transpose_more_words_than_poppler"]
gates = {name: f"test {name} ... ok" in log for name in names}
assert all(gates.values())
corpus = json.loads((p / "stream-final-pages.summary.json").read_text())
assert corpus["inputs_unchanged"] and corpus["files"] == 1802

def pages(name):
    return {(row["path"], page["page"]): page for row in map(json.loads, (p / name).read_text().splitlines()) for page in row.get("pages", []) if page["status"] == "compared"}
before = pages("length-pages.jsonl")
after = pages("stream-final-pages.jsonl")
common = before.keys() & after.keys()
delta = [{"path": key[0], "page": key[1], "before": before[key]["counts"], "after": after[key]["counts"]} for key in sorted(common) if before[key]["counts"] != after[key]["counts"]]
comparison = dict(common_pages=len(common), only_before=len(before.keys()-after.keys()), only_after=len(after.keys()-before.keys()), changed=delta)
(p / "stream-final-common-pages.json").write_text(json.dumps(comparison, indent=2))
summary = dict(targets=len(results), passed=counts[0], failed=counts[1], ignored=counts[2], real_gates=gates, candidate_unchanged=True, corpus=corpus, comparison_to_length_fix=comparison)
(p / "stream-final-summary.json").write_text(json.dumps(summary, indent=2))
print(json.dumps(summary, indent=2))
