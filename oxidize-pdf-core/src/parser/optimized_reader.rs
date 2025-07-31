//! Optimized PDF Reader with LRU caching
//!
//! This module provides an optimized version of PdfReader that uses
//! an LRU cache instead of unlimited HashMap caching to control memory usage.

use super::header::PdfHeader;
use super::object_stream::ObjectStream;
use super::objects::{PdfDictionary, PdfObject};
use super::stack_safe::StackSafeContext;
use super::trailer::PdfTrailer;
use super::xref::XRefTable;
use super::{ParseError, ParseOptions, ParseResult};
use crate::memory::{LruCache, MemoryOptions, MemoryStats};
use crate::objects::ObjectId;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

/// Optimized PDF reader with LRU caching
pub struct OptimizedPdfReader<R: Read + Seek> {
    reader: BufReader<R>,
    header: PdfHeader,
    xref: XRefTable,
    trailer: PdfTrailer,
    /// LRU cache for loaded objects
    object_cache: LruCache<ObjectId, Arc<PdfObject>>,
    /// Cache of object streams
    object_stream_cache: HashMap<u32, ObjectStream>,
    /// Page tree navigator
    #[allow(dead_code)]
    page_tree: Option<super::page_tree::PageTree>,
    /// Stack-safe parsing context
    #[allow(dead_code)]
    parse_context: StackSafeContext,
    /// Parsing options
    options: super::ParseOptions,
    /// Memory options
    #[allow(dead_code)]
    memory_options: MemoryOptions,
    /// Memory statistics
    memory_stats: MemoryStats,
}

impl<R: Read + Seek> OptimizedPdfReader<R> {
    /// Get parsing options
    pub fn options(&self) -> &super::ParseOptions {
        &self.options
    }

    /// Get memory statistics
    pub fn memory_stats(&self) -> &MemoryStats {
        &self.memory_stats
    }

    /// Clear the object cache
    pub fn clear_cache(&mut self) {
        self.object_cache.clear();
        self.object_stream_cache.clear();
    }
}

impl OptimizedPdfReader<File> {
    /// Open a PDF file from a path with memory optimization
    pub fn open<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let file = File::open(path)?;
        let options = super::ParseOptions::lenient();
        let memory_options = MemoryOptions::default();
        Self::new_with_options(file, options, memory_options)
    }

    /// Open a PDF file with custom memory options
    pub fn open_with_memory<P: AsRef<Path>>(
        path: P,
        memory_options: MemoryOptions,
    ) -> ParseResult<Self> {
        let file = File::open(path)?;
        let options = super::ParseOptions::lenient();
        Self::new_with_options(file, options, memory_options)
    }

    /// Open a PDF file with strict parsing
    pub fn open_strict<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let file = File::open(path)?;
        let options = super::ParseOptions::strict();
        let memory_options = MemoryOptions::default();
        Self::new_with_options(file, options, memory_options)
    }
}

impl<R: Read + Seek> OptimizedPdfReader<R> {
    /// Create a new PDF reader from a reader
    pub fn new(reader: R) -> ParseResult<Self> {
        Self::new_with_options(
            reader,
            super::ParseOptions::default(),
            MemoryOptions::default(),
        )
    }

    /// Create a new PDF reader with custom parsing and memory options
    pub fn new_with_options(
        reader: R,
        options: super::ParseOptions,
        memory_options: MemoryOptions,
    ) -> ParseResult<Self> {
        let mut buf_reader = BufReader::new(reader);

        // Check if file is empty
        let start_pos = buf_reader.stream_position()?;
        buf_reader.seek(SeekFrom::End(0))?;
        let file_size = buf_reader.stream_position()?;
        buf_reader.seek(SeekFrom::Start(start_pos))?;

        if file_size == 0 {
            return Err(ParseError::EmptyFile);
        }

        // Parse header
        let header = PdfHeader::parse(&mut buf_reader)?;

        // Parse xref table
        let xref = XRefTable::parse_with_options(&mut buf_reader, &options)?;

        // Get trailer
        let trailer_dict = xref.trailer().ok_or(ParseError::InvalidTrailer)?.clone();

        let xref_offset = xref.xref_offset();
        let trailer = PdfTrailer::from_dict(trailer_dict, xref_offset)?;

        // Validate trailer
        trailer.validate()?;

        // Create LRU cache with configured size
        let cache_size = memory_options.cache_size.max(1);
        let object_cache = LruCache::new(cache_size);

        Ok(Self {
            reader: buf_reader,
            header,
            xref,
            trailer,
            object_cache,
            object_stream_cache: HashMap::new(),
            page_tree: None,
            parse_context: StackSafeContext::new(),
            options,
            memory_options,
            memory_stats: MemoryStats::default(),
        })
    }

