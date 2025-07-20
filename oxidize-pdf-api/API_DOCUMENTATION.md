# oxidizePdf REST API Documentation

## Overview

The oxidizePdf REST API provides a comprehensive HTTP interface for PDF manipulation operations. This API is part of the Community Edition of oxidizePdf and provides essential PDF processing capabilities.

## Base URL

```
http://localhost:3000
```

## Authentication

The Community Edition API does not require authentication. All endpoints are publicly accessible.

## Response Format

All API responses use JSON format for structured data and binary format for PDF files.

### Success Response

```json
{
  "message": "Operation completed successfully",
  "data": { /* operation-specific data */ }
}
```

### Error Response

```json
{
  "error": "Human-readable error message describing what went wrong"
}
```

## Available Endpoints

### Core Operations

#### 1. Health Check

**GET** `/api/health`

Returns the service health status, version, and basic information.

**Response:**
```json
{
  "status": "ok",
  "service": "oxidizePdf API",
  "version": "0.1.0"
}
```

**Example:**
```bash
curl http://localhost:3000/api/health
```

#### 2. Create PDF from Text

**POST** `/api/create`

Creates a new PDF document from text content.

**Request Body:**
```json
{
  "text": "Your text content here",
  "font_size": 24.0  // Optional, defaults to 24.0
}
```

**Response:**
- **Content-Type:** `application/pdf`
- **Content-Disposition:** `attachment; filename="generated.pdf"`
- **Body:** PDF binary data

**Example:**
```bash
curl -X POST http://localhost:3000/api/create \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello, World!", "font_size": 18}' \
  --output generated.pdf
```

#### 3. Extract Text from PDF

**POST** `/api/extract`

Extracts text content from an uploaded PDF file.

**Request:**
- **Content-Type:** `multipart/form-data`
- **Body:** PDF file with field name `file`

**Response:**
```json
{
  "text": "Extracted text content from all pages",
  "pages": 5
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/extract \
  -F "file=@document.pdf"
```

### PDF Operations

#### 4. Merge PDFs

**POST** `/api/merge`

Merges multiple PDF files into a single document.

**Request:**
- **Content-Type:** `multipart/form-data`
- **Body:** 
  - Multiple PDF files with field name `files`
  - Optional configuration with field name `options`

**Options JSON:**
```json
{
  "preserve_bookmarks": true,  // Optional, defaults to true
  "optimize": false           // Optional, defaults to false
}
```

**Response:**
- **Content-Type:** `application/pdf`
- **Content-Disposition:** `attachment; filename="merged.pdf"`
- **X-Merge-Info:** JSON header with merge statistics
- **Body:** Merged PDF binary data

**Merge Info Header:**
```json
{
  "message": "PDFs merged successfully",
  "files_merged": 3,
  "output_size": 245760
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/merge \
  -F "files=@document1.pdf" \
  -F "files=@document2.pdf" \
  -F "files=@document3.pdf" \
  -F 'options={"preserve_bookmarks": true, "optimize": false}' \
  --output merged.pdf
```

## Error Handling

### HTTP Status Codes

- **200 OK:** Request successful
- **400 Bad Request:** Invalid request format or parameters
- **404 Not Found:** Endpoint not found
- **405 Method Not Allowed:** HTTP method not supported for endpoint
- **422 Unprocessable Entity:** Valid request format but processing failed
- **500 Internal Server Error:** Server processing error

### Common Error Messages

- `"No file provided in upload"` - File upload endpoint called without file
- `"At least 2 PDF files are required for merging"` - Merge operation with insufficient files
- `"Failed to parse PDF"` - Invalid or corrupted PDF file
- `"Failed to read multipart field"` - Invalid multipart form data

## Rate Limiting

The Community Edition does not implement rate limiting. For production use, consider implementing rate limiting at the reverse proxy level.

## File Size Limits

- **Maximum file size:** Limited by available system memory
- **Recommended maximum:** 50MB per PDF file
- **Merge operations:** Up to 10 files per request (recommended)

## CORS Support

The API includes permissive CORS headers for development:
- `Access-Control-Allow-Origin: *`
- `Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS`
- `Access-Control-Allow-Headers: *`

## Example Usage

### Complete Workflow Example

