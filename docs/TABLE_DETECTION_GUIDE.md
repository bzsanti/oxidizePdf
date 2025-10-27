# Table Detection Guide

**oxidize-pdf v1.6.3+**

Complete guide for extracting structured table data from PDF documents using oxidize-pdf's advanced table detection capabilities.

## Overview

Table detection in oxidize-pdf combines three powerful features:

1. **Font Metadata Extraction** (Phase 1) - Detects bold headers, font families
2. **Vector Line Extraction** (Phase 2) - Identifies table borders and grid structure
3. **Table Detection** (Phase 3) - Matches text to cells using spatial analysis

## Quick Start

```rust
use oxidize_pdf::text::table_detection::{TableDetector, TableDetectionConfig};
use oxidize_pdf::graphics::extraction::GraphicsExtractor;
use oxidize_pdf::text::extraction::{TextExtractor, ExtractionOptions};
use oxidize_pdf::parser::{PdfReader, PdfDocument};
use std::fs::File;

fn extract_tables(pdf_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open PDF
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let doc = PdfDocument::new(reader);

    // 2. Extract vector lines (borders)
    let mut graphics_ext = GraphicsExtractor::default();
    let graphics = graphics_ext.extract_from_page(&doc, 0)?;

    // 3. Extract text with positions
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut text_ext = TextExtractor::with_options(options);
    let text = text_ext.extract_from_page(&doc, 0)?;

    // 4. Detect tables
    let detector = TableDetector::default();
    let tables = detector.detect(&graphics, &text.fragments)?;

    // 5. Process results
    for table in &tables {
        println!("Table: {}×{} cells", table.row_count(), table.column_count());
        println!("Confidence: {:.1}%", table.confidence * 100.0);

        for row in 0..table.row_count() {
            for col in 0..table.column_count() {
                if let Some(cell) = table.get_cell(row, col) {
                    print!("{:15}", cell.text);
                }
            }
            println!();
        }
    }

    Ok(())
}
```

## Architecture

### Three-Phase Pipeline

#### Phase 1: Font Metadata
Extracts font information for style detection:

```rust
use oxidize_pdf::text::extraction::TextFragment;

// Each fragment includes font metadata
struct TextFragment {
    pub text: String,
    pub x: f64, pub y: f64,
    pub font_name: Option<String>,        // "Helvetica-Bold"
    pub font_family: Option<String>,      // "Helvetica"
    pub is_bold: bool,                    // true
    pub is_italic: bool,                  // false
    // ... other fields
}
```

**Use Cases**:
- Detect bold table headers
- Identify emphasized cells
- Distinguish labels from values

#### Phase 2: Vector Line Extraction
Identifies table borders from PDF graphics:

```rust
use oxidize_pdf::graphics::extraction::{GraphicsExtractor, VectorLine, LineOrientation};

let mut extractor = GraphicsExtractor::default();
let graphics = extractor.extract_from_page(&doc, 0)?;

// Access extracted lines
for line in graphics.horizontal_lines() {
    println!("H-Line: Y={:.2}, from X={:.2} to X={:.2}",
        line.y1, line.x1, line.x2);
}

for line in graphics.vertical_lines() {
    println!("V-Line: X={:.2}, from Y={:.2} to Y={:.2}",
        line.x1, line.y1, line.y2);
}
```

**Key Concepts**:
- **Line Orientation**: Horizontal, Vertical, Diagonal
- **Stroke Width**: Border thickness (default filter: stroked only)
- **CTM Transformation**: Handles rotated/scaled PDFs
- **Duplicate Merging**: Shared borders counted once

**Configuration**:
```rust
use oxidize_pdf::graphics::extraction::ExtractionConfig;

let config = ExtractionConfig {
    stroked_only: true,          // Ignore filled shapes
    extract_diagonals: false,    // Only H/V lines
    angle_tolerance: 2.0,        // 2° tolerance for H/V detection
};
let mut extractor = GraphicsExtractor::new(config);
```

#### Phase 3: Table Detection
Combines lines + text into structured table:

