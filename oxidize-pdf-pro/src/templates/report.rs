use super::ProTemplate;
use crate::error::Result;
use crate::license::FeatureGate;
use oxidize_pdf::Document;

pub struct ProReportTemplate {
    title: Option<String>,
    sections: Vec<ReportSection>,
}

#[derive(Debug, Clone)]
struct ReportSection {
    #[allow(dead_code)]
    title: String,
    #[allow(dead_code)]
    content: String,
}

impl ProReportTemplate {
    pub fn new() -> Self {
        Self {
            title: None,
            sections: Vec::new(),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn add_section(mut self, title: &str, content: &str) -> Self {
        self.sections.push(ReportSection {
            title: title.to_string(),
            content: content.to_string(),
        });
        self
    }
}

impl ProTemplate for ProReportTemplate {
    fn build(&self) -> Result<Document> {
        FeatureGate::check_template_features()?;

        let mut doc = Document::new();
        doc.set_title("Professional Report");
        doc.set_creator("oxidize-pdf-pro");

        let page = oxidize_pdf::Page::a4();
        doc.add_page(page);

        Ok(doc)
    }

    fn to_pdf_with_xmp(&self) -> Result<Document> {
        self.build()
    }
}

impl Default for ProReportTemplate {
    fn default() -> Self {
        Self::new()
    }
}
