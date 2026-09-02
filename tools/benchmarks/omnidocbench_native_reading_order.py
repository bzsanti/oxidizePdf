#!/usr/bin/env python3
"""Aggregate official OmniDocBench reading-order scores for native-text pages."""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path
from typing import Any


EXCLUDED_DATA_SOURCES = frozenset({"note"})
EXCLUDED_SPECIAL_ISSUES = frozenset({"fuzzy_scan"})
DIMENSIONS = ("data_source", "layout", "language")


def _load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


def aggregate_native_scores(
    dataset: list[dict[str, Any]], scores: dict[str, float]
) -> dict[str, Any]:
    """Return native-only averages from an official per-page score artifact."""
    metadata = {}
    for page in dataset:
        name = Path(page["page_info"]["image_path"]).name
        if name in metadata:
            raise ValueError(f"duplicate dataset page basename: {name}")
        metadata[name] = page["page_info"]["page_attribute"]
    missing = sorted(set(scores) - set(metadata))
    if missing:
        raise ValueError(f"scores reference {len(missing)} unknown pages; first: {missing[0]}")

    selected: list[tuple[str, float, dict[str, Any]]] = []
    excluded = 0
    for name, score in scores.items():
        if (
            not isinstance(score, (int, float))
            or isinstance(score, bool)
            or not math.isfinite(score)
            or not 0.0 <= score <= 1.0
        ):
            raise ValueError(f"invalid edit-distance score for {name}: {score!r}")
        attributes = metadata[name]
        special_issues = set(attributes.get("special_issue", []))
        if (
            attributes.get("data_source") in EXCLUDED_DATA_SOURCES
            or special_issues & EXCLUDED_SPECIAL_ISSUES
        ):
            excluded += 1
            continue
        selected.append((name, float(score), attributes))

    if not selected:
        raise ValueError("no native-text pages remain after filtering")

    categories: dict[str, dict[str, dict[str, float | int]]] = {}
    for dimension in DIMENSIONS:
        groups: dict[str, list[float]] = defaultdict(list)
        for _, score, attributes in selected:
            groups[str(attributes.get(dimension, "unknown"))].append(score)
        categories[dimension] = {
            label: {"pages": len(values), "edit_distance": sum(values) / len(values)}
            for label, values in sorted(groups.items())
        }

    values = [score for _, score, _ in selected]
    return {
        "protocol": "OmniDocBench end2end_eval/quick_match per-page reading-order Edit_dist",
        "filter": {
            "excluded_data_sources": sorted(EXCLUDED_DATA_SOURCES),
            "excluded_special_issues": sorted(EXCLUDED_SPECIAL_ISSUES),
        },
        "official_scored_pages": len(scores),
        "excluded_pages": excluded,
        "native_pages": len(values),
        "native_edit_distance": sum(values) / len(values),
        "categories": categories,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dataset", type=Path, help="Pinned OmniDocBench.json")
    parser.add_argument("scores", type=Path, help="Official reading_order_per_page_edit.json")
    parser.add_argument("--output", type=Path, help="Write JSON here instead of stdout")
    args = parser.parse_args()

    result = aggregate_native_scores(_load_json(args.dataset), _load_json(args.scores))
    rendered = json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
