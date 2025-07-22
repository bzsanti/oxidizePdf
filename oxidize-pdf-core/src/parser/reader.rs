//! High-level PDF Reader API
//!
//! Provides a simple interface for reading PDF files

use super::header::PdfHeader;
use super::object_stream::ObjectStream;
use super::objects::{PdfDictionary, PdfObject};
use super::trailer::PdfTrailer;
use super::xref::XRefTable;
use super::{ParseError, ParseResult};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

/// High-level PDF reader
pub struct PdfReader<R: Read + Seek> {
    reader: BufReader<R>,
    header: PdfHeader,
    xref: XRefTable,
    trailer: PdfTrailer,
    /// Cache of loaded objects
    object_cache: HashMap<(u32, u16), PdfObject>,
    /// Cache of object streams
    object_stream_cache: HashMap<u32, ObjectStream>,
    /// Page tree navigator
    page_tree: Option<super::page_tree::PageTree>,
}

impl PdfReader<File> {
    /// Open a PDF file from a path
    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let file = File::open(path)?;
        Self::new(file)
    }

    /// Open a PDF file as a PdfDocument
    pub fn open_document<P: AsRef<Path>>(
        path: P,
    ) -> ParseResult<super::document::PdfDocument<File>> {
        let reader = Self::open(path)?;
        Ok(reader.into_document())
    }
}

impl<R: Read + Seek> PdfReader<R> {
    /// Create a new PDF reader from a reader
    pub fn new(reader: R) -> ParseResult<Self> {
        let mut buf_reader = BufReader::new(reader);

        // Parse header
        let header = PdfHeader::parse(&mut buf_reader)?;
        // Parse xref table
        let xref = XRefTable::parse(&mut buf_reader)?;

        // Get trailer
        let trailer_dict = xref.trailer().ok_or(ParseError::InvalidTrailer)?.clone();

        let xref_offset = xref.xref_offset();
        let trailer = PdfTrailer::from_dict(trailer_dict, xref_offset)?;

        // Validate trailer
        trailer.validate()?;

        Ok(Self {
            reader: buf_reader,
            header,
            xref,
            trailer,
            object_cache: HashMap::new(),
            object_stream_cache: HashMap::new(),
            page_tree: None,
        })
    }

    /// Get the PDF version
    pub fn version(&self) -> &super::header::PdfVersion {
        &self.header.version
    }

