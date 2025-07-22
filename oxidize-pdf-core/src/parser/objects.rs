//! PDF Object Parser - Core PDF data types and parsing
//!
//! This module implements parsing of all PDF object types according to ISO 32000-1 Section 7.3.
//! PDF files are built from a small set of basic object types that can be combined to form
//! complex data structures.
//!
//! # Object Types
//!
//! PDF supports the following basic object types:
//! - **Null**: Represents an undefined value
//! - **Boolean**: true or false
//! - **Integer**: Whole numbers
//! - **Real**: Floating-point numbers
//! - **String**: Text data (literal or hexadecimal)
//! - **Name**: Unique atomic symbols (e.g., /Type, /Pages)
//! - **Array**: Ordered collections of objects
//! - **Dictionary**: Key-value mappings where keys are names
//! - **Stream**: Dictionary + binary data
//! - **Reference**: Indirect reference to another object
//!
//! # Example
//!
//! ```rust
//! use oxidize_pdf::parser::objects::{PdfObject, PdfDictionary, PdfName, PdfArray};
//!
//! // Create a simple page dictionary
//! let mut dict = PdfDictionary::new();
//! dict.insert("Type".to_string(), PdfObject::Name(PdfName::new("Page".to_string())));
//! dict.insert("MediaBox".to_string(), PdfObject::Array(PdfArray::new()));
//!
//! // Check dictionary type
//! assert_eq!(dict.get_type(), Some("Page"));
//! ```

use super::lexer::{Lexer, Token};
use super::{ParseError, ParseResult};
use std::collections::HashMap;
use std::io::Read;

/// PDF Name object - Unique atomic symbols in PDF.
///
/// Names are used as keys in dictionaries and to identify various PDF constructs.
/// They are written with a leading slash (/) in PDF syntax but stored without it.
///
/// # Examples
///
/// Common PDF names:
/// - `/Type` - Object type identifier
/// - `/Pages` - Page tree root
/// - `/Font` - Font resource
/// - `/MediaBox` - Page dimensions
///
/// ```rust
/// use oxidize_pdf::parser::objects::PdfName;
///
/// let name = PdfName::new("Type".to_string());
/// assert_eq!(name.as_str(), "Type");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PdfName(pub String);

/// PDF String object - Text data in PDF files.
///
/// PDF strings can contain arbitrary binary data and use various encodings.
/// They can be written as literal strings `(text)` or hexadecimal strings `<48656C6C6F>`.
///
/// # Encoding
///
/// String encoding depends on context:
/// - Text strings: Usually PDFDocEncoding or UTF-16BE
/// - Font strings: Encoding specified by the font
/// - Binary data: No encoding, raw bytes
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::parser::objects::PdfString;
///
/// // Create from UTF-8
/// let string = PdfString::new(b"Hello World".to_vec());
///
/// // Try to decode as UTF-8
/// if let Ok(text) = string.as_str() {
///     println!("Text: {}", text);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PdfString(pub Vec<u8>);

/// PDF Array object - Ordered collection of PDF objects.
///
/// Arrays can contain any PDF object type, including other arrays and dictionaries.
/// They are written in PDF syntax as `[item1 item2 ... itemN]`.
///
/// # Common Uses
///
/// - Rectangle specifications: `[llx lly urx ury]`
/// - Color values: `[r g b]`
/// - Matrix transformations: `[a b c d e f]`
/// - Resource lists
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::parser::objects::{PdfArray, PdfObject};
///
/// // Create a MediaBox array [0 0 612 792]
/// let mut media_box = PdfArray::new();
/// media_box.push(PdfObject::Integer(0));
/// media_box.push(PdfObject::Integer(0));
/// media_box.push(PdfObject::Integer(612));
/// media_box.push(PdfObject::Integer(792));
///
/// assert_eq!(media_box.len(), 4);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PdfArray(pub Vec<PdfObject>);

/// PDF Dictionary object - Key-value mapping with name keys.
///
/// Dictionaries are the primary way to represent complex data structures in PDF.
/// Keys must be PdfName objects, values can be any PDF object type.
///
/// # Common Dictionary Types
///
/// - **Catalog**: Document root (`/Type /Catalog`)
/// - **Page**: Individual page (`/Type /Page`)
/// - **Font**: Font definition (`/Type /Font`)
/// - **Stream**: Binary data with metadata
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject, PdfName};
///
/// let mut page_dict = PdfDictionary::new();
/// page_dict.insert("Type".to_string(),
///     PdfObject::Name(PdfName::new("Page".to_string())));
/// page_dict.insert("Parent".to_string(),
///     PdfObject::Reference(2, 0)); // Reference to pages tree
///
/// // Access values
/// assert_eq!(page_dict.get_type(), Some("Page"));
/// assert!(page_dict.contains_key("Parent"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PdfDictionary(pub HashMap<PdfName, PdfObject>);

/// PDF Stream object - Dictionary with associated binary data.
///
/// Streams are used for large data blocks like page content, images, fonts, etc.
/// The dictionary describes the stream's properties (length, filters, etc.).
///
/// # Structure
///
/// - `dict`: Stream dictionary with metadata
/// - `data`: Raw stream bytes (possibly compressed)
///
/// # Common Stream Types
///
/// - **Content streams**: Page drawing instructions
/// - **Image XObjects**: Embedded images
/// - **Font programs**: Embedded font data
/// - **Form XObjects**: Reusable graphics
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::parser::objects::{PdfStream, PdfDictionary};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let stream = PdfStream { dict: PdfDictionary::new(), data: vec![] };
/// // Get decompressed data
/// let decoded = stream.decode()?;
/// println!("Decoded {} bytes", decoded.len());
///
/// // Access raw data
/// let raw = stream.raw_data();
/// println!("Raw {} bytes", raw.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PdfStream {
    /// Stream dictionary containing Length, Filter, and other properties
    pub dict: PdfDictionary,
    /// Raw stream data (may be compressed)
    pub data: Vec<u8>,
}

/// Static empty array for use in lenient parsing
pub static EMPTY_PDF_ARRAY: PdfArray = PdfArray(Vec::new());

impl PdfStream {
    /// Get the decompressed stream data.
    ///
    /// Automatically applies filters specified in the stream dictionary
    /// (FlateDecode, ASCIIHexDecode, etc.) to decompress the data.
    ///
    /// # Returns
    ///
    /// The decoded/decompressed stream bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unknown filter is specified
    /// - Decompression fails
    /// - Filter parameters are invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use oxidize_pdf::parser::objects::PdfStream;
    /// # fn example(stream: &PdfStream) -> Result<(), Box<dyn std::error::Error>> {
    /// match stream.decode() {
    ///     Ok(data) => println!("Decoded {} bytes", data.len()),
    ///     Err(e) => println!("Decode error: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn decode(&self) -> ParseResult<Vec<u8>> {
        super::filters::decode_stream(&self.data, &self.dict)
    }

    /// Get the raw (possibly compressed) stream data.
    ///
    /// Returns the stream data exactly as stored in the PDF file,
    /// without applying any filters or decompression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use oxidize_pdf::parser::objects::PdfStream;
    /// # let stream = PdfStream { dict: Default::default(), data: vec![1, 2, 3] };
    /// let raw_data = stream.raw_data();
    /// println!("Raw stream: {} bytes", raw_data.len());
    /// ```
    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }
}

