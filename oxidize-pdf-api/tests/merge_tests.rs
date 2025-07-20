use axum::body::Body;
use axum::http::{Request, StatusCode};
use oxidize_pdf::{Document, Font, Page};
use oxidize_pdf_api::app;
use serde_json::json;
use std::io::Write;
use tower::ServiceExt;

/// Helper function to create a simple PDF for testing
fn create_test_pdf(text: &str) -> Vec<u8> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write(text)
        .unwrap();

    doc.add_page(page);

    let mut pdf_bytes = Vec::new();
    doc.write(&mut pdf_bytes).unwrap();
    pdf_bytes
}

/// Helper function to create multipart form data with PDF files
fn create_multipart_merge_request(pdf_files: Vec<Vec<u8>>) -> Request<Body> {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut body = Vec::new();

    // Add PDF files
    for (i, pdf_data) in pdf_files.iter().enumerate() {
        write!(body, "--{}\r\n", boundary).unwrap();
        write!(
            body,
            "Content-Disposition: form-data; name=\"files\"; filename=\"test{}.pdf\"\r\n",
            i + 1
        )
        .unwrap();
        write!(body, "Content-Type: application/pdf\r\n\r\n").unwrap();
        body.extend_from_slice(pdf_data);
        write!(body, "\r\n").unwrap();
    }

    // Add options
    write!(body, "--{}\r\n", boundary).unwrap();
    write!(
        body,
        "Content-Disposition: form-data; name=\"options\"\r\n\r\n"
    )
    .unwrap();
    let options = json!({
        "preserve_bookmarks": true,
        "optimize": false
    });
    write!(body, "{}\r\n", options).unwrap();

    write!(body, "--{}--\r\n", boundary).unwrap();

    Request::builder()
        .method("POST")
        .uri("/api/merge")
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn test_merge_endpoint_success() {
    let app = app();

    // Create two test PDFs
    let pdf1 = create_test_pdf("First PDF content");
    let pdf2 = create_test_pdf("Second PDF content");

    let request = create_multipart_merge_request(vec![pdf1, pdf2]);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    assert_eq!(headers.get("content-type").unwrap(), "application/pdf");
    assert!(headers
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("merged.pdf"));

    // Check that we have merge info header
    assert!(headers.get("x-merge-info").is_some());

    // Check that the response body contains PDF data
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(!body.is_empty());

    // Basic check that it's a PDF file
    assert!(body.starts_with(b"%PDF"));
}

#[tokio::test]
async fn test_merge_endpoint_insufficient_files() {
    let app = app();

    // Create only one PDF (should fail)
    let pdf1 = create_test_pdf("Single PDF content");

    let request = create_multipart_merge_request(vec![pdf1]);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_response: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(error_response["error"]
        .as_str()
        .unwrap()
        .contains("At least 2 PDF files are required"));
}

#[tokio::test]
async fn test_merge_endpoint_no_files() {
    let app = app();

    let request = create_multipart_merge_request(vec![]);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_response: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(error_response["error"]
        .as_str()
        .unwrap()
        .contains("At least 2 PDF files are required"));
}

#[tokio::test]
async fn test_merge_endpoint_three_files() {
    let app = app();

    // Create three test PDFs
    let pdf1 = create_test_pdf("First PDF content");
    let pdf2 = create_test_pdf("Second PDF content");
    let pdf3 = create_test_pdf("Third PDF content");

    let request = create_multipart_merge_request(vec![pdf1, pdf2, pdf3]);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    assert_eq!(headers.get("content-type").unwrap(), "application/pdf");

    // Check merge info header
    let merge_info = headers.get("x-merge-info").unwrap().to_str().unwrap();
    let merge_data: serde_json::Value = serde_json::from_str(merge_info).unwrap();

    assert_eq!(merge_data["files_merged"], 3);
    assert_eq!(merge_data["message"], "PDFs merged successfully");

    // Check that the response body contains PDF data
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(!body.is_empty());
    assert!(body.starts_with(b"%PDF"));
}

#[tokio::test]
async fn test_merge_endpoint_invalid_multipart() {
    let app = app();

    let request = Request::builder()
        .method("POST")
        .uri("/api/merge")
        .header("Content-Type", "multipart/form-data; boundary=invalid")
        .body(Body::from("invalid data"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
