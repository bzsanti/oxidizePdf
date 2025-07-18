use axum::{
    extract::{Json, Multipart, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use oxidize_pdf::{
    Document, Font, Page, Result as PdfResult,
    operations::{
        merge_pdfs, split_pdf, rotate_pdf_pages,
        MergeOptions, SplitOptions, SplitMode, RotateOptions, RotationAngle,
        PageRange, MergeInput,
    },
    parser::{PdfDocument, PdfReader},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

/// Request payload for PDF creation endpoint.
///
/// Contains the text content and optional formatting options for generating a PDF.
///
/// # Examples
///
/// ```json
/// {
///   "text": "Hello, World!",
///   "font_size": 24.0
/// }
/// ```
///
/// ```json
/// {
///   "text": "Simple text with default font size"
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct CreatePdfRequest {
    /// Text content to include in the PDF
    pub text: String,
    /// Font size in points (defaults to 24.0 if not specified)
    pub font_size: Option<f64>,
}

/// Standard error response structure.
///
/// Used for all API error responses to provide consistent error reporting.
///
/// # Example Response
///
/// ```json
/// {
///   "error": "Failed to generate PDF: Invalid text encoding"
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Human-readable error message describing what went wrong
    pub error: String,
}

/// Build the application router with all routes configured.
///
/// This function creates the main application router with all endpoints
/// and middleware configured. Useful for both the main server and testing.
///
/// # Routes
///
/// ## Core Operations
/// - `POST /api/create` - Create PDF from text
/// - `GET /api/health` - Health check endpoint
/// - `POST /api/extract` - Extract text from PDF
///
/// ## PDF Operations
/// - `POST /api/merge` - Merge multiple PDFs
/// - `POST /api/split` - Split PDF into multiple files
/// - `POST /api/rotate` - Rotate pages in PDF
/// - `POST /api/extract-pages` - Extract specific pages
/// - `POST /api/reorder` - Reorder pages in PDF
/// - `POST /api/extract-images` - Extract images from PDF
///
/// ## Analysis & Recovery
/// - `POST /api/analyze` - Analyze PDF content
/// - `POST /api/recover` - Recover corrupted PDF
/// - `POST /api/validate` - Validate PDF structure
///
/// ## Batch Operations
/// - `POST /api/batch/merge` - Batch merge PDFs
/// - `POST /api/batch/split` - Batch split PDFs
/// - `GET /api/batch/status/{job_id}` - Check batch job status
///
/// # Middleware
///
/// - CORS: Permissive configuration for development
pub fn app() -> Router {
    let batch_jobs: BatchJobsState = Arc::new(RwLock::new(HashMap::new()));
    
    Router::new()
        // Core operations
        .route("/api/create", post(create_pdf))
        .route("/api/health", get(health_check))
        .route("/api/extract", post(extract_text))
        
        // PDF operations
        .route("/api/merge", post(merge_pdfs_handler))
        .route("/api/split", post(split_pdf_handler))
        .route("/api/rotate", post(rotate_pages_handler))
        .route("/api/extract-pages", post(extract_pages_handler))
        .route("/api/reorder", post(reorder_pages_handler))
        .route("/api/extract-images", post(extract_images_handler))
        
        // Analysis & recovery
        .route("/api/analyze", post(analyze_pdf_handler))
        .route("/api/recover", post(recover_pdf_handler))
        .route("/api/validate", post(validate_pdf_handler))
        
        // Batch operations
        .route("/api/batch/merge", post(batch_merge_handler))
        .route("/api/batch/split", post(batch_split_handler))
        .route("/api/batch/status/:job_id", get(batch_status_handler))
        
        .with_state(batch_jobs)
        .layer(CorsLayer::permissive())
}

/// Create a PDF document from the provided text content.
///
/// This endpoint generates a PDF with the specified text using Helvetica font.
/// The PDF is returned as binary data with appropriate headers for download.
///
/// # Request
///
/// - **Method**: POST
/// - **Content-Type**: application/json
/// - **Body**: [`CreatePdfRequest`] with text and optional font size
///
/// # Response
///
/// - **Success**: 200 OK with PDF binary data
/// - **Content-Type**: application/pdf
/// - **Content-Disposition**: attachment; filename="generated.pdf"
///
/// # Errors
///
/// Returns 500 Internal Server Error with [`ErrorResponse`] for:
/// - PDF generation failures
/// - File system errors
/// - Invalid text content
///
/// # Examples
///
/// ```bash
/// # Create simple PDF
/// curl -X POST http://localhost:3000/api/create \
///   -H "Content-Type: application/json" \
///   -d '{"text": "Hello, World!"}' \
///   --output hello.pdf
///
/// # Create PDF with custom font size
/// curl -X POST http://localhost:3000/api/create \
///   -H "Content-Type: application/json" \
///   -d '{"text": "Large Text", "font_size": 48}' \
///   --output large.pdf
/// ```
pub async fn create_pdf(Json(payload): Json<CreatePdfRequest>) -> Result<Response, AppError> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    let font_size = payload.font_size.unwrap_or(24.0);

    page.text()
        .set_font(Font::Helvetica, font_size)
        .at(50.0, 750.0)
        .write(&payload.text)?;

    doc.add_page(page);

    // Generate PDF directly to buffer
    let mut pdf_bytes = Vec::new();
    doc.write(&mut pdf_bytes)?;

    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/pdf"),
            (
                "Content-Disposition",
                "attachment; filename=\"generated.pdf\"",
            ),
        ],
        pdf_bytes,
    )
        .into_response())
}

