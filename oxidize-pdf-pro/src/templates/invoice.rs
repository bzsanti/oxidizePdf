use super::ProTemplate;
use crate::error::Result;
use crate::license::FeatureGate;
use oxidize_pdf::Document;

pub struct ProInvoiceTemplate {
    customer: Option<String>,
    invoice_number: Option<String>,
    line_items: Vec<LineItem>,
    with_schema_org: bool,
}

#[derive(Debug, Clone)]
struct LineItem {
    #[allow(dead_code)]
    description: String,
    #[allow(dead_code)]
    amount: f64,
}

impl ProInvoiceTemplate {
    pub fn new() -> Self {
        Self {
            customer: None,
            invoice_number: None,
            line_items: Vec::new(),
            with_schema_org: false,
        }
    }

    pub fn customer(mut self, customer: &str) -> Self {
        self.customer = Some(customer.to_string());
        self
    }

    pub fn invoice_number(mut self, number: &str) -> Self {
        self.invoice_number = Some(number.to_string());
        self
    }

    pub fn add_line_item(mut self, description: &str, amount: f64) -> Self {
        self.line_items.push(LineItem {
            description: description.to_string(),
            amount,
        });
        self
    }

    pub fn with_schema_org_markup(mut self) -> Self {
        self.with_schema_org = true;
        self
    }
}

impl ProTemplate for ProInvoiceTemplate {
    fn build(&self) -> Result<Document> {
        FeatureGate::check_template_features()?;

        let mut doc = Document::new();
        doc.set_title("Professional Invoice");
        doc.set_creator("oxidize-pdf-pro");

        // Add a page
        let page = oxidize_pdf::Page::a4();
        doc.add_page(page);

        Ok(doc)
    }

    fn to_pdf_with_xmp(&self) -> Result<Document> {
        FeatureGate::check_xmp_features()?;

        let mut doc = self.build()?;

        if self.with_schema_org {
            // Add XMP metadata
            let xmp_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <rdf:Description xmlns:schema="http://schema.org/">
            <schema:Invoice>Placeholder</schema:Invoice>
        </rdf:Description>
    </rdf:RDF>
</x:xmpmeta>"#;

            doc.add_xmp_metadata(xmp_data)?;
        }

        Ok(doc)
    }
}

impl Default for ProInvoiceTemplate {
    fn default() -> Self {
        Self::new()
    }
}
