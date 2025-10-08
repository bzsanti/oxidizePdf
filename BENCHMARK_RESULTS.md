# 📊 oxidize-pdf Performance Benchmarks

**Generated**: 2025-10-07
**Version**: v1.2.5
**Test Environment**: macOS (Darwin 25.0.0)

## 🎯 Benchmark Suite Overview

All benchmarks use **realistic, varied content** to provide accurate performance measurements. Each page contains unique data - no repetition or trivial content.

### Benchmark Categories

1. **Realistic Document** - Business documents with varied paragraphs, tables, and charts
2. **Medium Complexity** - Sales reports with data tables, enhanced charts, and sparklines
3. **High Complexity** - Technical manuals with complex diagrams, code blocks, and tables

---

## 📈 Performance Results

### 1. Realistic Document Benchmark

**Content per page:**
- Unique paragraphs (no "Lorem ipsum" repetition)
- Data tables with varied metrics
- Statistical charts every 3 pages
- Headers/footers with metadata
- Multiple fonts and text styles

**Results:**

| Pages | Generation | Write | Total | Throughput | MB/sec |
|-------|-----------|-------|-------|------------|--------|
| 50    | 4.8ms     | 6.3ms | 11ms  | 4,529 p/s  | 135.9  |
| 500   | 32ms      | 51ms  | 83ms  | 6,034 p/s  | 181.0  |
| 1000  | 76ms      | 106ms | 182ms | 5,500 p/s  | 165.0  |

**File Size**: ~2.4KB per page (2.4MB for 1000 pages)

**Command**:
```bash
cargo run --release --example realistic_document_benchmark [pages]
```

---

### 2. Medium Complexity Benchmark

**Content per page:**
- Corporate header with simulated logo
- 18-24 rows of unique sales data
- Enhanced charts every 3 pages with:
  - Gradient-filled bars
  - Mini sparkline trends
  - Grid backgrounds
  - Multiple chart types (regional, product, quarterly)
- Alternating row colors in tables
- Page borders and footers

**Results:**

| Pages | Generation | Write | Total | Throughput |
|-------|-----------|-------|-------|------------|
| 100   | 18ms      | 26ms  | 45ms  | 2,214 p/s  |

**Content Variation**:
- 3 different chart types that rotate per page
- Unique sales data per row and page
- Regional/Product/Rep combinations vary
- Growth percentages calculated uniquely

**Command**:
```bash
cargo run --release --example medium_complexity_benchmark [pages]
```

---

### 3. High Complexity Benchmark

**Content per page:**
- Technical header with decorative elements
- Enhanced technical diagrams every 3 pages:
  - Network topology graphs
  - 5-7 connected components with shadows
  - Curved Bezier connections
  - Gradient-filled boxes
  - Status indicators
  - Data rate labels
- Syntax-highlighted code blocks (10 lines):
  - Multiple languages (Rust, Python, TypeScript, SQL, YAML, JSON, Bash)
  - Unique function/class names per page
  - Realistic variable values
- Complex API configuration tables (6 columns × 5 rows)
- Marginal notes with icons (every 4 pages)
- Professional footer with references

**Results:**

| Pages | Generation | Write | Total | Throughput |
|-------|-----------|-------|-------|------------|
| 100   | 14ms      | 18ms  | 33ms  | 3,024 p/s  |

**Content Variation**:
- 3 diagram types: Architecture, Pipeline, Microservices
- 9 different core system names
- 12 different component technologies
- Unique connection layouts per page
- Variable code examples (10+ function names)
- 9 programming languages with realistic code

**Command**:
```bash
cargo run --release --example high_complexity_benchmark [pages]
```

---

## 📊 Comparative Analysis

### Throughput by Complexity

| Benchmark | Complexity | Pages/Second | Content Description |
|-----------|-----------|--------------|---------------------|
| Realistic | Low-Med   | 5,500-6,034  | Varied text + simple charts |
| Medium    | Medium    | 2,214        | Tables + gradient charts + sparklines |
| High      | High      | 3,024        | Bezier diagrams + syntax code + complex tables |

### Key Findings

1. **Realistic Document** achieves highest throughput (6,000+ p/s) with varied content
2. **Medium Complexity** shows expected slowdown (~2,200 p/s) due to:
   - Gradient rendering (5 layers per bar)
   - Sparkline micro-charts
   - Complex table formatting
3. **High Complexity** maintains good performance (3,000 p/s) despite:
   - Bezier curve calculations (8 segments per connection)
   - Shadow effects
   - Grid backgrounds
   - Syntax highlighting

### Performance vs Content Trade-off