    /// Get the PDF version
    pub fn version(&self) -> &super::header::PdfVersion {
        &self.header.version
    }

    /// Get the document catalog
    pub fn catalog(&mut self) -> ParseResult<&PdfDictionary> {
        // Try to get root from trailer
        let (obj_num, gen_num) = match self.trailer.root() {
            Ok(root) => root,
            Err(_) => {
                // If Root is missing, try fallback methods
                #[cfg(debug_assertions)]
                eprintln!("Warning: Trailer missing Root entry, attempting recovery");

                // First try the fallback method
                if let Some(root) = self.trailer.find_root_fallback() {
                    root
                } else {
                    // Last resort: scan for Catalog object
                    if let Ok(catalog_ref) = self.find_catalog_object() {
                        catalog_ref
                    } else {
                        return Err(ParseError::MissingKey("Root".to_string()));
                    }
                }
            }
        };

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
        let object_id = ObjectId::new(obj_num, gen_num);

        // Check LRU cache first
        if let Some(cached_obj) = self.object_cache.get(&object_id) {
            self.memory_stats.cache_hits += 1;
            // Convert Arc<PdfObject> to &PdfObject
            // This is safe because we maintain the Arc in the cache
            let ptr = Arc::as_ptr(cached_obj);
            return Ok(unsafe { &*ptr });
        }

        self.memory_stats.cache_misses += 1;

        // Load object from disk
        let obj = self.load_object_from_disk(obj_num, gen_num)?;

        // Store in LRU cache
        let arc_obj = Arc::new(obj);
        self.object_cache.put(object_id, arc_obj.clone());
        self.memory_stats.cached_objects = self.object_cache.len();

        // Return reference to cached object
        // The Arc is owned by the cache, so we can safely return a reference
        // We need to get it from the cache to ensure lifetime
        self.object_cache
            .get(&object_id)
            .map(|arc| unsafe { &*Arc::as_ptr(arc) })
            .ok_or(ParseError::SyntaxError {
                position: 0,
                message: "Object not in cache after insertion".to_string(),
            })
    }

    /// Internal method to load an object from disk
    fn load_object_from_disk(&mut self, obj_num: u32, gen_num: u16) -> ParseResult<PdfObject> {
        // Check if this is a compressed object
        if let Some(ext_entry) = self.xref.get_extended_entry(obj_num) {
            if let Some((stream_obj_num, index_in_stream)) = ext_entry.compressed_info {
                // This is a compressed object - need to extract from object stream
                return self.get_compressed_object_direct(
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
            return Ok(PdfObject::Null);
        }

        if entry.generation != gen_num {
            return Err(ParseError::InvalidReference(obj_num, gen_num));
        }

        // Seek to object position
        self.reader.seek(std::io::SeekFrom::Start(entry.offset))?;

        // Parse object header (obj_num gen_num obj)
        let mut lexer =
            super::lexer::Lexer::new_with_options(&mut self.reader, self.options.clone());

        // Read object number with recovery
        let token = lexer.next_token()?;
        let read_obj_num = match token {
            super::lexer::Token::Integer(n) => n as u32,
            _ => {
                // Try fallback recovery
                if self.options.lenient_syntax {
                    if self.options.collect_warnings {
                        eprintln!(
                            "Warning: Using expected object number {obj_num} instead of parsed token"
                        );
                    }
                    obj_num
                } else {
                    return Err(ParseError::SyntaxError {
                        position: entry.offset as usize,
                        message: "Expected object number".to_string(),
                    });
                }
            }
        };

        if read_obj_num != obj_num && !self.options.lenient_syntax {
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
                if self.options.lenient_syntax {
                    if self.options.collect_warnings {
                        eprintln!(
                            "Warning: Using generation 0 instead of parsed token for object {obj_num}"
                        );
                    }
                    0
                } else {
                    return Err(ParseError::SyntaxError {
                        position: entry.offset as usize,
                        message: "Expected generation number".to_string(),
                    });
                }
            }
        };

        if read_gen_num != gen_num && !self.options.lenient_syntax {
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
                if self.options.lenient_syntax {
                    if self.options.collect_warnings {
                        eprintln!("Warning: Missing 'obj' keyword for object {obj_num}");
                    }
                } else {
                    return Err(ParseError::SyntaxError {
                        position: entry.offset as usize,
                        message: "Expected 'obj' keyword".to_string(),
                    });
                }
            }
        }

        // Parse the object
        let object = PdfObject::parse(&mut lexer)?;

        // Skip 'endobj' if present
        if let Ok(token) = lexer.peek_token() {
            if let super::lexer::Token::EndObj = token {
                let _ = lexer.next_token();
            } else if !self.options.lenient_syntax && self.options.collect_warnings {
                eprintln!("Warning: Missing 'endobj' for object {obj_num}");
            }
        }

        Ok(object)
    }