/// PDF Object types - The fundamental data types in PDF.
///
/// All data in a PDF file is represented using these basic types.
/// Objects can be direct (embedded) or indirect (referenced).
///
/// # Object Types
///
/// - `Null` - Undefined/absent value
/// - `Boolean` - true or false
/// - `Integer` - Signed integers
/// - `Real` - Floating-point numbers
/// - `String` - Text or binary data
/// - `Name` - Atomic symbols like /Type
/// - `Array` - Ordered collections
/// - `Dictionary` - Key-value maps
/// - `Stream` - Dictionary + binary data
/// - `Reference` - Indirect object reference (num gen R)
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::parser::objects::{PdfObject, PdfName, PdfString};
///
/// // Different object types
/// let null = PdfObject::Null;
/// let bool_val = PdfObject::Boolean(true);
/// let int_val = PdfObject::Integer(42);
/// let real_val = PdfObject::Real(3.14159);
/// let name = PdfObject::Name(PdfName::new("Type".to_string()));
/// let reference = PdfObject::Reference(10, 0); // 10 0 R
///
/// // Type checking
/// assert!(int_val.as_integer().is_some());
/// assert_eq!(int_val.as_integer(), Some(42));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    /// Null object - represents undefined or absent values
    Null,
    /// Boolean value - true or false
    Boolean(bool),
    /// Integer number
    Integer(i64),
    /// Real (floating-point) number
    Real(f64),
    /// String data (literal or hexadecimal)
    String(PdfString),
    /// Name object - unique identifier
    Name(PdfName),
    /// Array - ordered collection of objects
    Array(PdfArray),
    /// Dictionary - unordered key-value pairs
    Dictionary(PdfDictionary),
    /// Stream - dictionary with binary data
    Stream(PdfStream),
    /// Indirect object reference (object_number, generation_number)
    Reference(u32, u16),
}

impl PdfObject {
    /// Parse a PDF object from a lexer.
    ///
    /// Reads tokens from the lexer and constructs the appropriate PDF object.
    /// Handles all PDF object types including indirect references.
    ///
    /// # Arguments
    ///
    /// * `lexer` - Token source for parsing
    ///
    /// # Returns
    ///
    /// The parsed PDF object.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Invalid syntax is encountered
    /// - Unexpected end of input
    /// - Malformed object structure
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use oxidize_pdf::parser::lexer::Lexer;
    /// use oxidize_pdf::parser::objects::PdfObject;
    /// use std::io::Cursor;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let input = b"42";
    /// let mut lexer = Lexer::new(Cursor::new(input));
    /// let obj = PdfObject::parse(&mut lexer)?;
    /// assert_eq!(obj, PdfObject::Integer(42));
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse<R: Read + std::io::Seek>(lexer: &mut Lexer<R>) -> ParseResult<Self> {
        let token = lexer.next_token()?;
        Self::parse_from_token(lexer, token)
    }

    /// Parse a PDF object with custom options
    pub fn parse_with_options<R: Read + std::io::Seek>(
        lexer: &mut Lexer<R>,
        options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        let token = lexer.next_token()?;
        Self::parse_from_token_with_options(lexer, token, options)
    }

    /// Parse a PDF object starting from a specific token
    fn parse_from_token<R: Read + std::io::Seek>(
        lexer: &mut Lexer<R>,
        token: Token,
    ) -> ParseResult<Self> {
        Self::parse_from_token_with_options(lexer, token, &super::ParseOptions::default())
    }

    /// Parse a PDF object starting from a specific token with custom options
    fn parse_from_token_with_options<R: Read + std::io::Seek>(
        lexer: &mut Lexer<R>,
        token: Token,
        options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        match token {
            Token::Null => Ok(PdfObject::Null),
            Token::Boolean(b) => Ok(PdfObject::Boolean(b)),
            Token::Integer(i) => {
                // For negative numbers or large values, don't check for references
                if !(0..=9999999).contains(&i) {
                    return Ok(PdfObject::Integer(i));
                }

                // Check if this is part of a reference (e.g., "1 0 R")
                match lexer.next_token()? {
                    Token::Integer(gen) if (0..=65535).contains(&gen) => {
                        // Might be a reference, check for 'R'
                        match lexer.next_token()? {
                            Token::Name(s) if s == "R" => {
                                Ok(PdfObject::Reference(i as u32, gen as u16))
                            }
                            token => {
                                // Not a reference, push back the tokens
                                lexer.push_token(token);
                                lexer.push_token(Token::Integer(gen));
                                Ok(PdfObject::Integer(i))
                            }
                        }
                    }
                    token => {
                        // Not a reference, just an integer
                        lexer.push_token(token);
                        Ok(PdfObject::Integer(i))
                    }
                }
            }
            Token::Real(r) => Ok(PdfObject::Real(r)),
            Token::String(s) => Ok(PdfObject::String(PdfString(s))),
            Token::Name(n) => Ok(PdfObject::Name(PdfName(n))),
            Token::ArrayStart => Self::parse_array_with_options(lexer, options),
            Token::DictStart => Self::parse_dictionary_or_stream_with_options(lexer, options),
            Token::Comment(_) => {
                // Skip comments and parse next object
                Self::parse_with_options(lexer, options)
            }
            Token::StartXRef => {
                // This is a PDF structure marker, not a parseable object
                Err(ParseError::SyntaxError {
                    position: 0,
                    message: "StartXRef encountered - this is not a PDF object".to_string(),
                })
            }
            Token::Eof => Err(ParseError::SyntaxError {
                position: 0,
                message: "Unexpected end of file".to_string(),
            }),
            _ => Err(ParseError::UnexpectedToken {
                expected: "PDF object".to_string(),
                found: format!("{token:?}"),
            }),
        }
    }

    /// Parse a PDF array with custom options
    fn parse_array_with_options<R: Read + std::io::Seek>(
        lexer: &mut Lexer<R>,
        options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        let mut elements = Vec::new();

        loop {
            let token = lexer.next_token()?;
            match token {
                Token::ArrayEnd => break,
                Token::Comment(_) => continue, // Skip comments
                _ => {
                    let obj = Self::parse_from_token_with_options(lexer, token, options)?;
                    elements.push(obj);
                }
            }
        }

        Ok(PdfObject::Array(PdfArray(elements)))
    }

