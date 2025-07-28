//! PDF image extraction functionality
//!
//! This module provides functionality to extract images from PDF documents.

use super::{OperationError, OperationResult};
use crate::graphics::ImageFormat;
use crate::parser::objects::{PdfName, PdfObject, PdfStream};
use crate::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Options for image extraction
#[derive(Debug, Clone)]
pub struct ExtractImagesOptions {
    /// Output directory for extracted images
    pub output_dir: PathBuf,
    /// File name pattern for extracted images
    /// Supports placeholders: {page}, {index}, {format}
    pub name_pattern: String,
    /// Whether to extract inline images
    pub extract_inline: bool,
    /// Minimum size (width or height) to extract
    pub min_size: Option<u32>,
    /// Whether to create output directory if it doesn't exist
    pub create_dir: bool,
}

impl Default for ExtractImagesOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("."),
            name_pattern: "page_{page}_image_{index}.{format}".to_string(),
            extract_inline: true,
            min_size: Some(10),
            create_dir: true,
        }
    }
}

/// Result of image extraction
#[derive(Debug)]
pub struct ExtractedImage {
    /// Page number (0-indexed)
    pub page_number: usize,
    /// Image index on the page
    pub image_index: usize,
    /// Output file path
    pub file_path: PathBuf,
    /// Image dimensions
    pub width: u32,
    pub height: u32,
    /// Image format
    pub format: ImageFormat,
}

/// Image extractor
pub struct ImageExtractor {
    document: PdfDocument<File>,
    options: ExtractImagesOptions,
    /// Cache for already processed images
    processed_images: HashMap<String, PathBuf>,
}

impl ImageExtractor {
    /// Create a new image extractor
    pub fn new(document: PdfDocument<File>, options: ExtractImagesOptions) -> Self {
        Self {
            document,
            options,
            processed_images: HashMap::new(),
        }
    }

    /// Extract all images from the document
    pub fn extract_all(&mut self) -> OperationResult<Vec<ExtractedImage>> {
        // Create output directory if needed
        if self.options.create_dir && !self.options.output_dir.exists() {
            fs::create_dir_all(&self.options.output_dir)?;
        }

        let mut extracted_images = Vec::new();
        let page_count = self
            .document
            .page_count()
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        for page_idx in 0..page_count {
            let page_images = self.extract_from_page(page_idx as usize)?;
            extracted_images.extend(page_images);
        }

        Ok(extracted_images)
    }

    /// Extract images from a specific page
    pub fn extract_from_page(
        &mut self,
        page_number: usize,
    ) -> OperationResult<Vec<ExtractedImage>> {
        let mut extracted = Vec::new();

        // Get the page
        let page = self
            .document
            .get_page(page_number as u32)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Get page resources and collect XObject references
        let xobject_refs: Vec<(String, u32, u16)> = {
            let resources = self
                .document
                .get_page_resources(&page)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let mut refs = Vec::new();

            if let Some(resources) = resources {
                if let Some(PdfObject::Dictionary(xobjects)) =
                    resources.0.get(&PdfName("XObject".to_string()))
                {
                    for (name, obj_ref) in &xobjects.0 {
                        if let PdfObject::Reference(obj_num, gen_num) = obj_ref {
                            refs.push((name.0.clone(), *obj_num, *gen_num));
                        }
                    }
                }
            }

            refs
        };

        // Process each XObject reference
        let mut image_index = 0;
        for (name, obj_num, gen_num) in xobject_refs {
            if let Ok(xobject) = self.document.get_object(obj_num, gen_num) {
                if let Some(extracted_image) =
                    self.process_xobject(&xobject, page_number, image_index, &name)?
                {
                    extracted.push(extracted_image);
                    image_index += 1;
                }
            }
        }

        // TODO: Extract inline images from content stream if extract_inline is true

        Ok(extracted)
    }

    /// Process an XObject to see if it's an image
    fn process_xobject(
        &mut self,
        xobject: &PdfObject,
        page_number: usize,
        image_index: usize,
        _name: &str,
    ) -> OperationResult<Option<ExtractedImage>> {
        if let PdfObject::Stream(stream) = xobject {
            // Check if it's an image XObject
            if let Some(PdfObject::Name(subtype)) =
                stream.dict.0.get(&PdfName("Subtype".to_string()))
            {
                if subtype.0 == "Image" {
                    return self.extract_image_xobject(stream, page_number, image_index);
                }
            }
        }
        Ok(None)
    }