```rust
use oxidize_pdf::text::table_detection::{TableDetector, TableDetectionConfig};

let config = TableDetectionConfig {
    min_rows: 2,                      // At least 2 rows
    min_columns: 2,                   // At least 2 columns
    alignment_tolerance: 2.0,         // 2pt for line clustering
    min_table_area: 1000.0,           // Min 1000 sq pts
    detect_borderless: false,         // Bordered only (for now)
};

let detector = TableDetector::new(config);
let tables = detector.detect(&graphics, &text_fragments)?;
```

**Algorithm**:
1. **Line Clustering**: Group parallel lines by position (tolerance: 2pt)
2. **Grid Detection**: Find regular H/V patterns forming cells
3. **Cell Boundary Calculation**: Compute rectangles from grid intersections
4. **Text Assignment**: Map fragments to cells using spatial containment
5. **Confidence Scoring**: Based on cell population + regularity

## API Reference

### Core Types

#### `DetectedTable`
Represents an extracted table:

```rust
pub struct DetectedTable {
    pub bbox: BoundingBox,           // Table bounding box
    pub cells: Vec<TableCell>,       // All cells (row-major order)
    pub rows: usize,                 // Row count
    pub columns: usize,              // Column count
    pub confidence: f64,             // 0.0 - 1.0
}

// Methods
impl DetectedTable {
    pub fn row_count(&self) -> usize;
    pub fn column_count(&self) -> usize;
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&TableCell>;
}
```

**Indexing**:
- Row 0 = visual top (highest Y coordinate)
- Column 0 = visual left (lowest X coordinate)
- Row-major order: `[R0C0, R0C1, R0C2, R1C0, R1C1, ...]`

#### `TableCell`
Individual cell within a table:

```rust
pub struct TableCell {
    pub row: usize,                  // 0-based row index
    pub column: usize,               // 0-based column index
    pub bbox: BoundingBox,           // Cell rectangle
    pub text: String,                // Cell contents
    pub has_borders: bool,           // Border detection flag
}
```

#### `BoundingBox`
Spatial rectangle:

```rust
pub struct BoundingBox {
    pub x: f64,                      // Left edge
    pub y: f64,                      // Bottom edge (PDF coords)
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn contains_point(&self, px: f64, py: f64) -> bool;
    pub fn area(&self) -> f64;
    pub fn right(&self) -> f64;      // x + width
    pub fn top(&self) -> f64;        // y + height
}
```

### Error Handling

```rust
use oxidize_pdf::text::table_detection::TableDetectionError;

match detector.detect(&graphics, &fragments) {
    Ok(tables) => process_tables(tables),
    Err(TableDetectionError::InvalidCoordinate) => {
        eprintln!("PDF contains NaN/Infinity coordinates");
    }
    Err(TableDetectionError::InvalidGrid(msg)) => {
        eprintln!("Grid detection failed: {}", msg);
    }
    Err(TableDetectionError::InternalError(msg)) => {
        eprintln!("Internal error: {}", msg);
    }
}
```

## Configuration

### Tuning Detection Parameters

#### Alignment Tolerance
How close lines must be to cluster as the same boundary:

```rust
let config = TableDetectionConfig {
    alignment_tolerance: 1.0,  // Strict (high-quality PDFs)
    // alignment_tolerance: 5.0,  // Loose (low-quality scans)
    ..Default::default()
};
```

**Guidelines**:
- **1.0 pt**: High-quality digital PDFs
- **2.0 pt** (default): General-purpose
- **5.0 pt**: Low-resolution scans or hand-drawn borders

#### Minimum Table Size
Filter out small non-table structures:

```rust
let config = TableDetectionConfig {
    min_rows: 3,              // At least header + 2 data rows
    min_columns: 2,           // At least 2 columns
    min_table_area: 5000.0,   // At least 70×70 pt square
    ..Default::default()
};
```

#### DPI-Aware Tolerance
Adjust tolerance based on PDF resolution:

