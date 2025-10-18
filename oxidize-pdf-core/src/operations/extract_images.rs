//! PDF image extraction functionality
//!
//! This module provides functionality to extract images from PDF documents with
//! advanced preprocessing for scanned documents.

use super::{OperationError, OperationResult};
use crate::graphics::ImageFormat;
use crate::parser::objects::{PdfName, PdfObject, PdfStream};
use crate::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(feature = "external-images")]
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat as ImageLibFormat, Luma};

/// PDF transformation matrix (a, b, c, d, e, f)
/// Represents: [a c e] [x]   [a*x + c*y + e]
///             [b d f] [y] = [b*x + d*y + f]
///             [0 0 1] [1]   [      1      ]
#[derive(Debug, Clone)]
pub struct TransformMatrix {
    pub a: f64, // x scaling
    pub b: f64, // y skewing
    pub c: f64, // x skewing
    pub d: f64, // y scaling
    pub e: f64, // x translation
    pub f: f64, // y translation
}

impl TransformMatrix {
    fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Check if this matrix represents a 90-degree rotation
    #[allow(dead_code)]
    fn is_90_degree_rotation(&self) -> bool {
        // For 90-degree rotation: a ≈ 0, d ≈ 0, b and c are non-zero
        self.a.abs() < 0.001 && self.d.abs() < 0.001 && self.b.abs() > 0.001 && self.c.abs() > 0.001
    }

    /// Check if this matrix represents a simple scaling
    #[allow(dead_code)]
    fn is_simple_scale(&self) -> bool {
        // For scaling: b ≈ 0, c ≈ 0, a and d are scaling factors
        self.b.abs() < 0.001 && self.c.abs() < 0.001 && self.a.abs() > 0.001 && self.d.abs() > 0.001
    }

    /// Check if this is a matrix that needs rotation for proper OCR
    #[allow(dead_code)]
    fn is_fis2_like_matrix(&self) -> bool {
        // Some PDFs use 841.68 x 595.08 which are A4 dimensions (landscape fitting in portrait)
        // This indicates the image is landscape but being fit into portrait page
        (self.a - 841.68).abs() < 1.0
            && (self.d - 595.08).abs() < 1.0
            && self.b.abs() < 0.001
            && self.c.abs() < 0.001
    }
}

/// Preprocessing options for extracted images
#[derive(Debug, Clone)]
pub struct ImagePreprocessingOptions {
    /// Auto-detect and correct rotation
    pub auto_correct_rotation: bool,
    /// Enhance contrast for better OCR
    pub enhance_contrast: bool,
    /// Apply noise reduction
    pub denoise: bool,
    /// Upscale small images using bicubic interpolation
    pub upscale_small_images: bool,
    /// Minimum size to trigger upscaling
    pub upscale_threshold: u32,
    /// Upscale factor (2x, 3x, etc.)
    pub upscale_factor: u32,
    /// Convert to grayscale for better OCR on text documents
    pub force_grayscale: bool,
}

impl Default for ImagePreprocessingOptions {
    fn default() -> Self {
        Self {
            auto_correct_rotation: true,
            enhance_contrast: true,
            denoise: true,
            upscale_small_images: true,
            upscale_threshold: 300,
            upscale_factor: 2,
            force_grayscale: false,
        }
    }
}

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
    /// Preprocessing options for extracted images
    pub preprocessing: ImagePreprocessingOptions,
}