    /// Get the document catalog
    pub fn catalog(&mut self) -> ParseResult<&PdfDictionary> {
        let (obj_num, gen_num) = self.trailer.root()?;
        let catalog = self.get_object(obj_num, gen_num)?;

        catalog.as_dict().ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Catalog is not a dictionary".to_string(),
        })
    }

    /// Get the document info dictionary
    pub fn info(&mut self) -> ParseResult<Option<&PdfDictionary>> {
        match self.trailer.info() {
            Some((obj_num, gen_num)) => {
                let info = self.get_object(obj_num, gen_num)?;
                Ok(info.as_dict())
            }
            None => Ok(None),
        }
    }

    /// Get an object by reference
    pub fn get_object(&mut self, obj_num: u32, gen_num: u16) -> ParseResult<&PdfObject> {
        let key = (obj_num, gen_num);

        // Check cache first
        if self.object_cache.contains_key(&key) {
            return Ok(&self.object_cache[&key]);
        }

        // Check if this is a compressed object
        if let Some(ext_entry) = self.xref.get_extended_entry(obj_num) {
            if let Some((stream_obj_num, index_in_stream)) = ext_entry.compressed_info {
                // This is a compressed object - need to extract from object stream
                return self.get_compressed_object(
                    obj_num,
                    gen_num,
                    stream_obj_num,
                    index_in_stream,
                );
            }
        }

        // Get xref entry
        let entry = self
            .xref
            .get_entry(obj_num)
            .ok_or(ParseError::InvalidReference(obj_num, gen_num))?;

        if !entry.in_use {
            // Free object
            self.object_cache.insert(key, PdfObject::Null);
            return Ok(&self.object_cache[&key]);
        }

        if entry.generation != gen_num {
            return Err(ParseError::InvalidReference(obj_num, gen_num));
        }

        // Seek to object position
        self.reader.seek(std::io::SeekFrom::Start(entry.offset))?;

        // Parse object header (obj_num gen_num obj)
        let mut lexer = super::lexer::Lexer::new(&mut self.reader);

        // Read object number
        let token = lexer.next_token()?;
        let read_obj_num = match token {
            super::lexer::Token::Integer(n) => n as u32,
            _ => {
                return Err(ParseError::SyntaxError {
                    position: entry.offset as usize,
                    message: "Expected object number".to_string(),
                })
            }
        };

        if read_obj_num != obj_num {
            return Err(ParseError::SyntaxError {
                position: entry.offset as usize,
                message: format!(
                    "Object number mismatch: expected {obj_num}, found {read_obj_num}"
                ),
            });
        }

        // Read generation number
        let token = lexer.next_token()?;
        let read_gen_num = match token {
            super::lexer::Token::Integer(n) => n as u16,
            _ => {
                return Err(ParseError::SyntaxError {
                    position: entry.offset as usize,
                    message: "Expected generation number".to_string(),
                })
            }
        };

        if read_gen_num != gen_num {
            return Err(ParseError::SyntaxError {
                position: entry.offset as usize,
                message: format!(
                    "Generation number mismatch: expected {gen_num}, found {read_gen_num}"
                ),
            });
        }

        // Read 'obj' keyword
        let token = lexer.next_token()?;
        match token {
            super::lexer::Token::Obj => {}
            _ => {
                return Err(ParseError::SyntaxError {
                    position: entry.offset as usize,
                    message: "Expected 'obj' keyword".to_string(),
                })
            }
        };

        // Parse the actual object
        let obj = PdfObject::parse(&mut lexer)?;

        // Read 'endobj' keyword
        let token = lexer.next_token()?;
        match token {
            super::lexer::Token::EndObj => {}
            _ => {
                return Err(ParseError::SyntaxError {
                    position: entry.offset as usize,
                    message: "Expected 'endobj' keyword".to_string(),
                })
            }
        };

        // Cache the object
        self.object_cache.insert(key, obj);
        Ok(&self.object_cache[&key])
    }

    /// Resolve a reference to get the actual object
    pub fn resolve<'a>(&'a mut self, obj: &'a PdfObject) -> ParseResult<&'a PdfObject> {
        match obj {
            PdfObject::Reference(obj_num, gen_num) => self.get_object(*obj_num, *gen_num),
            _ => Ok(obj),
        }
    }

    /// Get a compressed object from an object stream
    fn get_compressed_object(
        &mut self,
        obj_num: u32,
        gen_num: u16,
        stream_obj_num: u32,
        _index_in_stream: u32,
    ) -> ParseResult<&PdfObject> {
        let key = (obj_num, gen_num);

        // Load the object stream if not cached
        if !self.object_stream_cache.contains_key(&stream_obj_num) {
            // Get the stream object
            let stream_obj = self.get_object(stream_obj_num, 0)?;

            if let Some(stream) = stream_obj.as_stream() {
                // Parse the object stream
                let obj_stream = ObjectStream::parse(stream.clone())?;
                self.object_stream_cache.insert(stream_obj_num, obj_stream);
            } else {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: format!("Object {stream_obj_num} is not a stream"),
                });
            }
        }

        // Get the object from the stream
        let obj_stream = &self.object_stream_cache[&stream_obj_num];
        let obj = obj_stream
            .get_object(obj_num)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: format!("Object {obj_num} not found in object stream {stream_obj_num}"),
            })?;

        // Cache the object
        self.object_cache.insert(key, obj.clone());
        Ok(&self.object_cache[&key])
    }

    /// Get the page tree root
    pub fn pages(&mut self) -> ParseResult<&PdfDictionary> {
        // Get the pages reference from catalog first
        let (pages_obj_num, pages_gen_num) = {
            let catalog = self.catalog()?;
            let pages_ref = catalog
                .get("Pages")
                .ok_or_else(|| ParseError::MissingKey("Pages".to_string()))?;

            match pages_ref {
                PdfObject::Reference(obj_num, gen_num) => (*obj_num, *gen_num),
                _ => {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Pages must be a reference".to_string(),
                    })
                }
            }
        };

        // Now we can get the pages object without holding a reference to catalog
        let pages_obj = self.get_object(pages_obj_num, pages_gen_num)?;
        pages_obj.as_dict().ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Pages is not a dictionary".to_string(),
        })
    }

    /// Get the number of pages
    pub fn page_count(&mut self) -> ParseResult<u32> {
        let pages = self.pages()?;
        pages
            .get("Count")
            .and_then(|obj| obj.as_integer())
            .map(|count| count as u32)
            .ok_or_else(|| ParseError::MissingKey("Count".to_string()))
    }

    /// Get metadata from the document
    pub fn metadata(&mut self) -> ParseResult<DocumentMetadata> {
        let mut metadata = DocumentMetadata::default();

        if let Some(info_dict) = self.info()? {
            if let Some(title) = info_dict.get("Title").and_then(|o| o.as_string()) {
                metadata.title = title.as_str().ok().map(|s| s.to_string());
            }
            if let Some(author) = info_dict.get("Author").and_then(|o| o.as_string()) {
                metadata.author = author.as_str().ok().map(|s| s.to_string());
            }
            if let Some(subject) = info_dict.get("Subject").and_then(|o| o.as_string()) {
                metadata.subject = subject.as_str().ok().map(|s| s.to_string());
            }
            if let Some(keywords) = info_dict.get("Keywords").and_then(|o| o.as_string()) {
                metadata.keywords = keywords.as_str().ok().map(|s| s.to_string());
            }
            if let Some(creator) = info_dict.get("Creator").and_then(|o| o.as_string()) {
                metadata.creator = creator.as_str().ok().map(|s| s.to_string());
            }
            if let Some(producer) = info_dict.get("Producer").and_then(|o| o.as_string()) {
                metadata.producer = producer.as_str().ok().map(|s| s.to_string());
            }
        }

        metadata.version = self.version().to_string();
        metadata.page_count = self.page_count().ok();

        Ok(metadata)
    }

    /// Initialize the page tree navigator if not already done
    fn ensure_page_tree(&mut self) -> ParseResult<()> {
        if self.page_tree.is_none() {
            let page_count = self.page_count()?;
            self.page_tree = Some(super::page_tree::PageTree::new(page_count));
        }
        Ok(())
    }

    /// Get a specific page by index (0-based)
    ///
    /// Note: This method is currently not implemented due to borrow checker constraints.
    /// The page_tree needs mutable access to both itself and the reader, which requires
    /// a redesign of the architecture. Use PdfDocument instead for page access.
    pub fn get_page(&mut self, _index: u32) -> ParseResult<&super::page_tree::ParsedPage> {
        self.ensure_page_tree()?;

        // The page_tree needs mutable access to both itself and the reader
        // This requires a redesign of the architecture to avoid the borrow checker issue
        // For now, users should convert to PdfDocument using into_document() for page access
        Err(ParseError::SyntaxError {
            position: 0,
            message: "get_page not implemented due to borrow checker constraints. Use PdfDocument instead.".to_string(),
        })
    }

    /// Get all pages
    pub fn get_all_pages(&mut self) -> ParseResult<Vec<super::page_tree::ParsedPage>> {
        let page_count = self.page_count()?;
        let mut pages = Vec::with_capacity(page_count as usize);

        for i in 0..page_count {
            let page = self.get_page(i)?.clone();
            pages.push(page);
        }

        Ok(pages)
    }

    /// Convert this reader into a PdfDocument for easier page access
    pub fn into_document(self) -> super::document::PdfDocument<R> {
        super::document::PdfDocument::new(self)
    }
}