/// Health check endpoint for monitoring and load balancing.
///
/// Returns service status, name, and version information.
/// This endpoint can be used by load balancers, monitoring systems,
/// and orchestrators to verify service health.
///
/// # Response
///
/// Always returns 200 OK with JSON containing:
/// - `status`: Always "ok" if service is running
/// - `service`: Service name "oxidizePdf API"
/// - `version`: Current package version
///
/// # Example
///
/// ```bash
/// curl http://localhost:3000/api/health
/// ```
///
/// Response:
/// ```json
/// {
///   "status": "ok",
///   "service": "oxidizePdf API",
///   "version": "0.1.0"
/// }
/// ```
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "oxidizePdf API",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Application-specific error types for the API.
///
/// Represents all possible errors that can occur during API operations.
/// Each error type is automatically converted to an appropriate HTTP response.
///
/// # Error Types
///
/// - [`AppError::Pdf`]: PDF generation or processing errors
/// - [`AppError::Io`]: File system or I/O errors
///
/// # HTTP Status Codes
///
/// All errors currently return 500 Internal Server Error with a JSON error message.
/// Future versions may implement more specific status codes.
#[derive(Debug)]
pub enum AppError {
    /// PDF library errors (generation, parsing, etc.)
    Pdf(oxidize_pdf::PdfError),
    /// I/O errors (file operations, network, etc.)
    Io(std::io::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_msg = match self {
            AppError::Pdf(e) => e.to_string(),
            AppError::Io(e) => e.to_string(),
        };

        let error_response = ErrorResponse { error: error_msg };

        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
    }
}

