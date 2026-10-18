//! XMP Metadata for PDF/A compliance

use super::error::{PdfAError, PdfAResult};
use super::types::PdfAConformance;
use super::xmp_packet;
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
            rdf.push_str(&format!(
                "\n      <pdfaid:amd>{}</pdfaid:amd>",
                XmpMetadata::xml_escape(amd)
            ));
        }

        if let Some(ref corr) = self.corr {
            rdf.push_str(&format!(
                "\n      <pdfaid:corr>{}</pdfaid:corr>",
                XmpMetadata::xml_escape(corr)
            ));
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

    /// Parse a well-formed, namespace-qualified XMP XML packet.
    ///
    /// Input is limited to 16 MiB, 100,000 nodes and 128 nesting levels.
    /// DTDs and external entities are disabled. Invalid, incomplete or unsupported
    /// PDF/A identifiers return an error, including unsupported RDF subject resolution.
    /// Ordinary properties support literal attributes/elements, Seq/Bag lists and
    /// title/description Alt values (x-default, otherwise the first alternative).
    /// Text whitespace and empty entries are preserved. Scalar PDF Keywords form
    /// one list entry; PDF Keywords takes precedence over dc:subject when both occur.
    /// Conflicting duplicates or properties for different RDF subjects are errors.
    pub fn parse(xml: &str) -> PdfAResult<Self> {
        let document = xmp_packet::parse(xml)?;
        let mut metadata = Self::new();

        metadata.pdfa_id = parse_identifier(&document)?;
        parse_properties(&document, &mut metadata)?;
        Ok(metadata)
    }

    /// Generate XMP XML for this metadata.
    ///
    /// Text must consist of XML 1.0 characters. All markup delimiters and carriage
    /// returns are escaped; invalid XML characters such as NUL remain invalid and
    /// are rejected by [`Self::parse`]. This infallible API does not discard text.
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

        if let Some(ref keywords) = self.keywords {
            xml.push_str("\n      <dc:subject><rdf:Bag>");
            for keyword in keywords {
                xml.push_str(&format!("<rdf:li>{}</rdf:li>", Self::xml_escape(keyword)));
            }
            xml.push_str("</rdf:Bag></dc:subject>");
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
                Self::xml_escape(date)
            ));
        }

        if let Some(ref date) = self.modify_date {
            xml.push_str(&format!(
                "\n      <xmp:ModifyDate>{}</xmp:ModifyDate>",
                Self::xml_escape(date)
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
                    Self::xml_escape(doc_id)
                ));
            }
            if let Some(ref inst_id) = self.instance_id {
                xml.push_str(&format!(
                    "\n      <xmpMM:InstanceID>{}</xmpMM:InstanceID>",
                    Self::xml_escape(inst_id)
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
            .replace('\r', "&#13;")
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

const DC: &str = "http://purl.org/dc/elements/1.1/";
const XMP: &str = "http://ns.adobe.com/xap/1.0/";
const XMP_MM: &str = "http://ns.adobe.com/xap/1.0/mm/";
const PDF: &str = "http://ns.adobe.com/pdf/1.3/";

fn ordinary_property(namespace: Option<&str>, name: &str) -> bool {
    matches!(
        (namespace, name),
        (Some(DC), "title" | "creator" | "description" | "subject")
            | (Some(XMP), "CreateDate" | "ModifyDate" | "CreatorTool")
            | (Some(XMP_MM), "DocumentID" | "InstanceID")
            | (Some(PDF), "Keywords")
    )
}

fn record<T: PartialEq>(target: &mut Option<T>, value: T) -> PdfAResult<()> {
    if target.as_ref().is_some_and(|previous| previous != &value) {
        return Err(xmp_packet::invalid("conflicting XMP property values"));
    }
    *target = Some(value);
    Ok(())
}

// Preserve decoded literal text, including whitespace and empty list entries.
// Language alternatives select x-default, falling back to the first alternative.
fn property_values(
    node: roxmltree::Node<'_, '_>,
    list: bool,
    alternative: bool,
) -> PdfAResult<Vec<String>> {
    let elements: Vec<_> = node.children().filter(|n| n.is_element()).collect();
    if elements.is_empty() {
        return Ok(vec![xmp_packet::literal(node)?]);
    }
    if elements.len() != 1
        || node
            .children()
            .filter(|n| n.is_text())
            .any(|n| !n.text().unwrap_or("").trim().is_empty())
        || node
            .attributes()
            .any(|a| a.namespace() == Some(xmp_packet::RDF))
    {
        return Err(xmp_packet::invalid("unsupported XMP property structure"));
    }
    let container = elements[0];
    let alt = alternative && container.has_tag_name((xmp_packet::RDF, "Alt"));
    if !alt
        && !(list
            && (container.has_tag_name((xmp_packet::RDF, "Seq"))
                || container.has_tag_name((xmp_packet::RDF, "Bag"))))
    {
        return Err(xmp_packet::invalid("unsupported XMP property container"));
    }
    if container
        .attributes()
        .any(|a| a.namespace() == Some(xmp_packet::RDF))
        || container
            .children()
            .filter(|n| n.is_text())
            .any(|n| !n.text().unwrap_or("").trim().is_empty())
    {
        return Err(xmp_packet::invalid("unsupported XMP container content"));
    }
    let mut values = Vec::new();
    let mut default = None;
    for item in container.children().filter(|n| n.is_element()) {
        if !item.has_tag_name((xmp_packet::RDF, "li")) {
            return Err(xmp_packet::invalid("XMP container requires RDF list items"));
        }
        let value = xmp_packet::literal(item)?;
        if alt && item.attribute((roxmltree::NS_XML_URI, "lang")) == Some("x-default") {
            record(&mut default, value.clone())?;
        }
        values.push(value);
    }
    if alt {
        let selected = default
            .or_else(|| values.into_iter().next())
            .ok_or_else(|| xmp_packet::invalid("empty XMP language alternative"))?;
        Ok(vec![selected])
    } else {
        Ok(values)
    }
}

fn parse_properties(
    document: &roxmltree::Document<'_>,
    metadata: &mut XmpMetadata,
) -> PdfAResult<()> {
    let mut subject: Option<String> = None;
    let mut pdf_keywords = None;
    let mut dc_keywords = None;
    let mut apply = |owner: roxmltree::Node<'_, '_>,
                     namespace: Option<&str>,
                     name: &str,
                     node: Option<roxmltree::Node<'_, '_>>,
                     attribute: Option<&str>|
     -> PdfAResult<()> {
        let ordinary = ordinary_property(namespace, name);
        let identifier = namespace == Some(xmp_packet::PDFA_ID)
            && matches!(name, "part" | "conformance" | "amd" | "corr");
        if !ordinary && !identifier {
            return Ok(());
        }
        xmp_packet::require_document_subject(owner)?;
        let current = owner.attribute((xmp_packet::RDF, "about")).unwrap_or("");
        if subject
            .as_deref()
            .is_some_and(|previous| previous != current)
        {
            return Err(xmp_packet::invalid(
                "XMP properties describe different RDF subjects",
            ));
        }
        subject = Some(current.to_owned());
        if identifier {
            return Ok(());
        }
        let list = matches!(
            (namespace, name),
            (Some(DC), "creator" | "subject") | (Some(PDF), "Keywords")
        );
        let alternative = matches!((namespace, name), (Some(DC), "title" | "description"));
        let values = if let Some(node) = node {
            property_values(node, list, alternative)?
        } else {
            vec![attribute.unwrap_or("").to_owned()]
        };
        if list {
            match (namespace, name) {
                (Some(DC), "creator") => record(&mut metadata.creator, values)?,
                (Some(DC), "subject") => record(&mut dc_keywords, values)?,
                (Some(PDF), "Keywords") => record(&mut pdf_keywords, values)?,
                _ => unreachable!("list property classification"),
            }
        } else {
            let value = values
                .into_iter()
                .next()
                .ok_or_else(|| xmp_packet::invalid("missing XMP literal"))?;
            match (namespace, name) {
                (Some(DC), "title") => record(&mut metadata.title, value)?,
                (Some(DC), "description") => record(&mut metadata.description, value)?,
                (Some(XMP), "CreateDate") => record(&mut metadata.create_date, value)?,
                (Some(XMP), "ModifyDate") => record(&mut metadata.modify_date, value)?,
                (Some(XMP), "CreatorTool") => record(&mut metadata.creator_tool, value)?,
                (Some(XMP_MM), "DocumentID") => record(&mut metadata.document_id, value)?,
                (Some(XMP_MM), "InstanceID") => record(&mut metadata.instance_id, value)?,
                _ => unreachable!("scalar property classification"),
            }
        }
        Ok(())
    };
    for node in document.descendants().filter(|n| n.is_element()) {
        if let Some(parent) = node.parent() {
            apply(
                parent,
                node.tag_name().namespace(),
                node.tag_name().name(),
                Some(node),
                None,
            )?;
        }
        for attribute in node.attributes() {
            apply(
                node,
                attribute.namespace(),
                attribute.name(),
                None,
                Some(attribute.value()),
            )?;
        }
    }
    // Retain the historical preference for PDF Keywords when both vocabularies occur.
    metadata.keywords = pdf_keywords.or(dc_keywords);
    Ok(())
}

fn parse_identifier(document: &roxmltree::Document<'_>) -> PdfAResult<Option<XmpPdfAIdentifier>> {
    let mut values = std::collections::BTreeMap::<String, String>::new();
    let mut subject: Option<String> = None;
    let mut check_subject = |node: roxmltree::Node<'_, '_>| -> PdfAResult<()> {
        xmp_packet::require_document_subject(node)?;
        let current = node.attribute((xmp_packet::RDF, "about")).unwrap_or("");
        if subject
            .as_deref()
            .is_some_and(|previous| previous != current)
        {
            return Err(xmp_packet::invalid(
                "PDF/A properties describe different RDF subjects",
            ));
        }
        subject = Some(current.to_owned());
        Ok(())
    };
    let mut record = |name: &str, value: String| -> PdfAResult<()> {
        if let Some(previous) = values.get(name) {
            if previous != &value {
                return Err(xmp_packet::invalid(format!("conflicting PDF/A {name}")));
            }
        } else {
            values.insert(name.to_owned(), value);
        }
        Ok(())
    };
    let known = |namespace, name| {
        namespace == Some(xmp_packet::PDFA_ID)
            && matches!(name, "part" | "conformance" | "amd" | "corr")
    };
    for node in document.descendants().filter(|n| n.is_element()) {
        if known(node.tag_name().namespace(), node.tag_name().name()) {
            let parent = node
                .parent()
                .ok_or_else(|| xmp_packet::invalid("missing RDF description"))?;
            check_subject(parent)?;
            record(node.tag_name().name(), xmp_packet::literal(node)?)?;
        }
        for attribute in node.attributes() {
            if known(attribute.namespace(), attribute.name()) {
                check_subject(node)?;
                record(attribute.name(), attribute.value().to_owned())?;
            }
        }
    }
    if values.is_empty() {
        return Ok(None);
    }
    let part = values
        .get("part")
        .ok_or_else(|| xmp_packet::invalid("missing PDF/A part"))?
        .trim()
        .parse::<u8>()
        .map_err(|_| xmp_packet::invalid("invalid PDF/A part"))?;
    let conformance = PdfAConformance::from_str(
        values
            .get("conformance")
            .ok_or_else(|| xmp_packet::invalid("missing PDF/A conformance"))?
            .trim(),
    )
    .map_err(xmp_packet::invalid)?;
    if !(1..=3).contains(&part) || (part == 1 && conformance == PdfAConformance::U) {
        return Err(xmp_packet::invalid("unsupported PDF/A part/conformance"));
    }
    Ok(Some(XmpPdfAIdentifier {
        part,
        conformance,
        amd: values.remove("amd"),
        corr: values.remove("corr"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(properties: &str) -> String {
        format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">{properties}</rdf:Description></rdf:RDF>"#
        )
    }

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
        let metadata = XmpMetadata::parse(&packet(xml)).unwrap();
        assert_eq!(metadata.title.as_deref(), Some("Test Title"));
    }

    #[test]
    fn test_xmp_metadata_parse_pdfa_id() {
        let xml = r#"
            <pdfaid:part>1</pdfaid:part>
            <pdfaid:conformance>B</pdfaid:conformance>
        "#;
        let metadata = XmpMetadata::parse(&packet(xml)).unwrap();
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
        let metadata = XmpMetadata::parse(&packet(xml)).unwrap();
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
        let metadata = XmpMetadata::parse(&packet(xml)).unwrap();
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
        let metadata = XmpMetadata::parse(&packet(xml)).unwrap();
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