```bash
# 1. Check API health
curl http://localhost:3000/api/health

# 2. Create a PDF from text
curl -X POST http://localhost:3000/api/create \
  -H "Content-Type: application/json" \
  -d '{"text": "Document 1 content", "font_size": 12}' \
  --output doc1.pdf

# 3. Create another PDF
curl -X POST http://localhost:3000/api/create \
  -H "Content-Type: application/json" \
  -d '{"text": "Document 2 content", "font_size": 12}' \
  --output doc2.pdf

# 4. Merge the PDFs
curl -X POST http://localhost:3000/api/merge \
  -F "files=@doc1.pdf" \
  -F "files=@doc2.pdf" \
  -F 'options={"preserve_bookmarks": true}' \
  --output merged.pdf

# 5. Extract text from merged PDF
curl -X POST http://localhost:3000/api/extract \
  -F "file=@merged.pdf"
```

### JavaScript Example

```javascript
// Create PDF from text
const createPdf = async (text, fontSize = 24) => {
  const response = await fetch('/api/create', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text, font_size: fontSize })
  });
  
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  
  return response.blob();
};

// Merge PDFs
const mergePdfs = async (files, options = {}) => {
  const formData = new FormData();
  
  files.forEach(file => {
    formData.append('files', file);
  });
  
  formData.append('options', JSON.stringify(options));
  
  const response = await fetch('/api/merge', {
    method: 'POST',
    body: formData
  });
  
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  
  return response.blob();
};

// Extract text from PDF
const extractText = async (pdfFile) => {
  const formData = new FormData();
  formData.append('file', pdfFile);
  
  const response = await fetch('/api/extract', {
    method: 'POST',
    body: formData
  });
  
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  
  return response.json();
};
```

### Python Example

```python
import requests
import json

# Create PDF from text
def create_pdf(text, font_size=24):
    url = "http://localhost:3000/api/create"
    payload = {"text": text, "font_size": font_size}
    
    response = requests.post(url, json=payload)
    response.raise_for_status()
    
    return response.content

# Merge PDFs
def merge_pdfs(pdf_files, options=None):
    url = "http://localhost:3000/api/merge"
    
    files = []
    for i, pdf_content in enumerate(pdf_files):
        files.append(('files', (f'file{i}.pdf', pdf_content, 'application/pdf')))
    
    data = {}
    if options:
        data['options'] = json.dumps(options)
    
    response = requests.post(url, files=files, data=data)
    response.raise_for_status()
    
    return response.content

# Extract text from PDF
def extract_text(pdf_content):
    url = "http://localhost:3000/api/extract"
    files = {'file': ('document.pdf', pdf_content, 'application/pdf')}
    
    response = requests.post(url, files=files)
    response.raise_for_status()
    
    return response.json()

# Example usage
if __name__ == "__main__":
    # Create two PDFs
    pdf1 = create_pdf("First document content", 12)
    pdf2 = create_pdf("Second document content", 12)
    
    # Merge them
    merged_pdf = merge_pdfs([pdf1, pdf2], {"preserve_bookmarks": True})
    
    # Extract text
    text_data = extract_text(merged_pdf)
    print(f"Extracted text: {text_data['text']}")
    print(f"Number of pages: {text_data['pages']}")
```

## Testing

The API includes comprehensive tests for all endpoints:

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test merge_tests

# Run with verbose output
cargo test -- --nocapture
```

## Performance Considerations

- **Memory Usage:** Each PDF operation loads the entire file into memory
- **Concurrent Requests:** The API handles multiple concurrent requests efficiently
- **Large Files:** For files > 100MB, consider using streaming operations (future enhancement)
- **Batch Operations:** Merge operations are optimized for up to 10 files

## Future Enhancements

The Community Edition will continue to evolve with additional endpoints:

- **PDF Split:** Split PDFs into multiple files
- **Page Operations:** Rotate, extract, and reorder pages
- **Image Operations:** Extract images from PDFs
- **Validation:** PDF structure validation
- **Batch Processing:** Asynchronous batch operations

## Support

For issues, feature requests, or questions:
- **GitHub Issues:** [oxidizePdf Issues](https://github.com/belowzero/oxidize-pdf/issues)
- **Community:** GitHub Discussions
- **Documentation:** [Full Documentation](https://docs.oxidizepdf.dev)

## License

This API is part of the oxidizePdf Community Edition, licensed under GPL v3.