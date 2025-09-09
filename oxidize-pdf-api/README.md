# oxidize-pdf-api

REST API for oxidizePdf - A high-performance PDF manipulation library written in Rust.

## Features

- **RESTful API**: Clean and intuitive HTTP endpoints for PDF operations
- **Async Processing**: Built on Tokio for high-performance async I/O
- **File Upload Support**: Multipart form data handling for PDF uploads
- **CORS Support**: Cross-origin resource sharing for web applications
- **Community Edition**: Full-featured API for common PDF operations

## Installation

### From Source
```bash
git clone https://github.com/yourusername/oxidize-pdf
cd oxidize-pdf/oxidize-pdf-api
cargo build --release
```

### Using Cargo
```bash
cargo install oxidize-pdf-api
```

## Running the Server

```bash
# Default port 3000
oxidize-pdf-api

# Custom port
oxidize-pdf-api --port 8080

# With environment variables
OXIDIZEPDF_API_PORT=8080 RUST_LOG=info oxidize-pdf-api
```

## API Endpoints

### Health Check
```
GET /health
```
Returns server status and version information.

### OCR Processing
```
POST /api/ocr
```
Extract text from PDF files using OCR (Optical Character Recognition). Ideal for scanned documents or PDFs with embedded images containing text.

**Request:**
- Method: POST
- Content-Type: multipart/form-data
- Body: PDF file in 'file' field

**Response:**
```json
{
  "text": "Extracted text from the PDF",
  "pages": 2,
  "confidence": 0.85,
  "processing_time_ms": 1250,
  "engine": "MockOCR-Community", 
  "language": "eng"
}
```

**Example using curl:**
```bash
curl -X POST http://localhost:3000/api/ocr \
  -F "file=@document.pdf"
```

### Merge PDFs
```
POST /api/merge
Content-Type: multipart/form-data
```

Merge multiple PDF files into a single document.

**Request:**
- `files`: Multiple PDF files (form field name: `file`)
- `preserve_metadata`: Boolean (optional, default: true)
- `optimize`: Boolean (optional, default: false)

**Response:**
```json
{
  "success": true,
  "pages": 42,
  "size": 1048576,
  "processing_time_ms": 150
}
```

Returns the merged PDF file with appropriate headers.

### Split PDF
```
POST /api/split
Content-Type: multipart/form-data
```

Split a PDF into individual pages or chunks.

**Request:**
- `file`: PDF file to split
- `mode`: "pages" | "chunks" (optional, default: "pages")
- `chunk_size`: Number (optional, for chunks mode)

**Response:** ZIP file containing split PDFs

### Extract Pages
```
POST /api/extract
Content-Type: multipart/form-data
```

Extract specific pages from a PDF.

**Request:**
- `file`: Source PDF file
- `pages`: Page range (e.g., "1-5,10,15-20")

**Response:** PDF containing only the specified pages

### Rotate Pages
```
POST /api/rotate
Content-Type: multipart/form-data
```

Rotate pages in a PDF.

**Request:**
- `file`: PDF file
- `angle`: 90 | 180 | 270
- `pages`: Page range (optional, rotates all if not specified)

**Response:** PDF with rotated pages

### Extract Text
```
POST /api/text
Content-Type: multipart/form-data
```

Extract text content from a PDF.

**Request:**
- `file`: PDF file
- `pages`: Page range (optional)
- `format`: "plain" | "json" (optional, default: "plain")

**Response:**
- Plain text or JSON structure with page-by-page text

### Get PDF Info
```
POST /api/info
Content-Type: multipart/form-data
```

Get metadata and information about a PDF.

**Request:**
- `file`: PDF file

**Response:**
```json
{
  "pages": 10,
  "title": "Document Title",
  "author": "Author Name",
  "subject": "Document Subject",
  "keywords": "pdf, document",
  "creator": "oxidize_pdf",
  "producer": "oxidize_pdf v0.1.0",
  "creation_date": "2024-01-01T12:00:00Z",
  "modification_date": "2024-01-15T14:30:00Z",
  "file_size": 1048576,
  "pdf_version": "1.7"
}
```

## Example Usage

### cURL
```bash
# Merge PDFs
curl -X POST http://localhost:3000/api/merge \
  -F "file=@doc1.pdf" \
  -F "file=@doc2.pdf" \
  -F "file=@doc3.pdf" \
  -o merged.pdf

# Extract pages
curl -X POST http://localhost:3000/api/extract \
  -F "file=@input.pdf" \
  -F "pages=1-5,10" \
  -o extracted.pdf

# Get PDF info
curl -X POST http://localhost:3000/api/info \
  -F "file=@document.pdf"
```

### JavaScript/Fetch
```javascript
// Merge PDFs
const formData = new FormData();
formData.append('file', file1);
formData.append('file', file2);
formData.append('preserve_metadata', 'true');

const response = await fetch('http://localhost:3000/api/merge', {
  method: 'POST',
  body: formData
});

const blob = await response.blob();
// Handle the merged PDF blob
```

### Python
```python
import requests

# Merge PDFs
files = [
    ('file', open('doc1.pdf', 'rb')),
    ('file', open('doc2.pdf', 'rb'))
]

response = requests.post(
    'http://localhost:3000/api/merge',
    files=files
)

with open('merged.pdf', 'wb') as f:
    f.write(response.content)
```

## Configuration

Environment variables:
- `OXIDIZEPDF_API_PORT`: Server port (default: 3000)
- `OXIDIZEPDF_API_HOST`: Server host (default: 0.0.0.0)
- `OXIDIZEPDF_MAX_FILE_SIZE`: Maximum upload size in bytes (default: 100MB)
- `RUST_LOG`: Logging level (trace, debug, info, warn, error)

## Error Handling

All endpoints return consistent error responses:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": "Additional error details"
}
```

HTTP status codes:
- 200: Success
- 400: Bad Request (invalid input)
- 413: Payload Too Large
- 415: Unsupported Media Type
- 500: Internal Server Error

## Security

- File size limits to prevent DoS
- Input validation for all parameters
- Temporary file cleanup after processing
- CORS configuration for web security

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## See Also

- [oxidize-pdf](https://crates.io/crates/oxidize-pdf) - Core PDF library
- [oxidize-pdf-cli](https://crates.io/crates/oxidize-pdf-cli) - CLI tool
- [API Documentation](https://github.com/yourusername/oxidize-pdf/blob/main/oxidize-pdf-api/API_DOCUMENTATION.md) - Detailed API docs