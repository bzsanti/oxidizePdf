//! PDF Cross-Reference Table Parser
//!
//! Parses xref tables according to ISO 32000-1 Section 7.5.4

use super::xref_stream;
use super::xref_types::{XRefEntryInfo, XRefEntryType};
use super::{ParseError, ParseOptions, ParseResult};
use crate::parser::reader::PDFLines;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

/// Cross-reference entry (traditional format)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XRefEntry {
    /// Byte offset in the file
    pub offset: u64,
    /// Generation number
    pub generation: u16,
    /// Whether the object is in use
    pub in_use: bool,
}

/// Extended XRef entry information for compressed objects
#[derive(Debug, Clone, PartialEq)]
pub struct XRefEntryExt {
    /// Basic entry information
    pub basic: XRefEntry,
    /// Additional info for compressed objects
    pub compressed_info: Option<(u32, u32)>, // (stream_obj_num, index_in_stream)
}

/// Cross-reference table
#[derive(Debug, Clone)]
pub struct XRefTable {
    /// Map of object number to xref entry
    entries: HashMap<u32, XRefEntry>,
    /// Extended entries for compressed objects
    extended_entries: HashMap<u32, XRefEntryExt>,
    /// Trailer dictionary
    trailer: Option<super::objects::PdfDictionary>,
    /// Offset of the xref table in the file
    xref_offset: u64,
}

impl Default for XRefTable {
    fn default() -> Self {
        Self::new()
    }
}

