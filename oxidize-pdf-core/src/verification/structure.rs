//! Structural gate for GeneratesPdf. Rendering and full ISO validation are separate.
use crate::parser::{ParseOptions, PdfDictionary, PdfObject, PdfReader};
use std::collections::HashSet;
use std::io::Cursor;

type Reference = (u32, u16);

// Work budgets, not PDF validity rules. Exhaustion reports verification incomplete.
const MAX_TREE_NODES: usize = 100_000;
const MAX_TREE_DEPTH: usize = 256;

fn invalid(reason: &str) -> String {
    format!("PDF structural check failed: {reason}")
}

fn parse_error(error: crate::parser::ParseError) -> String {
    format!("PDF structural check could not verify input: {error}")
}

pub(super) fn verify(bytes: &[u8]) -> Result<usize, String> {
    let mut reader = PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
        .map_err(parse_error)?;
    reader.disable_object_reconstruction();
    let root = reader.trailer().root().map_err(parse_error)?;
    // Do not use catalog()/pages(): those navigation APIs deliberately repair input.
    let catalog = dictionary(&mut reader, root)?;
    if name(catalog, "Type") != Some("Catalog") {
        return Err(invalid("trailer Root must reference a Catalog dictionary"));
    }
    let pages = reference(catalog, "Pages")?;
    verify_page_tree(&mut reader, pages, MAX_TREE_NODES, MAX_TREE_DEPTH)
}

fn dictionary<'a>(
    reader: &'a mut PdfReader<Cursor<&[u8]>>,
    reference: Reference,
) -> Result<&'a PdfDictionary, String> {
    reader
        .get_object(reference.0, reference.1)
        .map_err(parse_error)?
        .as_dict()
        .ok_or_else(|| invalid("structural reference is not a dictionary"))
}

fn name<'a>(dictionary: &'a PdfDictionary, key: &str) -> Option<&'a str> {
    dictionary
        .get(key)
        .and_then(PdfObject::as_name)
        .map(|n| n.as_str())
}

fn reference(dictionary: &PdfDictionary, key: &str) -> Result<Reference, String> {
    dictionary
        .get(key)
        .and_then(PdfObject::as_reference)
        .ok_or_else(|| invalid(&format!("{key} must be an indirect reference")))
}

enum Visit {
    Enter {
        node: Reference,
        parent: Option<Reference>,
        depth: usize,
    },
    Leave {
        expected: usize,
        before: usize,
    },
}

fn verify_page_tree(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    root: Reference,
    max_nodes: usize,
    max_depth: usize,
) -> Result<usize, String> {
    let mut work = vec![Visit::Enter {
        node: root,
        parent: None,
        depth: 0,
    }];
    let mut seen = HashSet::new();
    let mut pages = 0usize;
    let mut scheduled = 1usize;
    while let Some(visit) = work.pop() {
        let (node, parent, depth) = match visit {
            Visit::Leave { expected, before } => {
                if pages - before != expected {
                    return Err(invalid("Pages Count does not match descendant pages"));
                }
                continue;
            }
            Visit::Enter {
                node,
                parent,
                depth,
            } => (node, parent, depth),
        };
        if depth > max_depth || seen.len() >= max_nodes {
            return Err("PDF verification incomplete: page-tree resource limit exceeded".into());
        }
        if !seen.insert(node) {
            return Err(invalid("cycle or repeated child in page tree"));
        }
        let dict = dictionary(reader, node)?;
        if let Some(parent) = parent {
            if reference(dict, "Parent")? != parent {
                return Err(invalid("page-tree Parent does not match its owner"));
            }
        }
        match name(dict, "Type") {
            Some("Page") if parent.is_some() => pages += 1,
            Some("Pages") => {
                let count = dict
                    .get("Count")
                    .and_then(PdfObject::as_integer)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| invalid("Pages Count must be a nonnegative integer"))?;
                let kids = dict
                    .get("Kids")
                    .and_then(PdfObject::as_array)
                    .ok_or_else(|| invalid("Pages Kids must be an array"))?;
                // Check before allocating work from untrusted children. Do not allocate from Count.
                if kids.len() > max_nodes.saturating_sub(scheduled) {
                    return Err(
                        "PDF verification incomplete: page-tree resource limit exceeded".into(),
                    );
                }
                scheduled += kids.len();
                work.push(Visit::Leave {
                    expected: count,
                    before: pages,
                });
                for child in kids.0.iter().rev() {
                    let child = child
                        .as_reference()
                        .ok_or_else(|| invalid("page-tree Kids must contain references"))?;
                    work.push(Visit::Enter {
                        node: child,
                        parent: Some(node),
                        depth: depth + 1,
                    });
                }
            }
            _ => return Err(invalid("page tree has an invalid Type")),
        }
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traversal_budget_is_incomplete_not_invalid_or_success() {
        let bytes = include_bytes!("../../tests/fixtures/issue_722_tiny.pdf");
        for (nodes, depth) in [(1, 256), (100, 0)] {
            let mut reader =
                PdfReader::new_with_options(Cursor::new(bytes.as_slice()), ParseOptions::strict())
                    .unwrap();
            reader.disable_object_reconstruction();
            let error = verify_page_tree(&mut reader, (2, 0), nodes, depth).unwrap_err();
            assert!(error.contains("verification incomplete"), "{error}");
            assert!(error.contains("resource limit"), "{error}");
        }
    }
}
