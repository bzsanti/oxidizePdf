//! Signature detection in PDF documents

use super::error::{SignatureError, SignatureResult};
use super::types::{ByteRange, SignatureField};
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::parser::PdfReader;
use std::io::{Read, Seek};

/// Detects all signature fields in a PDF document
///
/// This function searches the document's AcroForm for signature fields
/// (fields with /FT /Sig) and extracts their signature dictionaries.
///
/// # Arguments
///
/// * `reader` - A PdfReader instance for the document
///
/// # Returns
///
/// A vector of SignatureField structs, one for each signature found.
/// Returns an empty vector if the document has no signatures.
///
/// # Errors
///
/// Returns an error if the PDF structure is malformed in a way that
/// prevents signature detection.
pub fn detect_signature_fields<R: Read + Seek>(
    reader: &mut PdfReader<R>,
) -> SignatureResult<Vec<SignatureField>> {
    let mut signatures = Vec::new();

    // Get the document catalog
    let catalog = match reader.catalog() {
        Ok(cat) => cat.clone(),          // Clone to avoid borrow issues
        Err(_) => return Ok(signatures), // No catalog = no signatures
    };

    // Look for AcroForm in catalog
    let acro_form = match catalog.get("AcroForm") {
        Some(obj) => resolve_object(reader, obj)?,
        None => return Ok(signatures), // No AcroForm = no signatures
    };

    let acro_form_dict = match acro_form {
        PdfObject::Dictionary(d) => d,
        PdfObject::Reference(obj_num, gen_num) => {
            let resolved = reader
                .get_object(obj_num, gen_num)
                .map_err(|e| SignatureError::ParseError {
                    message: e.to_string(),
                })?
                .clone();
            match resolved {
                PdfObject::Dictionary(d) => d,
                _ => return Ok(signatures),
            }
        }
        _ => return Ok(signatures),
    };

    // Get Fields array from AcroForm
    let fields = match acro_form_dict.get("Fields") {
        Some(obj) => resolve_to_array(reader, obj)?,
        None => return Ok(signatures),
    };

    // Recursively search for signature fields
    for field_obj in fields {
        collect_signature_fields(reader, &field_obj, &mut signatures)?;
    }

    Ok(signatures)
}

/// Recursively collects signature fields from a field tree
fn collect_signature_fields<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    field_obj: &PdfObject,
    signatures: &mut Vec<SignatureField>,
) -> SignatureResult<()> {
    let field_dict = match resolve_to_dict(reader, field_obj)? {
        Some(d) => d,
        None => return Ok(()),
    };

    // Check if this is a signature field (/FT /Sig)
    if is_signature_field(&field_dict) {
        if let Some(sig) = extract_signature_field(reader, &field_dict)? {
            signatures.push(sig);
        }
    }

    // Check for child fields (/Kids)
    if let Some(kids_obj) = field_dict.get("Kids") {
        let kids = resolve_to_array(reader, kids_obj)?;
        for kid in kids {
            collect_signature_fields(reader, &kid, signatures)?;
        }
    }

    Ok(())
}

/// Checks if a field dictionary is a signature field
fn is_signature_field(dict: &PdfDictionary) -> bool {
    if let Some(ft) = dict.get("FT") {
        matches!(ft, PdfObject::Name(n) if n.as_str() == "Sig")
    } else {
        false
    }
}

/// Extracts a SignatureField from a signature field dictionary
fn extract_signature_field<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    field_dict: &PdfDictionary,
) -> SignatureResult<Option<SignatureField>> {
    // Get the signature value dictionary (/V entry)
    let sig_dict = match field_dict.get("V") {
        Some(obj) => match resolve_to_dict(reader, obj)? {
            Some(d) => d,
            None => return Ok(None), // No signature value = unsigned field
        },
        None => return Ok(None), // No /V = unsigned field
    };

    // Extract required /Filter
    let filter = match sig_dict.get("Filter") {
        Some(PdfObject::Name(n)) => n.as_str().to_string(),
        Some(other) => {
            if let Ok(Some(resolved)) = resolve_to_dict(reader, other) {
                if let Some(PdfObject::Name(n)) = resolved.get("Filter") {
                    n.as_str().to_string()
                } else {
                    return Err(SignatureError::MissingField {
                        field: "Filter".to_string(),
                    });
                }
            } else {
                return Err(SignatureError::MissingField {
                    field: "Filter".to_string(),
                });
            }
        }
        None => {
            return Err(SignatureError::MissingField {
                field: "Filter".to_string(),
            })
        }
    };

    // Extract required /ByteRange
    let byte_range = match sig_dict.get("ByteRange") {
        Some(obj) => extract_byte_range(reader, obj)?,
        None => {
            return Err(SignatureError::MissingField {
                field: "ByteRange".to_string(),
            })
        }
    };

    // Extract required /Contents
    let contents = match sig_dict.get("Contents") {
        Some(obj) => extract_contents(obj)?,
        None => {
            return Err(SignatureError::MissingField {
                field: "Contents".to_string(),
            })
        }
    };

    let mut sig = SignatureField::new(filter, byte_range, contents);

    // Extract optional field name from parent field dict
    if let Some(PdfObject::String(name)) = field_dict.get("T") {
        sig.name = Some(String::from_utf8_lossy(name.as_bytes()).to_string());
    }

    // Extract optional /SubFilter
    if let Some(PdfObject::Name(sf)) = sig_dict.get("SubFilter") {
        sig.sub_filter = Some(sf.as_str().to_string());
    }

    // Extract optional /Reason
    if let Some(PdfObject::String(reason)) = sig_dict.get("Reason") {
        sig.reason = Some(String::from_utf8_lossy(reason.as_bytes()).to_string());
    }

    // Extract optional /Location
    if let Some(PdfObject::String(loc)) = sig_dict.get("Location") {
        sig.location = Some(String::from_utf8_lossy(loc.as_bytes()).to_string());
    }

    // Extract optional /ContactInfo
    if let Some(PdfObject::String(contact)) = sig_dict.get("ContactInfo") {
        sig.contact_info = Some(String::from_utf8_lossy(contact.as_bytes()).to_string());
    }

    // Extract optional /M (signing time)
    if let Some(PdfObject::String(time)) = sig_dict.get("M") {
        sig.signing_time = Some(String::from_utf8_lossy(time.as_bytes()).to_string());
    }

    Ok(Some(sig))
}

