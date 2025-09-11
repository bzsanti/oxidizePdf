use crate::error::Result;
use crate::license::FeatureGate;
use oxidize_pdf::Document;

pub mod contract;
pub mod invoice;
pub mod report;

pub use contract::ProContractTemplate;
pub use invoice::ProInvoiceTemplate;
pub use report::ProReportTemplate;

pub trait ProTemplate {
    fn build(&self) -> Result<Document>;
    fn to_pdf_with_xmp(&self) -> Result<Document>;
}

pub struct TemplateBuilder {
    #[allow(dead_code)]
    template_type: TemplateType,
    data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum TemplateType {
    Invoice,
    Contract,
    Report,
}

impl TemplateBuilder {
    pub fn new(template_type: TemplateType) -> Self {
        Self {
            template_type,
            data: serde_json::json!({}),
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn build(&self) -> Result<Document> {
        FeatureGate::check_template_features()?;

        let mut doc = Document::new();
        doc.set_title("Pro Template Document");
        doc.set_creator("oxidize-pdf-pro");

        // Placeholder implementation
        let page = oxidize_pdf::Page::a4();
        doc.add_page(page);

        Ok(doc)
    }
}
