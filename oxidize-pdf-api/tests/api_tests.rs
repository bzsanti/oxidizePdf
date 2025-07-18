//! Unit and integration tests for oxidize-pdf-api

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use oxidize_pdf_api::{app, CreatePdfRequest, ErrorResponse, ExtractTextResponse};
use serde_json::json;
use tower::util::ServiceExt;

#[cfg(test)]
mod unit_tests {
    use super::*;
    use axum::response::IntoResponse;
    use oxidize_pdf_api::AppError;

    #[test]
    fn test_create_pdf_request_deserialization() {
        let json = json!({
            "text": "Test text",
            "font_size": 24.0
        });

        let request: CreatePdfRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.text, "Test text");
        assert_eq!(request.font_size, Some(24.0));
    }

    #[test]
    fn test_create_pdf_request_default_font_size() {
        let json = json!({
            "text": "Test text"
        });

        let request: CreatePdfRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.text, "Test text");
        assert_eq!(request.font_size, None);
    }

    #[test]
    fn test_error_response_serialization() {
        let error = ErrorResponse {
            error: "Test error message".to_string(),
        };

        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["error"], "Test error message");
    }

    #[test]
    fn test_app_error_pdf_conversion() {
        let pdf_error = oxidize_pdf::PdfError::InvalidStructure("Invalid PDF data".to_string());
        let app_error: AppError = pdf_error.into();

        let response = app_error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_app_error_io_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let app_error: AppError = io_error.into();

        let response = app_error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_endpoint() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "oxidizePdf API");
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_create_pdf_endpoint_success() {
        let app = app();

        let request_body = json!({
            "text": "Test PDF content",
            "font_size": 24.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/pdf"
        );
        assert_eq!(
            response.headers().get("content-disposition").unwrap(),
            "attachment; filename=\"generated.pdf\""
        );

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(!body.is_empty());
        // PDF should start with %PDF
        assert!(body.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_create_pdf_endpoint_default_font_size() {
        let app = app();

        let request_body = json!({
            "text": "Test with default font"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_create_pdf_endpoint_empty_text() {
        let app = app();

        let request_body = json!({
            "text": "",
            "font_size": 24.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should still succeed with empty text
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_pdf_endpoint_invalid_json() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from("invalid json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_pdf_endpoint_missing_text_field() {
        let app = app();

        let request_body = json!({
            "font_size": 24.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_create_pdf_endpoint_large_text() {
        let app = app();

        let large_text = "Lorem ipsum ".repeat(1000);
        let request_body = json!({
            "text": large_text,
            "font_size": 12.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.len() > 1000); // Should be reasonably sized
    }

    #[tokio::test]
    async fn test_create_pdf_endpoint_special_characters() {
        let app = app();

        let request_body = json!({
            "text": "Special chars: © ® ™ € £ ¥ § ¶",
            "font_size": 24.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_404_for_unknown_endpoint() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/unknown")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_method_not_allowed() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_extract_text_endpoint_success() {
        let app = app();

        // Create a simple test PDF
        use oxidize_pdf::{Document, Font, Page};
        let mut doc = Document::new();
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(50.0, 750.0)
            .write("This is test text for extraction")
            .unwrap();
        doc.add_page(page);

        let mut pdf_bytes = Vec::new();
        doc.write(&mut pdf_bytes).unwrap();

        // Now test extraction
        let boundary = "----boundary----";
        let mut body = Vec::new();
        body.extend_from_slice(b"------boundary----\r\n");
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"test.pdf\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: application/pdf\r\n");
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(&pdf_bytes);
        body.extend_from_slice(b"\r\n------boundary------\r\n");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/extract")
                    .method("POST")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let extract_response: ExtractTextResponse = serde_json::from_slice(&body).unwrap();
        assert!(extract_response
            .text
            .contains("This is test text for extraction"));
        assert_eq!(extract_response.pages, 1);
    }

    #[tokio::test]
    async fn test_extract_text_endpoint_no_file() {
        let app = app();

        let boundary = "----boundary----";
        let body = "------boundary----\r\n\
             Content-Disposition: form-data; name=\"other\"\r\n\
             \r\n\
             Some data\r\n\
             ------boundary------\r\n"
            .to_string();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/extract")
                    .method("POST")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert!(error.error.contains("No file provided"));
    }

    #[tokio::test]
    async fn test_extract_text_endpoint_invalid_pdf() {
        let app = app();

        let boundary = "----boundary----";
        let mut body = Vec::new();
        body.extend_from_slice(b"------boundary----\r\n");
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"bad.pdf\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: application/pdf\r\n");
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(b"Not a valid PDF content");
        body.extend_from_slice(b"\r\n------boundary------\r\n");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/extract")
                    .method("POST")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert!(error.error.contains("Failed to parse PDF"));
    }

    #[tokio::test]
    async fn test_cors_headers_preflight() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("OPTIONS")
                    .header(header::ORIGIN, "http://example.com")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
        assert!(response
            .headers()
            .contains_key(header::ACCESS_CONTROL_ALLOW_METHODS));
    }

    #[tokio::test]
    async fn test_create_pdf_with_newlines() {
        let app = app();

        let request_body = json!({
            "text": "Line 1\nLine 2\nLine 3",
            "font_size": 16.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/pdf"
        );
    }
}

#[cfg(test)]
mod handler_tests {
    use super::*;
    use axum::extract::Json;
    use axum::response::IntoResponse;
    use oxidize_pdf_api::{create_pdf, health_check, AppError};

    #[tokio::test]
    async fn test_health_check_handler_directly() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_pdf_handler_with_mock_fs() {
        let request = CreatePdfRequest {
            text: "Direct handler test".to_string(),
            font_size: Some(18.0),
        };

        let result = create_pdf(Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_app_error_debug_trait() {
        let io_error = std::io::Error::other("test error");
        let app_error = AppError::Io(io_error);
        let debug_str = format!("{:?}", app_error);
        assert!(debug_str.contains("Io"));

        let pdf_error = oxidize_pdf::PdfError::InvalidStructure("test error".to_string());
        let app_error = AppError::Pdf(pdf_error);
        let debug_str = format!("{:?}", app_error);
        assert!(debug_str.contains("Pdf"));
    }

    #[test]
    fn test_extract_text_response_debug() {
        let response = ExtractTextResponse {
            text: "Test text".to_string(),
            pages: 5,
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("Test text"));
        assert!(debug_str.contains("5"));
    }
}