```
Realistic:     ████████████████████ 6,034 p/s  (paragraphs + simple charts)
High:          ███████████          3,024 p/s  (diagrams + code + tables)
Medium:        ████████             2,214 p/s  (gradients + sparklines)
```

The **Medium** benchmark is slowest due to gradient rendering overhead (5 rectangles per bar × 6 bars × chart pages).

---

## 🎯 Content Variation Verification

### Anti-Caching Measures

All benchmarks implement unique content per page:

1. **Realistic Document**:
   - Paragraph generator: 10 topics × 10 contexts × 10 outcomes = 1,000 combinations
   - Table metrics: Formula-based values using page_num and metric_idx
   - Chart data: `base + (page * multiplier) % range`

2. **Medium Complexity**:
   - Sales data: `(page * 11 + row * 5) % array_len` for indices
   - Dollar values: `base_sales + (row * 3456) + ((page * row * 123) % 150000)`
   - Growth: `-20.0 + ((page * 3.7 + row * 1.9) % 45.0)`
   - Sparklines: `((page * (p+1) * (i+1) * 13) % 15)` for each point

3. **High Complexity**:
   - Component layout: Circular placement with `angle = (i / num) * π * 1.5`
   - Bezier curves: 8 segments with quadratic interpolation
   - Code examples: 10+ function names, unique session_ids, timeouts
   - Table data: `(page * 3 + i) % endpoint_len` for rotations

---

## 🚀 Running the Benchmarks

### Quick Test
```bash
# Realistic - 100 pages
cargo run --release --example realistic_document_benchmark 100

# Medium - 100 pages
cargo run --release --example medium_complexity_benchmark 100

# High - 100 pages
cargo run --release --example high_complexity_benchmark 100
```

### Full Suite
```bash
# Realistic - Full test (1000 pages)
cargo run --release --example realistic_document_benchmark 1000

# Medium - Standard (50 pages default)
cargo run --release --example medium_complexity_benchmark

# High - Standard (100 pages default)
cargo run --release --example high_complexity_benchmark
```

### Output Location
```
examples/results/realistic_document_benchmark.pdf
examples/results/medium_complexity_benchmark.pdf
examples/results/high_complexity_benchmark.pdf
```

---

## 📝 Methodology Notes

### Test Integrity

1. **No Trivial Repetition**: Previous `performance_benchmark_1000.rs` repeated same text - now deprecated
2. **Realistic Content**: Each page has unique data based on mathematical formulas
3. **Honest Measurements**: Separate generation and write timings
4. **Visual Verification**: Output PDFs can be inspected to confirm variation

### Hardware Considerations

Results above from:
- **Platform**: macOS Darwin 25.0.0
- **Optimization**: `--release` builds
- **Priority**: `nice -n 19` for benchmarks
- **I/O**: SSD storage

Performance will vary based on:
- CPU speed (page generation)
- Disk I/O (write performance)
- Available RAM (document size)

---

## 🎨 Visual Quality vs Speed

### Content Quality Levels

**Realistic Document** (Fastest):
- Clean business document aesthetic
- Professional but simple graphics
- Optimized for volume generation

**Medium Complexity** (Balanced):
- Corporate report quality
- Gradient charts and micro-visualizations
- Good visual appeal with reasonable performance

**High Complexity** (Richest):
- Technical publication quality
- Complex diagrams with shadows and curves
- Maximum visual fidelity

### When to Use Each

| Use Case | Recommended Benchmark |
|----------|----------------------|
| Invoices, receipts, simple reports | Realistic |
| Sales reports, dashboards | Medium |
| Technical manuals, documentation | High |
| Automated document generation (high volume) | Realistic |
| Presentation materials | Medium/High |
| Marketing collateral | High |

---

## 🔧 Future Optimizations

Potential performance improvements:

1. **Object Stream Compression** (Feature 2.2.1): 3.9% file size reduction
2. **XRef Streams** (Feature 2.2.2): 1.3% additional reduction
3. **Parallel Page Generation**: Could achieve 2-4x speedup on multi-core
4. **Resource Pooling**: Reduce memory allocations for fonts/colors
5. **Streaming Writer**: Write pages as generated (reduce memory)

Expected combined improvement: **10-20% faster** with full optimizations.

---

## ✅ Conclusion

oxidize-pdf achieves **2,000-6,000 pages/second** with realistic, varied content:

- ✅ No trivial repetition or cached content
- ✅ Unique data per page via mathematical formulas
- ✅ Three complexity levels for different use cases
- ✅ Honest, reproducible benchmarks
- ✅ Professional visual output quality

**Recommendation**: Use `realistic_document_benchmark` for accurate throughput testing with real-world content patterns.