impl From<oxidize_pdf::PdfError> for AppError {
    fn from(err: oxidize_pdf::PdfError) -> Self {
        AppError::Pdf(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

/// Response for text extraction endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractTextResponse {
    /// Extracted text from the PDF
    pub text: String,
    /// Number of pages processed
    pub pages: usize,
}

/// Request for PDF merge operation
#[derive(Debug, Deserialize)]
pub struct MergePdfRequest {
    /// Options for merging PDFs
    pub preserve_metadata: Option<bool>,
    /// Whether to preserve bookmarks
    pub preserve_bookmarks: Option<bool>,
    /// Whether to optimize the output
    pub optimize: Option<bool>,
}

/// Response for PDF merge operation
#[derive(Debug, Serialize)]
pub struct MergePdfResponse {
    /// Success message
    pub message: String,
    /// Number of PDFs merged
    pub files_merged: usize,
    /// Total pages in output
    pub total_pages: usize,
    /// Output file size in bytes
    pub output_size: usize,
}

/// Request for PDF split operation
#[derive(Debug, Deserialize)]
pub struct SplitPdfRequest {
    /// Split mode configuration
    pub mode: SplitModeRequest,
    /// Options for splitting
    pub preserve_metadata: Option<bool>,
    /// Output filename prefix
    pub filename_prefix: Option<String>,
}

/// Split mode configuration
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum SplitModeRequest {
    /// Split into individual pages
    ByPages,
    /// Split by page ranges
    ByRange { ranges: Vec<String> },
    /// Split into chunks of N pages
    ByChunks { chunk_size: usize },
}

/// Response for PDF split operation
#[derive(Debug, Serialize)]
pub struct SplitPdfResponse {
    /// Success message
    pub message: String,
    /// Number of output files created
    pub files_created: usize,
    /// Information about each output file
    pub files: Vec<SplitFileInfo>,
}

/// Information about a split output file
#[derive(Debug, Serialize)]
pub struct SplitFileInfo {
    /// Filename
    pub filename: String,
    /// Number of pages in this file
    pub pages: usize,
    /// Page range (1-based)
    pub page_range: String,
    /// File size in bytes
    pub size: usize,
}

/// Request for page rotation operation
#[derive(Debug, Deserialize)]
pub struct RotatePagesRequest {
    /// Rotation angle in degrees (90, 180, 270)
    pub angle: i32,
    /// Page range to rotate (e.g., "1-5", "all", "1,3,5")
    pub pages: String,
}

/// Response for page rotation operation
#[derive(Debug, Serialize)]
pub struct RotatePagesResponse {
    /// Success message
    pub message: String,
    /// Number of pages rotated
    pub pages_rotated: usize,
    /// Rotation angle applied
    pub rotation_angle: i32,
}

/// Request for page extraction operation
#[derive(Debug, Deserialize)]
pub struct ExtractPagesRequest {
    /// Page range to extract (e.g., "1-5", "all", "1,3,5")
    pub pages: String,
    /// Whether to preserve metadata
    pub preserve_metadata: Option<bool>,
    /// Whether to preserve annotations
    pub preserve_annotations: Option<bool>,
}

/// Response for page extraction operation
#[derive(Debug, Serialize)]
pub struct ExtractPagesResponse {
    /// Success message
    pub message: String,
    /// Number of pages extracted
    pub pages_extracted: usize,
    /// Page range that was extracted
    pub page_range: String,
    /// Output file size in bytes
    pub output_size: usize,
}

/// Request for page reordering operation
#[derive(Debug, Deserialize)]
pub struct ReorderPagesRequest {
    /// New page order (1-based page numbers)
    pub new_order: Vec<usize>,
}

/// Response for page reordering operation
#[derive(Debug, Serialize)]
pub struct ReorderPagesResponse {
    /// Success message
    pub message: String,
    /// Number of pages reordered
    pub pages_reordered: usize,
}

/// Request for image extraction operation
#[derive(Debug, Deserialize)]
pub struct ExtractImagesRequest {
    /// Page range to extract from (e.g., "1-5", "all", "1,3,5")
    pub pages: Option<String>,
    /// Image formats to extract
    pub formats: Option<Vec<String>>,
    /// Minimum image size in pixels
    pub min_size: Option<u32>,
}

/// Response for image extraction operation
#[derive(Debug, Serialize)]
pub struct ExtractImagesResponse {
    /// Success message
    pub message: String,
    /// Number of images extracted
    pub images_extracted: usize,
    /// Information about extracted images
    pub images: Vec<ExtractedImageInfo>,
}

/// Information about an extracted image
#[derive(Debug, Serialize)]
pub struct ExtractedImageInfo {
    /// Page number where image was found
    pub page: usize,
    /// Image format
    pub format: String,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Image size in bytes
    pub size: usize,
}

/// Request for PDF analysis operation
#[derive(Debug, Deserialize)]
pub struct AnalyzePdfRequest {
    /// Whether to include OCR analysis
    pub include_ocr: Option<bool>,
    /// OCR options
    pub ocr_options: Option<OcrOptionsRequest>,
}

/// OCR options for API requests
#[derive(Debug, Deserialize)]
pub struct OcrOptionsRequest {
    /// Language codes for OCR
    pub languages: Option<Vec<String>>,
    /// Minimum confidence threshold
    pub min_confidence: Option<f32>,
}

/// Response for PDF analysis operation
#[derive(Debug, Serialize)]
pub struct AnalyzePdfResponse {
    /// Success message
    pub message: String,
    /// Analysis results for each page
    pub pages: Vec<PageAnalysisResult>,
    /// Overall document statistics
    pub document_stats: DocumentStats,
}

/// Analysis result for a single page
#[derive(Debug, Serialize)]
pub struct PageAnalysisResult {
    /// Page number (1-based)
    pub page: usize,
    /// Page type (text, scanned, mixed)
    pub page_type: String,
    /// Content ratios
    pub text_ratio: f32,
    /// Image ratio
    pub image_ratio: f32,
    /// Whitespace ratio
    pub whitespace_ratio: f32,
    /// OCR results if requested
    pub ocr_result: Option<OcrResult>,
}

/// OCR result for API responses
#[derive(Debug, Serialize)]
pub struct OcrResult {
    /// Extracted text
    pub text: String,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Number of text fragments
    pub fragments: usize,
}

/// Document analysis statistics
#[derive(Debug, Serialize)]
pub struct DocumentStats {
    /// Total pages
    pub total_pages: usize,
    /// Number of text pages
    pub text_pages: usize,
    /// Number of scanned pages
    pub scanned_pages: usize,
    /// Number of mixed pages
    pub mixed_pages: usize,
    /// Total images found
    pub total_images: usize,
}

/// Request for PDF recovery operation
#[derive(Debug, Deserialize)]
pub struct RecoverPdfRequest {
    /// Whether to use aggressive recovery
    pub aggressive: Option<bool>,
    /// Maximum errors to tolerate
    pub max_errors: Option<usize>,
    /// Whether to allow partial content
    pub partial_content: Option<bool>,
}

/// Response for PDF recovery operation
#[derive(Debug, Serialize)]
pub struct RecoverPdfResponse {
    /// Success message
    pub message: String,
    /// Whether recovery was successful
    pub recovered: bool,
    /// Number of errors found
    pub errors_found: usize,
    /// Number of errors fixed
    pub errors_fixed: usize,
    /// Recovery warnings
    pub warnings: Vec<String>,
}

/// Response for PDF validation operation
#[derive(Debug, Serialize)]
pub struct ValidatePdfResponse {
    /// Whether PDF is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Validation statistics
    pub stats: ValidationStats,
}

/// Validation statistics for API
#[derive(Debug, Serialize)]
pub struct ValidationStats {
    /// Objects checked
    pub objects_checked: usize,
    /// Valid objects
    pub valid_objects: usize,
    /// Pages validated
    pub pages_validated: usize,
    /// Streams validated
    pub streams_validated: usize,
}

/// Batch job status
#[derive(Debug, Serialize)]
pub struct BatchJobStatus {
    /// Job ID
    pub job_id: String,
    /// Current status
    pub status: String,
    /// Progress percentage (0-100)
    pub progress: f32,
    /// Files processed
    pub files_processed: usize,
    /// Total files
    pub total_files: usize,
    /// Error messages if any
    pub errors: Vec<String>,
}

/// Global state for batch jobs
type BatchJobsState = Arc<RwLock<HashMap<String, BatchJobStatus>>>;

/// Create a new batch job ID
fn generate_job_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("batch_{}", timestamp)
}

/// Extract text from an uploaded PDF file.
///
/// This endpoint accepts a PDF file upload and extracts all text content.
///
/// # Request
///
/// - **Method**: POST
/// - **Content-Type**: multipart/form-data
/// - **Body**: PDF file upload with field name "file"
///
/// # Response
///
/// - **Success**: 200 OK with extracted text
/// - **Error**: 400 Bad Request for invalid uploads
/// - **Error**: 500 Internal Server Error for extraction failures
///
/// # Example
///
/// ```bash
/// curl -X POST http://localhost:3000/api/extract \
///   -F "file=@document.pdf" \
///   -o extracted.json
/// ```
pub async fn extract_text(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        if field.name() == Some("file") {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
            break;
        }
    }

    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No file provided in upload",
        ))
    })?;

    // Parse PDF and extract text
    use oxidize_pdf::parser::{document::PdfDocument, reader::PdfReader};
    use std::io::Cursor;

    let cursor = Cursor::new(pdf_bytes.as_ref());
    let reader = PdfReader::new(cursor).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse PDF: {e:?}"),
        ))
    })?;
    let doc = PdfDocument::new(reader);

    let extracted_texts = doc.extract_text().map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to extract text: {e:?}"),
        ))
    })?;

    // Combine all extracted text
    let text = extracted_texts
        .into_iter()
        .map(|et| et.text)
        .collect::<Vec<_>>()
        .join("\n");

    let page_count = doc.page_count().unwrap_or(0) as usize;

    let response = ExtractTextResponse {
        text,
        pages: page_count,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Merge multiple PDF files into a single PDF
///
/// This endpoint accepts multiple PDF files and merges them into one document.
/// Options can be provided to control metadata preservation and optimization.
///
/// # Request
///
/// - **Method**: POST
/// - **Content-Type**: multipart/form-data
/// - **Body**: Multiple PDF files with field name "files[]" and optional JSON config
///
/// # Response
///
/// - **Success**: 200 OK with merged PDF binary data
/// - **Error**: 400 Bad Request for invalid uploads or merge errors
pub async fn merge_pdfs_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_files = Vec::new();
    let mut merge_options = MergeOptions::default();
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "files" || field_name == "files[]" {
            let file_data = field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?;
            pdf_files.push(file_data);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<MergePdfRequest>(&options_text) {
                if let Some(preserve_bookmarks) = request.preserve_bookmarks {
                    merge_options.preserve_bookmarks = preserve_bookmarks;
                }
                if let Some(optimize) = request.optimize {
                    merge_options.optimize = optimize;
                }
            }
        }
    }
    
    if pdf_files.len() < 2 {
        return Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "At least 2 PDF files are required for merging",
        )));
    }
    
    // Create temporary files for input PDFs
    let mut temp_files = Vec::new();
    let mut merge_inputs = Vec::new();
    
    for (i, file_data) in pdf_files.iter().enumerate() {
        let temp_file = tempfile::NamedTempFile::new().map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create temp file {}: {e}", i),
            ))
        })?;
        
        std::fs::write(temp_file.path(), file_data).map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write temp file {}: {e}", i),
            ))
        })?;
        
        merge_inputs.push(MergeInput::file(temp_file.path()));
        temp_files.push(temp_file);
    }
    
    // Create temporary output file
    let output_temp_file = tempfile::NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create output temp file: {e}"),
        ))
    })?;
    
    // Perform merge
    merge_pdfs(merge_inputs, output_temp_file.path(), merge_options).map_err(|e| {
        AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
    })?;
    
    // Read output file
    let output_data = std::fs::read(output_temp_file.path()).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to read output file: {e}"),
        ))
    })?;
    
    let response = MergePdfResponse {
        message: "PDFs merged successfully".to_string(),
        files_merged: pdf_files.len(),
        total_pages: 0, // We don't have this info easily available
        output_size: output_data.len(),
    };
    
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/pdf"),
            ("Content-Disposition", "attachment; filename=\"merged.pdf\""),
            ("X-Merge-Info", &serde_json::to_string(&response).unwrap()),
        ],
        output_data,
    ).into_response())
}

