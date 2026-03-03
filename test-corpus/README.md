# Test Corpus — oxidize-pdf (7K PDFs)

Comprehensive test corpus organized into 7 tiers (T0–T6) covering regression,
spec compliance, real-world diversity, stress testing, AI/RAG accuracy,
quality benchmarking, and adversarial safety.

## Quick Start

```bash
# Download T1 (spec compliance — ~500 MB)
bash test-corpus/t1-spec/download.sh

# Run T0 + T1 (every commit)
cargo test --test t0_regression --test t1_spec

# Run all tiers that have data
cargo test --test t0_regression --test t1_spec --test t2_realworld \
    --test t3_stress --test t4_ai_target --test t5_quality --test t6_adversarial
```

## Tier Overview

| Tier | PDFs | Trigger | Source | Purpose |
|------|------|---------|--------|---------|
| T0 | 749 | Every commit | Production fixtures | Regression prevention |
| T1 | ~2,000 | Every commit | veraPDF + pdf.js | Spec conformance |
| T2 | 2,000 | Nightly | GovDocs1 | Generator diversity |
| T3 | 750 | Nightly | SafeDocs (curated) | Error recovery |
| T4 | 500 | Weekly | PubMed Central OA | AI/RAG accuracy |
| T5 | ~900 | Weekly | OmniDocBench | Quality benchmarking |
| T6 | 200 | Weekly | Qiqqa + SafeDocs | Adversarial safety |

## Setup

Each tier has a `download.sh` script. Some corpora require manual curation.

- **T0**: Uses existing fixtures (no download needed)
- **T1**: `bash test-corpus/t1-spec/download.sh` (auto)
- **T2**: `bash test-corpus/t2-realworld/download.sh` (auto)
- **T3**: Manual curation from SafeDocs
- **T4**: Manual selection from PubMed Central
- **T5**: `bash test-corpus/t5-quality/download.sh` (auto)
- **T6**: Manual curation

## Tests gracefully skip when corpus is absent

All tier tests detect whether their corpus is available and skip with an
informational message when it's not. This means:

- CI works even without the corpus (tests pass, just do less)
- Local development doesn't require downloading 7K PDFs
- Only the tiers you need are exercised

## Results

Test results are saved to `test-corpus/results/YYYY-MM-DD/` as JSON files.
A `latest` symlink points to the most recent run. Results are gitignored.
