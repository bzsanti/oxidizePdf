//! Bounded, namespace-aware XML access shared by PDF/A XMP properties.
use super::error::{PdfAError, PdfAResult};
use roxmltree::{Document, Node, ParsingOptions};

pub(super) const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub(super) const PDFA_ID: &str = "http://www.aiim.org/pdfa/ns/id/";

pub(super) fn invalid(reason: impl std::fmt::Display) -> PdfAError {
    PdfAError::XmpParseError(reason.to_string())
}

pub(super) fn parse(xml: &str) -> PdfAResult<Document<'_>> {
    // Bound input before tree allocation; no resolver/file/network.
    if xml.len() > 16 * 1024 * 1024 {
        return Err(invalid("XMP exceeds the 16 MiB parsing limit"));
    }
    // The tree tokenizer recurses. Check depth with an iterative pull parser first.
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut depth = 0usize;
    loop {
        use quick_xml::events::Event;
        match reader.read_event().map_err(invalid)? {
            Event::Start(_) | Event::Empty(_) if depth >= 128 => {
                return Err(invalid("XMP exceeds the 128-level XML nesting limit"))
            }
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("unexpected XML end tag"))?
            }
            Event::DocType(_) => return Err(invalid("XMP DTDs are not supported")),
            Event::Eof => break,
            _ => {}
        }
    }
    Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            entity_resolver: None,
        },
    )
    .map_err(invalid)
}

pub(super) fn is_description(node: Node<'_, '_>) -> bool {
    node.has_tag_name((RDF, "Description"))
        && node.parent().is_some_and(|parent| {
            parent.has_tag_name((RDF, "RDF"))
                && parent.parent().is_some_and(|outer| {
                    outer.is_root()
                        || (outer.has_tag_name(("adobe:ns:meta/", "xmpmeta"))
                            && outer.parent().is_some_and(|root| root.is_root()))
                })
        })
}

pub(super) fn require_document_subject(node: Node<'_, '_>) -> PdfAResult<()> {
    if node.ancestors().any(|ancestor| {
        ancestor
            .attribute((roxmltree::NS_XML_URI, "base"))
            .is_some()
    }) {
        return Err(invalid("XMP xml:base subject resolution is not supported"));
    }
    if !is_description(node)
        || node.attribute((RDF, "nodeID")).is_some()
        || node.attribute((RDF, "ID")).is_some()
    {
        return Err(invalid("unsupported XMP property subject or placement"));
    }
    Ok(())
}

pub(super) fn literal(node: Node<'_, '_>) -> PdfAResult<String> {
    if node.children().any(|child| child.is_element())
        || node
            .attributes()
            .any(|attribute| attribute.namespace() == Some(RDF) && attribute.name() != "datatype")
    {
        return Err(invalid("XMP property requires a literal value"));
    }
    Ok(node
        .children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect())
}
