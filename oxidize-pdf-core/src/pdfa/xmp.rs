//! XMP Metadata for PDF/A compliance

use super::error::{PdfAError, PdfAResult};
use super::types::PdfAConformance;
use regex::Regex;
use std::str::FromStr;

/// PDF/A identification in XMP metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmpPdfAIdentifier {
    /// PDF/A part (1, 2, or 3)
    pub part: u8,
    /// PDF/A conformance level (A, B, or U)
    pub conformance: PdfAConformance,
    /// Amendment (optional, e.g., "amd1")
    pub amd: Option<String>,
    /// Corrigenda (optional)
    pub corr: Option<String>,
}

impl XmpPdfAIdentifier {
    /// Create a new PDF/A identifier
    pub fn new(part: u8, conformance: PdfAConformance) -> Self {
        Self {
            part,
            conformance,
            amd: None,
            corr: None,
        }
    }

    /// Generate XMP RDF for this identifier
    pub fn to_rdf(&self) -> String {
        let mut rdf = format!(
            r#"    <rdf:Description rdf:about=""
        xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
      <pdfaid:part>{}</pdfaid:part>
      <pdfaid:conformance>{}</pdfaid:conformance>"#,
            self.part, self.conformance
        );

        if let Some(ref amd) = self.amd {
            rdf.push_str(&format!("\n      <pdfaid:amd>{}</pdfaid:amd>", amd));
        }

        if let Some(ref corr) = self.corr {
            rdf.push_str(&format!("\n      <pdfaid:corr>{}</pdfaid:corr>", corr));
        }

        rdf.push_str("\n    </rdf:Description>");
        rdf
    }
}

/// XMP Metadata for PDF documents
#[derive(Debug, Clone, Default)]
pub struct XmpMetadata {
    /// Document title
    pub title: Option<String>,
    /// Document creator/author(s)
    pub creator: Option<Vec<String>>,
    /// Document description/subject
    pub description: Option<String>,
    /// Keywords
    pub keywords: Option<Vec<String>>,
    /// Creation date (ISO 8601)
    pub create_date: Option<String>,
    /// Modification date (ISO 8601)
    pub modify_date: Option<String>,
    /// Creator tool/application
    pub creator_tool: Option<String>,
    /// PDF/A identification (required for PDF/A)
    pub pdfa_id: Option<XmpPdfAIdentifier>,
    /// Document ID
    pub document_id: Option<String>,
    /// Instance ID
    pub instance_id: Option<String>,
}

impl XmpMetadata {
    /// Create a new empty XMP metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse XMP metadata from XML string
    pub fn parse(xml: &str) -> PdfAResult<Self> {
        let mut metadata = Self::new();

        // Parse title
        if let Some(title) = Self::extract_simple_value(xml, "dc:title") {
            metadata.title = Some(title);
        }

        // Parse creator (can be a list)
        if let Some(creator) = Self::extract_list_value(xml, "dc:creator") {
            metadata.creator = Some(creator);
        }

        // Parse description
        if let Some(desc) = Self::extract_simple_value(xml, "dc:description") {
            metadata.description = Some(desc);
        }

        // Parse keywords
        if let Some(keywords) = Self::extract_list_value(xml, "pdf:Keywords")
            .or_else(|| Self::extract_list_value(xml, "dc:subject"))
        {
            metadata.keywords = Some(keywords);
        }

        // Parse dates
        if let Some(date) = Self::extract_simple_value(xml, "xmp:CreateDate") {
            metadata.create_date = Some(date);
        }
        if let Some(date) = Self::extract_simple_value(xml, "xmp:ModifyDate") {
            metadata.modify_date = Some(date);
        }

        // Parse creator tool
        if let Some(tool) = Self::extract_simple_value(xml, "xmp:CreatorTool") {
            metadata.creator_tool = Some(tool);
        }

        // Parse PDF/A identification
        if let (Some(part_str), Some(conf_str)) = (
            Self::extract_simple_value(xml, "pdfaid:part"),
            Self::extract_simple_value(xml, "pdfaid:conformance"),
        ) {
            if let (Ok(part), Ok(conformance)) =
                (part_str.parse::<u8>(), PdfAConformance::from_str(&conf_str))
            {
                let mut pdfa_id = XmpPdfAIdentifier::new(part, conformance);
                pdfa_id.amd = Self::extract_simple_value(xml, "pdfaid:amd");
                pdfa_id.corr = Self::extract_simple_value(xml, "pdfaid:corr");
                metadata.pdfa_id = Some(pdfa_id);
            }
        }

        // Parse document/instance IDs
        metadata.document_id = Self::extract_simple_value(xml, "xmpMM:DocumentID");
        metadata.instance_id = Self::extract_simple_value(xml, "xmpMM:InstanceID");

        Ok(metadata)
    }