impl Default for ExtractImagesOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("."),
            name_pattern: "page_{page}_image_{index}.{format}".to_string(),
            extract_inline: true,
            min_size: Some(10),
            create_dir: true,
            preprocessing: ImagePreprocessingOptions::default(),
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

        // If no XObjects found via resources, try alternative method
        if extracted.is_empty() {
            eprintln!("DEBUG: No XObjects found via resources, trying content stream analysis...");

            // Analyze content streams for image references
            if let Ok(content_streams) = self.document.get_page_content_streams(&page) {
                for stream_data in &content_streams {
                    let referenced_images = self.extract_referenced_images_from_content(
                        stream_data,
                        page_number,
                        &mut image_index,
                    )?;
                    extracted.extend(referenced_images);
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
                    // JPEG data is already in correct format - use raw stream data
                    // DCTDecode streams contain complete JPEG data, don't decode
                    data = stream.data.clone();
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
                        "DCTDecode" => {
                            // JPEG data is already in correct format - use raw stream data
                            data = stream.data.clone();
                            ImageFormat::Jpeg
                        }
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

        // For scanned PDFs where all pages reference the same image object,
        // we need to create separate files per page for OCR processing
        // Don't deduplicate if we're extracting for OCR purposes
        let allow_deduplication = !self.options.name_pattern.contains("{page}");

        // Check if we've already extracted this image (only if deduplication is allowed)
        if allow_deduplication {
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

        // Apply preprocessing if enabled
        #[cfg(feature = "external-images")]
        let processed_data = if self.should_preprocess() {
            self.preprocess_image_data(&data, width, height, format)?
        } else {
            data
        };

        #[cfg(not(feature = "external-images"))]
        let processed_data = data;

        // Write image data
        let mut file = File::create(&output_path)?;
        file.write_all(&processed_data)?;

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

    /// Extract images referenced in content streams when resources are not available
    fn extract_referenced_images_from_content(
        &mut self,
        stream_data: &[u8],
        page_number: usize,
        image_index: &mut usize,
    ) -> OperationResult<Vec<ExtractedImage>> {
        let mut extracted = Vec::new();

        // Convert to string for parsing
        let content = String::from_utf8_lossy(stream_data);

        eprintln!(
            "DEBUG: Content stream analysis for page {}:",
            page_number + 1
        );
        eprintln!("       Content: {}", content);

        // Parse transformation matrices and image references together
        // Pattern: look for cm matrices followed by Do operators
        let image_with_transform = self.parse_images_with_transformations(&content)?;

        for (image_name, transform_matrix) in image_with_transform {
            eprintln!(
                "DEBUG: Found XObject reference: /{} with transform: {:?}",
                image_name, transform_matrix
            );

            // Try to find this object by scanning all objects in the document
            if let Some(mut extracted_image) =
                self.find_and_extract_xobject_by_name(&image_name, page_number, *image_index)?
            {
                // Apply transformation if one was found
                if let Some(matrix) = transform_matrix {
                    extracted_image =
                        self.apply_transformation_to_image(extracted_image, &matrix)?;
                }

                extracted.push(extracted_image);
                *image_index += 1;
            }
        }

        Ok(extracted)
    }

    /// Find an XObject by name by scanning through the document
    fn find_and_extract_xobject_by_name(
        &mut self,
        name: &str,
        page_number: usize,
        image_index: usize,
    ) -> OperationResult<Option<ExtractedImage>> {
        eprintln!("DEBUG: Searching for XObject named '{}'", name);

        // This is a brute force approach - scan through objects looking for image streams
        // In a real implementation, we would have better object mapping, but for now
        // this should work for common landscape-in-portrait cases

        // Try some common object numbers that might contain images
        // We'll scan a range and look for stream objects that look like images
        for obj_num in 1..1000 {
            if let Ok(obj) = self.document.get_object(obj_num, 0) {
                if let Some(extracted) =
                    self.try_extract_image_from_object(&obj, page_number, image_index, name)?
                {
                    eprintln!("DEBUG: Found image in object {}", obj_num);
                    return Ok(Some(extracted));
                }
            }
        }

        eprintln!("DEBUG: Could not find XObject '{}'", name);
        Ok(None)
    }

    /// Try to extract an image from any PDF object
    fn try_extract_image_from_object(
        &mut self,
        obj: &PdfObject,
        page_number: usize,
        image_index: usize,
        _expected_name: &str,
    ) -> OperationResult<Option<ExtractedImage>> {
        if let PdfObject::Stream(stream) = obj {
            // Check if this stream looks like an image
            if let Some(PdfObject::Name(subtype)) =
                stream.dict.0.get(&PdfName("Subtype".to_string()))
            {
                if subtype.0 == "Image" {
                    eprintln!("DEBUG: Found Image stream, attempting extraction...");
                    return self.extract_image_xobject(stream, page_number, image_index);
                }
            }

            // Also check for streams that might be images but don't have proper Subtype
            if let Some(PdfObject::Integer(width)) =
                stream.dict.0.get(&PdfName("Width".to_string()))
            {
                if let Some(PdfObject::Integer(height)) =
                    stream.dict.0.get(&PdfName("Height".to_string()))
                {
                    eprintln!(
                        "DEBUG: Found stream with Width/Height ({}x{}), treating as image",
                        width, height
                    );
                    return self.extract_image_xobject(stream, page_number, image_index);
                }
            }
        }

        Ok(None)
    }

    /// Parse content stream to find images with their transformation matrices
    fn parse_images_with_transformations(
        &self,
        content: &str,
    ) -> OperationResult<Vec<(String, Option<TransformMatrix>)>> {
        let mut results = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut current_matrix: Option<TransformMatrix> = None;

        for line in lines {
            let line = line.trim();

            // Look for transformation matrices: "a b c d e f cm"
            if line.ends_with(" cm") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 7 && parts[6] == "cm" {
                    // Parse the 6 matrix values
                    if let (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e), Ok(f)) = (
                        parts[0].parse::<f64>(),
                        parts[1].parse::<f64>(),
                        parts[2].parse::<f64>(),
                        parts[3].parse::<f64>(),
                        parts[4].parse::<f64>(),
                        parts[5].parse::<f64>(),
                    ) {
                        current_matrix = Some(TransformMatrix::new(a, b, c, d, e, f));
                        eprintln!(
                            "DEBUG: Found transformation matrix: {} {} {} {} {} {}",
                            a, b, c, d, e, f
                        );
                    }
                }
            }

            // Look for image draw commands: "/ImageName Do"
            if line.contains(" Do") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts {
                    if part.starts_with('/') && !part.contains("Do") {
                        let image_name = part[1..].to_string(); // Remove the '/'
                        results.push((image_name, current_matrix.clone()));
                        eprintln!(
                            "DEBUG: Parsed image '{}' with matrix: {:?}",
                            part, current_matrix
                        );
                    }
                }
            }

            // Reset matrix on graphics state restore
            if line.trim() == "Q" {
                current_matrix = None;
            }
        }

        Ok(results)
    }

    /// Apply transformation matrix to an extracted image
    #[allow(unused_mut)]
    fn apply_transformation_to_image(
        &self,
        mut extracted_image: ExtractedImage,
        matrix: &TransformMatrix,
    ) -> OperationResult<ExtractedImage> {
        eprintln!("DEBUG: Applying transformation to image: {:?}", matrix);

        #[cfg(feature = "external-images")]
        {
            // Read the extracted image file
            let image_data = std::fs::read(&extracted_image.file_path)?;

            // Load with image crate
            let img = image::load_from_memory(&image_data).map_err(|e| {
                OperationError::ParseError(format!("Failed to load image for transformation: {e}"))
            })?;

            // IGNORE TRANSFORMATION FOR NOW - FOCUS ON STRIDE PROBLEM
            eprintln!("DEBUG: Skipping transformation, focusing on stride correction");
            let transformed_img =
                self.fix_stride_problem(img, extracted_image.width, extracted_image.height)?;

            // Save the transformed image
            let output_filename = extracted_image
                .file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| OperationError::InvalidPath {
                    reason: format!("Image path has no valid filename: {:?}", extracted_image.file_path),
                })?;
            let output_extension = extracted_image
                .file_path
                .extension()
                .and_then(|s| s.to_str())
                .ok_or_else(|| OperationError::InvalidPath {
                    reason: format!("Image path has no valid extension: {:?}", extracted_image.file_path),
                })?;

            let parent_dir = extracted_image
                .file_path
                .parent()
                .ok_or_else(|| OperationError::InvalidPath {
                    reason: format!("Image path has no parent directory: {:?}", extracted_image.file_path),
                })?;
            let transformed_path = parent_dir.join(format!(
                "{}_transformed.{}",
                output_filename, output_extension
            ));

            transformed_img.save(&transformed_path).map_err(|e| {
                OperationError::ParseError(format!("Failed to save transformed image: {e}"))
            })?;

            // Update the extracted image info
            let (new_width, new_height) = transformed_img.dimensions();
            extracted_image.file_path = transformed_path;
            extracted_image.width = new_width;
            extracted_image.height = new_height;

            eprintln!(
                "DEBUG: Saved transformed image to: {}",
                extracted_image.file_path.display()
            );
        }

        #[cfg(not(feature = "external-images"))]
        {
            eprintln!("DEBUG: External-images feature not enabled, skipping transformation");
        }

        Ok(extracted_image)
    }

    /// Apply rotation transformation
    #[cfg(feature = "external-images")]
    #[allow(dead_code)]
    fn apply_rotation_transformation(
        &self,
        img: DynamicImage,
        matrix: &TransformMatrix,
    ) -> OperationResult<DynamicImage> {
        // Determine rotation direction based on matrix values
        // For 90-degree clockwise: a=0, b=1, c=-1, d=0
        // For 90-degree counter-clockwise: a=0, b=-1, c=1, d=0

        if matrix.b > 0.0 && matrix.c < 0.0 {
            Ok(img.rotate90()) // 90 degrees clockwise
        } else if matrix.b < 0.0 && matrix.c > 0.0 {
            Ok(img.rotate270()) // 90 degrees counter-clockwise (270 clockwise)
        } else {
            // Default to 90-degree rotation for landscape-in-portrait cases
            Ok(img.rotate90())
        }
    }

    /// Apply scaling transformation
    #[cfg(feature = "external-images")]
    #[allow(dead_code)]
    fn apply_scale_transformation(
        &self,
        img: DynamicImage,
        matrix: &TransformMatrix,
    ) -> OperationResult<DynamicImage> {
        let (current_width, current_height) = img.dimensions();

        // Calculate new dimensions based on scaling factors
        let new_width = (current_width as f64 * matrix.a.abs()) as u32;
        let new_height = (current_height as f64 * matrix.d.abs()) as u32;

        if new_width > 0 && new_height > 0 {
            Ok(img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3))
        } else {
            // If scaling results in invalid dimensions, return original
            Ok(img)
        }
    }

    /// Fix stride/row alignment problems in image data
    #[cfg(feature = "external-images")]
    fn fix_stride_problem(
        &self,
        img: DynamicImage,
        original_width: u32,
        original_height: u32,
    ) -> OperationResult<DynamicImage> {
        eprintln!(
            "DEBUG: Fixing stride problem for {}x{} image",
            original_width, original_height
        );

        // Convert to raw grayscale data
        let gray_img = img.to_luma8();
        let pixel_data = gray_img.as_raw();

        eprintln!("DEBUG: Image has {} bytes of pixel data", pixel_data.len());

        // Try different row strides to fix misalignment
        let bytes_per_row = original_width as usize;
        let min_bytes_per_row = bytes_per_row;

        // Possible stride alignments
        let possible_strides = [
            min_bytes_per_row,              // No padding
            (min_bytes_per_row + 1) & !1,   // 2-byte aligned
            (min_bytes_per_row + 3) & !3,   // 4-byte aligned
            (min_bytes_per_row + 7) & !7,   // 8-byte aligned
            (min_bytes_per_row + 15) & !15, // 16-byte aligned
            min_bytes_per_row + 1,          // +1 padding
            min_bytes_per_row + 2,          // +2 padding
            min_bytes_per_row + 4,          // +4 padding
        ];

        eprintln!(
            "DEBUG: Trying {} different stride alignments",
            possible_strides.len()
        );

        for (i, &stride) in possible_strides.iter().enumerate() {
            let expected_total = stride * original_height as usize;

            if expected_total <= pixel_data.len() {
                eprintln!(
                    "DEBUG: Trying stride {}: {} bytes/row (total needed: {})",
                    i, stride, expected_total
                );

                // Extract using this stride
                let mut corrected_data = Vec::new();
                for row in 0..original_height {
                    let row_start = row as usize * stride;
                    let row_end = row_start + bytes_per_row;

                    if row_end <= pixel_data.len() {
                        corrected_data.extend_from_slice(&pixel_data[row_start..row_end]);
                    } else {
                        // Fill with white if we run out of data
                        corrected_data.resize(corrected_data.len() + bytes_per_row, 255);
                    }
                }

                // Create corrected image
                if corrected_data.len() == (original_width * original_height) as usize {
                    if let Some(corrected_img) = ImageBuffer::<Luma<u8>, Vec<u8>>::from_raw(
                        original_width,
                        original_height,
                        corrected_data,
                    ) {
                        eprintln!(
                            "DEBUG: Successfully created corrected image with stride {}",
                            stride
                        );
                        return Ok(DynamicImage::ImageLuma8(corrected_img));
                    }
                }
            } else {
                eprintln!(
                    "DEBUG: Stride {} too large (need {}, have {})",
                    stride,
                    expected_total,
                    pixel_data.len()
                );
            }
        }

        eprintln!("DEBUG: No stride correction worked, returning original");
        Ok(img)
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

        // Don't deduplicate if we're extracting for OCR purposes (pattern contains {page})
        let allow_deduplication = !self.options.name_pattern.contains("{page}");

        // Check if we've already extracted this image (only if deduplication is allowed)
        if allow_deduplication {
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

        // Calculate potential row strides - try multiple alignments
        let bits_per_row = width as usize;
        let min_bytes_per_row = bits_per_row.div_ceil(8);

        // Try different row stride alignments (1, 2, 4, 8, 16 byte alignment)
        let possible_strides = [
            min_bytes_per_row,              // No padding
            (min_bytes_per_row + 1) & !1,   // 2-byte aligned
            (min_bytes_per_row + 3) & !3,   // 4-byte aligned
            (min_bytes_per_row + 7) & !7,   // 8-byte aligned
            (min_bytes_per_row + 15) & !15, // 16-byte aligned
        ];

        // Try to detect the correct stride by checking data patterns
        let correct_stride =
            self.detect_correct_row_stride(data, width, height, &possible_strides)?;

        eprintln!(
            "DEBUG: width={}, height={}, min_bytes={}, detected_stride={}",
            width, height, min_bytes_per_row, correct_stride
        );

        for row in 0..height {
            let row_start = row as usize * correct_stride;

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

    /// Detect the correct row stride by analyzing data patterns
    fn detect_correct_row_stride(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        possible_strides: &[usize],
    ) -> OperationResult<usize> {
        let bits_per_row = width as usize;
        let min_bytes_per_row = bits_per_row.div_ceil(8);

        // If we don't have enough data for analysis, use minimum stride
        if data.len() < min_bytes_per_row * 3 {
            return Ok(min_bytes_per_row);
        }

        // Calculate expected total size for each stride
        for &stride in possible_strides {
            let expected_size = stride * height as usize;

            // If this stride gives us a size close to actual data length, use it
            if expected_size <= data.len() && (data.len() - expected_size) < stride * 2 {
                // Allow some tolerance

                eprintln!(
                    "DEBUG: Selected stride {} (expected_size={}, actual={})",
                    stride,
                    expected_size,
                    data.len()
                );
                return Ok(stride);
            }
        }

        // If no stride fits perfectly, calculate from data length
        let calculated_stride = data.len() / height as usize;
        if calculated_stride >= min_bytes_per_row {
            eprintln!(
                "DEBUG: Calculated stride from data length: {}",
                calculated_stride
            );
            return Ok(calculated_stride);
        }

        // Fallback to minimum
        eprintln!(
            "DEBUG: Using fallback minimum stride: {}",
            min_bytes_per_row
        );
        Ok(min_bytes_per_row)
    }

    /// Check if preprocessing should be applied
    #[allow(dead_code)]
    fn should_preprocess(&self) -> bool {
        self.options.preprocessing.auto_correct_rotation
            || self.options.preprocessing.enhance_contrast
            || self.options.preprocessing.denoise
            || self.options.preprocessing.upscale_small_images
            || self.options.preprocessing.force_grayscale
    }

    /// Apply image preprocessing
    #[cfg(feature = "external-images")]
    fn preprocess_image_data(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        format: ImageFormat,
    ) -> OperationResult<Vec<u8>> {
        // Load image using the image crate
        let img_format = match format {
            ImageFormat::Jpeg => ImageLibFormat::Jpeg,
            ImageFormat::Png => ImageLibFormat::Png,
            ImageFormat::Tiff => ImageLibFormat::Tiff,
            ImageFormat::Raw => {
                // For raw data, create a simple RGB image
                return self.preprocess_raw_image_data(data, width, height);
            }
        };

        let img = image::load_from_memory_with_format(data, img_format)
            .map_err(|e| OperationError::ParseError(format!("Failed to load image: {e}")))?;

        let mut processed_img = img;

        // Apply preprocessing steps
        processed_img = self.apply_rotation_correction(processed_img)?;
        processed_img = self.apply_contrast_enhancement(processed_img)?;
        processed_img = self.apply_noise_reduction(processed_img)?;
        processed_img = self.apply_upscaling(processed_img, width, height)?;

        if self.options.preprocessing.force_grayscale {
            processed_img = DynamicImage::ImageLuma8(processed_img.to_luma8());
        }

        // Encode back to bytes
        let mut output = Vec::new();
        processed_img
            .write_to(&mut std::io::Cursor::new(&mut output), img_format)
            .map_err(|e| OperationError::ParseError(format!("Failed to encode image: {e}")))?;

        Ok(output)
    }

    /// Preprocess raw image data
    #[cfg(feature = "external-images")]
    fn preprocess_raw_image_data(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> OperationResult<Vec<u8>> {
        // Create a simple grayscale image from raw data
        if data.len() < (width * height) as usize {
            return Err(OperationError::ParseError(
                "Raw image data too small".to_string(),
            ));
        }

        let img_buffer = ImageBuffer::<Luma<u8>, Vec<u8>>::from_raw(
            width,
            height,
            data[..(width * height) as usize].to_vec(),
        )
        .ok_or_else(|| OperationError::ParseError("Failed to create image buffer".to_string()))?;

        let img = DynamicImage::ImageLuma8(img_buffer);
        let mut processed_img = img;

        // Apply preprocessing
        processed_img = self.apply_rotation_correction(processed_img)?;
        processed_img = self.apply_contrast_enhancement(processed_img)?;
        processed_img = self.apply_noise_reduction(processed_img)?;
        processed_img = self.apply_upscaling(processed_img, width, height)?;

        // Encode to PNG
        let mut output = Vec::new();
        processed_img
            .write_to(&mut std::io::Cursor::new(&mut output), ImageLibFormat::Png)
            .map_err(|e| OperationError::ParseError(format!("Failed to encode image: {e}")))?;

        Ok(output)
    }

    /// Auto-detect and correct rotation
    #[cfg(feature = "external-images")]
    fn apply_rotation_correction(&self, img: DynamicImage) -> OperationResult<DynamicImage> {
        if !self.options.preprocessing.auto_correct_rotation {
            return Ok(img);
        }

        // Simple rotation detection based on aspect ratio and content analysis
        let (width, height) = img.dimensions();

        // If image is wider than it is tall but contains mostly vertical text,
        // it might need rotation. This is a simplified heuristic.
        if width > height * 2 {
            // Likely rotated 90 degrees - try rotating
            return Ok(img.rotate90());
        }

        // For now, return as-is. In a more sophisticated implementation,
        // we could use OCR or edge detection to determine optimal rotation.
        Ok(img)
    }

    /// Enhance contrast for better OCR
    #[cfg(feature = "external-images")]
    fn apply_contrast_enhancement(&self, img: DynamicImage) -> OperationResult<DynamicImage> {
        if !self.options.preprocessing.enhance_contrast {
            return Ok(img);
        }

        // Apply histogram equalization by adjusting brightness and contrast
        let enhanced = img.adjust_contrast(20.0); // Increase contrast by 20%
        Ok(enhanced.brighten(10)) // Slightly brighten
    }

    /// Apply noise reduction
    #[cfg(feature = "external-images")]
    fn apply_noise_reduction(&self, img: DynamicImage) -> OperationResult<DynamicImage> {
        if !self.options.preprocessing.denoise {
            return Ok(img);
        }

        // Simple blur to reduce noise
        Ok(img.blur(0.5))
    }

    /// Upscale small images for better OCR
    #[cfg(feature = "external-images")]
    fn apply_upscaling(
        &self,
        img: DynamicImage,
        original_width: u32,
        original_height: u32,
    ) -> OperationResult<DynamicImage> {
        if !self.options.preprocessing.upscale_small_images {
            return Ok(img);
        }

        let min_dimension = original_width.min(original_height);
        if min_dimension < self.options.preprocessing.upscale_threshold {
            let new_width = original_width * self.options.preprocessing.upscale_factor;
            let new_height = original_height * self.options.preprocessing.upscale_factor;

            return Ok(img.resize(
                new_width,
                new_height,
                image::imageops::FilterType::CatmullRom,
            ));
        }

        Ok(img)
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
            preprocessing: ImagePreprocessingOptions::default(),
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
            preprocessing: ImagePreprocessingOptions::default(),
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