    /// Extract an image XObject
    fn extract_image_xobject(
        &mut self,
        stream: &PdfStream,
        page_number: usize,
        image_index: usize,
    ) -> OperationResult<Option<ExtractedImage>> {
        // Get image properties
        let width = match stream.dict.0.get(&PdfName("Width".to_string())) {
            Some(PdfObject::Integer(w)) => *w as u32,
            _ => return Ok(None),
        };

        let height = match stream.dict.0.get(&PdfName("Height".to_string())) {
            Some(PdfObject::Integer(h)) => *h as u32,
            _ => return Ok(None),
        };

        // Check minimum size
        if let Some(min_size) = self.options.min_size {
            if width < min_size || height < min_size {
                return Ok(None);
            }
        }

        // Get the decoded image data
        let parse_options = self.document.options();
        let data = stream.decode(&parse_options).map_err(|e| {
            OperationError::ParseError(format!("Failed to decode image stream: {e}"))
        })?;

        // Determine format from filter
        let format = match stream.dict.0.get(&PdfName("Filter".to_string())) {
            Some(PdfObject::Name(filter)) => match filter.0.as_str() {
                "DCTDecode" => ImageFormat::Jpeg,
                "FlateDecode" => {
                    // FlateDecode can be used for PNG or other formats
                    // Try to detect from the actual data
                    self.detect_image_format_from_data(&data)?
                }
                "LZWDecode" => ImageFormat::Tiff, // Common for TIFF
                _ => {
                    eprintln!("Unsupported image filter: {}", filter.0);
                    return Ok(None);
                }
            },
            Some(PdfObject::Array(filters)) => {
                // Handle filter arrays - use the first filter
                if let Some(PdfObject::Name(filter)) = filters.0.first() {
                    match filter.0.as_str() {
                        "DCTDecode" => ImageFormat::Jpeg,
                        "FlateDecode" => {
                            // Try to detect from the actual data
                            self.detect_image_format_from_data(&data)?
                        }
                        "LZWDecode" => ImageFormat::Tiff,
                        _ => {
                            eprintln!("Unsupported image filter: {}", filter.0);
                            return Ok(None);
                        }
                    }
                } else {
                    return Ok(None);
                }
            }
            _ => {
                eprintln!("No filter found for image");
                return Ok(None);
            }
        };

        // Generate unique key for this image data
        let image_key = format!("{:x}", md5::compute(&data));

        // Check if we've already extracted this image
        if let Some(existing_path) = self.processed_images.get(&image_key) {
            // Return reference to already extracted image
            return Ok(Some(ExtractedImage {
                page_number,
                image_index,
                file_path: existing_path.clone(),
                width,
                height,
                format,
            }));
        }

        // Generate output filename
        let extension = match format {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Tiff => "tiff",
        };

        let filename = self
            .options
            .name_pattern
            .replace("{page}", &(page_number + 1).to_string())
            .replace("{index}", &(image_index + 1).to_string())
            .replace("{format}", extension);

        let output_path = self.options.output_dir.join(filename);

        // Write image data
        let mut file = File::create(&output_path)?;
        file.write_all(&data)?;

        // Cache the path
        self.processed_images.insert(image_key, output_path.clone());

        Ok(Some(ExtractedImage {
            page_number,
            image_index,
            file_path: output_path,
            width,
            height,
            format,
        }))
    }

    /// Detect image format from raw data by examining magic bytes
    fn detect_image_format_from_data(&self, data: &[u8]) -> OperationResult<ImageFormat> {
        if data.len() < 8 {
            return Err(OperationError::ParseError(
                "Image data too short to detect format".to_string(),
            ));
        }

        // Check for PNG signature
        if data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
            return Ok(ImageFormat::Png);
        }

        // Check for TIFF signatures
        if data.len() >= 4 {
            if &data[0..2] == b"II" && &data[2..4] == b"\x2A\x00" {
                return Ok(ImageFormat::Tiff); // Little endian TIFF
            }
            if &data[0..2] == b"MM" && &data[2..4] == b"\x00\x2A" {
                return Ok(ImageFormat::Tiff); // Big endian TIFF
            }
        }

        // Check for JPEG signature
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
            return Ok(ImageFormat::Jpeg);
        }

        // Default to PNG for FlateDecode if no other format detected
        // This is a fallback since FlateDecode is commonly used for PNG in PDFs
        Ok(ImageFormat::Png)
    }
}

/// Extract all images from a PDF file
pub fn extract_images_from_pdf<P: AsRef<Path>>(
    input_path: P,
    options: ExtractImagesOptions,
) -> OperationResult<Vec<ExtractedImage>> {
    let document = PdfReader::open_document(input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let mut extractor = ImageExtractor::new(document, options);
    extractor.extract_all()
}

/// Extract images from specific pages
pub fn extract_images_from_pages<P: AsRef<Path>>(
    input_path: P,
    pages: &[usize],
    options: ExtractImagesOptions,
) -> OperationResult<Vec<ExtractedImage>> {
    let document = PdfReader::open_document(input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let mut extractor = ImageExtractor::new(document, options);
    let mut all_images = Vec::new();

    for &page_num in pages {
        let page_images = extractor.extract_from_page(page_num)?;
        all_images.extend(page_images);
    }

    Ok(all_images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_options_default() {
        let options = ExtractImagesOptions::default();
        assert_eq!(options.output_dir, PathBuf::from("."));
        assert!(options.extract_inline);
        assert_eq!(options.min_size, Some(10));
        assert!(options.create_dir);
    }

    #[test]
    fn test_filename_pattern() {
        let options = ExtractImagesOptions {
            name_pattern: "img_{page}_{index}.{format}".to_string(),
            ..Default::default()
        };

        let pattern = options
            .name_pattern
            .replace("{page}", "1")
            .replace("{index}", "2")
            .replace("{format}", "jpg");

        assert_eq!(pattern, "img_1_2.jpg");
    }
}

#[cfg(test)]
#[path = "extract_images_tests.rs"]
mod extract_images_tests;