    /// Extract a simple value from XMP
    fn extract_simple_value(xml: &str, tag: &str) -> Option<String> {
        // Try element form: <tag>value</tag>
        let pattern = format!(r"<{tag}[^>]*>([^<]*)</{tag}>", tag = regex::escape(tag));
        if let Ok(re) = Regex::new(&pattern) {
            if let Some(caps) = re.captures(xml) {
                return Some(caps[1].trim().to_string());
            }
        }

        // Try Alt form: <tag><rdf:Alt><rdf:li...>value</rdf:li></rdf:Alt></tag>
        let alt_pattern = format!(
            r"<{tag}[^>]*>\s*<rdf:Alt[^>]*>\s*<rdf:li[^>]*>([^<]*)</rdf:li>",
            tag = regex::escape(tag)
        );
        if let Ok(re) = Regex::new(&alt_pattern) {
            if let Some(caps) = re.captures(xml) {
                return Some(caps[1].trim().to_string());
            }
        }

        None
    }

    /// Extract a list value from XMP (Seq or Bag)
    fn extract_list_value(xml: &str, tag: &str) -> Option<Vec<String>> {
        // Match the entire tag content (use [\s\S] to match newlines)
        let pattern = format!(r"(?s)<{tag}[^>]*>(.*?)</{tag}>", tag = regex::escape(tag));

        if let Ok(re) = Regex::new(&pattern) {
            if let Some(caps) = re.captures(xml) {
                let content = &caps[1];
                // Extract all li elements
                if let Ok(li_re) = Regex::new(r"<rdf:li[^>]*>([^<]*)</rdf:li>") {
                    let values: Vec<String> = li_re
                        .captures_iter(content)
                        .map(|c| c[1].trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !values.is_empty() {
                        return Some(values);
                    }
                }
            }
        }

        None
    }

    /// Generate XMP XML for this metadata
    pub fn to_xml(&self) -> String {
        let mut xml = String::from(
            r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#,
        );

        // DC (Dublin Core) namespace
        xml.push_str("\n    <rdf:Description rdf:about=\"\"\n        xmlns:dc=\"http://purl.org/dc/elements/1.1/\">");

        if let Some(ref title) = self.title {
            xml.push_str(&format!(
                "\n      <dc:title>\n        <rdf:Alt>\n          <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n        </rdf:Alt>\n      </dc:title>",
                Self::xml_escape(title)
            ));
        }

        if let Some(ref creators) = self.creator {
            xml.push_str("\n      <dc:creator>\n        <rdf:Seq>");
            for creator in creators {
                xml.push_str(&format!(
                    "\n          <rdf:li>{}</rdf:li>",
                    Self::xml_escape(creator)
                ));
            }
            xml.push_str("\n        </rdf:Seq>\n      </dc:creator>");
        }

        if let Some(ref desc) = self.description {
            xml.push_str(&format!(
                "\n      <dc:description>\n        <rdf:Alt>\n          <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n        </rdf:Alt>\n      </dc:description>",
                Self::xml_escape(desc)
            ));
        }

        xml.push_str("\n    </rdf:Description>");

        // XMP namespace
        xml.push_str("\n    <rdf:Description rdf:about=\"\"\n        xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">");

        if let Some(ref tool) = self.creator_tool {
            xml.push_str(&format!(
                "\n      <xmp:CreatorTool>{}</xmp:CreatorTool>",
                Self::xml_escape(tool)
            ));
        }

        if let Some(ref date) = self.create_date {
            xml.push_str(&format!(
                "\n      <xmp:CreateDate>{}</xmp:CreateDate>",
                date
            ));
        }

        if let Some(ref date) = self.modify_date {
            xml.push_str(&format!(
                "\n      <xmp:ModifyDate>{}</xmp:ModifyDate>",
                date
            ));
        }

        xml.push_str("\n    </rdf:Description>");

        // PDF/A identification
        if let Some(ref pdfa_id) = self.pdfa_id {
            xml.push_str(&format!("\n{}", pdfa_id.to_rdf()));
        }

        // XMP Media Management
        if self.document_id.is_some() || self.instance_id.is_some() {
            xml.push_str("\n    <rdf:Description rdf:about=\"\"\n        xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\">");
            if let Some(ref doc_id) = self.document_id {
                xml.push_str(&format!(
                    "\n      <xmpMM:DocumentID>{}</xmpMM:DocumentID>",
                    doc_id
                ));
            }
            if let Some(ref inst_id) = self.instance_id {
                xml.push_str(&format!(
                    "\n      <xmpMM:InstanceID>{}</xmpMM:InstanceID>",
                    inst_id
                ));
            }
            xml.push_str("\n    </rdf:Description>");
        }

        xml.push_str("\n  </rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
        xml
    }

    /// Escape special XML characters
    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Validate this metadata for PDF/A compliance
    pub fn validate_for_pdfa(&self) -> PdfAResult<()> {
        // PDF/A requires PDF/A identification
        if self.pdfa_id.is_none() {
            return Err(PdfAError::XmpParseError(
                "PDF/A identification is required".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xmp_pdfa_identifier_new() {
        let id = XmpPdfAIdentifier::new(1, PdfAConformance::B);
        assert_eq!(id.part, 1);
        assert_eq!(id.conformance, PdfAConformance::B);
        assert!(id.amd.is_none());
        assert!(id.corr.is_none());
    }

    #[test]
    fn test_xmp_pdfa_identifier_to_rdf() {
        let id = XmpPdfAIdentifier::new(2, PdfAConformance::U);
        let rdf = id.to_rdf();
        assert!(rdf.contains("<pdfaid:part>2</pdfaid:part>"));
        assert!(rdf.contains("<pdfaid:conformance>U</pdfaid:conformance>"));
    }

    #[test]
    fn test_xmp_metadata_new() {
        let metadata = XmpMetadata::new();
        assert!(metadata.title.is_none());
        assert!(metadata.creator.is_none());
        assert!(metadata.pdfa_id.is_none());
    }

    #[test]
    fn test_xmp_metadata_parse_title() {
        let xml = r#"<dc:title><rdf:Alt><rdf:li xml:lang="x-default">Test Title</rdf:li></rdf:Alt></dc:title>"#;
        let metadata = XmpMetadata::parse(xml).unwrap();
        assert_eq!(metadata.title.as_deref(), Some("Test Title"));
    }

    #[test]
    fn test_xmp_metadata_parse_pdfa_id() {
        let xml = r#"
            <pdfaid:part>1</pdfaid:part>
            <pdfaid:conformance>B</pdfaid:conformance>
        "#;
        let metadata = XmpMetadata::parse(xml).unwrap();
        assert!(metadata.pdfa_id.is_some());
        let pdfa_id = metadata.pdfa_id.unwrap();
        assert_eq!(pdfa_id.part, 1);
        assert_eq!(pdfa_id.conformance, PdfAConformance::B);
    }

    #[test]
    fn test_xmp_metadata_parse_creator_list() {
        let xml = r#"
            <dc:creator>
                <rdf:Seq>
                    <rdf:li>Author One</rdf:li>
                    <rdf:li>Author Two</rdf:li>
                </rdf:Seq>
            </dc:creator>
        "#;
        let metadata = XmpMetadata::parse(xml).unwrap();
        assert!(metadata.creator.is_some());
        let creators = metadata.creator.unwrap();
        assert_eq!(creators.len(), 2);
        assert_eq!(creators[0], "Author One");
        assert_eq!(creators[1], "Author Two");
    }

    #[test]
    fn test_xmp_metadata_to_xml() {
        let mut metadata = XmpMetadata::new();
        metadata.title = Some("Test Document".to_string());
        metadata.creator = Some(vec!["Test Author".to_string()]);
        metadata.pdfa_id = Some(XmpPdfAIdentifier::new(1, PdfAConformance::B));

        let xml = metadata.to_xml();
        assert!(xml.contains("Test Document"));
        assert!(xml.contains("Test Author"));
        assert!(xml.contains("pdfaid:part"));
    }

    #[test]
    fn test_xmp_metadata_validate_for_pdfa_missing_id() {
        let metadata = XmpMetadata::new();
        assert!(metadata.validate_for_pdfa().is_err());
    }

    #[test]
    fn test_xmp_metadata_validate_for_pdfa_with_id() {
        let mut metadata = XmpMetadata::new();
        metadata.pdfa_id = Some(XmpPdfAIdentifier::new(1, PdfAConformance::B));
        assert!(metadata.validate_for_pdfa().is_ok());
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(XmpMetadata::xml_escape("<test>"), "&lt;test&gt;");
        assert_eq!(XmpMetadata::xml_escape("a & b"), "a &amp; b");
        assert_eq!(XmpMetadata::xml_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_xmp_pdfa_identifier_with_amd() {
        let mut id = XmpPdfAIdentifier::new(1, PdfAConformance::B);
        id.amd = Some("amd1".to_string());
        let rdf = id.to_rdf();
        assert!(rdf.contains("<pdfaid:amd>amd1</pdfaid:amd>"));
    }

    #[test]
    fn test_xmp_metadata_parse_dates() {
        let xml = r#"
            <xmp:CreateDate>2024-01-15T10:30:00Z</xmp:CreateDate>
            <xmp:ModifyDate>2024-01-16T14:00:00Z</xmp:ModifyDate>
        "#;
        let metadata = XmpMetadata::parse(xml).unwrap();
        assert_eq!(
            metadata.create_date.as_deref(),
            Some("2024-01-15T10:30:00Z")
        );
        assert_eq!(
            metadata.modify_date.as_deref(),
            Some("2024-01-16T14:00:00Z")
        );
    }

    #[test]
    fn test_xmp_metadata_roundtrip() {
        let mut original = XmpMetadata::new();
        original.title = Some("Roundtrip Test".to_string());
        original.creator = Some(vec!["Author".to_string()]);
        original.pdfa_id = Some(XmpPdfAIdentifier::new(2, PdfAConformance::U));

        let xml = original.to_xml();
        let parsed = XmpMetadata::parse(&xml).unwrap();

        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.pdfa_id.as_ref().unwrap().part, 2);
        assert_eq!(
            parsed.pdfa_id.as_ref().unwrap().conformance,
            PdfAConformance::U
        );
    }

    #[test]
    fn test_xmp_metadata_parse_simple_tag() {
        let xml = r#"<xmp:CreatorTool>oxidize-pdf 1.6.0</xmp:CreatorTool>"#;
        let metadata = XmpMetadata::parse(xml).unwrap();
        assert_eq!(metadata.creator_tool.as_deref(), Some("oxidize-pdf 1.6.0"));
    }

    #[test]
    fn test_xmp_pdfa_identifier_clone() {
        let id1 = XmpPdfAIdentifier::new(3, PdfAConformance::A);
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_xmp_metadata_clone() {
        let mut metadata = XmpMetadata::new();
        metadata.title = Some("Clone Test".to_string());
        let cloned = metadata.clone();
        assert_eq!(cloned.title, metadata.title);
    }
}