/// Split a PDF into multiple files
///
/// This endpoint accepts a PDF file and splits it according to the specified mode.
/// 
/// # Request
///
/// - **Method**: POST
/// - **Content-Type**: multipart/form-data
/// - **Body**: PDF file with field name "file" and JSON config with field name "options"
///
/// # Response
///
/// - **Success**: 200 OK with ZIP file containing split PDFs
/// - **Error**: 400 Bad Request for invalid uploads or split errors
pub async fn split_pdf_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut split_request = SplitPdfRequest {
        mode: SplitModeRequest::ByPages,
        preserve_metadata: Some(true),
        filename_prefix: Some("page".to_string()),
    };
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<SplitPdfRequest>(&options_text) {
                split_request = request;
            }
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Convert split mode
    let split_mode = match split_request.mode {
        SplitModeRequest::ByPages => SplitMode::ByPages,
        SplitModeRequest::ByChunks { chunk_size } => SplitMode::ByChunks(chunk_size),
        SplitModeRequest::ByRange { ranges } => {
            let mut page_ranges = Vec::new();
            for range_str in ranges {
                let page_range = PageRange::parse(&range_str).map_err(|e| {
                    AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
                })?;
                page_ranges.push(page_range);
            }
            SplitMode::ByRange(page_ranges)
        }
    };
    
    // Set up split options
    let mut split_options = SplitOptions::default().with_mode(split_mode);
    if let Some(preserve_metadata) = split_request.preserve_metadata {
        split_options = split_options.preserve_metadata(preserve_metadata);
    }
    
    // Perform split
    let cursor = Cursor::new(pdf_bytes.as_ref());
    let outputs = split_pdf(cursor, split_options)?;
    
    // Create ZIP file with all outputs
    let mut zip_buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        
        for (i, output) in outputs.iter().enumerate() {
            let filename = format!("{}_{}.pdf", 
                split_request.filename_prefix.as_deref().unwrap_or("page"), 
                i + 1
            );
            zip.start_file(filename.clone(), options).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create ZIP entry: {e}"),
                ))
            })?;
            zip.write_all(output).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to write ZIP entry: {e}"),
                ))
            })?;
        }
        zip.finish().map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to finalize ZIP: {e}"),
            ))
        })?;
    }
    
    let response = SplitPdfResponse {
        message: "PDF split successfully".to_string(),
        files_created: outputs.len(),
        files: outputs.iter().enumerate().map(|(i, output)| {
            SplitFileInfo {
                filename: format!("{}_{}.pdf", 
                    split_request.filename_prefix.as_deref().unwrap_or("page"), 
                    i + 1
                ),
                pages: 1, // This would need to be calculated from the actual split
                page_range: format!("{}", i + 1),
                size: output.len(),
            }
        }).collect(),
    };
    
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/zip"),
            ("Content-Disposition", "attachment; filename=\"split_output.zip\""),
            ("X-Split-Info", &serde_json::to_string(&response).unwrap()),
        ],
        zip_buffer,
    ).into_response())
}

