//! # oxidize-pdf-api
//!
//! REST API server for oxidize-pdf library
//!

mod api;
// mod api_old; // Not using old API implementation

pub use api::{
    app,
    create_pdf,
    extract_text,
    health_check,
    // PDF Operations
    merge_pdfs_handler,
    AppError,
    CreatePdfRequest,
    ErrorResponse,
    ExtractTextResponse,
    // Request/Response Types
    MergePdfRequest,
    MergePdfResponse,
};

#[cfg(test)]
mod api_tests;
