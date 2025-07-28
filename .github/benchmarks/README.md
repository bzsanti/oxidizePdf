# Benchmark System

This directory contains the baseline benchmark results used for performance regression detection.

## How it works

1. **Creating a baseline**: Run the "Create Benchmark Baseline" workflow from GitHub Actions
2. **Weekly comparison**: The benchmark workflow runs every Sunday and compares against the baseline
3. **Performance monitoring**: Any regression >10% will be flagged with a warning

## Manual operations

### Create a new baseline
1. Go to Actions tab
2. Select "Create Benchmark Baseline" workflow
3. Click "Run workflow"
4. Select the branch (default: main)
5. The baseline will be committed to this directory

### Run benchmarks manually
1. Go to Actions tab
2. Select "Benchmarks" workflow
3. Click "Run workflow"
4. Optionally check "Create baseline" to update the baseline

## Benchmark suites

- `core_benchmarks`: Core PDF operations
- `parser_bench`: PDF parsing performance
- `memory_benchmarks`: Memory allocation patterns
- `cli_benchmarks`: CLI command performance

## Performance tracking

Results are stored in Criterion format and can be analyzed using tools like:
- `critcmp`: Compare benchmark runs
- `criterion-table`: Generate comparison tables
- GitHub Actions artifacts: Download and analyze locally