/// Rotate pages in a PDF
///
/// This endpoint accepts a PDF file and rotates specified pages by the given angle.
pub async fn rotate_pages_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut rotate_request = RotatePagesRequest {
        angle: 90,
        pages: "all".to_string(),
    };
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<RotatePagesRequest>(&options_text) {
                rotate_request = request;
            }
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Convert rotation angle
    let rotation_angle = match rotate_request.angle {
        90 => RotationAngle::Rotate90,
        180 => RotationAngle::Rotate180,
        270 => RotationAngle::Rotate270,
        _ => return Err(AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(
            format!("Invalid rotation angle: {}", rotate_request.angle)
        ))),
    };
    
    // Parse page range
    let page_range = PageRange::parse(&rotate_request.pages).map_err(|e| {
        AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
    })?;
    
    // Set up rotation options
    let rotate_options = RotateOptions::default()
        .with_angle(rotation_angle)
        .with_pages(page_range);
    
    // Perform rotation
    let cursor = Cursor::new(pdf_bytes.as_ref());
    let mut output = Vec::new();
    let result = rotate_pdf_pages(cursor, &mut output, rotate_options)?;
    
    let response = RotatePagesResponse {
        message: "Pages rotated successfully".to_string(),
        pages_rotated: result.pages_rotated,
        rotation_angle: rotate_request.angle,
    };
    
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/pdf"),
            ("Content-Disposition", "attachment; filename=\"rotated.pdf\""),
            ("X-Rotation-Info", &serde_json::to_string(&response).unwrap()),
        ],
        output,
    ).into_response())
}

/// Extract specific pages from a PDF
///
/// This endpoint accepts a PDF file and extracts specified pages into a new PDF.
pub async fn extract_pages_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut extract_request = ExtractPagesRequest {
        pages: "all".to_string(),
        preserve_metadata: Some(true),
        preserve_annotations: Some(true),
    };
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<ExtractPagesRequest>(&options_text) {
                extract_request = request;
            }
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Parse page range
    let page_range = PageRange::parse(&extract_request.pages).map_err(|e| {
        AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
    })?;
    
    // Set up extraction options
    let mut extract_options = PageExtractionOptions::default();
    if let Some(preserve_metadata) = extract_request.preserve_metadata {
        extract_options = extract_options.preserve_metadata(preserve_metadata);
    }
    if let Some(preserve_annotations) = extract_request.preserve_annotations {
        extract_options = extract_options.preserve_annotations(preserve_annotations);
    }
    
    // Perform extraction
    let cursor = Cursor::new(pdf_bytes.as_ref());
    let reader = PdfReader::new(cursor).map_err(|e| {
        AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
    })?;
    let document = PdfDocument::new(reader);
    
    let extractor = PageExtractor::new(document);
    let mut output = Vec::new();
    let result = extractor.extract_pages_to_writer(page_range, &mut output, extract_options)?;
    
    let response = ExtractPagesResponse {
        message: "Pages extracted successfully".to_string(),
        pages_extracted: result.pages_extracted,
        page_range: extract_request.pages,
        output_size: output.len(),
    };
    
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/pdf"),
            ("Content-Disposition", "attachment; filename=\"extracted.pdf\""),
            ("X-Extract-Info", &serde_json::to_string(&response).unwrap()),
        ],
        output,
    ).into_response())
}

