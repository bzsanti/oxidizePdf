# oxidize-pdf Performance Benchmarks

This directory contains comprehensive benchmarking tools for validating oxidize-pdf performance claims against other PDF libraries.

## 📊 Benchmark Results Summary

**Last Updated**: August 31, 2025

### oxidize-pdf Performance
- **Measured**: 9.0 pages/second
- **Claimed**: 215 pages/second  
- **Status**: ❌ Claims NOT validated (95.8% slower than claimed)

## 🛠️ Available Tools

### 1. Quick Performance Check
```bash
python3 quick_oxidize_benchmark.py
```
- Measures actual oxidize-pdf performance
- Runs multiple iterations for accuracy
- Compares with documented claims
- **Output**: JSON report with detailed metrics

### 2. Cross-Language PDF Library Comparison
```bash
python3 python_pdf_benchmark.py
```
- Benchmarks Python PDF libraries (ReportLab, PyMuPDF, pypdf)
- Measures generation time, file size, memory usage
- Supports multiple page counts (1, 10, 50, 100)
- **Output**: Comparative analysis with tables

### 3. Rust Library Comparison
```bash
cd tools/benchmarks && cargo bench
```
- Compares oxidize-pdf vs lopdf vs printpdf
- Uses Criterion.rs for statistical analysis
- **Status**: ⚠️ Compilation errors need fixing

## 📁 Test Cases

### `test_cases/simple_text.json`
Basic PDF with text-only content for baseline performance testing.

### `test_cases/complex_report.json`  
Complex PDF with tables, graphics, and mixed content.

### `test_cases/multi_page.json`
Template for testing scalability with varying page counts.

## 🔍 Usage Examples

### Run Quick Benchmark
```bash
cd tools/benchmarks
python3 quick_oxidize_benchmark.py
```

Expected output:
```
Quick oxidize-pdf Performance Benchmark
==================================================
--- Testing font_spacing_test ---
✅ Build successful (360ms)
📊 Average: 111ms, 1.4KB, 9.0 pages/sec

PERFORMANCE SUMMARY
==================================================  
Tests run: 2
Average pages/second: 9.0
❌ PERFORMANCE CLAIMS NOT VALIDATED
```

### Run Python Library Comparison
```bash
python3 python_pdf_benchmark.py
```

This will:
1. Install required dependencies automatically
2. Benchmark ReportLab, PyMuPDF, pypdf
3. Generate comparative report
4. Show performance table

## 📋 Benchmark Methodology

### Measurement Approach
1. **Multiple Iterations**: 3 runs per test for statistical accuracy
2. **Release Builds**: All Rust code compiled with `--release`
3. **Real File I/O**: Measures actual PDF generation including disk writes
4. **Clean Runs**: Each iteration starts fresh (no caching effects)

### Metrics Collected
- **Execution Time**: Wall-clock time for PDF generation
- **File Size**: Resulting PDF file size in bytes
- **Pages/Second**: Calculated throughput metric
- **Success Rate**: Percentage of successful generations
- **Memory Usage**: Where supported by the platform

### Test Environment
- **OS**: macOS (Darwin 24.6.0)
- **Hardware**: Standard development machine
- **Rust**: 1.85+ with 2024 edition features
- **Python**: 3.9+ with pip package management

## 🚨 Current Issues

### 1. Performance Claims Overstated
- **Claimed**: 215 pages/second
- **Measured**: 9.0 pages/second
- **Gap**: 23.9x slower than claimed
- **Action Required**: Update all documentation

### 2. Rust Comparison Incomplete
- Benchmark for lopdf/printpdf comparison has compilation errors
- Need to fix imports and API usage
- Cross-compilation issues in workspace setup

### 3. Missing Baselines
- No historical performance data
- No regression testing in CI/CD
- No automated benchmark runs

## 🔧 Fixes Needed

### High Priority
1. **Update performance claims** in all documentation
2. **Fix Rust benchmark compilation** errors
3. **Add benchmark to CI/CD** pipeline

### Medium Priority  
1. Create memory usage profiling
2. Add benchmark visualization (charts/graphs)
3. Expand test case coverage

### Low Priority
1. Add network latency simulation
2. Create performance regression alerts
3. Cross-platform benchmark validation

## 📈 Expected vs Actual Performance

| Library | Claimed | Measured | Gap | Status |
|---------|---------|----------|-----|--------|
| oxidize-pdf | 215 pages/sec | 9.0 pages/sec | -95.8% | ❌ |
| lopdf | Not tested | Not tested | N/A | 🔄 |
| printpdf | Not tested | Not tested | N/A | 🔄 |
| PyMuPDF | ~10x faster | Not tested | N/A | 🔄 |
| ReportLab | Industry standard | Not tested | N/A | 🔄 |

## 🎯 Honest Performance Assessment

### What oxidize-pdf Does Well
✅ **Reliability**: 100% success rate in tests  
✅ **Correctness**: Generated PDFs are valid and render properly  
✅ **Features**: Rich feature set (tables, charts, forms)  
✅ **File Sizes**: Reasonable output sizes  

### Performance Reality
- **9 pages/second** is respectable for complex documents
- Performance is likely **comparable** to other Rust PDF libraries
- Focus should be on **features and correctness**, not speed
- **"Extreme performance"** claims are misleading

## 📞 Running Benchmarks

### Prerequisites
```bash
# For Python benchmarks
pip install reportlab pypdf PyMuPDF matplotlib tabulate

# For Rust benchmarks (when fixed)
cargo install criterion
```

### Full Benchmark Suite
```bash
# Quick oxidize-pdf validation
python3 quick_oxidize_benchmark.py

# Cross-language comparison  
python3 python_pdf_benchmark.py

# Rust library comparison (needs fixing)
cargo bench

# View results
ls -la *.json
cat docs/PERFORMANCE_HONEST_REPORT.md
```

## 📝 Contributing

When adding new benchmarks:

1. Follow the established JSON test case format
2. Include multiple iterations for statistical validity  
3. Measure both time and file size
4. Document any platform-specific considerations
5. Update this README with new findings

## 🎖️ Acknowledgments

This benchmarking suite was created to provide **honest, transparent performance validation** for oxidize-pdf. The goal is to replace marketing claims with real, measurable data that developers can trust.

**Honesty over hype. Data over claims. Reliability over marketing.**