/// Document metadata
#[derive(Debug, Default, Clone)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub version: String,
    pub page_count: Option<u32>,
}

pub struct EOLIter<'s> {
    remainder: &'s str,
}
impl<'s> Iterator for EOLIter<'s> {
    type Item = &'s str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remainder.is_empty() {
            return None;
        }

        if let Some((i, sep)) = ["\r\n", "\n", "\r"]
            .iter()
            .filter_map(|&sep| self.remainder.find(sep).map(|i| (i, sep)))
            .min_by_key(|(i, _)| *i)
        {
            let (line, rest) = self.remainder.split_at(i);
            self.remainder = &rest[sep.len()..];
            Some(line)
        } else {
            let line = self.remainder;
            self.remainder = "";
            Some(line)
        }
    }
}
pub trait PDFLines: AsRef<str> {
    fn pdf_lines(&self) -> EOLIter<'_> {
        EOLIter {
            remainder: self.as_ref(),
        }
    }
}
impl PDFLines for &str {}
impl<'a> PDFLines for std::borrow::Cow<'a, str> {}
impl PDFLines for String {}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::parser::objects::{PdfName, PdfString};
    use crate::parser::test_helpers::*;
    use std::io::Cursor;

    #[test]
    fn test_reader_construction() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let result = PdfReader::new(cursor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reader_version() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();
        assert_eq!(reader.version().major, 1);
        assert_eq!(reader.version().minor, 4);
    }

    #[test]
    fn test_reader_different_versions() {
        let versions = vec![
            "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "2.0",
        ];

        for version in versions {
            let pdf_data = create_pdf_with_version(version);
            let cursor = Cursor::new(pdf_data);
            let reader = PdfReader::new(cursor).unwrap();

            let parts: Vec<&str> = version.split('.').collect();
            assert_eq!(reader.version().major, parts[0].parse::<u8>().unwrap());
            assert_eq!(reader.version().minor, parts[1].parse::<u8>().unwrap());
        }
    }

    #[test]
    fn test_reader_catalog() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let catalog = reader.catalog();
        assert!(catalog.is_ok());

        let catalog_dict = catalog.unwrap();
        assert_eq!(
            catalog_dict.get("Type"),
            Some(&PdfObject::Name(PdfName("Catalog".to_string())))
        );
    }

    #[test]
    fn test_reader_info_none() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let info = reader.info().unwrap();
        assert!(info.is_none());
    }

    #[test]
    fn test_reader_info_present() {
        let pdf_data = create_pdf_with_info();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let info = reader.info().unwrap();
        assert!(info.is_some());

        let info_dict = info.unwrap();
        assert_eq!(
            info_dict.get("Title"),
            Some(&PdfObject::String(PdfString(
                "Test PDF".to_string().into_bytes()
            )))
        );
        assert_eq!(
            info_dict.get("Author"),
            Some(&PdfObject::String(PdfString(
                "Test Author".to_string().into_bytes()
            )))
        );
    }

    #[test]
    fn test_reader_get_object() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Get catalog object (1 0 obj)
        let obj = reader.get_object(1, 0);
        assert!(obj.is_ok());

        let catalog = obj.unwrap();
        assert!(catalog.as_dict().is_some());
    }

    #[test]
    fn test_reader_get_invalid_object() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Try to get non-existent object
        let obj = reader.get_object(999, 0);
        assert!(obj.is_err());
    }

    #[test]
    fn test_reader_get_free_object() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Object 0 is always free (f flag in xref)
        let obj = reader.get_object(0, 65535);
        assert!(obj.is_ok());
        assert_eq!(obj.unwrap(), &PdfObject::Null);
    }

    #[test]
    fn test_reader_resolve_reference() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Create a reference to catalog
        let ref_obj = PdfObject::Reference(1, 0);
        let resolved = reader.resolve(&ref_obj);

        assert!(resolved.is_ok());
        assert!(resolved.unwrap().as_dict().is_some());
    }

    #[test]
    fn test_reader_resolve_non_reference() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Resolve a non-reference object
        let int_obj = PdfObject::Integer(42);
        let resolved = reader.resolve(&int_obj).unwrap();

        assert_eq!(resolved, &PdfObject::Integer(42));
    }

    #[test]
    fn test_reader_cache_behavior() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Get object first time
        let obj1 = reader.get_object(1, 0).unwrap();
        assert!(obj1.as_dict().is_some());

        // Get same object again - should use cache
        let obj2 = reader.get_object(1, 0).unwrap();
        assert!(obj2.as_dict().is_some());
    }

    #[test]
    fn test_reader_wrong_generation() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Try to get object with wrong generation number
        let obj = reader.get_object(1, 99);
        assert!(obj.is_err());
    }

    #[test]
    fn test_reader_invalid_pdf() {
        let invalid_data = b"This is not a PDF file";
        let cursor = Cursor::new(invalid_data.to_vec());
        let result = PdfReader::new(cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_reader_corrupt_xref() {
        let corrupt_pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
corrupted xref table
trailer
<< /Size 2 /Root 1 0 R >>
startxref
24
%%EOF"
            .to_vec();

        let cursor = Cursor::new(corrupt_pdf);
        let result = PdfReader::new(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_reader_missing_trailer() {
        let pdf_no_trailer = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f 
0000000009 00000 n 
startxref
24
%%EOF"
            .to_vec();

        let cursor = Cursor::new(pdf_no_trailer);
        let result = PdfReader::new(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_reader_empty_pdf() {
        let cursor = Cursor::new(Vec::new());
        let result = PdfReader::new(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_reader_page_count() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let count = reader.page_count();
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 0); // Minimal PDF has no pages
    }

    #[test]
    fn test_reader_into_document() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let reader = PdfReader::new(cursor).unwrap();

        let document = reader.into_document();
        // Document should be valid
        let page_count = document.page_count();
        assert!(page_count.is_ok());
    }

    #[test]
    fn test_reader_pages_dict() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let pages = reader.pages();
        assert!(pages.is_ok());
        let pages_dict = pages.unwrap();
        assert_eq!(
            pages_dict.get("Type"),
            Some(&PdfObject::Name(PdfName("Pages".to_string())))
        );
    }

    #[test]
    fn test_reader_pdf_with_binary_data() {
        let pdf_data = create_pdf_with_binary_marker();

        let cursor = Cursor::new(pdf_data);
        let result = PdfReader::new(cursor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reader_metadata() {
        let pdf_data = create_pdf_with_info();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let metadata = reader.metadata().unwrap();
        assert_eq!(metadata.title, Some("Test PDF".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.subject, Some("Testing".to_string()));
        assert_eq!(metadata.version, "1.4".to_string());
    }

    #[test]
    fn test_reader_metadata_empty() {
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        let metadata = reader.metadata().unwrap();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert_eq!(metadata.version, "1.4".to_string());
        assert_eq!(metadata.page_count, Some(0));
    }

    #[test]
    fn test_reader_object_number_mismatch() {
        // This test validates that the reader properly handles
        // object number mismatches. We'll create a valid PDF
        // and then try to access an object with wrong generation number
        let pdf_data = create_minimal_pdf();
        let cursor = Cursor::new(pdf_data);
        let mut reader = PdfReader::new(cursor).unwrap();

        // Object 1 exists with generation 0
        // Try to get it with wrong generation number
        let result = reader.get_object(1, 99);
        assert!(result.is_err());

        // Also test with a non-existent object number
        let result2 = reader.get_object(999, 0);
        assert!(result2.is_err());
    }

    #[test]
    fn test_document_metadata_struct() {
        let metadata = DocumentMetadata {
            title: Some("Title".to_string()),
            author: Some("Author".to_string()),
            subject: Some("Subject".to_string()),
            keywords: Some("Keywords".to_string()),
            creator: Some("Creator".to_string()),
            producer: Some("Producer".to_string()),
            creation_date: Some("D:20240101".to_string()),
            modification_date: Some("D:20240102".to_string()),
            version: "1.5".to_string(),
            page_count: Some(10),
        };

        assert_eq!(metadata.title, Some("Title".to_string()));
        assert_eq!(metadata.page_count, Some(10));
    }

    #[test]
    fn test_document_metadata_default() {
        let metadata = DocumentMetadata::default();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.subject.is_none());
        assert!(metadata.keywords.is_none());
        assert!(metadata.creator.is_none());
        assert!(metadata.producer.is_none());
        assert!(metadata.creation_date.is_none());
        assert!(metadata.modification_date.is_none());
        assert_eq!(metadata.version, "".to_string());
        assert!(metadata.page_count.is_none());
    }

    #[test]
    fn test_document_metadata_clone() {
        let metadata = DocumentMetadata {
            title: Some("Test".to_string()),
            version: "1.4".to_string(),
            ..Default::default()
        };

        let cloned = metadata.clone();
        assert_eq!(cloned.title, Some("Test".to_string()));
        assert_eq!(cloned.version, "1.4".to_string());
    }

    #[test]
    fn test_reader_trailer_validation_error() {
        // PDF with invalid trailer (missing required keys)
        let bad_pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f 
0000000009 00000 n 
trailer
<< /Size 2 >>
startxref
46
%%EOF"
            .to_vec();

        let cursor = Cursor::new(bad_pdf);
        let result = PdfReader::new(cursor);
        assert!(result.is_err()); // Should fail because trailer is missing /Root
    }
}
