use axum::{
    extract::{Json, Multipart},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use oxidize_pdf::{
    operations::{
        merge_pdfs, reorder_pdf_pages, rotate_pdf_pages, split_pdf, MergeInput, MergeOptions,
        PageRange, RotateOptions, RotationAngle, SplitMode, SplitOptions,
    },
    parser::{PdfDocument, PdfReader},
    Document, Font, Page,
};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use tempfile::NamedTempFile;
use tower_http::cors::CorsLayer;

/// Request payload for PDF creation endpoint
#[derive(Debug, Deserialize)]
pub struct CreatePdfRequest {
    /// Text content to include in the PDF
    pub text: String,
    /// Font size in points (defaults to 24.0 if not specified)
    pub font_size: Option<f64>,
}

/// Standard error response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Human-readable error message describing what went wrong
    pub error: String,
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
    /// Output file size in bytes
    pub output_size: usize,
}

/// Request for PDF split operation
#[derive(Debug, Deserialize)]
pub struct SplitPdfRequest {
    /// Split mode: "pages" for individual pages, "chunks" for groups
    pub mode: String,
    /// Number of pages per chunk (for chunks mode)
    pub chunk_size: Option<usize>,
    /// Specific page ranges (for custom splits)
    #[allow(dead_code)]
    pub ranges: Option<Vec<String>>,
}

/// Request for PDF rotation
#[derive(Debug, Deserialize)]
pub struct RotatePdfRequest {
    /// Rotation angle: 90, 180, or 270
    pub angle: i32,
    /// Page range to rotate (e.g., "all", "1", "1-5", "1,3,5")
    pub pages: Option<String>,
}

/// Request for page reordering
#[derive(Debug, Deserialize)]
pub struct ReorderPdfRequest {
    /// New page order (0-based indices)
    pub page_order: Vec<usize>,
}

/// Response for PDF info endpoint
#[derive(Debug, Serialize)]
pub struct PdfInfoResponse {
    /// PDF version
    pub version: String,
    /// Number of pages
    pub page_count: usize,
    /// Document title
    pub title: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Creation date
    pub created: Option<String>,
    /// Modification date
    pub modified: Option<String>,
    /// File size in bytes
    pub file_size: usize,
}

/// Application-specific error types for the API
#[derive(Debug)]
pub enum AppError {
    /// PDF library errors (generation, parsing, etc.)
    Pdf(oxidize_pdf::PdfError),
    /// I/O errors (file operations, network, etc.)
    Io(std::io::Error),
    /// Operation errors from oxidize-pdf operations
    Operation(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_msg = match self {
            AppError::Pdf(e) => e.to_string(),
            AppError::Io(e) => e.to_string(),
            AppError::Operation(e) => e,
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

impl From<oxidize_pdf::operations::OperationError> for AppError {
    fn from(err: oxidize_pdf::operations::OperationError) -> Self {
        AppError::Operation(err.to_string())
    }
}

impl From<oxidize_pdf::parser::ParseError> for AppError {
    fn from(err: oxidize_pdf::parser::ParseError) -> Self {
        AppError::Pdf(oxidize_pdf::PdfError::ParseError(err.to_string()))
    }
}

/// Build the application router with all routes configured
pub fn app() -> Router {
    Router::new()
        // Core operations
        .route("/api/create", post(create_pdf))
        .route("/api/health", get(health_check))
        .route("/api/extract/text", post(extract_text))
        .route("/api/extract", post(extract_text)) // Keep for backward compatibility
        // PDF operations
        .route("/api/merge", post(merge_pdfs_handler))
        .route("/api/split", post(split_pdf_handler))
        .route("/api/rotate", post(rotate_pages_handler))
        .route("/api/reorder", post(reorder_pages_handler))
        .route("/api/extract/images", post(extract_images_handler))
        .route("/api/info", post(pdf_info_handler))
        .layer(CorsLayer::permissive())
}

/// Create a PDF document from the provided text content
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

/// Health check endpoint for monitoring and load balancing
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "oxidizePdf API",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Extract text from an uploaded PDF file
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
        let temp_file = NamedTempFile::new().map_err(|e| {
            AppError::Io(std::io::Error::other(format!(
                "Failed to create temp file {i}: {e}"
            )))
        })?;

        std::fs::write(temp_file.path(), file_data).map_err(|e| {
            AppError::Io(std::io::Error::other(format!(
                "Failed to write temp file {i}: {e}"
            )))
        })?;

        merge_inputs.push(MergeInput::new(temp_file.path()));
        temp_files.push(temp_file);
    }

    // Create temporary output file
    let output_temp_file = NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to create output temp file: {e}"
        )))
    })?;

    // Perform merge
    merge_pdfs(merge_inputs, output_temp_file.path(), merge_options)
        .map_err(|e| AppError::Operation(format!("Failed to merge PDFs: {e}")))?;

    // Read output file
    let output_data = std::fs::read(output_temp_file.path()).map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to read output file: {e}"
        )))
    })?;

    let response = MergePdfResponse {
        message: "PDFs merged successfully".to_string(),
        files_merged: pdf_files.len(),
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
    )
        .into_response())
}

