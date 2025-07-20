use axum::{
    extract::{Json, Multipart},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use oxidize_pdf::{
    operations::{merge_pdfs, MergeInput, MergeOptions},
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

/// Build the application router with all routes configured
pub fn app() -> Router {
    Router::new()
        // Core operations
        .route("/api/create", post(create_pdf))
        .route("/api/health", get(health_check))
        .route("/api/extract", post(extract_text))
        // PDF operations
        .route("/api/merge", post(merge_pdfs_handler))
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
