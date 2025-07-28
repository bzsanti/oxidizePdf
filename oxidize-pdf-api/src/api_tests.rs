//! Tests for the API endpoints

#[cfg(test)]
mod tests {
    use super::super::api::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;

    #[test]
    fn test_sync_simple() {
        // Basic synchronous test to verify tests run
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_async_simple() {
        // Basic async test to verify tokio runtime works
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), async { 42 }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = app();

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            app.oneshot(
                Request::builder()
                    .uri("/api/health")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            ),
        )
        .await
        .expect("Request timed out")
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "oxidizePdf API");
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_create_pdf_simple() {
        let app = app();

        let request_body = json!({
            "text": "Hello, World!"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok());
        assert_eq!(content_type, Some("application/pdf"));

        let content_disposition = response
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok());
        assert_eq!(
            content_disposition,
            Some("attachment; filename=\"generated.pdf\"")
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(!body.is_empty());
        // PDF files start with %PDF
        assert!(body.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_create_pdf_with_font_size() {
        let app = app();

        let request_body = json!({
            "text": "Large Text",
            "font_size": 48.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(!body.is_empty());
        assert!(body.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_create_pdf_empty_text() {
        let app = app();

        let request_body = json!({
            "text": ""
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should still succeed with empty text
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_create_pdf_long_text() {
        let app = app();

        let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(50);
        let request_body = json!({
            "text": long_text,
            "font_size": 12.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
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

    #[tokio::test]
    async fn test_create_pdf_unicode_text() {
        let app = app();

        let request_body = json!({
            "text": "Hello 世界 🌍 مرحبا",
            "font_size": 24.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_create_pdf_invalid_json() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from("{invalid json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum will return 400 for invalid JSON
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_pdf_missing_text_field() {
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
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Missing required field should return 422 or 400
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_create_pdf_negative_font_size() {
        let app = app();

        let request_body = json!({
            "text": "Test",
            "font_size": -10.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should handle negative font size gracefully (might succeed or return error)
        // For now, just check that we get a response
        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_extract_text_no_file() {
        let app = app();

        let boundary = "boundary123456";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"other\"\r\n\r\n\
             not a file\r\n\
             --{boundary}--\r\n"
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/extract")
                    .method("POST")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].is_string());
        let error_msg = json["error"].as_str().unwrap();
        assert!(
            error_msg.contains("No file provided"),
            "Error was: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_extract_text_invalid_pdf() {
        let app = app();

        let boundary = "boundary123456";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"test.pdf\"\r\n\
             Content-Type: application/pdf\r\n\r\n\
             This is not a valid PDF file\r\n\
             --{boundary}--\r\n"
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/extract")
                    .method("POST")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].is_string());
        let error_msg = json["error"].as_str().unwrap();
        assert!(
            error_msg.contains("Failed to parse PDF"),
            "Error was: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_extract_text_valid_pdf() {
        let app = app();

        // First create a PDF using the create endpoint
        let create_body = json!({
            "text": "Test content for extraction",
            "font_size": 12.0
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&create_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let pdf_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();

        // Now test extracting text from this PDF
        let boundary = "boundary123456";
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"test.pdf\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
        body.extend_from_slice(&pdf_bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/extract")
                    .method("POST")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["text"].is_string());
        assert!(json["pages"].is_number());
        assert_eq!(json["pages"], 1);
        // The extracted text should contain our original text
        assert!(json["text"]
            .as_str()
            .unwrap()
            .contains("Test content for extraction"));
    }

    #[tokio::test]
    async fn test_404_not_found() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nonexistent")
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

        // Try DELETE on create endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/create")
                    .method("DELETE")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_create_pdf_request_deserialize() {
        let json = r#"{"text": "Hello", "font_size": 24.0}"#;
        let request: CreatePdfRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.text, "Hello");
        assert_eq!(request.font_size, Some(24.0));

        let json = r#"{"text": "Hello"}"#;
        let request: CreatePdfRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.text, "Hello");
        assert_eq!(request.font_size, None);
    }

    #[test]
    fn test_error_response_serialize() {
        let error = ErrorResponse {
            error: "Test error".to_string(),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(json, r#"{"error":"Test error"}"#);
    }

    #[test]
    fn test_extract_text_response_serialize() {
        let response = ExtractTextResponse {
            text: "Extracted text".to_string(),
            pages: 5,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"text\":\"Extracted text\""));
        assert!(json.contains("\"pages\":5"));
    }

    #[test]
    fn test_app_error_from_pdf_error() {
        let pdf_error = oxidize_pdf::PdfError::InvalidStructure("test".to_string());
        let app_error: AppError = pdf_error.into();
        match app_error {
            AppError::Pdf(_) => {}
            _ => panic!("Expected AppError::Pdf"),
        }
    }

    #[test]
    fn test_app_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_error: AppError = io_error.into();
        match app_error {
            AppError::Io(_) => {}
            _ => panic!("Expected AppError::Io"),
        }
    }
}
