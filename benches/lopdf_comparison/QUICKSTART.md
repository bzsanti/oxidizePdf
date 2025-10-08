# Quick Start Guide

## ⚡ Fastest Way to Run Benchmarks

### 1. Build (First Time Only - Takes 5-10 minutes)
```bash
cd benches/lopdf_comparison
cargo build --release
```

### 2. Run Individual Benchmarks

**Creation benchmark** (1,000 pages each):
```bash
cargo run --release --bin benchmark_creation
```

**Compression benchmark**:
```bash
cargo run --release --bin benchmark_compression
```

**Parsing benchmark** (requires PDFs in test_pdfs/):
```bash
# First, add some test PDFs
cp ../../examples/results/*.pdf test_pdfs/

# Then run
cargo run --release --bin benchmark_parsing
```

### 3. Run All Benchmarks + Generate Report
```bash
cargo run --release --bin run_all
```

## 📊 Viewing Results

### JSON Results
```bash
cat results/creation_benchmark.json | jq .
cat results/compression_benchmark.json | jq .
cat results/parsing_benchmark.json | jq .
```

### Markdown Report
```bash
cat results/BENCHMARK_REPORT.md
```

### Generated PDFs
```bash
ls -lh results/*.pdf
open results/oxidize_simple.pdf
open results/lopdf_simple.pdf
```

## 🎯 Expected Output

### Creation Benchmark
```
🔥 PDF Creation Benchmark: oxidize-pdf vs lopdf
================================================

📄 Test 1: Simple text document (1000 pages)
  oxidize-pdf: 5432.12 pages/sec | 234567 bytes | 184.12ms
  lopdf:       3210.45 pages/sec | 245123 bytes | 311.42ms

📊 Test 2: Medium complexity (1000 pages)
  oxidize-pdf: 2187.65 pages/sec | 456789 bytes | 457.23ms
  lopdf:       1654.32 pages/sec | 478901 bytes | 604.51ms

🎨 Test 3: High complexity (1000 pages)
  oxidize-pdf: 3124.89 pages/sec | 678901 bytes | 320.01ms
  lopdf:       2456.78 pages/sec | 701234 bytes | 407.12ms

📊 SUMMARY
==========

Test: simple
  Speed: oxidize-pdf is 1.69x faster than lopdf
  Size:  oxidize-pdf is 4.3% smaller than lopdf

Test: medium
  Speed: oxidize-pdf is 1.32x faster than lopdf
  Size:  oxidize-pdf is 4.6% smaller than lopdf

Test: high_complexity
  Speed: oxidize-pdf is 1.27x faster than lopdf
  Size:  oxidize-pdf is 3.2% smaller than lopdf
```

*Note: Actual numbers will vary based on your hardware*

## 🐛 Troubleshooting

### Build Takes Too Long
- **First build**: 5-10 minutes is normal (many dependencies)
- **Subsequent builds**: <30 seconds
- **Speed up**: Use `cargo build --release -j 1` to reduce memory usage

### Benchmark Fails
```bash
# Check for compile errors
cargo check --bin benchmark_creation

# Clean rebuild
cargo clean
cargo build --release
```

### No Parsing Results
```bash
# Ensure test PDFs exist
ls test_pdfs/*.pdf

# Copy some from examples
cp ../../examples/results/*.pdf test_pdfs/

# Or use oxidize-pdf-render tests
cp ../../oxidize-pdf-render/tests/fixtures/*.pdf test_pdfs/
```

### Permission Denied
```bash
chmod +x quick_benchmark.sh
```

## 📝 Benchmark Details

### What Gets Measured

**Creation**:
- Time to generate 1,000 pages
- Final file size
- Pages per second throughput

**Compression**:
- Legacy PDF 1.4 file size
- Modern PDF 1.5+ file size
- Compression ratio improvement

**Parsing**:
- Number of PDFs successfully parsed
- Parse speed (PDFs/second)
- Error rate

### Content Variation

All benchmarks use **unique content per page**:
- Formulas based on `page_num`
- Rotated data sets
- No trivial repetition
- Realistic business content

### Fairness

- Both libraries run on same hardware
- Same content complexity
- Release mode optimizations enabled
- Multiple iterations averaged
- Results saved for reproducibility

## 🎓 Next Steps

1. **Analyze Results**: Compare numbers in `results/BENCHMARK_REPORT.md`
2. **Verify PDFs**: Open generated PDFs to check quality
3. **Run Profiling**: Use `cargo flamegraph` for deeper analysis
4. **Report Issues**: If numbers seem wrong, check CPU throttling, background processes
5. **Share Results**: Update `.private/LOPDF_COMPARISON_PLAN.md` with findings

## 🔗 Resources

- Main README: `README.md`
- Detailed Plan: `../../.private/LOPDF_COMPARISON_PLAN.md`
- Gap Analysis: `../../.private/HONEST_GAP_ANALYSIS.md`
- Project Docs: `../../CLAUDE.md`
