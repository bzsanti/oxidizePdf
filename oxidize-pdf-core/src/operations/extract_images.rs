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
            ImageFormat::Raw => "rgb",
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
    use tempfile::TempDir;

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

    #[test]
    fn test_extract_options_custom() {
        let temp_dir = TempDir::new().unwrap();
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().to_path_buf(),
            name_pattern: "custom_{page}_{index}.{format}".to_string(),
            extract_inline: false,
            min_size: Some(50),
            create_dir: false,
        };

        assert_eq!(options.output_dir, temp_dir.path());
        assert_eq!(options.name_pattern, "custom_{page}_{index}.{format}");
        assert!(!options.extract_inline);
        assert_eq!(options.min_size, Some(50));
        assert!(!options.create_dir);
    }

    #[test]
    fn test_extract_options_debug_clone() {
        let options = ExtractImagesOptions {
            output_dir: PathBuf::from("/test/path"),
            name_pattern: "test.{format}".to_string(),
            extract_inline: true,
            min_size: None,
            create_dir: true,
        };

        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("ExtractImagesOptions"));
        assert!(debug_str.contains("/test/path"));

        let cloned = options.clone();
        assert_eq!(cloned.output_dir, options.output_dir);
        assert_eq!(cloned.name_pattern, options.name_pattern);
        assert_eq!(cloned.extract_inline, options.extract_inline);
        assert_eq!(cloned.min_size, options.min_size);
        assert_eq!(cloned.create_dir, options.create_dir);
    }

    #[test]
    fn test_extracted_image_struct() {
        let image = ExtractedImage {
            page_number: 0,
            image_index: 1,
            file_path: PathBuf::from("/test/image.jpg"),
            width: 100,
            height: 200,
            format: ImageFormat::Jpeg,
        };

        assert_eq!(image.page_number, 0);
        assert_eq!(image.image_index, 1);
        assert_eq!(image.file_path, PathBuf::from("/test/image.jpg"));
        assert_eq!(image.width, 100);
        assert_eq!(image.height, 200);
        assert_eq!(image.format, ImageFormat::Jpeg);
    }

    #[test]
    fn test_extracted_image_debug() {
        let image = ExtractedImage {
            page_number: 5,
            image_index: 3,
            file_path: PathBuf::from("output.png"),
            width: 512,
            height: 768,
            format: ImageFormat::Png,
        };

        let debug_str = format!("{:?}", image);
        assert!(debug_str.contains("ExtractedImage"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("3"));
        assert!(debug_str.contains("output.png"));
        assert!(debug_str.contains("512"));
        assert!(debug_str.contains("768"));
    }

    #[test]
    fn test_detect_image_format_png() {
        // Create a mock ImageExtractor for testing
        // We need to create a dummy PDF document for this
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        std::fs::write(&temp_file, b"%PDF-1.7\n%%EOF").unwrap();

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // PNG magic bytes
        let png_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0DIHDR";
        let format = extractor.detect_image_format_from_data(png_data).unwrap();
        assert_eq!(format, ImageFormat::Png);
    }

    #[test]
    fn test_detect_image_format_jpeg() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        std::fs::write(&temp_file, b"%PDF-1.7\n%%EOF").unwrap();

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // JPEG magic bytes
        let jpeg_data = b"\xFF\xD8\xFF\xE0\x00\x10JFIF";
        let format = extractor.detect_image_format_from_data(jpeg_data).unwrap();
        assert_eq!(format, ImageFormat::Jpeg);
    }

    #[test]
    fn test_detect_image_format_tiff_little_endian() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        std::fs::write(&temp_file, b"%PDF-1.7\n%%EOF").unwrap();

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // TIFF little endian magic bytes
        let tiff_data = b"II\x2A\x00\x08\x00\x00\x00";
        let format = extractor.detect_image_format_from_data(tiff_data).unwrap();
        assert_eq!(format, ImageFormat::Tiff);
    }

    #[test]
    fn test_detect_image_format_tiff_big_endian() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        std::fs::write(&temp_file, b"%PDF-1.7\n%%EOF").unwrap();

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // TIFF big endian magic bytes
        let tiff_data = b"MM\x00\x2A\x00\x00\x00\x08";
        let format = extractor.detect_image_format_from_data(tiff_data).unwrap();
        assert_eq!(format, ImageFormat::Tiff);
    }

    #[test]
    fn test_detect_image_format_unknown() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        std::fs::write(&temp_file, b"%PDF-1.7\n%%EOF").unwrap();

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Unknown format - should default to PNG
        let unknown_data = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08";
        let format = extractor
            .detect_image_format_from_data(unknown_data)
            .unwrap();
        assert_eq!(format, ImageFormat::Png); // Default fallback
    }

    #[test]
    fn test_detect_image_format_short_data() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        std::fs::write(&temp_file, b"%PDF-1.7\n%%EOF").unwrap();

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Too short data
        let short_data = b"\xFF\xD8";
        let result = extractor.detect_image_format_from_data(short_data);
        assert!(result.is_err());
        match result {
            Err(OperationError::ParseError(msg)) => {
                assert!(msg.contains("too short"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_filename_pattern_replacements() {
        let options = ExtractImagesOptions {
            name_pattern: "page_{page}_img_{index}_{format}.{format}".to_string(),
            ..Default::default()
        };

        let pattern = options
            .name_pattern
            .replace("{page}", "10")
            .replace("{index}", "5")
            .replace("{format}", "png");

        assert_eq!(pattern, "page_10_img_5_png.png");
    }

    #[test]
    fn test_extract_options_no_min_size() {
        let options = ExtractImagesOptions {
            min_size: None,
            ..Default::default()
        };

        assert_eq!(options.min_size, None);
    }

    #[test]
    fn test_create_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("new_dir");

        let options = ExtractImagesOptions {
            output_dir: output_dir.clone(),
            create_dir: true,
            ..Default::default()
        };

        // In real usage, ImageExtractor would create this directory
        assert!(!output_dir.exists());
        assert_eq!(options.output_dir, output_dir);
        assert!(options.create_dir);
    }

    #[test]
    fn test_pattern_with_special_chars() {
        let options = ExtractImagesOptions {
            name_pattern: "img-{page}_{index}.{format}".to_string(),
            ..Default::default()
        };

        let pattern = options
            .name_pattern
            .replace("{page}", "1")
            .replace("{index}", "1")
            .replace("{format}", "jpg");

        assert_eq!(pattern, "img-1_1.jpg");
    }

    #[test]
    fn test_multiple_format_extensions() {
        let formats = vec![
            (ImageFormat::Jpeg, "jpg"),
            (ImageFormat::Png, "png"),
            (ImageFormat::Tiff, "tiff"),
        ];

        for (format, expected_ext) in formats {
            let extension = match format {
                ImageFormat::Jpeg => "jpg",
                ImageFormat::Png => "png",
                ImageFormat::Tiff => "tiff",
            };
            assert_eq!(extension, expected_ext);
        }
    }

    #[test]
    fn test_extract_inline_option() {
        let mut options = ExtractImagesOptions::default();
        assert!(options.extract_inline);

        options.extract_inline = false;
        assert!(!options.extract_inline);
    }

    #[test]
    fn test_min_size_filtering() {
        let options_with_min = ExtractImagesOptions {
            min_size: Some(100),
            ..Default::default()
        };

        let options_no_min = ExtractImagesOptions {
            min_size: None,
            ..Default::default()
        };

        assert_eq!(options_with_min.min_size, Some(100));
        assert_eq!(options_no_min.min_size, None);
    }

    #[test]
    fn test_output_path_combinations() {
        let base_dir = PathBuf::from("/output");
        let options = ExtractImagesOptions {
            output_dir: base_dir.clone(),
            name_pattern: "img_{page}_{index}.{format}".to_string(),
            ..Default::default()
        };

        let filename = options
            .name_pattern
            .replace("{page}", "1")
            .replace("{index}", "2")
            .replace("{format}", "png");

        let full_path = options.output_dir.join(filename);
        assert_eq!(full_path, PathBuf::from("/output/img_1_2.png"));
    }

    #[test]
    fn test_pattern_without_placeholders() {
        let options = ExtractImagesOptions {
            name_pattern: "static_name.jpg".to_string(),
            ..Default::default()
        };

        let pattern = options
            .name_pattern
            .replace("{page}", "1")
            .replace("{index}", "2")
            .replace("{format}", "png");

        assert_eq!(pattern, "static_name.jpg"); // No placeholders replaced
    }

    #[test]
    fn test_detect_format_edge_cases() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        std::fs::write(&temp_file, b"%PDF-1.7\n%%EOF").unwrap();

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Empty data
        let empty_data = b"";
        assert!(extractor.detect_image_format_from_data(empty_data).is_err());

        // Data exactly 8 bytes (minimum for PNG check)
        let exact_8 = b"\x89PNG\r\n\x1a\n";
        let format = extractor.detect_image_format_from_data(exact_8).unwrap();
        assert_eq!(format, ImageFormat::Png);

        // Data exactly 4 bytes (minimum for TIFF check)
        let exact_4 = b"II\x2A\x00";
        let format = extractor.detect_image_format_from_data(exact_4).unwrap();
        assert_eq!(format, ImageFormat::Tiff);

        // Data exactly 2 bytes (minimum for JPEG check)
        let exact_2 = b"\xFF\xD8";
        let result = extractor.detect_image_format_from_data(exact_2);
        assert!(result.is_err()); // Too short (needs at least 8 bytes)
    }

    #[test]
    fn test_complex_filename_pattern() {
        let options = ExtractImagesOptions {
            name_pattern: "{format}/page{page}/image_{index}_{page}.{format}".to_string(),
            ..Default::default()
        };

        let pattern = options
            .name_pattern
            .replace("{page}", "5")
            .replace("{index}", "3")
            .replace("{format}", "jpeg");

        assert_eq!(pattern, "jpeg/page5/image_3_5.jpeg");
    }

    #[test]
    fn test_image_dimensions() {
        let small_image = ExtractedImage {
            page_number: 0,
            image_index: 0,
            file_path: PathBuf::from("small.jpg"),
            width: 5,
            height: 5,
            format: ImageFormat::Jpeg,
        };

        let large_image = ExtractedImage {
            page_number: 0,
            image_index: 1,
            file_path: PathBuf::from("large.jpg"),
            width: 2000,
            height: 3000,
            format: ImageFormat::Jpeg,
        };

        assert_eq!(small_image.width, 5);
        assert_eq!(small_image.height, 5);
        assert_eq!(large_image.width, 2000);
        assert_eq!(large_image.height, 3000);
    }

    #[test]
    fn test_page_and_index_numbering() {
        // Test that page numbers and indices work correctly
        let image1 = ExtractedImage {
            page_number: 0, // 0-indexed
            image_index: 0,
            file_path: PathBuf::from("first.jpg"),
            width: 100,
            height: 100,
            format: ImageFormat::Jpeg,
        };

        let image2 = ExtractedImage {
            page_number: 99,  // Large page number
            image_index: 255, // Large index
            file_path: PathBuf::from("last.jpg"),
            width: 100,
            height: 100,
            format: ImageFormat::Jpeg,
        };

        assert_eq!(image1.page_number, 0);
        assert_eq!(image1.image_index, 0);
        assert_eq!(image2.page_number, 99);
        assert_eq!(image2.image_index, 255);
    }
}

#[cfg(test)]
#[path = "extract_images_tests.rs"]
mod extract_images_tests;
