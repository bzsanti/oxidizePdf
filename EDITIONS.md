# oxidize-pdf Editions

## Overview

oxidize-pdf is available in three editions to meet different user needs:

- **Community Edition** - Open source (GPL v3) with essential PDF features
- **PRO Edition** - Commercial license with advanced professional features  
- **Enterprise Edition** - Full-scale deployment with premium support

## Edition Comparison

### 🌍 Community Edition (Open Source)

**License**: GPL v3  
**Target Users**: Individual developers, open source projects, small businesses  
**ISO 32000 Compliance**: ~75-80%

#### ✅ Included Features

**Core PDF Operations**
- Parse and read PDF files (99.7% success rate)
- All PDF object types (100% support)
- Extract individual pages
- Merge multiple PDFs
- Split PDFs by pages or ranges
- Rotate pages (90°, 180°, 270°)
- Reorder and rearrange pages
- Delete pages

**Content Extraction**
- Advanced text extraction with layout analysis
- Column detection and reading order
- Image extraction (JPEG, PNG, TIFF)
- Basic metadata reading/writing
- Font information tracking (internal use)

**Compression & Optimization**
- FlateDecode (zlib)
- ASCII85Decode
- ASCIIHexDecode
- Basic file size reduction

**Graphics & Rendering**
- Basic transparency (opacity settings)
- Device color spaces (RGB, CMYK, Gray)
- Basic graphics operations
- Path construction and painting

**Additional Features**
- Memory-efficient large file handling
- Stream processing without full load
- Batch processing capabilities
- Robust error recovery
- Stack-safe parsing
- CLI tool with full functionality
- REST API for basic operations

#### 🔜 Coming to Community (Q1 2026)
- LZWDecode and RunLengthDecode filters
- CalGray and CalRGB color spaces
- Form data extraction (read-only)
- Annotation extraction (read-only)
- Basic document layout features

**Note on Font Information:** While oxidize-pdf tracks font information internally during PDF processing, the Community Edition does not expose font details (names, metrics, encodings) in the public API. This information is used internally for accurate text extraction and rendering but is not available for direct access. Font metadata extraction is reserved for the PRO edition.

### 💼 PRO Edition (Commercial License)

**License**: Commercial (per-seat or per-server)  
**Target Users**: Professional developers, businesses, SaaS providers  
**ISO 32000 Compliance**: ~95-100%

#### ✅ All Community Features Plus:

**Security & Encryption**
- Full encryption/decryption support (RC4, AES-128, AES-256)
- Digital signature creation and validation
- Certificate management
- Permission controls (print, copy, modify)
- Secure redaction tools

**Advanced Document Features**
- Form creation, editing, and flattening
- All annotation types (creation/editing)
- Advanced color spaces (ICCBased, Separation, DeviceN, Pattern)
- Blend modes and transparency groups
- Shading patterns and gradients
- Font embedding and subsetting

**Additional Compression**
- JBIG2 (for scanned documents)
- JPEG2000
- CCITT Group 3/4 (for fax)
- Advanced optimization algorithms

**OCR & Intelligence**
- OCR engine integration (Tesseract, cloud providers)
- Searchable PDF creation from scans
- AI-ready PDF generation
- Entity recognition and marking
- Confidence scoring

**Format Conversions**
- PDF to Word (DOCX) with layout preservation
- PDF to Excel (XLSX) with table detection
- PDF to PowerPoint (PPTX)
- High-quality PDF to images
- HTML/CSS to PDF with full rendering
- Markdown to PDF
- Data-driven PDF generation (JSON/XML)

**PDF Generation**
- Create PDFs from scratch programmatically
- Advanced templating with Tera
- Charts and data visualization
- Barcode and QR code generation
- JavaScript-enabled forms
- Rich media embedding

**Advanced Features**
- PDF/A compliance validation and conversion
- Tagged PDF for accessibility (Section 508, WCAG)
- Linearization for web optimization
- Professional REST API with OAuth2
- Native SDK libraries (Python, Node.js, Java, .NET)

### 🏢 Enterprise Edition

