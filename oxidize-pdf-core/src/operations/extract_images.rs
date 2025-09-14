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

        // Extract inline images from content stream if requested
        if self.options.extract_inline {
            if let Ok(parsed_page) = self.document.get_page(page_number as u32) {
                if let Ok(content_streams) = self.document.get_page_content_streams(&parsed_page) {
                    for stream_data in &content_streams {
                        let inline_images = self.extract_inline_images_from_stream(
                            stream_data,
                            page_number,
                            &mut image_index,
                        )?;
                        extracted.extend(inline_images);
                    }
                }
            }
        }

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

        // Get color space information
        let color_space = stream.dict.0.get(&PdfName("ColorSpace".to_string()));
        let bits_per_component = match stream.dict.0.get(&PdfName("BitsPerComponent".to_string())) {
            Some(PdfObject::Integer(bits)) => *bits as u8,
            _ => 8, // Default to 8 bits per component
        };

        // Get the decoded image data
        let parse_options = self.document.options();
        let mut data = stream.decode(&parse_options).map_err(|e| {
            OperationError::ParseError(format!("Failed to decode image stream: {e}"))
        })?;

        // Determine format from filter and process data accordingly
        let format = match stream.dict.0.get(&PdfName("Filter".to_string())) {
            Some(PdfObject::Name(filter)) => match filter.0.as_str() {
                "DCTDecode" => {
                    // JPEG data is already in correct format
                    ImageFormat::Jpeg
                }
                "FlateDecode" => {
                    // FlateDecode contains raw pixel data - need to convert to image format
                    data = self.convert_raw_image_data_to_png(
                        &data,
                        width,
                        height,
                        color_space,
                        bits_per_component,
                    )?;
                    ImageFormat::Png
                }
                "CCITTFaxDecode" => {
                    // CCITT data for scanned documents - convert to PNG
                    data = self.convert_ccitt_to_png(&data, width, height)?;
                    ImageFormat::Png
                }
                "LZWDecode" => {
                    // LZW compressed raw data - convert to PNG
                    data = self.convert_raw_image_data_to_png(
                        &data,
                        width,
                        height,
                        color_space,
                        bits_per_component,
                    )?;
                    ImageFormat::Png
                }
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
                            data = self.convert_raw_image_data_to_png(
                                &data,
                                width,
                                height,
                                color_space,
                                bits_per_component,
                            )?;
                            ImageFormat::Png
                        }
                        "CCITTFaxDecode" => {
                            data = self.convert_ccitt_to_png(&data, width, height)?;
                            ImageFormat::Png
                        }
                        "LZWDecode" => {
                            data = self.convert_raw_image_data_to_png(
                                &data,
                                width,
                                height,
                                color_space,
                                bits_per_component,
                            )?;
                            ImageFormat::Png
                        }
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
                // No filter - raw image data
                data = self.convert_raw_image_data_to_png(
                    &data,
                    width,
                    height,
                    color_space,
                    bits_per_component,
                )?;
                ImageFormat::Png
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
        if data.is_empty() {
            return Err(OperationError::ParseError(
                "Image data too short to detect format".to_string(),
            ));
        }

        // Check for PNG signature (needs 8 bytes)
        if data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
            return Ok(ImageFormat::Png);
        }

        // Check for TIFF signatures (needs 4 bytes)
        if data.len() >= 4 {
            if &data[0..2] == b"II" && &data[2..4] == b"\x2A\x00" {
                return Ok(ImageFormat::Tiff); // Little endian TIFF
            }
            if &data[0..2] == b"MM" && &data[2..4] == b"\x00\x2A" {
                return Ok(ImageFormat::Tiff); // Big endian TIFF
            }
        }

        // Check for JPEG signature (needs 2 bytes)
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
            return Ok(ImageFormat::Jpeg);
        }

        // If data is too short for any meaningful detection
        if data.len() < 2 {
            return Err(OperationError::ParseError(
                "Image data too short to detect format".to_string(),
            ));
        }

        // Default to PNG for FlateDecode if no other format detected
        // This is a fallback since FlateDecode is commonly used for PNG in PDFs
        Ok(ImageFormat::Png)
    }

    /// Extract inline images from a content stream
    fn extract_inline_images_from_stream(
        &mut self,
        stream_data: &[u8],
        page_number: usize,
        image_index: &mut usize,
    ) -> OperationResult<Vec<ExtractedImage>> {
        let mut inline_images = Vec::new();

        // Convert bytes to string for parsing
        let stream_str = String::from_utf8_lossy(stream_data);

        // Find inline image operators: BI (Begin Image), ID (Image Data), EI (End Image)
        let mut pos = 0;
        while let Some(bi_pos) = stream_str[pos..].find("BI") {
            let absolute_bi_pos = pos + bi_pos;

            // Find the ID operator after BI
            if let Some(relative_id_pos) = stream_str[absolute_bi_pos..].find("ID") {
                let absolute_id_pos = absolute_bi_pos + relative_id_pos;

                // Find the EI operator after ID
                if let Some(relative_ei_pos) = stream_str[absolute_id_pos..].find("EI") {
                    let absolute_ei_pos = absolute_id_pos + relative_ei_pos;

                    // Extract image dictionary (between BI and ID)
                    let dict_section = &stream_str[absolute_bi_pos + 2..absolute_id_pos].trim();

                    // Extract image data (between ID and EI)
                    let data_start = absolute_id_pos + 2;
                    let data_end = absolute_ei_pos;

                    if data_start < data_end && data_end <= stream_data.len() {
                        let image_data = &stream_data[data_start..data_end];

                        // Parse basic image properties from dictionary
                        let (width, height) = self.parse_inline_image_dict(dict_section);

                        // Create extracted image
                        if let Ok(extracted_image) = self.save_inline_image(
                            image_data,
                            page_number,
                            *image_index,
                            width,
                            height,
                        ) {
                            inline_images.push(extracted_image);
                            *image_index += 1;
                        }
                    }

                    // Continue searching after this EI
                    pos = absolute_ei_pos + 2;
                } else {
                    break; // No matching EI found
                }
            } else {
                break; // No matching ID found
            }
        }

        Ok(inline_images)
    }

    /// Parse inline image dictionary to extract width and height
    fn parse_inline_image_dict(&self, dict_str: &str) -> (u32, u32) {
        let mut width = 100; // Default width
        let mut height = 100; // Default height

        // Simple parsing - look for /W and /H parameters
        for line in dict_str.lines() {
            let line = line.trim();

            // Parse width: /W 123 or /Width 123
            if line.starts_with("/W ") || line.starts_with("/Width ") {
                if let Some(value_str) = line.split_whitespace().nth(1) {
                    if let Ok(w) = value_str.parse::<u32>() {
                        width = w;
                    }
                }
            }

            // Parse height: /H 123 or /Height 123
            if line.starts_with("/H ") || line.starts_with("/Height ") {
                if let Some(value_str) = line.split_whitespace().nth(1) {
                    if let Ok(h) = value_str.parse::<u32>() {
                        height = h;
                    }
                }
            }
        }

        (width, height)
    }

    /// Save an inline image to disk
    fn save_inline_image(
        &mut self,
        data: &[u8],
        page_number: usize,
        image_index: usize,
        width: u32,
        height: u32,
    ) -> OperationResult<ExtractedImage> {
        // Generate unique key for deduplication
        let image_key = format!("{:x}", md5::compute(data));

        // Check if we've already extracted this image
        if let Some(existing_path) = self.processed_images.get(&image_key) {
            return Ok(ExtractedImage {
                page_number,
                image_index,
                file_path: existing_path.clone(),
                width,
                height,
                format: ImageFormat::Raw, // Inline images are often raw
            });
        }

        // Determine format and extension
        let format = self
            .detect_image_format_from_data(data)
            .unwrap_or(ImageFormat::Raw);
        let extension = match format {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Tiff => "tif",
            ImageFormat::Raw => "raw",
        };

        // Generate filename
        let filename = format!(
            "inline_page_{}_{:03}.{}",
            page_number + 1,
            image_index + 1,
            extension
        );
        let file_path = self.options.output_dir.join(filename);

        // Write image data to file
        fs::write(&file_path, data)?;

        // Cache the extracted image
        self.processed_images.insert(image_key, file_path.clone());

        Ok(ExtractedImage {
            page_number,
            image_index,
            file_path,
            width,
            height,
            format,
        })
    }

    /// Convert raw image data to PNG format
    fn convert_raw_image_data_to_png(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        color_space: Option<&PdfObject>,
        bits_per_component: u8,
    ) -> OperationResult<Vec<u8>> {
        // Determine color components and channels
        let (components, _channels) = match color_space {
            Some(PdfObject::Name(cs)) => match cs.0.as_str() {
                "DeviceGray" => (1, 1),
                "DeviceRGB" => (3, 3),
                "DeviceCMYK" => (4, 4),
                _ => (3, 3), // Default to RGB
            },
            Some(PdfObject::Array(cs_array)) if !cs_array.0.is_empty() => {
                if let Some(PdfObject::Name(cs_name)) = cs_array.0.first() {
                    match cs_name.0.as_str() {
                        "ICCBased" | "CalRGB" => (3, 3),
                        "CalGray" => (1, 1),
                        _ => (3, 3),
                    }
                } else {
                    (3, 3)
                }
            }
            _ => (3, 3), // Default to RGB
        };

        // Calculate expected data size
        let bytes_per_sample = if bits_per_component <= 8 { 1 } else { 2 };
        let expected_size = (width * height * components as u32 * bytes_per_sample as u32) as usize;

        // Validate data size
        if data.len() < expected_size {
            return Err(OperationError::ParseError(format!(
                "Image data too small: expected {}, got {}",
                expected_size,
                data.len()
            )));
        }

        // Convert to PNG format using simple PNG encoding
        self.create_png_from_raw_data(data, width, height, components, bits_per_component)
    }

    /// Create PNG from raw pixel data
    fn create_png_from_raw_data(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        components: u8,
        bits_per_component: u8,
    ) -> OperationResult<Vec<u8>> {
        // Simple PNG creation - create a basic PNG structure
        let mut png_data = Vec::new();

        // PNG signature
        png_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

        // IHDR chunk
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.push(bits_per_component);

        // Color type: 0 = grayscale, 2 = RGB, 6 = RGBA
        let color_type = match components {
            1 => 0, // Grayscale
            3 => 2, // RGB
            4 => 6, // RGBA
            _ => 2, // Default to RGB
        };
        ihdr.push(color_type);
        ihdr.push(0); // Compression method
        ihdr.push(0); // Filter method
        ihdr.push(0); // Interlace method

        self.write_png_chunk(&mut png_data, b"IHDR", &ihdr);

        // IDAT chunk - compress the image data
        let compressed_data = self.compress_image_data(data, width, height, components)?;
        self.write_png_chunk(&mut png_data, b"IDAT", &compressed_data);

        // IEND chunk
        self.write_png_chunk(&mut png_data, b"IEND", &[]);

        Ok(png_data)
    }

    /// Write a PNG chunk with proper CRC
    fn write_png_chunk(&self, output: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
        // Length (4 bytes, big endian)
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());

        // Chunk type (4 bytes)
        output.extend_from_slice(chunk_type);

        // Data
        output.extend_from_slice(data);

        // CRC (4 bytes, big endian)
        let crc = self.calculate_crc32(chunk_type, data);
        output.extend_from_slice(&crc.to_be_bytes());
    }

    /// Simple CRC32 calculation for PNG
    fn calculate_crc32(&self, chunk_type: &[u8; 4], data: &[u8]) -> u32 {
        // Simple CRC32 - in a real implementation we'd use a proper CRC library
        let mut crc: u32 = 0xFFFFFFFF;

        // Process chunk type
        for &byte in chunk_type {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }

        // Process data
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }

        crc ^ 0xFFFFFFFF
    }

    /// Compress image data for PNG IDAT chunk
    fn compress_image_data(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        components: u8,
    ) -> OperationResult<Vec<u8>> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());

        // PNG requires scanline filtering - add filter byte (0 = None) to each row
        let bytes_per_pixel = components as usize;
        let bytes_per_row = width as usize * bytes_per_pixel;

        for row in 0..height {
            // Filter byte (0 = no filter)
            encoder.write_all(&[0])?;

            // Row data
            let start = row as usize * bytes_per_row;
            let end = start + bytes_per_row;
            if end <= data.len() {
                encoder.write_all(&data[start..end])?;
            }
        }

        encoder
            .finish()
            .map_err(|e| OperationError::ParseError(format!("Failed to compress PNG data: {e}")))
    }

    /// Convert CCITT Fax decoded data to PNG (for scanned documents)
    fn convert_ccitt_to_png(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> OperationResult<Vec<u8>> {
        // CCITT is typically 1-bit monochrome
        // Convert 1-bit to 8-bit grayscale
        let mut rgb_data = Vec::new();

        let bits_per_row = width as usize;
        let bytes_per_row = bits_per_row.div_ceil(8); // Round up to nearest byte

        for row in 0..height {
            let row_start = row as usize * bytes_per_row;

            for col in 0..width {
                let byte_idx = row_start + (col as usize / 8);
                let bit_idx = 7 - (col as usize % 8);

                if byte_idx < data.len() {
                    let bit = (data[byte_idx] >> bit_idx) & 1;
                    // CCITT: 0 = black, 1 = white
                    let gray_value = if bit == 0 { 0 } else { 255 };
                    rgb_data.push(gray_value);
                } else {
                    rgb_data.push(255); // White for missing data
                }
            }
        }

        // Create PNG from grayscale data
        self.create_png_from_raw_data(&rgb_data, width, height, 1, 8)
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

        let debug_str = format!("{options:?}");
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

        let debug_str = format!("{image:?}");
        assert!(debug_str.contains("ExtractedImage"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("3"));
        assert!(debug_str.contains("output.png"));
        assert!(debug_str.contains("512"));
        assert!(debug_str.contains("768"));
    }

    // Helper function to create minimal valid PDF for testing
    fn create_minimal_pdf(temp_file: &std::path::Path) {
        let minimal_pdf = b"%PDF-1.7\n\
1 0 obj\n\
<< /Type /Catalog /Pages 2 0 R >>\n\
endobj\n\
2 0 obj\n\
<< /Type /Pages /Kids [] /Count 0 >>\n\
endobj\n\
xref\n\
0 3\n\
0000000000 65535 f \n\
0000000009 00000 n \n\
0000000055 00000 n \n\
trailer\n\
<< /Size 3 /Root 1 0 R >>\n\
startxref\n\
105\n\
%%EOF";
        std::fs::write(temp_file, minimal_pdf).unwrap();
    }

    #[test]
    fn test_detect_image_format_png() {
        // Create a minimal valid PDF document for testing
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.pdf");
        create_minimal_pdf(&temp_file);

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
        create_minimal_pdf(&temp_file);

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
        create_minimal_pdf(&temp_file);

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
        create_minimal_pdf(&temp_file);

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
        create_minimal_pdf(&temp_file);

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
        create_minimal_pdf(&temp_file);

        let document = PdfReader::open_document(&temp_file).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Too short data (less than 2 bytes)
        let short_data = b"\xFF";
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
                ImageFormat::Raw => "raw",
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
        create_minimal_pdf(&temp_file);

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
        let format = extractor.detect_image_format_from_data(exact_2).unwrap();
        assert_eq!(format, ImageFormat::Jpeg); // JPEG only needs 2 bytes
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
