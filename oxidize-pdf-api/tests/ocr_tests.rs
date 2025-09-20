//! Tests for OCR functionality in the community API
//!
//! These tests verify that the OCR endpoint works correctly with mock data
//! and handles various error cases appropriately.

use axum::http::StatusCode;
use http_body_util::BodyExt;
use oxidize_pdf_api::{app, OcrResponse};
use serde_json::Value;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_ocr_endpoint_success() {
    // Create a simple PDF for testing
    let pdf_data = create_simple_pdf();

    // Create multipart form data
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.pdf\"\r\n\
         Content-Type: application/pdf\r\n\r\n",
        boundary = boundary
    );

    let mut request_body = body.into_bytes();
    request_body.extend_from_slice(&pdf_data);
    request_body
        .extend_from_slice(format!("\r\n--{boundary}--\r\n", boundary = boundary).as_bytes());

    // Create the request
    let app = app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/ocr")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(axum::body::Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Get response body for debugging
    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();

    if status != StatusCode::OK {
        let error_body = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Expected OK status, got {:?}. Error body: {}",
            status, error_body
        );
    }

    let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify response structure
    assert!(response_json["text"].is_string());
    assert!(response_json["pages"].is_number());
    assert!(response_json["confidence"].is_number());
    assert!(response_json["processing_time_ms"].is_number());
    assert!(response_json["engine"].is_string());
    assert!(response_json["language"].is_string());

    // Verify mock data content
    assert_eq!(response_json["language"], "eng");
    assert!(response_json["confidence"].as_f64().unwrap() >= 0.0);
    assert!(response_json["confidence"].as_f64().unwrap() <= 1.0);
}

#[tokio::test]
async fn test_ocr_endpoint_no_file() {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!("--{boundary}--\r\n", boundary = boundary);

    let app = app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/ocr")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error for missing file
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_ocr_endpoint_invalid_pdf() {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.pdf\"\r\n\
         Content-Type: application/pdf\r\n\r\n\
         This is not a valid PDF file content\
         \r\n--{boundary}--\r\n",
        boundary = boundary
    );

    let app = app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/ocr")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error for invalid PDF
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

/// Create a simple PDF for testing
/// This generates a minimal but valid PDF document
fn create_simple_pdf() -> Vec<u8> {
    use oxidize_pdf::{Document, Font, Page};

    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Sample text for OCR testing")
        .unwrap();

    doc.add_page(page);

    let mut pdf_bytes = Vec::new();
    doc.write(&mut pdf_bytes).unwrap();

    pdf_bytes
}

#[tokio::test]
async fn test_ocr_response_structure() {
    // Test that our OCR response structure serializes correctly
    let response = OcrResponse {
        text: "Sample extracted text".to_string(),
        pages: 1,
        confidence: 0.95,
        processing_time_ms: 150,
        engine: "MockOCR".to_string(),
        language: "eng".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["text"], "Sample extracted text");
    assert_eq!(parsed["pages"], 1);
    assert_eq!(parsed["confidence"], 0.95);
    assert_eq!(parsed["processing_time_ms"], 150);
    assert_eq!(parsed["engine"], "MockOCR");
    assert_eq!(parsed["language"], "eng");
}