/// Extracts ByteRange from an object
fn extract_byte_range<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    obj: &PdfObject,
) -> SignatureResult<ByteRange> {
    let array = resolve_to_array(reader, obj)?;

    let mut values = Vec::with_capacity(array.len());
    for item in array {
        match item {
            PdfObject::Integer(i) => values.push(i as i64),
            PdfObject::Real(r) => values.push(r as i64),
            _ => {
                return Err(SignatureError::InvalidByteRange {
                    details: "ByteRange must contain only numbers".to_string(),
                })
            }
        }
    }

    ByteRange::from_array(&values).map_err(|e| SignatureError::InvalidByteRange { details: e })
}

/// Extracts signature contents from an object
fn extract_contents(obj: &PdfObject) -> SignatureResult<Vec<u8>> {
    match obj {
        PdfObject::String(s) => Ok(s.as_bytes().to_vec()),
        _ => Err(SignatureError::ContentsExtractionFailed {
            details: "Contents must be a string".to_string(),
        }),
    }
}

/// Resolves an object, following references if needed
fn resolve_object<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    obj: &PdfObject,
) -> SignatureResult<PdfObject> {
    match obj {
        PdfObject::Reference(obj_num, gen_num) => reader
            .get_object(*obj_num, *gen_num)
            .map(|o| o.clone())
            .map_err(|e| SignatureError::ParseError {
                message: e.to_string(),
            }),
        _ => Ok(obj.clone()),
    }
}

/// Resolves an object to a dictionary
fn resolve_to_dict<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    obj: &PdfObject,
) -> SignatureResult<Option<PdfDictionary>> {
    let resolved = resolve_object(reader, obj)?;
    match resolved {
        PdfObject::Dictionary(d) => Ok(Some(d)),
        _ => Ok(None),
    }
}

/// Resolves an object to an array
fn resolve_to_array<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    obj: &PdfObject,
) -> SignatureResult<Vec<PdfObject>> {
    let resolved = resolve_object(reader, obj)?;
    match resolved {
        PdfObject::Array(arr) => Ok(arr.0),
        _ => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::{PdfName, PdfString};

    #[test]
    fn test_is_signature_field_true() {
        let mut dict = PdfDictionary::new();
        dict.insert(
            "FT".to_string(),
            PdfObject::Name(PdfName::new("Sig".to_string())),
        );
        assert!(is_signature_field(&dict));
    }

    #[test]
    fn test_is_signature_field_false_text() {
        let mut dict = PdfDictionary::new();
        dict.insert(
            "FT".to_string(),
            PdfObject::Name(PdfName::new("Tx".to_string())),
        );
        assert!(!is_signature_field(&dict));
    }

    #[test]
    fn test_is_signature_field_false_no_ft() {
        let dict = PdfDictionary::new();
        assert!(!is_signature_field(&dict));
    }

    #[test]
    fn test_extract_contents_string() {
        let obj = PdfObject::String(PdfString::new(b"test".to_vec()));
        let result = extract_contents(&obj).unwrap();
        assert_eq!(result, b"test");
    }

    #[test]
    fn test_extract_contents_binary() {
        let obj = PdfObject::String(PdfString::new(vec![0xAB, 0xCD, 0xEF]));
        let result = extract_contents(&obj).unwrap();
        assert_eq!(result, vec![0xAB, 0xCD, 0xEF]);
    }

    #[test]
    fn test_extract_contents_invalid() {
        let obj = PdfObject::Integer(123);
        let result = extract_contents(&obj);
        assert!(result.is_err());
    }
}