**License**: Enterprise agreement  
**Target Users**: Large organizations, government, high-volume SaaS  
**Support**: Premium support with SLA

#### ✅ All PRO Features Plus:

**Scalability**
- Distributed processing across clusters
- Queue management (Redis, RabbitMQ)
- Auto-scaling capabilities
- Load balancing
- High availability with failover

**Cloud Integration**
- AWS S3 direct integration
- Azure Blob Storage support
- Google Cloud Storage
- CDN edge processing
- Serverless deployment options

**Enterprise Features**
- Multi-tenancy with isolation
- SSO/SAML authentication
- Comprehensive audit logging
- Custom workflow builder
- Webhook integrations
- GDPR/HIPAA compliance tools

**Advanced Processing**
- Unlimited batch processing
- Priority queue management
- Custom processing pipelines
- Real-time progress tracking
- Advanced caching strategies

**AI & Analytics**
- Custom AI schema definitions
- ML training data generation
- Batch AI processing
- Document type detection
- Relationship mapping
- Direct ML pipeline integration

## Choosing the Right Edition

### Choose Community Edition if:
- You need basic PDF operations (read, merge, split, extract)
- You're building open source software
- You're evaluating oxidize-pdf
- Your use case doesn't require encryption or forms
- You can work within GPL v3 license terms

### Choose PRO Edition if:
- You need to handle encrypted PDFs
- You work with forms or annotations
- You require format conversions (PDF to Word/Excel)
- You need OCR capabilities
- You want commercial licensing
- You need professional support

### Choose Enterprise Edition if:
- You process millions of PDFs
- You need distributed processing
- You require cloud integrations
- You need multi-tenant isolation
- You want custom features
- You need SLA-backed support

## Licensing

### Community Edition (GPL v3)
- Free to use, modify, and distribute
- Source code must remain open
- Derivative works must use GPL v3
- No warranty or support included

### PRO Edition
- Per-seat licensing for developers
- Per-server licensing for deployments
- Annual or perpetual licenses available
- Includes updates and email support
- Commercial use permitted

### Enterprise Edition
- Custom licensing agreements
- Volume discounts available
- Includes premium support
- Custom feature development
- Training and consulting available

## Migration Between Editions

### Community → PRO
- No code changes required
- Add license key to unlock features
- Existing PDFs remain compatible
- Gradual feature adoption supported

### PRO → Enterprise
- Architecture consultation included
- Migration assistance provided
- Custom deployment support
- Performance optimization guidance

## Support

### Community Edition
- GitHub Issues for bug reports
- Community Discord server
- Documentation and examples
- Best-effort responses

### PRO Edition
- Priority email support
- 48-hour response SLA
- Feature request consideration
- Migration assistance

### Enterprise Edition
- Dedicated support team
- 24-hour response SLA
- Phone and video support
- Custom feature development
- On-site training available

## Getting Started

### Community Edition
```bash
# Install from crates.io
cargo add oxidize-pdf

# Or use CLI
cargo install oxidize-pdf-cli
```

### PRO Edition
```bash
# Add to Cargo.toml with license
[dependencies]
oxidize-pdf-pro = { version = "1.0", license-key = "YOUR_KEY" }
```

### Enterprise Edition
Contact enterprise@oxidizepdf.dev for custom setup and deployment options.

## FAQ

**Q: Can I try PRO features before buying?**  
A: Yes, we offer a 30-day trial license for evaluation.

**Q: Can I use Community Edition commercially?**  
A: Yes, but you must comply with GPL v3 terms.

**Q: How do I upgrade from Community to PRO?**  
A: Simply purchase a license and add the key to your configuration.

**Q: Is the PRO edition source available?**  
A: PRO edition is closed source but includes comprehensive documentation.

**Q: Can I self-host Enterprise edition?**  
A: Yes, Enterprise edition supports on-premise deployment.

## Contact

- **Sales**: sales@oxidizepdf.dev
- **Support**: support@oxidizepdf.dev  
- **Enterprise**: enterprise@oxidizepdf.dev
- **Community**: https://github.com/oxidize-pdf/discussions