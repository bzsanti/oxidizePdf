# oxidize-pdf vs lopdf Benchmark Suite

Comprehensive performance and feature comparison between oxidize-pdf and lopdf.

## 📊 Benchmark Categories

### 1. Creation Performance
Tests PDF generation speed for different complexity levels:
- **Simple**: Text-only documents (1,000 pages)
- **Medium**: Text + tables + charts (1,000 pages)
- **High**: Complex graphics with gradients (1,000 pages)

**Metrics**: pages/second, file size, duration

### 2. Parsing Performance
Tests PDF parsing/loading speed on real-world PDFs:
- Uses PDFs from `test_pdfs/` directory
- Measures success rate and speed

**Metrics**: PDFs/second, success rate, duration

### 3. Compression Performance
Compares modern PDF compression features:
- **Legacy Mode**: PDF 1.4 (no object streams)
- **Modern Mode**: PDF 1.5+ with Object Streams

**Metrics**: file size reduction, compression ratio

## 🚀 Running Benchmarks

### Quick Start (All Benchmarks)
```bash
cargo run --release --bin run_all --manifest-path benches/lopdf_comparison/Cargo.toml
```

### Individual Benchmarks
```bash
# Creation benchmark
cargo run --release --bin benchmark_creation --manifest-path benches/lopdf_comparison/Cargo.toml

# Parsing benchmark (requires PDFs in test_pdfs/)
cargo run --release --bin benchmark_parsing --manifest-path benches/lopdf_comparison/Cargo.toml

# Compression benchmark
cargo run --release --bin benchmark_compression --manifest-path benches/lopdf_comparison/Cargo.toml
```

## 📁 Directory Structure

```
benches/lopdf_comparison/
├── Cargo.toml              # Benchmark project config
├── README.md               # This file
├── src/
│   ├── benchmark_creation.rs     # PDF creation benchmarks
│   ├── benchmark_parsing.rs      # PDF parsing benchmarks
│   ├── benchmark_compression.rs  # Compression benchmarks
│   └── run_all.rs               # Run all benchmarks
├── test_pdfs/              # Test PDFs for parsing (optional)
└── results/                # Benchmark results (generated)
    ├── creation_benchmark.json
    ├── parsing_benchmark.json
    ├── compression_benchmark.json
    ├── BENCHMARK_REPORT.md
    └── *.pdf (generated test files)
```

## 📋 Adding Test PDFs for Parsing

To test parsing performance:

1. Create the directory:
   ```bash
   mkdir -p benches/lopdf_comparison/test_pdfs
   ```

2. Add PDF files (any source):
   ```bash
   cp path/to/pdfs/*.pdf benches/lopdf_comparison/test_pdfs/
   ```

3. Run parsing benchmark:
   ```bash
   cargo run --release --bin benchmark_parsing --manifest-path benches/lopdf_comparison/Cargo.toml
   ```

## 📊 Results

Results are saved in JSON format for programmatic analysis and as a Markdown report for human reading:

- **JSON files**: Machine-readable detailed results
- **BENCHMARK_REPORT.md**: Summary tables and comparisons
- **Generated PDFs**: Sample outputs for visual inspection

## 🎯 Interpreting Results

### Speed Comparisons
- **pages/second**: Higher is better (creation)
- **PDFs/second**: Higher is better (parsing)

### File Size Comparisons
- **Smaller is better** for same content
- **Modern mode** should be smaller than legacy
- Compare compression ratios between libraries

### Success Rate
- **Parsing**: Percentage of PDFs successfully loaded
- Higher success rate indicates better compatibility

## 🔧 Configuration

Edit the benchmarks to adjust:
- `NUM_PAGES`: Number of pages to generate (default: 1,000)
- Content complexity (modify generation functions)
- Compression settings

## 📝 Notes

- All benchmarks run in `--release` mode for accurate performance
- Results may vary based on system specs
- File sizes depend on content complexity
- lopdf 0.37 supports modern PDF features (Object Streams)

## 🐛 Troubleshooting

**Benchmark fails to compile:**
```bash
cd benches/lopdf_comparison
cargo clean
cargo build --release
```

**No parsing results:**
- Add PDFs to `test_pdfs/` directory
- Parsing benchmark requires test files

**Unexpected results:**
- Check Rust version (1.77+ required)
- Ensure `--release` mode is used
- Verify both libraries are latest versions

## 📚 References

- [oxidize-pdf Documentation](https://docs.oxidizepdf.dev)
- [lopdf Crate](https://crates.io/crates/lopdf)
- [PDF 1.7 Specification](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf)