/// Split a PDF file into multiple PDFs
pub async fn split_pdf_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut split_request = SplitPdfRequest {
        mode: "pages".to_string(),
        chunk_size: None,
        ranges: None,
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
            if let Ok(req) = serde_json::from_str::<SplitPdfRequest>(&options_text) {
                split_request = req;
            }
        }
    }

    let pdf_data = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;

    // Create split options based on request
    let split_options = match split_request.mode.as_str() {
        "pages" => SplitOptions {
            mode: SplitMode::SinglePages,
            ..Default::default()
        },
        "chunks" => SplitOptions {
            mode: SplitMode::ChunkSize(split_request.chunk_size.unwrap_or(1)),
            ..Default::default()
        },
        _ => {
            return Err(AppError::Operation(format!(
                "Invalid split mode: {}",
                split_request.mode
            )));
        }
    };

    // Write PDF to temporary file
    let mut temp_file = NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to create temp file: {e}"
        )))
    })?;
    std::io::Write::write_all(&mut temp_file, &pdf_data)?;
    let temp_path = temp_file.path().to_path_buf();

    // Perform split operation
    let output_files = split_pdf(&temp_path, split_options)?;

    // For simplicity, return the first split file
    // In a real implementation, you might zip all files or provide a different response
    if let Some(first_file_path) = output_files.first() {
        let file_data = std::fs::read(first_file_path)?;
        Ok((
            StatusCode::OK,
            [
                ("Content-Type", "application/pdf"),
                (
                    "Content-Disposition",
                    "attachment; filename=\"split_1.pdf\"",
                ),
            ],
            file_data,
        )
            .into_response())
    } else {
        Err(AppError::Operation("No output files generated".to_string()))
    }
}

/// Rotate pages in a PDF
pub async fn rotate_pages_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut rotate_request = RotatePdfRequest {
        angle: 90,
        pages: None,
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
            if let Ok(req) = serde_json::from_str::<RotatePdfRequest>(&options_text) {
                rotate_request = req;
            }
        }
    }

    let pdf_data = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;

    // Parse rotation angle
    let angle = match rotate_request.angle {
        90 => RotationAngle::Clockwise90,
        180 => RotationAngle::Rotate180,
        270 => RotationAngle::Clockwise270,
        _ => {
            return Err(AppError::Operation(format!(
                "Invalid rotation angle: {}. Must be 90, 180, or 270",
                rotate_request.angle
            )));
        }
    };

    // Parse page range
    let page_range = if let Some(pages) = rotate_request.pages {
        PageRange::parse(&pages)?
    } else {
        PageRange::All
    };

    // Write PDF to temporary file
    let mut input_temp = NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to create temp file: {e}"
        )))
    })?;
    std::io::Write::write_all(&mut input_temp, &pdf_data)?;
    let input_path = input_temp.path().to_path_buf();

    // Create output temp file
    let output_temp = NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to create temp file: {e}"
        )))
    })?;
    let output_path = output_temp.path().to_path_buf();

    let options = RotateOptions {
        angle,
        pages: page_range,
        preserve_page_size: true,
    };

    // Perform rotation
    rotate_pdf_pages(&input_path, &output_path, options)?;

    // Read output
    let output = std::fs::read(&output_path)?;

    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/pdf"),
            (
                "Content-Disposition",
                "attachment; filename=\"rotated.pdf\"",
            ),
        ],
        output,
    )
        .into_response())
}