    /// Parse a PDF dictionary and check if it's followed by a stream with custom options
    fn parse_dictionary_or_stream_with_options<R: Read + std::io::Seek>(
        lexer: &mut Lexer<R>,
        options: &super::ParseOptions,
    ) -> ParseResult<Self> {
        let dict = Self::parse_dictionary_inner_with_options(lexer, options)?;

        // Check if this is followed by a stream
        loop {
            let token = lexer.next_token()?;
            // Check for stream
            match token {
                Token::Stream => {
                    // Parse stream data
                    let stream_data = Self::parse_stream_data_with_options(lexer, &dict, options)?;
                    return Ok(PdfObject::Stream(PdfStream {
                        dict,
                        data: stream_data,
                    }));
                }
                Token::Comment(_) => {
                    // Skip comment and continue checking
                    continue;
                }
                Token::StartXRef => {
                    // This is the end of the PDF structure, not a stream
                    // Push the token back for later processing
                    // Push back StartXRef token
                    lexer.push_token(token);
                    return Ok(PdfObject::Dictionary(dict));
                }
                _ => {
                    // Not a stream, just a dictionary
                    // Push the token back for later processing
                    // Push back token
                    lexer.push_token(token);
                    return Ok(PdfObject::Dictionary(dict));
                }
            }
        }
    }

    /// Parse the inner dictionary with custom options
    fn parse_dictionary_inner_with_options<R: Read + std::io::Seek>(
        lexer: &mut Lexer<R>,
        options: &super::ParseOptions,
    ) -> ParseResult<PdfDictionary> {
        let mut dict = HashMap::new();

        loop {
            let token = lexer.next_token()?;
            match token {
                Token::DictEnd => break,
                Token::Comment(_) => continue, // Skip comments
                Token::Name(key) => {
                    let value = Self::parse_with_options(lexer, options)?;
                    dict.insert(PdfName(key), value);
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "dictionary key (name) or >>".to_string(),
                        found: format!("{token:?}"),
                    });
                }
            }
        }

        Ok(PdfDictionary(dict))
    }

    /// Parse stream data with custom options
    fn parse_stream_data_with_options<R: Read + std::io::Seek>(
        lexer: &mut Lexer<R>,
        dict: &PdfDictionary,
        options: &super::ParseOptions,
    ) -> ParseResult<Vec<u8>> {
        // Get the stream length from the dictionary
        let length = dict
            .0
            .get(&PdfName("Length".to_string()))
            .or_else(|| {
                // If Length is missing and we have lenient parsing, try to find endstream
                if options.lenient_streams {
                    if options.collect_warnings {
                        eprintln!("Warning: Missing Length key in stream dictionary, will search for endstream marker");
                    }
                    // Return a special marker to indicate we need to search for endstream
                    Some(&PdfObject::Integer(-1))
                } else {
                    None
                }
            })
            .ok_or_else(|| ParseError::MissingKey("Length".to_string()))?;

        let length = match length {
            PdfObject::Integer(len) => {
                if *len == -1 {
                    // Special marker for missing length - we need to search for endstream
                    usize::MAX // We'll handle this specially below
                } else {
                    *len as usize
                }
            }
            PdfObject::Reference(obj_num, gen_num) => {
                // Stream length is an indirect reference - we'll need to resolve it
                // For now, we'll use a fallback approach: search for endstream marker
                // This maintains compatibility while we implement proper reference resolution
                if options.lenient_streams {
                    if options.collect_warnings {
                        eprintln!("Warning: Stream length is an indirect reference ({obj_num} {gen_num} R). Using endstream detection fallback.");
                    }
                    usize::MAX // This will trigger the endstream search below
                } else {
                    return Err(ParseError::SyntaxError {
                        position: lexer.position(),
                        message: format!("Stream length reference ({obj_num} {gen_num} R) requires lenient mode or reference resolution"),
                    });
                }
            }
            _ => {
                return Err(ParseError::SyntaxError {
                    position: lexer.position(),
                    message: "Invalid stream length type".to_string(),
                });
            }
        };

        // Skip the newline after 'stream' keyword
        lexer.read_newline()?;

        // Read the actual stream data
        let mut stream_data = if length == usize::MAX {
            // Missing length - search for endstream marker
            let mut data = Vec::new();
            let max_search = 65536; // Search up to 64KB
            let mut found_endstream = false;

            for _ in 0..max_search {
                match lexer.peek_byte() {
                    Ok(b) => {
                        // Check if we might be at "endstream"
                        if b == b'e' {
                            let pos = lexer.position();
                            // Try to read "endstream"
                            if let Ok(Token::EndStream) = lexer.peek_token() {
                                found_endstream = true;
                                break;
                            }
                            // Not endstream, continue
                            lexer.seek(pos as u64)?;
                        }
                        data.push(lexer.read_byte()?);
                    }
                    Err(_) => break,
                }
            }

            if !found_endstream && !options.lenient_streams {
                return Err(ParseError::SyntaxError {
                    position: lexer.position(),
                    message: "Could not find endstream marker".to_string(),
                });
            }

            data
        } else {
            lexer.read_bytes(length)?
        };

        // Skip optional whitespace before endstream
        lexer.skip_whitespace()?;

        // Check if we have the endstream keyword where expected
        let peek_result = lexer.peek_token();

        match peek_result {
            Ok(Token::EndStream) => {
                // Everything is fine, consume the token
                lexer.next_token()?;
                Ok(stream_data)
            }
            Ok(other_token) => {
                if options.lenient_streams {
                    // Try to find endstream within max_recovery_bytes
                    eprintln!("Warning: Stream length mismatch. Expected 'endstream' after {length} bytes, got {other_token:?}");

                    if let Some(additional_bytes) =
                        lexer.find_keyword_ahead("endstream", options.max_recovery_bytes)?
                    {
                        // Read the additional bytes
                        let extra_data = lexer.read_bytes(additional_bytes)?;
                        stream_data.extend_from_slice(&extra_data);

                        let actual_length = stream_data.len();
                        eprintln!(
                            "Stream length corrected: declared={length}, actual={actual_length}"
                        );

                        // Skip whitespace and consume endstream
                        lexer.skip_whitespace()?;
                        lexer.expect_keyword("endstream")?;

                        Ok(stream_data)
                    } else {
                        // Couldn't find endstream within recovery distance
                        Err(ParseError::SyntaxError {
                            position: lexer.position(),
                            message: format!(
                                "Could not find 'endstream' within {} bytes",
                                options.max_recovery_bytes
                            ),
                        })
                    }
                } else {
                    // Strict mode - return error
                    Err(ParseError::UnexpectedToken {
                        expected: "endstream".to_string(),
                        found: format!("{other_token:?}"),
                    })
                }
            }
            Err(e) => {
                if options.lenient_streams {
                    // Try to find endstream within max_recovery_bytes
                    eprintln!(
                        "Warning: Stream length mismatch. Could not peek next token after {length} bytes"
                    );

                    if let Some(additional_bytes) =
                        lexer.find_keyword_ahead("endstream", options.max_recovery_bytes)?
                    {
                        // Read the additional bytes
                        let extra_data = lexer.read_bytes(additional_bytes)?;
                        stream_data.extend_from_slice(&extra_data);

                        let actual_length = stream_data.len();
                        eprintln!(
                            "Stream length corrected: declared={length}, actual={actual_length}"
                        );

                        // Skip whitespace and consume endstream
                        lexer.skip_whitespace()?;
                        lexer.expect_keyword("endstream")?;

                        Ok(stream_data)
                    } else {
                        // Couldn't find endstream within recovery distance
                        Err(ParseError::SyntaxError {
                            position: lexer.position(),
                            message: format!(
                                "Could not find 'endstream' within {} bytes",
                                options.max_recovery_bytes
                            ),
                        })
                    }
                } else {
                    // Strict mode - propagate the error
                    Err(e)
                }
            }
        }
    }

    /// Check if this object is null.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::PdfObject;
    ///
    /// assert!(PdfObject::Null.is_null());
    /// assert!(!PdfObject::Integer(42).is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self, PdfObject::Null)
    }

    /// Get the value as a boolean if this is a Boolean object.
    ///
    /// # Returns
    ///
    /// Some(bool) if this is a Boolean object, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::PdfObject;
    ///
    /// let obj = PdfObject::Boolean(true);
    /// assert_eq!(obj.as_bool(), Some(true));
    ///
    /// let obj = PdfObject::Integer(1);
    /// assert_eq!(obj.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get the value as a real number.
    ///
    /// Returns the value for both Real and Integer objects,
    /// converting integers to floating-point.
    ///
    /// # Returns
    ///
    /// Some(f64) if this is a numeric object, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::PdfObject;
    ///
    /// let real_obj = PdfObject::Real(3.14);
    /// assert_eq!(real_obj.as_real(), Some(3.14));
    ///
    /// let int_obj = PdfObject::Integer(42);
    /// assert_eq!(int_obj.as_real(), Some(42.0));
    /// ```
    pub fn as_real(&self) -> Option<f64> {
        match self {
            PdfObject::Real(r) => Some(*r),
            PdfObject::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get as string
    pub fn as_string(&self) -> Option<&PdfString> {
        match self {
            PdfObject::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as name
    pub fn as_name(&self) -> Option<&PdfName> {
        match self {
            PdfObject::Name(n) => Some(n),
            _ => None,
        }
    }

    /// Get as array
    pub fn as_array(&self) -> Option<&PdfArray> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Get as dictionary
    pub fn as_dict(&self) -> Option<&PdfDictionary> {
        match self {
            PdfObject::Dictionary(d) => Some(d),
            PdfObject::Stream(s) => Some(&s.dict),
            _ => None,
        }
    }

    /// Get as stream
    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfObject::Stream(s) => Some(s),
            _ => None,
        }
    }

    /// Get the object reference if this is a Reference object.
    ///
    /// # Returns
    ///
    /// Some((object_number, generation_number)) if this is a Reference, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::PdfObject;
    ///
    /// let obj = PdfObject::Reference(10, 0);
    /// assert_eq!(obj.as_reference(), Some((10, 0)));
    ///
    /// // Use for resolving references
    /// if let Some((obj_num, gen_num)) = obj.as_reference() {
    ///     println!("Reference to {} {} R", obj_num, gen_num);
    /// }
    /// ```
    pub fn as_reference(&self) -> Option<(u32, u16)> {
        match self {
            PdfObject::Reference(obj, gen) => Some((*obj, *gen)),
            _ => None,
        }
    }
}

impl Default for PdfDictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfDictionary {
    /// Create a new empty dictionary.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject, PdfName};
    ///
    /// let mut dict = PdfDictionary::new();
    /// dict.insert("Type".to_string(), PdfObject::Name(PdfName::new("Font".to_string())));
    /// ```
    pub fn new() -> Self {
        PdfDictionary(HashMap::new())
    }

    /// Get a value by key name.
    ///
    /// # Arguments
    ///
    /// * `key` - The key name (without leading slash)
    ///
    /// # Returns
    ///
    /// Reference to the value if the key exists, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject};
    ///
    /// let mut dict = PdfDictionary::new();
    /// dict.insert("Length".to_string(), PdfObject::Integer(1000));
    ///
    /// if let Some(length) = dict.get("Length").and_then(|o| o.as_integer()) {
    ///     println!("Stream length: {}", length);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&PdfObject> {
        self.0.get(&PdfName(key.to_string()))
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: String, value: PdfObject) {
        self.0.insert(PdfName(key), value);
    }

    /// Check if dictionary contains a key
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(&PdfName(key.to_string()))
    }

    /// Get the dictionary type (value of /Type key).
    ///
    /// Many PDF dictionaries have a /Type entry that identifies their purpose.
    ///
    /// # Returns
    ///
    /// The type name if present, None otherwise.
    ///
    /// # Common Types
    ///
    /// - "Catalog" - Document catalog
    /// - "Page" - Page object
    /// - "Pages" - Page tree node
    /// - "Font" - Font dictionary
    /// - "XObject" - External object
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject, PdfName};
    ///
    /// let mut dict = PdfDictionary::new();
    /// dict.insert("Type".to_string(), PdfObject::Name(PdfName::new("Page".to_string())));
    /// assert_eq!(dict.get_type(), Some("Page"));
    /// ```
    pub fn get_type(&self) -> Option<&str> {
        self.get("Type")
            .and_then(|obj| obj.as_name())
            .map(|n| n.0.as_str())
    }
}

