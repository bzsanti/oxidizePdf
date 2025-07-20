# oxidize-pdf-cli

Command-line interface for oxidizePdf - A high-performance PDF manipulation library written in Rust.

## Installation

```bash
cargo install oxidize-pdf-cli
```

## Usage

```bash
oxidizepdf [COMMAND] [OPTIONS]
```

### Commands

#### Merge PDFs
Combine multiple PDF files into a single document:
```bash
oxidizepdf merge input1.pdf input2.pdf -o output.pdf
```

#### Split PDF
Split a PDF into individual pages or chunks:
```bash
# Split into individual pages
oxidizepdf split input.pdf -o output_dir/

# Split into chunks of N pages
oxidizepdf split input.pdf --chunk-size 5 -o output_dir/
```

#### Extract Pages
Extract specific pages from a PDF:
```bash
# Extract pages 1-5 and 10
oxidizepdf extract input.pdf --pages 1-5,10 -o output.pdf
```

#### Rotate Pages
Rotate pages in a PDF:
```bash
# Rotate all pages 90 degrees clockwise
oxidizepdf rotate input.pdf --angle 90 -o output.pdf

# Rotate specific pages
oxidizepdf rotate input.pdf --pages 1,3,5 --angle 180 -o output.pdf
```

#### Extract Text
Extract text content from PDFs:
```bash
# Extract all text
oxidizepdf text input.pdf

# Extract text from specific pages
oxidizepdf text input.pdf --pages 1-10
```

#### PDF Information
Display PDF metadata and information:
```bash
oxidizepdf info input.pdf
```

### Global Options

- `-o, --output <FILE>`: Output file path
- `-v, --verbose`: Enable verbose logging
- `-q, --quiet`: Suppress all output except errors
- `--help`: Display help information
- `--version`: Display version information

## Examples

### Batch Processing
Process multiple PDFs in a directory:
```bash
# Merge all PDFs in a directory
oxidizepdf merge *.pdf -o combined.pdf

# Split multiple PDFs
for file in *.pdf; do
  oxidizepdf split "$file" -o "split_${file%.pdf}/"
done
```

### Advanced Usage
```bash
# Extract specific pages and rotate them
oxidizepdf extract input.pdf --pages 1-5 | \
  oxidizepdf rotate --angle 90 -o rotated_pages.pdf

# Merge PDFs with metadata preservation
oxidizepdf merge doc1.pdf doc2.pdf \
  --preserve-metadata \
  --title "Combined Document" \
  -o merged.pdf
```

## Features

- **Fast Performance**: Built on the high-performance oxidize-pdf core library
- **Memory Efficient**: Streaming operations for large PDFs
- **Cross-Platform**: Works on Windows, macOS, and Linux
- **Unicode Support**: Full UTF-8 text extraction
- **Error Recovery**: Handles corrupted PDFs gracefully

## Configuration

oxidize-pdf-cli can be configured using environment variables:

- `OXIDIZEPDF_LOG_LEVEL`: Set logging level (trace, debug, info, warn, error)
- `OXIDIZEPDF_BUFFER_SIZE`: Set buffer size for streaming operations

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## See Also

- [oxidize-pdf](https://crates.io/crates/oxidize-pdf) - Core PDF manipulation library
- [oxidize-pdf-api](https://crates.io/crates/oxidize-pdf-api) - REST API for PDF operations