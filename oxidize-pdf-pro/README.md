# oxidize-pdf-pro

**Professional and Enterprise AI-Ready PDF Features**

⚠️ **This is a commercial extension for oxidize-pdf** ⚠️

This crate provides advanced AI-Ready PDF capabilities for professional and enterprise use cases.

## Features

- 🔍 **XMP Metadata Embedding** - Schema.org compliant semantic markup
- 🤖 **AI/ML Integration** - Direct export to training datasets
- 📊 **Advanced Templates** - Professional invoice, contract, and report templates
- 🔐 **License Management** - Commercial licensing with usage tracking
- ⚡ **High Performance** - Optimized for enterprise-scale document processing

## Quick Start

```rust
use oxidize_pdf_pro::prelude::*;

// Create an AI-Ready invoice with XMP metadata
let invoice = ProInvoiceTemplate::new()
    .customer("ACME Corp")
    .invoice_number("INV-2024-001")
    .add_line_item("Services", 2500.00)
    .with_schema_org_markup()
    .build()?;

// Generate PDF with embedded semantic metadata
let pdf = invoice.to_pdf_with_xmp()?;
pdf.save("invoice.pdf")?;

// Extract entities for ML processing
let extractor = SemanticExtractor::from_pdf("invoice.pdf")?;
let training_data = extractor.to_training_dataset()?;
```

## License

This software requires a valid commercial license for use in production environments.

For licensing information, visit: https://oxidizepdf.dev/pro/licensing

## Features Matrix

| Feature | Community (MIT) | Professional | Enterprise |
|---------|----------------|--------------|------------|
| Basic PDF Generation | ✅ | ✅ | ✅ |
| Semantic Entity Marking | ✅ | ✅ | ✅ |
| XMP Metadata Embedding | ❌ | ✅ | ✅ |
| ML Training Export | ❌ | ✅ | ✅ |
| Pro Templates | ❌ | ✅ | ✅ |
| Schema.org Validation | ❌ | ✅ | ✅ |
| Batch Processing | ❌ | Limited | ✅ |
| Priority Support | ❌ | ✅ | ✅ |
| SLA Guarantees | ❌ | ❌ | ✅ |
| Custom Development | ❌ | ❌ | ✅ |

## Documentation

- [Pro API Reference](https://docs.oxidizepdf.dev/pro/)
- [XMP Embedding Guide](https://docs.oxidizepdf.dev/pro/xmp/)
- [ML Integration Tutorial](https://docs.oxidizepdf.dev/pro/ml/)
- [License Management](https://docs.oxidizepdf.dev/pro/licensing/)

## Support

- Professional: support@oxidizepdf.dev
- Enterprise: enterprise@oxidizepdf.dev

---

**Note**: This crate is in active development. APIs may change before 1.0 release.