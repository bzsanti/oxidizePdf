use super::ProTemplate;
use crate::error::{ProError, Result};
use crate::license::FeatureGate;
use oxidize_pdf::Document;

pub struct ProContractTemplate {
    title: Option<String>,
    parties: Vec<String>,
}

impl ProContractTemplate {
    pub fn new() -> Self {
        Self {
            title: None,
            parties: Vec::new(),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn add_party(mut self, party: &str) -> Self {
        self.parties.push(party.to_string());
        self
    }
}

impl ProTemplate for ProContractTemplate {
    fn build(&self) -> Result<Document> {
        FeatureGate::check_template_features()?;

        let mut doc = Document::new();
        doc.set_title("Professional Contract");
        doc.set_creator("oxidize-pdf-pro");

        let page = oxidize_pdf::Page::a4();
        doc.add_page(page);

        Ok(doc)
    }

    fn to_pdf_with_xmp(&self) -> Result<Document> {
        self.build()
    }
}

impl Default for ProContractTemplate {
    fn default() -> Self {
        Self::new()
    }
}