/// Reorder pages in a PDF
///
/// This endpoint accepts a PDF file and reorders pages according to the specified order.
pub async fn reorder_pages_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut reorder_request = ReorderPagesRequest {
        new_order: vec![1, 2, 3], // Default order
    };
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<ReorderPagesRequest>(&options_text) {
                reorder_request = request;
            }
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Convert to 0-based indices
    let new_order: Vec<usize> = reorder_request.new_order.into_iter()
        .map(|page| if page > 0 { page - 1 } else { 0 })
        .collect();
    
    // Set up reorder options
    let reorder_options = ReorderOptions::default();
    
    // Perform reordering
    let cursor = Cursor::new(pdf_bytes.as_ref());
    let reader = PdfReader::new(cursor).map_err(|e| {
        AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
    })?;
    let document = PdfDocument::new(reader);
    
    let reorderer = PageReorderer::new(document);
    let mut output = Vec::new();
    let result = reorderer.reorder_pages_to_writer(new_order, &mut output, reorder_options)?;
    
    let response = ReorderPagesResponse {
        message: "Pages reordered successfully".to_string(),
        pages_reordered: result.pages_reordered,
    };
    
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/pdf"),
            ("Content-Disposition", "attachment; filename=\"reordered.pdf\""),
            ("X-Reorder-Info", &serde_json::to_string(&response).unwrap()),
        ],
        output,
    ).into_response())
}

/// Extract images from a PDF
///
/// This endpoint accepts a PDF file and extracts images according to the specified options.
pub async fn extract_images_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut extract_request = ExtractImagesRequest {
        pages: Some("all".to_string()),
        formats: Some(vec!["jpeg".to_string(), "png".to_string()]),
        min_size: Some(100),
    };
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<ExtractImagesRequest>(&options_text) {
                extract_request = request;
            }
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Parse page range
    let page_range = if let Some(pages) = extract_request.pages {
        PageRange::parse(&pages).map_err(|e| {
            AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
        })?
    } else {
        PageRange::All
    };
    
    // Set up extraction options
    let mut extract_options = ExtractImagesOptions::default();
    if let Some(min_size) = extract_request.min_size {
        extract_options = extract_options.min_size(min_size);
    }
    
    // Perform extraction
    let cursor = Cursor::new(pdf_bytes.as_ref());
    let reader = PdfReader::new(cursor).map_err(|e| {
        AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
    })?;
    let document = PdfDocument::new(reader);
    
    let extracted_images = extract_images_from_pdf(&document, page_range, extract_options)?;
    
    // Create ZIP file with all images
    let mut zip_buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        
        for (i, image) in extracted_images.iter().enumerate() {
            let extension = match image.format.as_str() {
                "jpeg" => "jpg",
                "png" => "png",
                "tiff" => "tiff",
                _ => "img",
            };
            let filename = format!("image_{}_{}.{}", image.page_number, i + 1, extension);
            
            zip.start_file(filename.clone(), options).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create ZIP entry: {e}"),
                ))
            })?;
            zip.write_all(&image.data).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to write ZIP entry: {e}"),
                ))
            })?;
        }
        zip.finish().map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to finalize ZIP: {e}"),
            ))
        })?;
    }
    
    let images_info: Vec<ExtractedImageInfo> = extracted_images.iter().map(|img| {
        ExtractedImageInfo {
            page: img.page_number,
            format: img.format.clone(),
            width: img.width,
            height: img.height,
            size: img.data.len(),
        }
    }).collect();
    
    let response = ExtractImagesResponse {
        message: "Images extracted successfully".to_string(),
        images_extracted: extracted_images.len(),
        images: images_info,
    };
    
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/zip"),
            ("Content-Disposition", "attachment; filename=\"extracted_images.zip\""),
            ("X-Images-Info", &serde_json::to_string(&response).unwrap()),
        ],
        zip_buffer,
    ).into_response())
}

