#!/bin/bash
#
# Verify that all examples compile and that the RAG showcase examples produce
# non-trivial output. This is the gate that prevents a broken or empty example
# from slipping through — CI runs `cargo build --all`, which does NOT build
# examples (they are not default targets), so a broken example is invisible
# without this check.
#
# Usage:
#   ./scripts/verify-examples.sh            # compile gate (always)
#   ./scripts/verify-examples.sh --run      # also run the RAG showcase if the
#                                           # corpus cache is present (no network)
#
# Exit codes:
#   0  all checks passed
#   1  an example failed to compile, or showcase output was trivial/empty

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CORE_DIR="$PROJECT_ROOT/oxidize-pdf-core"

RUN_EXAMPLES=false
if [[ "${1:-}" == "--run" ]]; then
    RUN_EXAMPLES=true
fi

cd "$PROJECT_ROOT"

# 1. Compile gate: every example must build, with all features.
echo -e "${YELLOW}==> Compiling all examples (--examples --all-features)${NC}"
cargo check --examples --all-features --locked
echo -e "${GREEN}    all examples compile${NC}"

# 2. Output gate: run the RAG showcase only if the corpus is already cached
#    (no network in CI). Assert the output is non-trivial: every document must
#    produce at least one chunk and at least one non-empty chunk line.
if [[ "$RUN_EXAMPLES" == true ]]; then
    if [[ -d "$CORE_DIR/corpus_cache" ]] && \
       [[ -n "$(ls -A "$CORE_DIR/corpus_cache" 2>/dev/null)" ]]; then
        echo -e "${YELLOW}==> Running rag_realworld against cached corpus${NC}"
        ( cd "$CORE_DIR" && cargo run --quiet --example rag_realworld )

        echo -e "${YELLOW}==> Asserting non-trivial JSONL output${NC}"
        OUT_DIR="$CORE_DIR/out"
        if [[ ! -d "$OUT_DIR" ]]; then
            echo -e "${RED}    FAIL: no out/ directory produced${NC}"
            exit 1
        fi
        fail=0
        for f in "$OUT_DIR"/*.jsonl; do
            [[ -e "$f" ]] || { echo -e "${RED}    FAIL: no jsonl files${NC}"; exit 1; }
            lines=$(wc -l < "$f")
            # at least one line whose "text" field is non-empty
            nonempty=$(grep -c '"text": *"[^"]' "$f" || true)
            if [[ "$lines" -lt 1 ]] || [[ "$nonempty" -lt 1 ]]; then
                echo -e "${RED}    FAIL: $(basename "$f") trivial ($lines lines, $nonempty non-empty)${NC}"
                fail=1
            else
                echo -e "${GREEN}    OK: $(basename "$f") — $lines chunks${NC}"
            fi
        done
        [[ "$fail" -eq 0 ]] || exit 1
    else
        echo -e "${YELLOW}==> Skipping run gate: corpus_cache not present (no network in CI)${NC}"
    fi
fi

echo -e "${GREEN}==> verify-examples: all checks passed${NC}"