    /// Get a compressed object directly (returns owned object)
    fn get_compressed_object_direct(
        &mut self,
        obj_num: u32,
        _gen_num: u16,
        stream_obj_num: u32,
        _index_in_stream: u32,
    ) -> ParseResult<PdfObject> {
        // First get the object stream
        if !self.object_stream_cache.contains_key(&stream_obj_num) {
            // Load the stream object
            let stream_obj = self.load_object_from_disk(stream_obj_num, 0)?;

            if let PdfObject::Stream(stream) = stream_obj {
                let obj_stream = ObjectStream::parse(stream, &ParseOptions::default())?;
                self.object_stream_cache.insert(stream_obj_num, obj_stream);
            } else {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Object stream is not a stream object".to_string(),
                });
            }
        }

        // Get object from stream
        let obj_stream = self
            .object_stream_cache
            .get(&stream_obj_num)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Object stream not found in cache".to_string(),
            })?;

        obj_stream
            .get_object(obj_num)
            .cloned()
            .ok_or(ParseError::InvalidReference(obj_num, 0))
    }

    /// Find catalog object by scanning (fallback method)
    fn find_catalog_object(&mut self) -> ParseResult<(u32, u16)> {
        // This is a simplified implementation
        // In a real scenario, we would scan through objects to find the catalog
        for obj_num in 1..100 {
            if let Ok(PdfObject::Dictionary(dict)) = self.get_object(obj_num, 0) {
                if let Some(PdfObject::Name(type_name)) = dict.get("Type") {
                    if type_name.0.as_bytes() == b"Catalog" {
                        return Ok((obj_num, 0));
                    }
                }
            }
        }
        Err(ParseError::MissingKey("Catalog".to_string()))
    }

    /// Get a reference to the inner reader
    pub fn reader(&mut self) -> &mut BufReader<R> {
        &mut self.reader
    }
}

/// Helper function to get memory usage info for a PdfObject
pub fn estimate_object_size(obj: &PdfObject) -> usize {
    match obj {
        PdfObject::Null => 8,
        PdfObject::Boolean(_) => 16,
        PdfObject::Integer(_) => 16,
        PdfObject::Real(_) => 16,
        PdfObject::String(s) => 24 + s.as_bytes().len(),
        PdfObject::Name(n) => 24 + n.0.len(),
        PdfObject::Array(arr) => {
            24 + arr.len() * 8 + arr.0.iter().map(estimate_object_size).sum::<usize>()
        }
        PdfObject::Dictionary(dict) => {
            24 + dict.0.len() * 16
                + dict
                    .0
                    .iter()
                    .map(|(k, v)| k.0.len() + estimate_object_size(v))
                    .sum::<usize>()
        }
        PdfObject::Stream(s) => {
            48 + s.data.len() + estimate_object_size(&PdfObject::Dictionary(s.dict.clone()))
        }
        PdfObject::Reference(_, _) => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_options_integration() {
        // This is a placeholder test - would need actual PDF files to test properly
        let options = MemoryOptions::default().with_cache_size(100);
        assert_eq!(options.cache_size, 100);
    }

    #[test]
    fn test_object_size_estimation() {
        let obj = PdfObject::Integer(42);
        assert_eq!(estimate_object_size(&obj), 16);

        let obj = PdfObject::String(crate::parser::PdfString::new(b"Hello".to_vec()));
        assert_eq!(estimate_object_size(&obj), 24 + 5);
    }
}