impl XRefTable {
    /// Create a new empty xref table
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            extended_entries: HashMap::new(),
            trailer: None,
            xref_offset: 0,
        }
    }

    /// Get all entries in the xref table
    pub fn entries(&self) -> &HashMap<u32, XRefEntry> {
        &self.entries
    }

    /// Parse xref table from a reader with fallback recovery
    pub fn parse<R: Read + Seek>(reader: &mut BufReader<R>) -> ParseResult<Self> {
        Self::parse_with_options(reader, &super::ParseOptions::default())
    }

    /// Parse xref table from a reader with custom options
    pub fn parse_with_options<R: Read + Seek>(
        reader: &mut BufReader<R>,
        options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        // Try normal parsing first
        match Self::parse_with_incremental_updates_options(reader, options) {
            Ok(table) => Ok(table),
            Err(e) => {
                if options.lenient_syntax {
                    eprintln!("Primary XRef parsing failed: {e:?}, attempting recovery");

                    // Reset reader position and try recovery
                    reader.seek(SeekFrom::Start(0))?;
                    Self::parse_with_recovery_options(reader, options)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Parse xref table with support for incremental updates
    #[allow(dead_code)]
    fn parse_with_incremental_updates<R: Read + Seek>(
        reader: &mut BufReader<R>,
    ) -> ParseResult<Self> {
        Self::parse_with_incremental_updates_options(reader, &super::ParseOptions::default())
    }

    /// Parse xref table with support for incremental updates and options
    fn parse_with_incremental_updates_options<R: Read + Seek>(
        reader: &mut BufReader<R>,
        options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        // Find the most recent xref offset
        let xref_offset = Self::find_xref_offset(reader)?;

        // Parse all xref tables in the chain
        let mut merged_table = Self::new();
        let mut current_offset = Some(xref_offset);
        let mut visited_offsets = std::collections::HashSet::new();

        while let Some(offset) = current_offset {
            // Prevent infinite loops
            if visited_offsets.contains(&offset) {
                eprintln!("Circular reference in XRef chain at offset {offset}");
                break;
            }
            visited_offsets.insert(offset);

            // Parse the xref table at this offset
            reader.seek(SeekFrom::Start(offset))?;
            let table = Self::parse_primary_with_options(reader, options)?;

            // Get the previous offset from trailer
            let prev_offset = table
                .trailer
                .as_ref()
                .and_then(|t| t.get("Prev"))
                .and_then(|obj| obj.as_integer())
                .map(|i| i as u64);

            // Merge entries (newer entries override older ones)
            for (obj_num, entry) in table.entries {
                merged_table.entries.entry(obj_num).or_insert(entry);
            }
            for (obj_num, ext_entry) in table.extended_entries {
                merged_table
                    .extended_entries
                    .entry(obj_num)
                    .or_insert(ext_entry);
            }

            // Use the most recent trailer
            if merged_table.trailer.is_none() {
                merged_table.trailer = table.trailer;
                merged_table.xref_offset = table.xref_offset;
            }

            current_offset = prev_offset;
        }

        Ok(merged_table)
    }

    /// Parse xref table from a reader (handles both traditional and stream xrefs)
    #[allow(dead_code)]
    fn parse_primary<R: Read + Seek>(reader: &mut BufReader<R>) -> ParseResult<Self> {
        Self::parse_primary_with_options(reader, &super::ParseOptions::default())
    }

    /// Parse xref table from a reader with options
    fn parse_primary_with_options<R: Read + Seek>(
        reader: &mut BufReader<R>,
        options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        let mut table = Self::new();

        // First, check if this is a linearized PDF with XRef at the beginning
        // Save current position
        let saved_pos = reader.stream_position()?;

        // Check for linearized PDF by looking for the first object
        reader.seek(SeekFrom::Start(0))?;
        if let Ok(xref_offset) = Self::find_linearized_xref(reader) {
            eprintln!("Found linearized PDF with XRef at offset {xref_offset}");

            // Validate offset before using it
            Self::validate_offset(reader, xref_offset)?;

            table.xref_offset = xref_offset;
            reader.seek(SeekFrom::Start(xref_offset))?;
        } else {
            // Restore position and try traditional approach
            reader.seek(SeekFrom::Start(saved_pos))?;

            // Find and parse xref the traditional way
            let xref_offset = Self::find_xref_offset(reader)?;

            // Validate offset before using it
            Self::validate_offset(reader, xref_offset)?;

            table.xref_offset = xref_offset;
            reader.seek(SeekFrom::Start(xref_offset))?;
        }

        // Check if this is a traditional xref table or xref stream
        let mut line = String::new();
        let pos = reader.stream_position()?;
        reader.read_line(&mut line)?;

        if line.trim() == "xref" {
            // Traditional xref table
            Self::parse_traditional_xref_with_options(reader, &mut table, options)?;
        } else {
            eprintln!(
                "Not a traditional xref, checking for xref stream. Line: {:?}",
                line.trim()
            );

            // Might be an xref stream, seek back
            reader.seek(SeekFrom::Start(pos))?;

            // Try to parse as an object
            let mut lexer = super::lexer::Lexer::new_with_options(&mut *reader, options.clone());

            // Read object header
            let obj_num = match lexer.next_token()? {
                super::lexer::Token::Integer(n) => n as u32,
                _ => return Err(ParseError::InvalidXRef),
            };

            eprintln!("Found object {obj_num} at xref position");

            let _gen_num = match lexer.next_token()? {
                super::lexer::Token::Integer(n) => n as u16,
                _ => return Err(ParseError::InvalidXRef),
            };

            match lexer.next_token()? {
                super::lexer::Token::Obj => {}
                _ => return Err(ParseError::InvalidXRef),
            };

            // Parse the object (should be a stream)
            let obj = super::objects::PdfObject::parse_with_options(&mut lexer, options)?;

            if let Some(stream) = obj.as_stream() {
                // Check if it's an xref stream
                if stream
                    .dict
                    .get("Type")
                    .and_then(|o| o.as_name())
                    .map(|n| n.as_str())
                    == Some("XRef")
                {
                    eprintln!("Parsing XRef stream");

                    // Use the new xref_stream module
                    let xref_stream_parser = xref_stream::XRefStream::parse(
                        &mut *reader,
                        stream.dict.clone(),
                        stream.decode(options)?,
                        options,
                    )?;

                    // Convert entries to our format
                    let entries = xref_stream_parser.to_xref_entries()?;
                    eprintln!("XRef stream parsed, found {} entries", entries.len());

                    // Copy entries from xref stream
                    for (obj_num, entry) in entries {
                        match entry {
                            xref_stream::XRefEntry::Free {
                                next_free_object,
                                generation,
                            } => {
                                table.entries.insert(
                                    obj_num,
                                    XRefEntry {
                                        offset: next_free_object as u64,
                                        generation,
                                        in_use: false,
                                    },
                                );
                            }
                            xref_stream::XRefEntry::InUse { offset, generation } => {
                                table.entries.insert(
                                    obj_num,
                                    XRefEntry {
                                        offset,
                                        generation,
                                        in_use: true,
                                    },
                                );
                            }
                            xref_stream::XRefEntry::Compressed {
                                stream_object_number,
                                index_within_stream,
                            } => {
                                // Create extended entry for compressed object
                                let ext_entry = XRefEntryExt {
                                    basic: XRefEntry {
                                        offset: 0,
                                        generation: 0,
                                        in_use: true,
                                    },
                                    compressed_info: Some((
                                        stream_object_number,
                                        index_within_stream,
                                    )),
                                };
                                table.extended_entries.insert(obj_num, ext_entry);
                                table.entries.insert(
                                    obj_num,
                                    XRefEntry {
                                        offset: 0,
                                        generation: 0,
                                        in_use: true,
                                    },
                                );
                            }
                        }
                    }

                    // Set trailer from xref stream
                    table.trailer = Some(xref_stream_parser.trailer_dict().clone());
                } else {
                    return Err(ParseError::InvalidXRef);
                }
            } else {
                return Err(ParseError::InvalidXRef);
            }
        }

        Ok(table)
    }

    /// Parse traditional xref table
    #[allow(dead_code)]
    fn parse_traditional_xref<R: Read + Seek>(
        reader: &mut BufReader<R>,
        table: &mut XRefTable,
    ) -> ParseResult<()> {
        Self::parse_traditional_xref_with_options(reader, table, &super::ParseOptions::default())
    }

    /// Parse traditional xref table with options
    fn parse_traditional_xref_with_options<R: Read + Seek>(
        reader: &mut BufReader<R>,
        table: &mut XRefTable,
        options: &super::ParseOptions,
    ) -> ParseResult<()> {
        let mut line = String::new();

        // Parse subsections
        loop {
            line.clear();
            reader.read_line(&mut line)?;
            let trimmed_line = line.trim();

            // Skip empty lines
            if trimmed_line.is_empty() {
                continue;
            }

            // Check if we've reached the trailer
            if trimmed_line == "trailer" {
                break;
            }

            // Also check if the line looks like a trailer (might have been reached prematurely)
            if trimmed_line.starts_with("<<") {
                eprintln!("Warning: Found trailer dictionary without 'trailer' keyword");
                // Try to parse it as trailer
                break;
            }

            // Parse subsection header (first_obj_num count)
            let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
            if parts.len() != 2 {
                // Invalid subsection header
                return Err(ParseError::InvalidXRef);
            }

            let first_obj_num = parts[0]
                .parse::<u32>()
                .map_err(|_| ParseError::InvalidXRef)?;
            let count = parts[1]
                .parse::<u32>()
                .map_err(|_| ParseError::InvalidXRef)?;

            // Parse entries
            // Parse xref entries
            let mut entries_parsed = 0;
            for i in 0..count {
                line.clear();
                let bytes_read = reader.read_line(&mut line)?;

                // Check if we've hit EOF or trailer prematurely
                if bytes_read == 0 || line.trim() == "trailer" {
                    eprintln!(
                        "Warning: XRef subsection incomplete - expected {count} entries but found only {entries_parsed}"
                    );
                    // Put the "trailer" line back for the next phase
                    if line.trim() == "trailer" {
                        // Can't put it back easily, so we'll handle this case later
                        break;
                    }
                    break;
                }

                match Self::parse_xref_entry(&line) {
                    Ok(entry) => {
                        table.entries.insert(first_obj_num + i, entry);
                        entries_parsed += 1;
                    }
                    Err(_) => {
                        eprintln!(
                            "Warning: Invalid XRef entry at position {}: {:?}",
                            i,
                            line.trim()
                        );
                        // Continue parsing to get as much as possible
                    }
                }
            }
            // Finished parsing xref entries
        }

        // Parse trailer dictionary
        let mut lexer = super::lexer::Lexer::new_with_options(reader, options.clone());
        let trailer_obj = super::objects::PdfObject::parse_with_options(&mut lexer, options)?;
        // Trailer object parsed successfully

        table.trailer = trailer_obj.as_dict().cloned();

        // Validate xref table against trailer Size
        if let Some(trailer) = &table.trailer {
            if let Some(size_obj) = trailer.get("Size") {
                if let Some(expected_size) = size_obj.as_integer() {
                    let actual_entries = table.entries.len() as i64;
                    if actual_entries < expected_size {
                        eprintln!(
                            "Warning: XRef table incomplete - found {actual_entries} entries but trailer indicates {expected_size}. Attempting recovery..."
                        );
                        // Don't fail here, let the recovery mode handle it
                        return Err(ParseError::InvalidXRef);
                    }
                }
            }
        }

        // After parsing the trailer, the reader is positioned after the dictionary
        // We don't need to parse anything else - startxref/offset/%%EOF are handled elsewhere

        Ok(())
    }

    /// Find linearized XRef by checking if there's an XRef stream near the beginning
    fn find_linearized_xref<R: Read + Seek>(reader: &mut BufReader<R>) -> ParseResult<u64> {
        // Skip PDF header
        reader.seek(SeekFrom::Start(0))?;
        let mut header = String::new();
        reader.read_line(&mut header)?;

        if !header.starts_with("%PDF-") {
            return Err(ParseError::InvalidHeader);
        }

        // Skip any binary marker line
        let mut line = String::new();
        reader.read_line(&mut line)?;

        // Now we should be at the first object if this is linearized
        // Read a bit more to check
        let pos = reader.stream_position()?;
        let mut buffer = vec![0u8; 1024];
        let bytes_read = reader.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        let content = String::from_utf8_lossy(&buffer);

        // Look for patterns that indicate a linearized PDF
        // Linearized PDFs typically have a linearization dictionary as the first object
        eprintln!(
            "Checking for linearized PDF, first 100 chars: {:?}",
            &content.chars().take(100).collect::<String>()
        );

        if content.contains("/Linearized") {
            // This is likely a linearized PDF
            // The XRef is usually right after the linearization dictionary
            // Look for either "xref" or an XRef stream object

            // First, try to find "xref" keyword
            if let Some(xref_pos) = content.find("xref") {
                return Ok(pos + xref_pos as u64);
            }

            // Otherwise, look for an XRef stream (object with /Type /XRef)
            if content.contains("/Type/XRef") || content.contains("/Type /XRef") {
                // Need to parse to find the exact position
                // For now, we'll use a heuristic
                if let Some(obj_pos) = content.find(" obj") {
                    // Look for the next object after linearization dict
                    let after_first_obj = &content[obj_pos + 4..];
                    if let Some(next_obj) = after_first_obj.find(" obj") {
                        // Position of second object
                        let second_obj_start = pos + (obj_pos + 4 + next_obj - 10) as u64;
                        return Ok(second_obj_start);
                    }
                }
            }
        }

        Err(ParseError::InvalidXRef)
    }

    /// Find the xref offset by looking for startxref at the end of the file
    fn find_xref_offset<R: Read + Seek>(reader: &mut BufReader<R>) -> ParseResult<u64> {
        // Go to end of file
        reader.seek(SeekFrom::End(0))?;
        let file_size = reader.stream_position()?;

        // Read last 1024 bytes (should be enough for EOL + startxref + offset + %%EOF)
        let read_size = std::cmp::min(1024, file_size);
        reader.seek(SeekFrom::End(-(read_size as i64)))?;

        let mut buffer = vec![0u8; read_size as usize];
        reader.read_exact(&mut buffer)?;

        // Convert to string and find startxref
        let content = String::from_utf8_lossy(&buffer);

        // Debug: print last part of file
        let debug_content = content.chars().take(200).collect::<String>();
        eprintln!("XRef search in last {read_size} bytes: {debug_content:?}");

        let mut lines = content.pdf_lines();

        // Find startxref line - need to iterate forward after finding it
        while let Some(line) = lines.next() {
            if line.trim() == "startxref" {
                // The offset should be on the next line
                if let Some(offset_line) = lines.next() {
                    let offset = offset_line
                        .trim()
                        .parse::<u64>()
                        .map_err(|_| ParseError::InvalidXRef)?;
                    return Ok(offset);
                }
            }
        }

        Err(ParseError::InvalidXRef)
    }

    /// Parse XRef table using recovery mode (scan for objects)
    #[allow(dead_code)]
    fn parse_with_recovery<R: Read + Seek>(reader: &mut BufReader<R>) -> ParseResult<Self> {
        Self::parse_with_recovery_options(reader, &super::ParseOptions::default())
    }

    /// Parse XRef table using recovery mode with options
    fn parse_with_recovery_options<R: Read + Seek>(
        reader: &mut BufReader<R>,
        _options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        let mut table = Self::new();

        // Read entire file into memory for scanning
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let content = String::from_utf8_lossy(&buffer);

        eprintln!("XRef recovery: scanning {} bytes for objects", buffer.len());

        let mut objects_found = 0;
        let mut object_streams = Vec::new();

        // Scan using pattern matching for object headers
        // Also look for patterns like "1 0 obj" at the start of lines
        let mut pos = 0;
        while pos < content.len() {
            // Look for " obj" or check if we're at the start of a line with a number
            let remaining = &content[pos..];

            // Find the next "obj" keyword
            if let Some(obj_pos) = remaining.find("obj") {
                // Make sure it's preceded by whitespace and numbers
                let abs_pos = pos + obj_pos;
                if abs_pos < 4 {
                    pos += obj_pos + 3;
                    continue;
                }

                // Look backwards for the object number and generation
                // Handle both \n and \r as line endings
                let line_start = content[..abs_pos]
                    .rfind(['\n', '\r'])
                    .map(|p| p + 1)
                    .unwrap_or(0);
                let line_end = abs_pos + 3; // Include "obj"

                // Make sure we don't go out of bounds
                if line_end <= content.len() {
                    let line = &content[line_start..line_end];

                    if let Some((obj_num, gen_num)) = Self::parse_obj_header(line.trim()) {
                        let offset = line_start;

                        // Add entry if not already present (avoid duplicates)
                        if !table.entries.contains_key(&obj_num) {
                            table.add_entry(
                                obj_num,
                                XRefEntry {
                                    offset: offset as u64,
                                    generation: gen_num,
                                    in_use: true,
                                },
                            );
                            objects_found += 1;

                            // Check if this might be an object stream
                            let obj_end_pos = line_end;
                            // Use byte operations to avoid UTF-8 boundary issues
                            if obj_end_pos + 200 < buffer.len() {
                                let search_bytes = &buffer[obj_end_pos..obj_end_pos + 200];
                                if let Some(stream_pos) =
                                    search_bytes.windows(6).position(|w| w == b"stream")
                                {
                                    // Check if this is likely an object stream by looking for /Type /ObjStm
                                    let check_bytes =
                                        &buffer[obj_end_pos..obj_end_pos + stream_pos];
                                    let check_str = String::from_utf8_lossy(check_bytes);
                                    if check_str.contains("/Type") && check_str.contains("/ObjStm")
                                    {
                                        object_streams.push(obj_num);
                                        eprintln!(
                                            "XRef recovery: found object stream at object {obj_num}"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                pos = abs_pos + 3;
            } else {
                break;
            }
        }

        eprintln!(
            "XRef recovery: found {} objects and {} object streams",
            objects_found,
            object_streams.len()
        );

        if objects_found == 0 {
            return Err(ParseError::InvalidXRef);
        }

        // Note: In a full implementation, we would parse the object streams
        // to extract compressed objects, but for now we just note their existence

        // Create minimal trailer
        let mut trailer = super::objects::PdfDictionary::new();
        trailer.insert(
            "Size".to_string(),
            super::objects::PdfObject::Integer(table.len() as i64),
        );

        // Try to find Root (Catalog) object by looking for common object numbers
        // This is a heuristic - most PDFs have the catalog at object 1, 2, or 3
        for obj_num in [1, 2, 3, 4, 5] {
            if table.entries.contains_key(&obj_num) {
                // Assume this might be the catalog
                trailer.insert(
                    "Root".to_string(),
                    super::objects::PdfObject::Reference(obj_num, 0),
                );
                break;
            }
        }

        // If no Root found, use object 1 as a last resort
        if !trailer.contains_key("Root") && !table.entries.is_empty() {
            let first_obj = *table.entries.keys().min().unwrap_or(&1);
            trailer.insert(
                "Root".to_string(),
                super::objects::PdfObject::Reference(first_obj, 0),
            );
        }

        table.set_trailer(trailer);

        Ok(table)
    }

    /// Parse object header from line
    fn parse_obj_header(line: &str) -> Option<(u32, u16)> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 3 && parts[2] == "obj" {
            if let (Ok(obj_num), Ok(gen_num)) = (parts[0].parse::<u32>(), parts[1].parse::<u16>()) {
                return Some((obj_num, gen_num));
            }
        }

        None
    }

    /// Validate XRef offset before using it
    fn validate_offset<R: Read + Seek>(reader: &mut BufReader<R>, offset: u64) -> ParseResult<()> {
        // Get file size
        let file_size = reader.seek(SeekFrom::End(0))?;

        if offset >= file_size {
            #[cfg(debug_assertions)]
            eprintln!("Warning: XRef offset {offset} exceeds file size {file_size}");
            return Err(ParseError::InvalidXRef);
        }

        // Check if offset points to valid content
        reader.seek(SeekFrom::Start(offset))?;
        let mut peek = [0u8; 20];
        let read_bytes = reader.read(&mut peek)?;

        if read_bytes == 0 {
            #[cfg(debug_assertions)]
            eprintln!("Warning: XRef offset {offset} points to EOF");
            return Err(ParseError::InvalidXRef);
        }

        // Look for expected XRef markers
        let content = String::from_utf8_lossy(&peek[..read_bytes]);
        if !content.starts_with("xref") && !content.chars().next().unwrap_or(' ').is_ascii_digit() {
            #[cfg(debug_assertions)]
            eprintln!(
                "Warning: XRef offset {} does not point to valid XRef content: {:?}",
                offset,
                &content[..std::cmp::min(10, content.len())]
            );
            // Don't fail here, as some PDFs might have variations
        }

        Ok(())
    }

    /// Parse a single xref entry line (enhanced with flexible parsing)
    fn parse_xref_entry(line: &str) -> ParseResult<XRefEntry> {
        let line = line.trim();

        // First try standard format: nnnnnnnnnn ggggg n/f
        if line.len() >= 18 {
            if let Ok(entry) = Self::parse_xref_entry_standard(line) {
                return Ok(entry);
            }
        }

        // If standard parsing fails, try flexible parsing
        Self::parse_xref_entry_flexible(line)
    }

    /// Parse XRef entry using standard fixed-width format
    fn parse_xref_entry_standard(line: &str) -> ParseResult<XRefEntry> {
        // Entry format: nnnnnnnnnn ggggg n/f
        // Where n = offset (10 digits), g = generation (5 digits), n/f = in use flag
        if line.len() < 18 {
            return Err(ParseError::InvalidXRef);
        }

        let offset_str = &line[0..10];
        let gen_str = &line[11..16];
        let flag = line.chars().nth(17);

        let offset = offset_str
            .trim()
            .parse::<u64>()
            .map_err(|_| ParseError::InvalidXRef)?;
        let generation = gen_str
            .trim()
            .parse::<u16>()
            .map_err(|_| ParseError::InvalidXRef)?;

        let in_use = match flag {
            Some('n') => true,
            Some('f') => false,
            _ => return Err(ParseError::InvalidXRef),
        };

        Ok(XRefEntry {
            offset,
            generation,
            in_use,
        })
    }

    /// Parse XRef entry using flexible whitespace-based format
    fn parse_xref_entry_flexible(line: &str) -> ParseResult<XRefEntry> {
        // Handle variations like:
        // - Extra spaces: "0000000017  00000  n"
        // - Missing spaces: "0000000017 00000n"
        // - Different padding: "17 0 n"
        // - Tabs instead of spaces

        // Split by any whitespace and filter empty parts
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return Err(ParseError::InvalidXRef);
        }

        // Extract offset
        let offset = parts[0]
            .parse::<u64>()
            .map_err(|_| ParseError::InvalidXRef)?;

        // Extract generation (default to 0 if missing)
        let (generation, flag_from_gen) = if parts.len() >= 2 {
            let gen_part = parts[1];
            // Check if flag is attached to generation (e.g., "0n", "1f")
            if gen_part.ends_with('n') || gen_part.ends_with('f') {
                let flag_char = gen_part.chars().last().unwrap();
                let gen_str = &gen_part[..gen_part.len() - 1];
                (gen_str.parse::<u16>().unwrap_or(0), Some(flag_char))
            } else {
                (gen_part.parse::<u16>().unwrap_or(0), None)
            }
        } else {
            (0, None)
        };

        // Extract flag (default to 'n' if missing or invalid)
        let in_use = if let Some(flag_char) = flag_from_gen {
            // Flag was attached to generation
            match flag_char {
                'n' => true,
                'f' => false,
                _ => true, // Default to in-use
            }
        } else if parts.len() >= 3 {
            // Flag is separate
            match parts[2].chars().next() {
                Some('n') => true,
                Some('f') => false,
                _ => {
                    // Unknown flag, log warning in debug mode and assume in-use
                    #[cfg(debug_assertions)]
                    eprintln!("Warning: Invalid xref flag '{}', assuming 'n'", parts[2]);
                    true
                }
            }
        } else {
            // Missing flag, assume in-use
            true
        };

        Ok(XRefEntry {
            offset,
            generation,
            in_use,
        })
    }

    /// Get an xref entry by object number
    pub fn get_entry(&self, obj_num: u32) -> Option<&XRefEntry> {
        self.entries.get(&obj_num)
    }

    /// Get the trailer dictionary
    pub fn trailer(&self) -> Option<&super::objects::PdfDictionary> {
        self.trailer.as_ref()
    }

    /// Get the xref offset
    pub fn xref_offset(&self) -> u64 {
        self.xref_offset
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the table is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &XRefEntry)> {
        self.entries.iter()
    }

    /// Get extended entry information (for compressed objects)
    pub fn get_extended_entry(&self, obj_num: u32) -> Option<&XRefEntryExt> {
        self.extended_entries.get(&obj_num)
    }

    /// Check if an object is compressed
    pub fn is_compressed(&self, obj_num: u32) -> bool {
        self.extended_entries
            .get(&obj_num)
            .map(|e| e.compressed_info.is_some())
            .unwrap_or(false)
    }

    /// Add an entry to the xref table
    pub fn add_entry(&mut self, obj_num: u32, entry: XRefEntry) {
        self.entries.insert(obj_num, entry);
    }

    /// Set the trailer dictionary
    pub fn set_trailer(&mut self, trailer: super::objects::PdfDictionary) {
        self.trailer = Some(trailer);
    }

    /// Add an extended entry to the xref table
    pub fn add_extended_entry(&mut self, obj_num: u32, entry: XRefEntryExt) {
        self.extended_entries.insert(obj_num, entry);
    }
}

/// Cross-reference stream (PDF 1.5+)
/// This is a more compact representation using streams
#[derive(Debug, Clone)]
pub struct XRefStream {
    /// The stream object containing xref data
    stream: super::objects::PdfStream,
    /// Decoded entries
    entries: HashMap<u32, XRefEntry>,
    /// Extended entries for compressed objects
    extended_entries: HashMap<u32, XRefEntryExt>,
}

impl XRefStream {
    /// Parse an xref stream object
    pub fn parse(stream: super::objects::PdfStream) -> ParseResult<Self> {
        let mut xref_stream = Self {
            stream,
            entries: HashMap::new(),
            extended_entries: HashMap::new(),
        };

        xref_stream.decode_entries()?;
        Ok(xref_stream)
    }

    /// Decode the xref stream entries
    fn decode_entries(&mut self) -> ParseResult<()> {
        // Get stream dictionary values
        let dict = &self.stream.dict;

        // Get the Size (number of entries)
        let size = dict
            .get("Size")
            .and_then(|obj| obj.as_integer())
            .ok_or_else(|| ParseError::MissingKey("Size".to_string()))?;

        // Get the Index array [first_obj_num, count, ...]
        let index = match dict.get("Index") {
            Some(obj) => {
                let array = obj.as_array().ok_or_else(|| ParseError::SyntaxError {
                    position: 0,
                    message: "Index must be an array".to_string(),
                })?;

                // Convert to pairs of (first_obj_num, count)
                let mut pairs = Vec::new();
                for chunk in array.0.chunks(2) {
                    if chunk.len() != 2 {
                        return Err(ParseError::SyntaxError {
                            position: 0,
                            message: "Index array must have even number of elements".to_string(),
                        });
                    }
                    let first = chunk[0]
                        .as_integer()
                        .ok_or_else(|| ParseError::SyntaxError {
                            position: 0,
                            message: "Index values must be integers".to_string(),
                        })? as u32;
                    let count = chunk[1]
                        .as_integer()
                        .ok_or_else(|| ParseError::SyntaxError {
                            position: 0,
                            message: "Index values must be integers".to_string(),
                        })? as u32;
                    pairs.push((first, count));
                }
                pairs
            }
            None => {
                // Default: single subsection starting at 0
                vec![(0, size as u32)]
            }
        };

        // Get the W array (field widths)
        let w_array = dict
            .get("W")
            .and_then(|obj| obj.as_array())
            .ok_or_else(|| ParseError::MissingKey("W".to_string()))?;

        if w_array.len() != 3 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "W array must have exactly 3 elements".to_string(),
            });
        }

        let w: Vec<usize> = w_array
            .0
            .iter()
            .map(|obj| {
                obj.as_integer()
                    .ok_or_else(|| ParseError::SyntaxError {
                        position: 0,
                        message: "W values must be integers".to_string(),
                    })
                    .map(|i| i as usize)
            })
            .collect::<ParseResult<Vec<_>>>()?;

        // Decode the stream data
        let data = self.stream.decode(&ParseOptions::default())?;
        let mut offset = 0;

        // Process each subsection
        for (first_obj_num, count) in index {
            for i in 0..count {
                if offset + w[0] + w[1] + w[2] > data.len() {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Xref stream data truncated".to_string(),
                    });
                }

                // Read fields according to widths
                let field1 = Self::read_field(&data[offset..], w[0]);
                offset += w[0];

                let field2 = Self::read_field(&data[offset..], w[1]);
                offset += w[1];

                let field3 = Self::read_field(&data[offset..], w[2]);
                offset += w[2];

                // Parse entry type and create entry info
                let entry_info =
                    XRefEntryInfo::new(XRefEntryType::from_value(field1), field2, field3);

                // Create XRefEntry based on type
                let entry = match entry_info.entry_type {
                    XRefEntryType::Free => XRefEntry {
                        offset: entry_info.field2,
                        generation: entry_info.field3 as u16,
                        in_use: false,
                    },
                    XRefEntryType::Uncompressed => XRefEntry {
                        offset: entry_info.field2,
                        generation: entry_info.field3 as u16,
                        in_use: true,
                    },
                    XRefEntryType::Compressed => {
                        // Store extended info for compressed objects
                        let ext_entry = XRefEntryExt {
                            basic: XRefEntry {
                                offset: 0,
                                generation: 0,
                                in_use: true,
                            },
                            compressed_info: entry_info.get_compressed_info(),
                        };
                        self.extended_entries
                            .insert(first_obj_num + i, ext_entry.clone());
                        ext_entry.basic
                    }
                    XRefEntryType::Custom(_type_num) => {
                        // Custom types are treated as in-use objects
                        // Log only in debug mode to avoid spam
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "Note: Custom xref entry type {} for object {} (treating as in-use)",
                            _type_num,
                            first_obj_num + i
                        );

                        // Store as extended entry with custom type info
                        let ext_entry = XRefEntryExt {
                            basic: XRefEntry {
                                offset: entry_info.field2,
                                generation: entry_info.field3 as u16,
                                in_use: entry_info.entry_type.is_in_use(),
                            },
                            compressed_info: None,
                        };
                        self.extended_entries
                            .insert(first_obj_num + i, ext_entry.clone());
                        ext_entry.basic
                    }
                };

                self.entries.insert(first_obj_num + i, entry);
            }
        }

        Ok(())
    }

    /// Read a field of given width from data
    fn read_field(data: &[u8], width: usize) -> u64 {
        let mut value = 0u64;
        for i in 0..width {
            if i < data.len() {
                value = (value << 8) | (data[i] as u64);
            }
        }
        value
    }

    /// Get an entry by object number
    pub fn get_entry(&self, obj_num: u32) -> Option<&XRefEntry> {
        self.entries.get(&obj_num)
    }

    /// Get the trailer dictionary from the stream
    pub fn trailer(&self) -> &super::objects::PdfDictionary {
        &self.stream.dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::objects::{PdfDictionary, PdfObject};
    use std::io::Cursor;

    #[test]
    fn test_parse_xref_entry() {
        let entry1 = XRefTable::parse_xref_entry("0000000000 65535 f ").unwrap();
        assert_eq!(entry1.offset, 0);
        assert_eq!(entry1.generation, 65535);
        assert!(!entry1.in_use);

        let entry2 = XRefTable::parse_xref_entry("0000000017 00000 n ").unwrap();
        assert_eq!(entry2.offset, 17);
        assert_eq!(entry2.generation, 0);
        assert!(entry2.in_use);
    }

    #[test]
    fn test_parse_xref_entry_flexible() {
        // Test various flexible formats

        // Extra spaces
        let entry1 = XRefTable::parse_xref_entry("17   0   n").unwrap();
        assert_eq!(entry1.offset, 17);
        assert_eq!(entry1.generation, 0);
        assert!(entry1.in_use);

        // Different padding
        let entry2 = XRefTable::parse_xref_entry("123 5 f").unwrap();
        assert_eq!(entry2.offset, 123);
        assert_eq!(entry2.generation, 5);
        assert!(!entry2.in_use);

        // Missing generation (defaults to 0)
        let entry3 = XRefTable::parse_xref_entry("456 n").unwrap();
        assert_eq!(entry3.offset, 456);
        assert_eq!(entry3.generation, 0);
        assert!(entry3.in_use);

        // Missing flag (defaults to true)
        let entry4 = XRefTable::parse_xref_entry("789 2").unwrap();
        assert_eq!(entry4.offset, 789);
        assert_eq!(entry4.generation, 2);
        assert!(entry4.in_use);

        // Flag attached to generation
        let entry5 = XRefTable::parse_xref_entry("1000 0n").unwrap();
        assert_eq!(entry5.offset, 1000);
        assert_eq!(entry5.generation, 0);
        assert!(entry5.in_use);

        let entry6 = XRefTable::parse_xref_entry("2000 1f").unwrap();
        assert_eq!(entry6.offset, 2000);
        assert_eq!(entry6.generation, 1);
        assert!(!entry6.in_use);

        // Tabs instead of spaces
        let entry7 = XRefTable::parse_xref_entry("3000\t0\tn").unwrap();
        assert_eq!(entry7.offset, 3000);
        assert_eq!(entry7.generation, 0);
        assert!(entry7.in_use);
    }

    #[test]
    fn test_parse_xref_entry_invalid_flag_fallback() {
        // Invalid flag should default to 'n' with warning
        let entry = XRefTable::parse_xref_entry("100 0 x").unwrap();
        assert_eq!(entry.offset, 100);
        assert_eq!(entry.generation, 0);
        assert!(entry.in_use); // Should default to true
    }

    #[test]
    fn test_parse_xref_entry_malformed() {
        // Empty line
        let result = XRefTable::parse_xref_entry("");
        assert!(result.is_err());

        // Non-numeric offset
        let result = XRefTable::parse_xref_entry("abc 0 n");
        assert!(result.is_err());

        // Only whitespace
        let result = XRefTable::parse_xref_entry("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_xref_table_new() {
        let table = XRefTable::new();
        assert!(table.entries.is_empty());
        assert!(table.extended_entries.is_empty());
        assert!(table.trailer.is_none());
        assert_eq!(table.xref_offset, 0);
    }

    #[test]
    fn test_xref_table_default() {
        let table = XRefTable::default();
        assert!(table.entries.is_empty());
        assert!(table.extended_entries.is_empty());
        assert!(table.trailer.is_none());
    }

    #[test]
    fn test_xref_entry_struct() {
        let entry = XRefEntry {
            offset: 12345,
            generation: 7,
            in_use: true,
        };
        assert_eq!(entry.offset, 12345);
        assert_eq!(entry.generation, 7);
        assert!(entry.in_use);
    }

    #[test]
    fn test_xref_entry_equality() {
        let entry1 = XRefEntry {
            offset: 100,
            generation: 0,
            in_use: true,
        };
        let entry2 = XRefEntry {
            offset: 100,
            generation: 0,
            in_use: true,
        };
        assert_eq!(entry1, entry2);
    }

    #[test]
    fn test_xref_entry_clone() {
        let entry = XRefEntry {
            offset: 999,
            generation: 3,
            in_use: false,
        };
        let cloned = entry;
        assert_eq!(cloned.offset, 999);
        assert_eq!(cloned.generation, 3);
        assert!(!cloned.in_use);
    }

    #[test]
    fn test_xref_entry_ext() {
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 500,
                generation: 0,
                in_use: true,
            },
            compressed_info: Some((10, 5)),
        };
        assert_eq!(ext_entry.basic.offset, 500);
        assert_eq!(ext_entry.compressed_info, Some((10, 5)));
    }

    #[test]
    fn test_xref_entry_ext_no_compression() {
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 1000,
                generation: 1,
                in_use: true,
            },
            compressed_info: None,
        };
        assert!(ext_entry.compressed_info.is_none());
    }

    #[test]
    fn test_add_entry() {
        let mut table = XRefTable::new();
        table.add_entry(
            5,
            XRefEntry {
                offset: 1000,
                generation: 0,
                in_use: true,
            },
        );
        assert_eq!(table.entries.len(), 1);
        assert!(table.entries.contains_key(&5));
    }

    #[test]
    fn test_get_entry() {
        let mut table = XRefTable::new();
        let entry = XRefEntry {
            offset: 2000,
            generation: 1,
            in_use: true,
        };
        table.add_entry(10, entry);

        let retrieved = table.get_entry(10);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().offset, 2000);

        let missing = table.get_entry(999);
        assert!(missing.is_none());
    }

    #[test]
    fn test_set_trailer() {
        let mut table = XRefTable::new();
        let mut trailer = PdfDictionary::new();
        trailer.insert("Size".to_string(), PdfObject::Integer(10));

        table.set_trailer(trailer.clone());
        assert!(table.trailer.is_some());
        assert_eq!(
            table.trailer().unwrap().get("Size"),
            Some(&PdfObject::Integer(10))
        );
    }

    #[test]
    fn test_parse_xref_entry_invalid() {
        // Too short
        let result = XRefTable::parse_xref_entry("0000000000 65535");
        assert!(result.is_ok()); // Now handled by flexible parsing

        // Invalid format (non-numeric offset)
        let result = XRefTable::parse_xref_entry("not_a_number 65535 f ");
        assert!(result.is_err());

        // Invalid flag (now accepted with warning, defaults to 'n')
        let result = XRefTable::parse_xref_entry("0000000000 65535 x ");
        assert!(result.is_ok()); // Flexible parsing accepts this
        assert!(result.unwrap().in_use); // Should default to true
    }

    #[test]
    fn test_parse_xref_entry_various_offsets() {
        // Small offset
        let entry = XRefTable::parse_xref_entry("0000000001 00000 n ").unwrap();
        assert_eq!(entry.offset, 1);

        // Large offset
        let entry = XRefTable::parse_xref_entry("9999999999 00000 n ").unwrap();
        assert_eq!(entry.offset, 9999999999);

        // Max generation
        let entry = XRefTable::parse_xref_entry("0000000000 65535 f ").unwrap();
        assert_eq!(entry.generation, 65535);
    }

    #[test]
    fn test_add_extended_entry() {
        let mut table = XRefTable::new();
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 0,
                generation: 0,
                in_use: true,
            },
            compressed_info: Some((5, 10)),
        };

        table.add_extended_entry(15, ext_entry.clone());
        assert_eq!(table.extended_entries.len(), 1);
        assert!(table.extended_entries.contains_key(&15));
    }

    #[test]
    fn test_get_extended_entry() {
        let mut table = XRefTable::new();
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 0,
                generation: 0,
                in_use: true,
            },
            compressed_info: Some((20, 3)),
        };

        table.add_extended_entry(7, ext_entry);

        let retrieved = table.get_extended_entry(7);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().compressed_info, Some((20, 3)));
    }

    #[test]
    fn test_xref_offset() {
        let mut table = XRefTable::new();
        assert_eq!(table.xref_offset(), 0);

        table.xref_offset = 12345;
        assert_eq!(table.xref_offset(), 12345);
    }

    #[test]
    fn test_find_xref_offset_simple() {
        let pdf_data = b"startxref\n12345\n%%EOF";
        let cursor = Cursor::new(pdf_data.to_vec());
        let mut reader = BufReader::new(cursor);

        let offset = XRefTable::find_xref_offset(&mut reader).unwrap();
        assert_eq!(offset, 12345);
    }

    #[test]
    fn test_find_xref_offset_with_spaces() {
        let pdf_data = b"startxref  \n  12345  \n%%EOF";
        let cursor = Cursor::new(pdf_data.to_vec());
        let mut reader = BufReader::new(cursor);

        let offset = XRefTable::find_xref_offset(&mut reader).unwrap();
        assert_eq!(offset, 12345);
    }

    #[test]
    fn test_find_xref_offset_missing() {
        let pdf_data = b"no startxref here";
        let cursor = Cursor::new(pdf_data.to_vec());
        let mut reader = BufReader::new(cursor);

        let result = XRefTable::find_xref_offset(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_trailer_getter() {
        let mut table = XRefTable::new();
        assert!(table.trailer().is_none());

        let trailer = PdfDictionary::new();
        table.set_trailer(trailer);
        assert!(table.trailer().is_some());
    }

    #[test]
    fn test_xref_table_clone() {
        let mut table = XRefTable::new();
        table.add_entry(
            1,
            XRefEntry {
                offset: 100,
                generation: 0,
                in_use: true,
            },
        );
        table.xref_offset = 5000;

        let cloned = table.clone();
        assert_eq!(cloned.entries.len(), 1);
        assert_eq!(cloned.xref_offset, 5000);
    }

    #[test]
    fn test_parse_obj_header() {
        // Valid headers
        assert_eq!(XRefTable::parse_obj_header("1 0 obj"), Some((1, 0)));
        assert_eq!(XRefTable::parse_obj_header("123 5 obj"), Some((123, 5)));
        assert_eq!(
            XRefTable::parse_obj_header("  42   3   obj  "),
            Some((42, 3))
        );

        // Invalid headers
        assert_eq!(XRefTable::parse_obj_header("1 obj"), None);
        assert_eq!(XRefTable::parse_obj_header("abc 0 obj"), None);
        assert_eq!(XRefTable::parse_obj_header("1 0 object"), None);
        assert_eq!(XRefTable::parse_obj_header(""), None);
    }

    #[test]
    fn test_xref_recovery_parsing() {
        // Create a mock PDF content with objects but no valid xref
        let pdf_content =
            b"1 0 obj\n<< /Type /Catalog >>\nendobj\n2 0 obj\n<< /Type /Page >>\nendobj\n";
        let mut reader = BufReader::new(Cursor::new(pdf_content));

        let table = XRefTable::parse_with_recovery(&mut reader).unwrap();

        // Should find both objects
        assert_eq!(table.len(), 2);
        assert!(table.get_entry(1).is_some());
        assert!(table.get_entry(2).is_some());

        // Both should be marked as in-use
        assert!(table.get_entry(1).unwrap().in_use);
        assert!(table.get_entry(2).unwrap().in_use);
    }

    #[test]
    fn test_xref_recovery_no_objects() {
        // Create content with no valid objects
        let pdf_content = b"This is not a PDF file\nNo objects here\n";
        let mut reader = BufReader::new(Cursor::new(pdf_content));

        let result = XRefTable::parse_with_recovery(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_offset_validation() {
        let pdf_data = b"small file";
        let mut reader = BufReader::new(Cursor::new(pdf_data));

        // Valid offset
        assert!(XRefTable::validate_offset(&mut reader, 5).is_ok());

        // Invalid offset (beyond file size)
        assert!(XRefTable::validate_offset(&mut reader, 100).is_err());

        // Offset at end of file
        assert!(XRefTable::validate_offset(&mut reader, 10).is_err());
    }

    #[test]
    fn test_xref_parse_with_fallback() {
        // Test that fallback works when primary parsing fails
        let pdf_content =
            b"1 0 obj\n<< /Type /Catalog >>\nendobj\n2 0 obj\n<< /Type /Page >>\nendobj\n";
        let mut reader = BufReader::new(Cursor::new(pdf_content));

        // PDF without any xref structure cannot be parsed by XRefTable::parse
        // This would need a higher-level recovery mechanism
        let result = XRefTable::parse(&mut reader);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, ParseError::InvalidXRef));
        }
    }
}