/// Reorder pages in a PDF
pub async fn reorder_pages_handler(mut multipart: Multipart) -> Result<Response, AppError> {
    let mut pdf_data = None;
    let mut reorder_request = None;

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
            if let Ok(req) = serde_json::from_str::<ReorderPdfRequest>(&options_text) {
                reorder_request = Some(req);
            }
        }
    }

    let pdf_data = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;

    let reorder_request = reorder_request.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No reorder options provided",
        ))
    })?;

    // Write PDF to temporary file
    let mut input_temp = NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to create temp file: {e}"
        )))
    })?;
    std::io::Write::write_all(&mut input_temp, &pdf_data)?;
    let input_path = input_temp.path().to_path_buf();

    // Create output temp file
    let output_temp = NamedTempFile::new().map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to create temp file: {e}"
        )))
    })?;
    let output_path = output_temp.path().to_path_buf();

    // Perform reordering
    reorder_pdf_pages(&input_path, &output_path, reorder_request.page_order)?;

    // Read output
    let output = std::fs::read(&output_path)?;

    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "application/pdf"),
            (
                "Content-Disposition",
                "attachment; filename=\"reordered.pdf\"",
            ),
        ],
        output,
    )
        .into_response())
}

/// Extract images from a PDF
pub async fn extract_images_handler(mut multipart: Multipart) -> Result<Response, AppError> {
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

    let pdf_data = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;

    // For now, just parse the PDF and return image count
    // Full image extraction would require writing to disk
    let cursor = Cursor::new(pdf_data.as_ref());
    let mut reader = PdfReader::new(cursor)?;
    let page_count = reader.page_count()? as usize;

    // TODO: Implement actual image extraction
    // This would require parsing each page's content stream
    // and extracting image objects

    let response = serde_json::json!({
        "images_found": 0,
        "message": "Image extraction endpoint is under development",
        "page_count": page_count
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Get PDF information/metadata
pub async fn pdf_info_handler(mut multipart: Multipart) -> Result<Response, AppError> {
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

    let pdf_data = pdf_data.ok_or_else(|| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No PDF file provided",
        ))
    })?;

    // Read PDF info
    let cursor = Cursor::new(pdf_data.as_ref());
    let mut reader = PdfReader::new(cursor)?;

    let version = reader.version().to_string();
    let page_count = reader.page_count()? as usize;

    let mut title = None;
    let mut author = None;
    let mut created = None;
    let mut modified = None;

    if let Ok(Some(info)) = reader.info() {
        title = info.get("Title").and_then(|obj| {
            obj.as_string()
                .map(|s| String::from_utf8_lossy(&s.0).to_string())
        });
        author = info.get("Author").and_then(|obj| {
            obj.as_string()
                .map(|s| String::from_utf8_lossy(&s.0).to_string())
        });
        created = info.get("CreationDate").and_then(|obj| {
            obj.as_string()
                .map(|s| String::from_utf8_lossy(&s.0).to_string())
        });
        modified = info.get("ModDate").and_then(|obj| {
            obj.as_string()
                .map(|s| String::from_utf8_lossy(&s.0).to_string())
        });
    }

    let response = PdfInfoResponse {
        version,
        page_count,
        title,
        author,
        created,
        modified,
        file_size: pdf_data.len(),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}