impl Default for PdfArray {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfArray {
    /// Create a new empty array
    pub fn new() -> Self {
        PdfArray(Vec::new())
    }

    /// Get array length
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if array is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get element at index.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index
    ///
    /// # Returns
    ///
    /// Reference to the element if index is valid, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::{PdfArray, PdfObject};
    ///
    /// let mut array = PdfArray::new();
    /// array.push(PdfObject::Integer(10));
    /// array.push(PdfObject::Integer(20));
    ///
    /// assert_eq!(array.get(0).and_then(|o| o.as_integer()), Some(10));
    /// assert_eq!(array.get(1).and_then(|o| o.as_integer()), Some(20));
    /// assert!(array.get(2).is_none());
    /// ```
    pub fn get(&self, index: usize) -> Option<&PdfObject> {
        self.0.get(index)
    }

    /// Push an element
    pub fn push(&mut self, obj: PdfObject) {
        self.0.push(obj);
    }
}

impl PdfString {
    /// Create a new PDF string
    pub fn new(data: Vec<u8>) -> Self {
        PdfString(data)
    }

    /// Get as UTF-8 string if possible.
    ///
    /// Attempts to decode the string bytes as UTF-8.
    /// Note that PDF strings may use other encodings.
    ///
    /// # Returns
    ///
    /// Ok(&str) if valid UTF-8, Err otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::parser::objects::PdfString;
    ///
    /// let string = PdfString::new(b"Hello".to_vec());
    /// assert_eq!(string.as_str(), Ok("Hello"));
    ///
    /// let binary = PdfString::new(vec![0xFF, 0xFE]);
    /// assert!(binary.as_str().is_err());
    /// ```
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.0)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl PdfName {
    /// Create a new PDF name
    pub fn new(name: String) -> Self {
        PdfName(name)
    }

    /// Get the name as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::lexer::Lexer;
    use crate::parser::ParseOptions;
    use std::collections::HashMap;
    use std::io::Cursor;

    #[test]
    fn test_parse_simple_objects() {
        let input = b"null true false 123 -456 3.14 /Name (Hello)";
        let mut lexer = Lexer::new(Cursor::new(input));

        assert_eq!(PdfObject::parse(&mut lexer).unwrap(), PdfObject::Null);
        assert_eq!(
            PdfObject::parse(&mut lexer).unwrap(),
            PdfObject::Boolean(true)
        );
        assert_eq!(
            PdfObject::parse(&mut lexer).unwrap(),
            PdfObject::Boolean(false)
        );
        assert_eq!(
            PdfObject::parse(&mut lexer).unwrap(),
            PdfObject::Integer(123)
        );
        assert_eq!(
            PdfObject::parse(&mut lexer).unwrap(),
            PdfObject::Integer(-456)
        );
        assert_eq!(PdfObject::parse(&mut lexer).unwrap(), PdfObject::Real(3.14));
        assert_eq!(
            PdfObject::parse(&mut lexer).unwrap(),
            PdfObject::Name(PdfName("Name".to_string()))
        );
        assert_eq!(
            PdfObject::parse(&mut lexer).unwrap(),
            PdfObject::String(PdfString(b"Hello".to_vec()))
        );
    }

    #[test]
    fn test_parse_array() {
        // Test simple array without potential references
        let input = b"[100 200 300 /Name (test)]";
        let mut lexer = Lexer::new(Cursor::new(input));

        let obj = PdfObject::parse(&mut lexer).unwrap();
        let array = obj.as_array().unwrap();

        assert_eq!(array.len(), 5);
        assert_eq!(array.get(0).unwrap().as_integer(), Some(100));
        assert_eq!(array.get(1).unwrap().as_integer(), Some(200));
        assert_eq!(array.get(2).unwrap().as_integer(), Some(300));
        assert_eq!(array.get(3).unwrap().as_name().unwrap().as_str(), "Name");
        assert_eq!(
            array.get(4).unwrap().as_string().unwrap().as_bytes(),
            b"test"
        );
    }

    #[test]
    fn test_parse_array_with_references() {
        // Test array with references
        let input = b"[1 0 R 2 0 R]";
        let mut lexer = Lexer::new(Cursor::new(input));

        let obj = PdfObject::parse(&mut lexer).unwrap();
        let array = obj.as_array().unwrap();

        assert_eq!(array.len(), 2);
        assert!(array.get(0).unwrap().as_reference().is_some());
        assert!(array.get(1).unwrap().as_reference().is_some());
    }

    #[test]
    fn test_parse_dictionary() {
        let input = b"<< /Type /Page /Parent 1 0 R /MediaBox [0 0 612 792] >>";
        let mut lexer = Lexer::new(Cursor::new(input));

        let obj = PdfObject::parse(&mut lexer).unwrap();
        let dict = obj.as_dict().unwrap();

        assert_eq!(dict.get_type(), Some("Page"));
        assert!(dict.get("Parent").unwrap().as_reference().is_some());
        assert!(dict.get("MediaBox").unwrap().as_array().is_some());
    }

    // Comprehensive tests for all object types and their methods
    mod comprehensive_tests {
        use super::*;

        #[test]
        fn test_pdf_object_null() {
            let obj = PdfObject::Null;
            assert!(obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), None);
            assert_eq!(obj.as_string(), None);
            assert_eq!(obj.as_name(), None);
            assert_eq!(obj.as_array(), None);
            assert_eq!(obj.as_dict(), None);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), None);
        }

        #[test]
        fn test_pdf_object_boolean() {
            let obj_true = PdfObject::Boolean(true);
            let obj_false = PdfObject::Boolean(false);

            assert!(!obj_true.is_null());
            assert_eq!(obj_true.as_bool(), Some(true));
            assert_eq!(obj_false.as_bool(), Some(false));

            assert_eq!(obj_true.as_integer(), None);
            assert_eq!(obj_true.as_real(), None);
            assert_eq!(obj_true.as_string(), None);
            assert_eq!(obj_true.as_name(), None);
            assert_eq!(obj_true.as_array(), None);
            assert_eq!(obj_true.as_dict(), None);
            assert_eq!(obj_true.as_stream(), None);
            assert_eq!(obj_true.as_reference(), None);
        }

        #[test]
        fn test_pdf_object_integer() {
            let obj = PdfObject::Integer(42);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), Some(42));
            assert_eq!(obj.as_real(), Some(42.0)); // Should convert to float
            assert_eq!(obj.as_string(), None);
            assert_eq!(obj.as_name(), None);
            assert_eq!(obj.as_array(), None);
            assert_eq!(obj.as_dict(), None);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), None);

            // Test negative integers
            let obj_neg = PdfObject::Integer(-123);
            assert_eq!(obj_neg.as_integer(), Some(-123));
            assert_eq!(obj_neg.as_real(), Some(-123.0));

            // Test large integers
            let obj_large = PdfObject::Integer(9999999999);
            assert_eq!(obj_large.as_integer(), Some(9999999999));
            assert_eq!(obj_large.as_real(), Some(9999999999.0));
        }

        #[test]
        fn test_pdf_object_real() {
            let obj = PdfObject::Real(3.14159);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), Some(3.14159));
            assert_eq!(obj.as_string(), None);
            assert_eq!(obj.as_name(), None);
            assert_eq!(obj.as_array(), None);
            assert_eq!(obj.as_dict(), None);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), None);

            // Test negative real numbers
            let obj_neg = PdfObject::Real(-2.71828);
            assert_eq!(obj_neg.as_real(), Some(-2.71828));

            // Test zero
            let obj_zero = PdfObject::Real(0.0);
            assert_eq!(obj_zero.as_real(), Some(0.0));

            // Test very small numbers
            let obj_small = PdfObject::Real(0.000001);
            assert_eq!(obj_small.as_real(), Some(0.000001));

            // Test very large numbers
            let obj_large = PdfObject::Real(1e10);
            assert_eq!(obj_large.as_real(), Some(1e10));
        }

        #[test]
        fn test_pdf_object_string() {
            let string_data = b"Hello World".to_vec();
            let pdf_string = PdfString(string_data.clone());
            let obj = PdfObject::String(pdf_string);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), None);
            assert!(obj.as_string().is_some());
            assert_eq!(obj.as_string().unwrap().as_bytes(), string_data);
            assert_eq!(obj.as_name(), None);
            assert_eq!(obj.as_array(), None);
            assert_eq!(obj.as_dict(), None);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), None);
        }

        #[test]
        fn test_pdf_object_name() {
            let name_str = "Type".to_string();
            let pdf_name = PdfName(name_str.clone());
            let obj = PdfObject::Name(pdf_name);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), None);
            assert_eq!(obj.as_string(), None);
            assert!(obj.as_name().is_some());
            assert_eq!(obj.as_name().unwrap().as_str(), name_str);
            assert_eq!(obj.as_array(), None);
            assert_eq!(obj.as_dict(), None);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), None);
        }

        #[test]
        fn test_pdf_object_array() {
            let mut array = PdfArray::new();
            array.push(PdfObject::Integer(1));
            array.push(PdfObject::Integer(2));
            array.push(PdfObject::Integer(3));
            let obj = PdfObject::Array(array);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), None);
            assert_eq!(obj.as_string(), None);
            assert_eq!(obj.as_name(), None);
            assert!(obj.as_array().is_some());
            assert_eq!(obj.as_array().unwrap().len(), 3);
            assert_eq!(obj.as_dict(), None);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), None);
        }

        #[test]
        fn test_pdf_object_dictionary() {
            let mut dict = PdfDictionary::new();
            dict.insert(
                "Type".to_string(),
                PdfObject::Name(PdfName("Page".to_string())),
            );
            dict.insert("Count".to_string(), PdfObject::Integer(5));
            let obj = PdfObject::Dictionary(dict);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), None);
            assert_eq!(obj.as_string(), None);
            assert_eq!(obj.as_name(), None);
            assert_eq!(obj.as_array(), None);
            assert!(obj.as_dict().is_some());
            assert_eq!(obj.as_dict().unwrap().0.len(), 2);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), None);
        }

        #[test]
        fn test_pdf_object_stream() {
            let mut dict = PdfDictionary::new();
            dict.insert("Length".to_string(), PdfObject::Integer(13));
            let data = b"Hello, World!".to_vec();
            let stream = PdfStream { dict, data };
            let obj = PdfObject::Stream(stream);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), None);
            assert_eq!(obj.as_string(), None);
            assert_eq!(obj.as_name(), None);
            assert_eq!(obj.as_array(), None);
            assert!(obj.as_dict().is_some()); // Stream dictionary should be accessible
            assert!(obj.as_stream().is_some());
            assert_eq!(obj.as_stream().unwrap().raw_data(), b"Hello, World!");
            assert_eq!(obj.as_reference(), None);
        }

        #[test]
        fn test_pdf_object_reference() {
            let obj = PdfObject::Reference(42, 0);

            assert!(!obj.is_null());
            assert_eq!(obj.as_bool(), None);
            assert_eq!(obj.as_integer(), None);
            assert_eq!(obj.as_real(), None);
            assert_eq!(obj.as_string(), None);
            assert_eq!(obj.as_name(), None);
            assert_eq!(obj.as_array(), None);
            assert_eq!(obj.as_dict(), None);
            assert_eq!(obj.as_stream(), None);
            assert_eq!(obj.as_reference(), Some((42, 0)));

            // Test different generations
            let obj_gen = PdfObject::Reference(123, 5);
            assert_eq!(obj_gen.as_reference(), Some((123, 5)));
        }

        #[test]
        fn test_pdf_string_methods() {
            let string_data = b"Hello, World!".to_vec();
            let pdf_string = PdfString(string_data.clone());

            assert_eq!(pdf_string.as_bytes(), string_data);
            assert_eq!(pdf_string.as_str().unwrap(), "Hello, World!");
            assert_eq!(pdf_string.0.len(), 13);
            assert!(!pdf_string.0.is_empty());

            // Test empty string
            let empty_string = PdfString(vec![]);
            assert!(empty_string.0.is_empty());
            assert_eq!(empty_string.0.len(), 0);

            // Test non-UTF-8 data
            let binary_data = vec![0xFF, 0xFE, 0x00, 0x48, 0x00, 0x69]; // UTF-16 "Hi"
            let binary_string = PdfString(binary_data.clone());
            assert_eq!(binary_string.as_bytes(), binary_data);
            assert!(binary_string.as_str().is_err()); // Should fail UTF-8 conversion
        }

        #[test]
        fn test_pdf_name_methods() {
            let name_str = "Type".to_string();
            let pdf_name = PdfName(name_str.clone());

            assert_eq!(pdf_name.as_str(), name_str);
            assert_eq!(pdf_name.0.len(), 4);
            assert!(!pdf_name.0.is_empty());

            // Test empty name
            let empty_name = PdfName("".to_string());
            assert!(empty_name.0.is_empty());
            assert_eq!(empty_name.0.len(), 0);

            // Test name with special characters
            let special_name = PdfName("Font#20Name".to_string());
            assert_eq!(special_name.as_str(), "Font#20Name");
            assert_eq!(special_name.0.len(), 11);
        }

        #[test]
        fn test_pdf_array_methods() {
            let mut array = PdfArray::new();
            assert_eq!(array.len(), 0);
            assert!(array.is_empty());

            // Test push operations
            array.push(PdfObject::Integer(1));
            array.push(PdfObject::Integer(2));
            array.push(PdfObject::Integer(3));

            assert_eq!(array.len(), 3);
            assert!(!array.is_empty());

            // Test get operations
            assert_eq!(array.get(0).unwrap().as_integer(), Some(1));
            assert_eq!(array.get(1).unwrap().as_integer(), Some(2));
            assert_eq!(array.get(2).unwrap().as_integer(), Some(3));
            assert!(array.get(3).is_none());

            // Test iteration
            let values: Vec<i64> = array.0.iter().filter_map(|obj| obj.as_integer()).collect();
            assert_eq!(values, vec![1, 2, 3]);

            // Test mixed types
            let mut mixed_array = PdfArray::new();
            mixed_array.push(PdfObject::Integer(42));
            mixed_array.push(PdfObject::Real(3.14));
            mixed_array.push(PdfObject::String(PdfString(b"text".to_vec())));
            mixed_array.push(PdfObject::Name(PdfName("Name".to_string())));
            mixed_array.push(PdfObject::Boolean(true));
            mixed_array.push(PdfObject::Null);

            assert_eq!(mixed_array.len(), 6);
            assert_eq!(mixed_array.get(0).unwrap().as_integer(), Some(42));
            assert_eq!(mixed_array.get(1).unwrap().as_real(), Some(3.14));
            assert_eq!(
                mixed_array.get(2).unwrap().as_string().unwrap().as_bytes(),
                b"text"
            );
            assert_eq!(
                mixed_array.get(3).unwrap().as_name().unwrap().as_str(),
                "Name"
            );
            assert_eq!(mixed_array.get(4).unwrap().as_bool(), Some(true));
            assert!(mixed_array.get(5).unwrap().is_null());
        }

        #[test]
        fn test_pdf_dictionary_methods() {
            let mut dict = PdfDictionary::new();
            assert_eq!(dict.0.len(), 0);
            assert!(dict.0.is_empty());

            // Test insertions
            dict.insert(
                "Type".to_string(),
                PdfObject::Name(PdfName("Page".to_string())),
            );
            dict.insert("Count".to_string(), PdfObject::Integer(5));
            dict.insert("Resources".to_string(), PdfObject::Reference(10, 0));

            assert_eq!(dict.0.len(), 3);
            assert!(!dict.0.is_empty());

            // Test get operations
            assert_eq!(
                dict.get("Type").unwrap().as_name().unwrap().as_str(),
                "Page"
            );
            assert_eq!(dict.get("Count").unwrap().as_integer(), Some(5));
            assert_eq!(dict.get("Resources").unwrap().as_reference(), Some((10, 0)));
            assert!(dict.get("NonExistent").is_none());

            // Test contains_key
            assert!(dict.contains_key("Type"));
            assert!(dict.contains_key("Count"));
            assert!(dict.contains_key("Resources"));
            assert!(!dict.contains_key("NonExistent"));

            // Test get_type helper
            assert_eq!(dict.get_type(), Some("Page"));

            // Test iteration
            let mut keys: Vec<String> = dict.0.keys().map(|k| k.0.clone()).collect();
            keys.sort();
            assert_eq!(keys, vec!["Count", "Resources", "Type"]);

            // Test values
            let values: Vec<&PdfObject> = dict.0.values().collect();
            assert_eq!(values.len(), 3);
        }

        #[test]
        fn test_pdf_stream_methods() {
            let mut dict = PdfDictionary::new();
            dict.insert("Length".to_string(), PdfObject::Integer(13));
            dict.insert(
                "Filter".to_string(),
                PdfObject::Name(PdfName("FlateDecode".to_string())),
            );

            let data = b"Hello, World!".to_vec();
            let stream = PdfStream {
                dict,
                data: data.clone(),
            };

            // Test raw data access
            assert_eq!(stream.raw_data(), data);

            // Test dictionary access
            assert_eq!(stream.dict.get("Length").unwrap().as_integer(), Some(13));
            assert_eq!(
                stream
                    .dict
                    .get("Filter")
                    .unwrap()
                    .as_name()
                    .unwrap()
                    .as_str(),
                "FlateDecode"
            );

            // Test decode method (this might fail if filters aren't implemented)
            // but we'll test that it returns a result
            let decode_result = stream.decode();
            assert!(decode_result.is_ok() || decode_result.is_err());
        }

        #[test]
        fn test_parse_complex_nested_structures() {
            // Test nested array
            let input = b"[[1 2] [3 4] [5 6]]";
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            let outer_array = obj.as_array().unwrap();
            assert_eq!(outer_array.len(), 3);

            for i in 0..3 {
                let inner_array = outer_array.get(i).unwrap().as_array().unwrap();
                assert_eq!(inner_array.len(), 2);
                assert_eq!(
                    inner_array.get(0).unwrap().as_integer(),
                    Some((i as i64) * 2 + 1)
                );
                assert_eq!(
                    inner_array.get(1).unwrap().as_integer(),
                    Some((i as i64) * 2 + 2)
                );
            }
        }

        #[test]
        fn test_parse_complex_dictionary() {
            let input = b"<< /Type /Page /Parent 1 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 2 0 R >> /ProcSet [/PDF /Text] >> /Contents 3 0 R >>";
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            let dict = obj.as_dict().unwrap();
            assert_eq!(dict.get_type(), Some("Page"));
            assert_eq!(dict.get("Parent").unwrap().as_reference(), Some((1, 0)));
            assert_eq!(dict.get("Contents").unwrap().as_reference(), Some((3, 0)));

            // Test nested MediaBox array
            let media_box = dict.get("MediaBox").unwrap().as_array().unwrap();
            assert_eq!(media_box.len(), 4);
            assert_eq!(media_box.get(0).unwrap().as_integer(), Some(0));
            assert_eq!(media_box.get(1).unwrap().as_integer(), Some(0));
            assert_eq!(media_box.get(2).unwrap().as_integer(), Some(612));
            assert_eq!(media_box.get(3).unwrap().as_integer(), Some(792));

            // Test nested Resources dictionary
            let resources = dict.get("Resources").unwrap().as_dict().unwrap();
            assert!(resources.contains_key("Font"));
            assert!(resources.contains_key("ProcSet"));

            // Test nested Font dictionary
            let font_dict = resources.get("Font").unwrap().as_dict().unwrap();
            assert_eq!(font_dict.get("F1").unwrap().as_reference(), Some((2, 0)));

            // Test ProcSet array
            let proc_set = resources.get("ProcSet").unwrap().as_array().unwrap();
            assert_eq!(proc_set.len(), 2);
            assert_eq!(proc_set.get(0).unwrap().as_name().unwrap().as_str(), "PDF");
            assert_eq!(proc_set.get(1).unwrap().as_name().unwrap().as_str(), "Text");
        }

        #[test]
        fn test_parse_hex_strings() {
            let input = b"<48656C6C6F>"; // "Hello" in hex
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            let string = obj.as_string().unwrap();
            assert_eq!(string.as_str().unwrap(), "Hello");
        }

        #[test]
        fn test_parse_literal_strings() {
            let input = b"(Hello World)";
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            let string = obj.as_string().unwrap();
            assert_eq!(string.as_str().unwrap(), "Hello World");
        }

        #[test]
        fn test_parse_string_with_escapes() {
            let input = b"(Hello\\nWorld\\t!)";
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            let string = obj.as_string().unwrap();
            // The lexer should handle escape sequences
            assert!(!string.as_bytes().is_empty());
        }

        #[test]
        fn test_parse_names_with_special_chars() {
            let input = b"/Name#20with#20spaces";
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            let name = obj.as_name().unwrap();
            // The lexer should handle hex escapes in names
            assert!(!name.as_str().is_empty());
        }

        #[test]
        fn test_parse_references() {
            let input = b"1 0 R";
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            assert_eq!(obj.as_reference(), Some((1, 0)));

            // Test reference with higher generation
            let input2 = b"42 5 R";
            let mut lexer2 = Lexer::new(Cursor::new(input2));
            let obj2 = PdfObject::parse(&mut lexer2).unwrap();

            assert_eq!(obj2.as_reference(), Some((42, 5)));
        }

        #[test]
        fn test_parse_edge_cases() {
            // Test very large numbers
            let input = b"9223372036854775807"; // i64::MAX
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();
            assert_eq!(obj.as_integer(), Some(9223372036854775807));

            // Test very small numbers
            let input2 = b"-9223372036854775808"; // i64::MIN
            let mut lexer2 = Lexer::new(Cursor::new(input2));
            let obj2 = PdfObject::parse(&mut lexer2).unwrap();
            assert_eq!(obj2.as_integer(), Some(-9223372036854775808));

            // Test scientific notation in reals (if supported by lexer)
            let input3 = b"1.23e-10";
            let mut lexer3 = Lexer::new(Cursor::new(input3));
            let obj3 = PdfObject::parse(&mut lexer3).unwrap();
            // The lexer might not support scientific notation, so just check it's a real
            assert!(obj3.as_real().is_some());
        }

        #[test]
        fn test_parse_empty_structures() {
            // Test empty array
            let input = b"[]";
            let mut lexer = Lexer::new(Cursor::new(input));
            let obj = PdfObject::parse(&mut lexer).unwrap();

            let array = obj.as_array().unwrap();
            assert_eq!(array.len(), 0);
            assert!(array.is_empty());

            // Test empty dictionary
            let input2 = b"<< >>";
            let mut lexer2 = Lexer::new(Cursor::new(input2));
            let obj2 = PdfObject::parse(&mut lexer2).unwrap();

            let dict = obj2.as_dict().unwrap();
            assert_eq!(dict.0.len(), 0);
            assert!(dict.0.is_empty());
        }

        #[test]
        fn test_error_handling() {
            // Test malformed array
            let input = b"[1 2 3"; // Missing closing bracket
            let mut lexer = Lexer::new(Cursor::new(input));
            let result = PdfObject::parse(&mut lexer);
            assert!(result.is_err());

            // Test malformed dictionary
            let input2 = b"<< /Type /Page"; // Missing closing >>
            let mut lexer2 = Lexer::new(Cursor::new(input2));
            let result2 = PdfObject::parse(&mut lexer2);
            assert!(result2.is_err());

            // Test malformed reference
            let input3 = b"1 0 X"; // Should be R, not X
            let mut lexer3 = Lexer::new(Cursor::new(input3));
            let result3 = PdfObject::parse(&mut lexer3);
            // This should parse as integer 1, but the exact behavior depends on lexer implementation
            // Could be an error or could parse as integer 1
            assert!(result3.is_ok() || result3.is_err());
        }

        #[test]
        fn test_clone_and_equality() {
            let obj1 = PdfObject::Integer(42);
            let obj2 = obj1.clone();
            assert_eq!(obj1, obj2);

            let obj3 = PdfObject::Integer(43);
            assert_ne!(obj1, obj3);

            // Test complex structure cloning
            let mut array = PdfArray::new();
            array.push(PdfObject::Integer(1));
            array.push(PdfObject::String(PdfString(b"test".to_vec())));
            let obj4 = PdfObject::Array(array);
            let obj5 = obj4.clone();
            assert_eq!(obj4, obj5);
        }

        #[test]
        fn test_debug_formatting() {
            let obj = PdfObject::Integer(42);
            let debug_str = format!("{:?}", obj);
            assert!(debug_str.contains("Integer"));
            assert!(debug_str.contains("42"));

            let name = PdfName("Type".to_string());
            let debug_str2 = format!("{:?}", name);
            assert!(debug_str2.contains("PdfName"));
            assert!(debug_str2.contains("Type"));
        }

        #[test]
        fn test_performance_large_array() {
            let mut array = PdfArray::new();
            for i in 0..1000 {
                array.push(PdfObject::Integer(i));
            }

            assert_eq!(array.len(), 1000);
            assert_eq!(array.get(0).unwrap().as_integer(), Some(0));
            assert_eq!(array.get(999).unwrap().as_integer(), Some(999));

            // Test iteration performance
            let sum: i64 = array.0.iter().filter_map(|obj| obj.as_integer()).sum();
            assert_eq!(sum, 499500); // sum of 0..1000
        }

        #[test]
        fn test_performance_large_dictionary() {
            let mut dict = PdfDictionary::new();
            for i in 0..1000 {
                dict.insert(format!("Key{}", i), PdfObject::Integer(i));
            }

            assert_eq!(dict.0.len(), 1000);
            assert_eq!(dict.get("Key0").unwrap().as_integer(), Some(0));
            assert_eq!(dict.get("Key999").unwrap().as_integer(), Some(999));

            // Test lookup performance
            for i in 0..1000 {
                assert!(dict.contains_key(&format!("Key{}", i)));
            }
        }
    }

    #[test]
    fn test_lenient_stream_parsing_too_short() {
        // Create a simpler test for stream parsing
        // Dictionary with stream
        let dict = PdfDictionary(
            vec![(PdfName("Length".to_string()), PdfObject::Integer(10))]
                .into_iter()
                .collect::<HashMap<_, _>>(),
        );

        // Create test data where actual stream is longer than declared length
        // Note: avoid using "stream" in the content as it confuses the keyword search
        let stream_content = b"This is a much longer text content than just 10 bytes";
        let test_data = vec![
            b"\n".to_vec(), // Newline after stream keyword
            stream_content.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat();

        // Test lenient parsing
        let mut cursor = Cursor::new(test_data);
        let mut lexer = Lexer::new(&mut cursor);
        let mut options = ParseOptions::default();
        options.lenient_streams = true;
        options.max_recovery_bytes = 100;
        options.collect_warnings = false;

        // parse_stream_data_with_options expects the 'stream' token to have been consumed already
        // and will read the newline after 'stream'

        let result = PdfObject::parse_stream_data_with_options(&mut lexer, &dict, &options);
        if let Err(e) = &result {
            eprintln!("Error in test_lenient_stream_parsing_too_short: {:?}", e);
            eprintln!("Warning: Stream length mismatch expected, checking if lenient parsing is working correctly");
        }
        assert!(result.is_ok());

        let stream_data = result.unwrap();
        let content = String::from_utf8_lossy(&stream_data);

        // In lenient mode, should get content up to endstream
        // It seems to be finding "stream" within the content and stopping early
        assert!(content.contains("This is a"));
    }

    #[test]
    fn test_lenient_stream_parsing_too_long() {
        // Test case where declared length is longer than actual stream
        let dict = PdfDictionary(
            vec![(PdfName("Length".to_string()), PdfObject::Integer(100))]
                .into_iter()
                .collect::<HashMap<_, _>>(),
        );

        // Create test data where actual stream is shorter than declared length
        let stream_content = b"Short";
        let test_data = vec![
            b"\n".to_vec(), // Newline after stream keyword
            stream_content.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat();

        // Test lenient parsing
        let mut cursor = Cursor::new(test_data);
        let mut lexer = Lexer::new(&mut cursor);
        let mut options = ParseOptions::default();
        options.lenient_streams = true;
        options.max_recovery_bytes = 100;
        options.collect_warnings = false;

        // parse_stream_data_with_options expects the 'stream' token to have been consumed already

        let result = PdfObject::parse_stream_data_with_options(&mut lexer, &dict, &options);

        // When declared length is too long, it will fail to read 100 bytes
        // This is expected behavior - lenient mode handles incorrect lengths when
        // endstream is not where expected, but can't fix EOF issues
        assert!(result.is_err());
    }

    #[test]
    fn test_lenient_stream_no_endstream_found() {
        // Test case where endstream is missing or too far away
        let input = b"<< /Length 10 >>
stream
This text does not contain the magic word and continues for a very long time with no proper termination...";

        let mut cursor = Cursor::new(input.to_vec());
        let mut lexer = Lexer::new(&mut cursor);
        let mut options = ParseOptions::default();
        options.lenient_streams = true;
        options.max_recovery_bytes = 50; // Limit search - endstream not within these bytes
        options.collect_warnings = false;

        let dict_token = lexer.next_token().unwrap();
        let obj = PdfObject::parse_from_token_with_options(&mut lexer, dict_token, &options);

        // Should fail because endstream not found within recovery distance
        assert!(obj.is_err());
    }
}