```rust
fn get_config_for_dpi(dpi: f64) -> TableDetectionConfig {
    let tolerance = 2.0 * (dpi / 300.0); // Scale by DPI ratio
    TableDetectionConfig {
        alignment_tolerance: tolerance,
        ..Default::default()
    }
}
```

## Examples

### Example 1: Simple Invoice Table

```rust
use oxidize_pdf::*;

fn extract_invoice_table(pdf_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let doc = PdfDocument::new(reader);

    // Extract graphics and text
    let graphics = GraphicsExtractor::default()
        .extract_from_page(&doc, 0)?;

    let text = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    }).extract_from_page(&doc, 0)?;

    // Detect tables
    let tables = TableDetector::default()
        .detect(&graphics, &text.fragments)?;

    // Extract line items (skip header row)
    let mut items = Vec::new();
    if let Some(table) = tables.first() {
        for row in 1..table.row_count() {
            let description = table.get_cell(row, 0)
                .map(|c| c.text.as_str())
                .unwrap_or("");
            let amount = table.get_cell(row, 3)
                .map(|c| c.text.as_str())
                .unwrap_or("");

            items.push(format!("{}: {}", description, amount));
        }
    }

    Ok(items)
}
```

### Example 2: Multi-Page Report

```rust
fn extract_all_tables(pdf_path: &str) -> Result<Vec<DetectedTable>, Box<dyn std::error::Error>> {
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let doc = PdfDocument::new(reader);

    let mut all_tables = Vec::new();

    for page_num in 0..doc.page_count()? {
        let graphics = GraphicsExtractor::default()
            .extract_from_page(&doc, page_num)?;

        let text = TextExtractor::default()
            .extract_from_page(&doc, page_num)?;

        let tables = TableDetector::default()
            .detect(&graphics, &text.fragments)?;

        all_tables.extend(tables);
    }

    Ok(all_tables)
}
```

### Example 3: CSV Export

```rust
use std::fs::File;
use std::io::Write;

fn export_table_to_csv(table: &DetectedTable, output_path: &str) -> std::io::Result<()> {
    let mut file = File::create(output_path)?;

    for row in 0..table.row_count() {
        let row_data: Vec<String> = (0..table.column_count())
            .map(|col| {
                table.get_cell(row, col)
                    .map(|c| escape_csv(&c.text))
                    .unwrap_or_default()
            })
            .collect();

        writeln!(file, "{}", row_data.join(","))?;
    }

    Ok(())
}

fn escape_csv(text: &str) -> String {
    if text.contains(',') || text.contains('"') || text.contains('\n') {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}
```

## Best Practices

### 1. Always Use preserve_layout

```rust
// ✅ Correct
let options = ExtractionOptions {
    preserve_layout: true,  // Required for table detection
    ..Default::default()
};

// ❌ Incorrect
let options = ExtractionOptions::default(); // Loses position data
```

### 2. Check Table Confidence

```rust
for table in tables {
    if table.confidence < 0.7 {
        eprintln!("Warning: Low confidence table detected ({:.1}%)",
            table.confidence * 100.0);
        continue; // Skip or flag for review
    }
    process_table(table);
}
```

### 3. Handle Empty Cells

```rust
for row in 0..table.row_count() {
    for col in 0..table.column_count() {
        let text = table.get_cell(row, col)
            .map(|c| &c.text)
            .filter(|t| !t.is_empty())
            .unwrap_or("N/A");
        print!("{:15}", text);
    }
    println!();
}
```

### 4. Validate Table Structure

```rust
fn validate_invoice_table(table: &DetectedTable) -> bool {
    // Check expected structure
    if table.column_count() != 4 {
        return false;
    }

    // Check header row
    if let Some(header) = table.get_cell(0, 0) {
        if !header.text.to_lowercase().contains("description") {
            return false;
        }
    }

    true
}
```

## Performance Considerations

### Memory Usage
- **Small tables** (<100 cells): ~1 KB
- **Large tables** (1000 cells): ~50 KB
- Text is cloned into cells (not referenced)