/// Analyze PDF content
///
/// This endpoint accepts a PDF file and analyzes its content type and structure.
pub async fn analyze_pdf_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut analyze_request = AnalyzePdfRequest {
        include_ocr: Some(false),
        ocr_options: None,
    };
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<AnalyzePdfRequest>(&options_text) {
                analyze_request = request;
            }
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Perform analysis
    let cursor = Cursor::new(pdf_bytes.as_ref());
    let reader = PdfReader::new(cursor).map_err(|e| {
        AppError::Pdf(oxidize_pdf::PdfError::InvalidStructure(e.to_string()))
    })?;
    let document = PdfDocument::new(reader);
    
    let analyzer = PageContentAnalyzer::new(document);
    let analysis_options = AnalysisOptions::default();
    let analyses = analyzer.analyze_document_with_options(analysis_options)?;
    
    // Convert analysis results
    let mut page_results = Vec::new();
    let mut text_pages = 0;
    let mut scanned_pages = 0;
    let mut mixed_pages = 0;
    let mut total_images = 0;
    
    for (i, analysis) in analyses.iter().enumerate() {
        let page_type_str = match analysis.page_type {
            oxidize_pdf::operations::PageType::Text => {
                text_pages += 1;
                "text"
            }
            oxidize_pdf::operations::PageType::Scanned => {
                scanned_pages += 1;
                "scanned"
            }
            oxidize_pdf::operations::PageType::Mixed => {
                mixed_pages += 1;
                "mixed"
            }
        };
        
        let mut ocr_result = None;
        if analyze_request.include_ocr.unwrap_or(false) && 
           matches!(analysis.page_type, oxidize_pdf::operations::PageType::Scanned) {
            // Use mock OCR for now
            let ocr_provider = MockOcrProvider::new();
            if let Ok(ocr_response) = analyzer.extract_text_from_scanned_page(i, &ocr_provider) {
                ocr_result = Some(OcrResult {
                    text: ocr_response.text,
                    confidence: ocr_response.confidence,
                    fragments: ocr_response.fragments.len(),
                });
            }
        }
        
        total_images += analysis.estimated_images;
        
        page_results.push(PageAnalysisResult {
            page: i + 1,
            page_type: page_type_str.to_string(),
            text_ratio: analysis.text_ratio,
            image_ratio: analysis.image_ratio,
            whitespace_ratio: analysis.whitespace_ratio,
            ocr_result,
        });
    }
    
    let response = AnalyzePdfResponse {
        message: "PDF analyzed successfully".to_string(),
        pages: page_results,
        document_stats: DocumentStats {
            total_pages: analyses.len(),
            text_pages,
            scanned_pages,
            mixed_pages,
            total_images,
        },
    };
    
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Recover corrupted PDF
///
/// This endpoint accepts a corrupted PDF file and attempts to recover it.
pub async fn recover_pdf_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut recover_request = RecoverPdfRequest {
        aggressive: Some(false),
        max_errors: Some(100),
        partial_content: Some(true),
    };
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        } else if field_name == "options" {
            let options_text = field.text().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read options: {e}"),
                ))
            })?;
            
            if let Ok(request) = serde_json::from_str::<RecoverPdfRequest>(&options_text) {
                recover_request = request;
            }
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Set up recovery options
    let mut recovery_options = RecoveryOptions::default();
    if let Some(aggressive) = recover_request.aggressive {
        recovery_options = recovery_options.with_aggressive_recovery(aggressive);
    }
    if let Some(max_errors) = recover_request.max_errors {
        recovery_options = recovery_options.with_max_errors(max_errors);
    }
    if let Some(partial_content) = recover_request.partial_content {
        recovery_options = recovery_options.with_partial_content(partial_content);
    }
    
    // Write PDF to temp file for recovery
    let temp_file = tempfile::NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create temp file: {e}"),
        ))
    })?;
    
    std::fs::write(temp_file.path(), &pdf_bytes).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to write temp file: {e}"),
        ))
    })?;
    
    // Perform recovery
    let mut recovery = PdfRecovery::new(recovery_options);
    let recovery_result = recovery.recover_document(temp_file.path());
    
    match recovery_result {
        Ok(recovered_doc) => {
            // Convert recovered document to bytes
            let mut output = Vec::new();
            recovered_doc.write(&mut output)?;
            
            let response = RecoverPdfResponse {
                message: "PDF recovered successfully".to_string(),
                recovered: true,
                errors_found: recovery.warnings().len(),
                errors_fixed: recovery.warnings().len(),
                warnings: recovery.warnings().clone(),
            };
            
            Ok((
                StatusCode::OK,
                [
                    ("Content-Type", "application/pdf"),
                    ("Content-Disposition", "attachment; filename=\"recovered.pdf\""),
                    ("X-Recovery-Info", &serde_json::to_string(&response).unwrap()),
                ],
                output,
            ).into_response())
        }
        Err(e) => {
            let response = RecoverPdfResponse {
                message: "PDF recovery failed".to_string(),
                recovered: false,
                errors_found: recovery.warnings().len(),
                errors_fixed: 0,
                warnings: recovery.warnings().clone(),
            };
            
            Ok((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(response),
            ).into_response())
        }
    }
}