### CPU Time
- **Graphics extraction**: O(n) where n = graphics operations
- **Text extraction**: O(m) where m = text fragments
- **Table detection**: O(n×m) for text assignment
  - Clustered grid detection: O(h log h + v log v) where h/v = line counts
  - Text assignment: O(cells × fragments)

**Optimization Tips**:
- Limit `max_cells` for very large tables
- Process pages in parallel
- Use `detect_borderless: false` initially

### Scalability
Tested on:
- ✅ 50-cell tables: < 10ms
- ✅ 500-cell tables: < 100ms
- ⚠️ 5000-cell tables: ~1s (may need optimization)

## Troubleshooting

### Issue: No Tables Detected

**Causes**:
1. No vector borders in PDF
2. Lines too irregular (exceed `alignment_tolerance`)
3. Table too small (`min_table_area`)
4. Missing text extraction (`preserve_layout: false`)

**Solutions**:
```rust
// Check if borders exist
if graphics.lines.is_empty() {
    eprintln!("No vector lines found - PDF may use images");
}

// Check for table structure
if graphics.has_table_structure() {
    println!("Table structure detected: {} H, {} V",
        graphics.horizontal_count, graphics.vertical_count);
}

// Increase tolerance
let config = TableDetectionConfig {
    alignment_tolerance: 5.0,  // More lenient
    min_table_area: 500.0,     // Smaller tables
    ..Default::default()
};
```

### Issue: Wrong Cell Contents

**Causes**:
1. Text not spatially contained in cells
2. Coordinate system mismatch
3. Overlapping cells

**Solutions**:
```rust
// Debug cell boundaries
for cell in &table.cells {
    println!("Cell({},{}) bbox: ({:.1},{:.1}) {}×{}",
        cell.row, cell.column,
        cell.bbox.x, cell.bbox.y,
        cell.bbox.width, cell.bbox.height);
}

// Check text positions
for fragment in text_fragments {
    println!("Text '{}' at ({:.1},{:.1})",
        fragment.text, fragment.x, fragment.y);
}
```

### Issue: Multiple Tables Merged

**Cause**: Tables too close together

**Solution**:
```rust
// Increase minimum gap between tables
let config = TableDetectionConfig {
    min_table_area: 2000.0,  // Larger minimum size
    ..Default::default()
};
```

## Limitations

### Current Limitations (v1.6.3)

1. **Borderless Tables**: Not yet supported
   - Tables must have visible borders (vector lines)
   - Alignment-based detection planned for v2.0

2. **Merged Cells**: Not detected
   - Each cell assumed to be 1×1
   - Colspan/rowspan not supported

3. **Nested Tables**: Not supported
   - Only detects top-level tables
   - Inner tables treated as text

4. **Large Tables**: Performance degrades
   - O(cells × fragments) can be slow
   - Consider `max_cells` limit

### Known Edge Cases

- **Rotated Tables**: Requires CTM transformation (supported)
- **Irregular Grids**: Lines must be relatively aligned
- **Image-Based Tables**: Vector lines required (no OCR)
- **Very Dense Text**: May assign to wrong cell

## Roadmap

### v2.0 (Planned)
- **Borderless table detection** using text alignment
- **Merged cell detection** with colspan/rowspan
- **Spatial indexing** (R-tree) for O(n log m) text assignment
- **Confidence heuristics**: regularity scoring
- **Column width variance** analysis

### Future Enhancements
- OCR integration for image-based tables
- Table type classification (invoice, schedule, data grid)
- Header row detection heuristics
- Auto-format detection (currency, dates, numbers)

## See Also

- [INVOICE_EXTRACTION_GUIDE.md](./INVOICE_EXTRACTION_GUIDE.md) - Invoice-specific extraction
- [Examples](../oxidize-pdf-core/examples/) - Working code examples
- [API Documentation](https://docs.rs/oxidize-pdf) - Full rustdoc reference

## Contributing

Found a bug or have a feature request? Please open an issue on GitHub:
https://github.com/BelowZero/oxidize-pdf/issues

---

**Version**: 1.6.3
**Last Updated**: 2025-10-22
**License**: AGPL-3.0