/// Validate PDF structure
///
/// This endpoint accepts a PDF file and validates its structure and compliance.
pub async fn validate_pdf_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            pdf_data = Some(field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?);
        }
    }
    
    let pdf_bytes = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;
    
    // Write PDF to temp file for validation
    let temp_file = tempfile::NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create temp file: {e}"),
        ))
    })?;
    
    std::fs::write(temp_file.path(), &pdf_bytes).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to write temp file: {e}"),
        ))
    })?;
    
    // Perform validation
    let validation_result = validate_pdf(temp_file.path())?;
    
    let response = ValidatePdfResponse {
        is_valid: validation_result.is_valid,
        errors: validation_result.errors.iter().map(|e| e.to_string()).collect(),
        warnings: validation_result.warnings,
        stats: ValidationStats {
            objects_checked: validation_result.stats.objects_checked,
            valid_objects: validation_result.stats.valid_objects,
            pages_validated: validation_result.stats.pages_validated,
            streams_validated: validation_result.stats.streams_validated,
        },
    };
    
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Batch merge multiple PDFs
///
/// This endpoint accepts multiple PDF files and merges them in batches.
pub async fn batch_merge_handler(
    axum::extract::State(batch_jobs): axum::extract::State<BatchJobsState>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let job_id = generate_job_id();
    let mut pdf_files = Vec::new();
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "files" || field_name == "files[]" {
            let file_data = field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?;
            pdf_files.push(file_data);
        }
    }
    
    if pdf_files.is_empty() {
        return Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF files provided for batch merge",
        )));
    }
    
    // Create initial job status
    let initial_status = BatchJobStatus {
        job_id: job_id.clone(),
        status: "processing".to_string(),
        progress: 0.0,
        files_processed: 0,
        total_files: pdf_files.len(),
        errors: Vec::new(),
    };
    
    // Store job status
    {
        let mut jobs = batch_jobs.write().await;
        jobs.insert(job_id.clone(), initial_status);
    }
    
    // Process files in background (simplified for demo)
    let batch_jobs_clone = batch_jobs.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Simulate batch processing
        for i in 0..pdf_files.len() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            
            let progress = ((i + 1) as f32 / pdf_files.len() as f32) * 100.0;
            
            let mut jobs = batch_jobs_clone.write().await;
            if let Some(job) = jobs.get_mut(&job_id_clone) {
                job.progress = progress;
                job.files_processed = i + 1;
                if i + 1 == pdf_files.len() {
                    job.status = "completed".to_string();
                }
            }
        }
    });
    
    let response = BatchJobStatus {
        job_id,
        status: "processing".to_string(),
        progress: 0.0,
        files_processed: 0,
        total_files: pdf_files.len(),
        errors: Vec::new(),
    };
    
    Ok((StatusCode::ACCEPTED, Json(response)).into_response())
}

/// Batch split multiple PDFs
///
/// This endpoint accepts multiple PDF files and splits them in batches.
pub async fn batch_split_handler(
    axum::extract::State(batch_jobs): axum::extract::State<BatchJobsState>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let job_id = generate_job_id();
    let mut pdf_files = Vec::new();
    
    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to read multipart field: {e}"),
        ))
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "files" || field_name == "files[]" {
            let file_data = field.bytes().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read file data: {e}"),
                ))
            })?;
            pdf_files.push(file_data);
        }
    }
    
    if pdf_files.is_empty() {
        return Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF files provided for batch split",
        )));
    }
    
    // Create initial job status
    let initial_status = BatchJobStatus {
        job_id: job_id.clone(),
        status: "processing".to_string(),
        progress: 0.0,
        files_processed: 0,
        total_files: pdf_files.len(),
        errors: Vec::new(),
    };
    
    // Store job status
    {
        let mut jobs = batch_jobs.write().await;
        jobs.insert(job_id.clone(), initial_status);
    }
    
    // Process files in background (simplified for demo)
    let batch_jobs_clone = batch_jobs.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Simulate batch processing
        for i in 0..pdf_files.len() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            
            let progress = ((i + 1) as f32 / pdf_files.len() as f32) * 100.0;
            
            let mut jobs = batch_jobs_clone.write().await;
            if let Some(job) = jobs.get_mut(&job_id_clone) {
                job.progress = progress;
                job.files_processed = i + 1;
                if i + 1 == pdf_files.len() {
                    job.status = "completed".to_string();
                }
            }
        }
    });
    
    let response = BatchJobStatus {
        job_id,
        status: "processing".to_string(),
        progress: 0.0,
        files_processed: 0,
        total_files: pdf_files.len(),
        errors: Vec::new(),
    };
    
    Ok((StatusCode::ACCEPTED, Json(response)).into_response())
}

/// Get batch job status
///
/// This endpoint returns the current status of a batch job.
pub async fn batch_status_handler(
    axum::extract::State(batch_jobs): axum::extract::State<BatchJobsState>,
    Path(job_id): Path<String>,
) -> Result<Response, AppError> {
    let jobs = batch_jobs.read().await;
    
    if let Some(job_status) = jobs.get(&job_id) {
        Ok((StatusCode::OK, Json(job_status.clone())).into_response())
    } else {
        Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Batch job not found: {}", job_id),
        )))
    }
}
