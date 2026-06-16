#![cfg(feature = "ocr-vlm")]

use oxidize_pdf::graphics::ImageFormat;
use oxidize_pdf::text::ocr::{OcrEngine, OcrError, OcrProvider};
use oxidize_pdf::text::vlm_provider::{VlmConfig, VlmOcrProvider};

// ===========================================================================
// 1. Fail-safe: missing API key → AuthenticationError at construction
// ===========================================================================

#[test]
fn construction_fails_without_api_key_when_required() {
    let config = VlmConfig::builder()
        .endpoint("https://api.openai.com/v1")
        .model("gpt-4o")
        .build();

    let result = VlmOcrProvider::new(config);
    assert!(
        matches!(result, Err(OcrError::AuthenticationError(ref msg)) if msg.contains("API key required")),
        "expected AuthenticationError, got: {result:?}"
    );
}

#[test]
fn construction_succeeds_without_api_key_for_ollama() {
    let config = VlmConfig::ollama("llava");
    assert!(
        VlmOcrProvider::new(config).is_ok(),
        "Ollama should not require API key"
    );
}

// ===========================================================================
// 2. Engine metadata
// ===========================================================================

#[test]
fn engine_name_contains_model() {
    let config = VlmConfig::ollama("llava:13b");
    let provider = VlmOcrProvider::new(config).unwrap();
    assert_eq!(provider.engine_name(), "vlm:llava:13b");
}

#[test]
fn engine_type_is_vlm() {
    let config = VlmConfig::ollama("llava");
    let provider = VlmOcrProvider::new(config).unwrap();
    assert_eq!(provider.engine_type(), OcrEngine::Vlm);
}

// ===========================================================================
// 3. OcrEngine::Vlm variant
// ===========================================================================

#[test]
fn ocr_engine_vlm_name_and_formats() {
    assert_eq!(OcrEngine::Vlm.name(), "VLM");
    assert!(OcrEngine::Vlm.supports_format(ImageFormat::Png));
    assert!(OcrEngine::Vlm.supports_format(ImageFormat::Jpeg));
    assert!(OcrEngine::Vlm.supports_format(ImageFormat::Tiff));
    assert!(!OcrEngine::Vlm.supports_format(ImageFormat::Raw));
}

// ===========================================================================
// 4. MIME / validate_image_data
// ===========================================================================

#[test]
fn validate_image_data_accepts_png() {
    let png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let p = VlmOcrProvider::new(VlmConfig::ollama("m")).unwrap();
    assert!(p.validate_image_data(&png).is_ok());
}

#[test]
fn validate_image_data_accepts_jpeg() {
    let jpeg: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    let p = VlmOcrProvider::new(VlmConfig::ollama("m")).unwrap();
    assert!(p.validate_image_data(&jpeg).is_ok());
}

#[test]
fn validate_image_data_accepts_tiff_le() {
    let tiff: Vec<u8> = vec![b'I', b'I', 0x2A, 0x00, 0x08, 0x00];
    let p = VlmOcrProvider::new(VlmConfig::ollama("m")).unwrap();
    assert!(p.validate_image_data(&tiff).is_ok());
}

#[test]
fn validate_image_data_accepts_tiff_be() {
    let tiff: Vec<u8> = vec![b'M', b'M', 0x00, 0x2A, 0x00, 0x08];
    let p = VlmOcrProvider::new(VlmConfig::ollama("m")).unwrap();
    assert!(p.validate_image_data(&tiff).is_ok());
}

#[test]
fn validate_image_data_rejects_unknown() {
    let garbage: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
    let p = VlmOcrProvider::new(VlmConfig::ollama("m")).unwrap();
    assert!(p.validate_image_data(&garbage).is_err());
}

#[test]
fn validate_image_data_rejects_too_short() {
    let p = VlmOcrProvider::new(VlmConfig::ollama("m")).unwrap();
    assert!(p.validate_image_data(&[0xFF, 0xD8]).is_err());
}

// ===========================================================================
// 5. Config convenience constructors
// ===========================================================================

#[test]
fn config_openai_defaults() {
    let c = VlmConfig::openai("sk-test", "gpt-4o");
    assert_eq!(c.endpoint, "https://api.openai.com/v1");
    assert_eq!(c.model, "gpt-4o");
    assert_eq!(c.api_key.as_deref(), Some("sk-test"));
    assert_eq!(c.max_tokens, 4096);
    assert_eq!(c.temperature, 0.0);
    assert_eq!(c.timeout_seconds, 60);
    assert!(c.prompt_override.is_none());
}

#[test]
fn config_anthropic_defaults() {
    let c = VlmConfig::anthropic("sk-ant", "claude-sonnet-4-20250514");
    assert_eq!(c.endpoint, "https://api.anthropic.com");
    assert_eq!(c.model, "claude-sonnet-4-20250514");
    assert_eq!(c.api_key.as_deref(), Some("sk-ant"));
}

#[test]
fn config_ollama_defaults() {
    let c = VlmConfig::ollama("llava");
    assert_eq!(c.endpoint, "http://localhost:11434");
    assert_eq!(c.model, "llava");
    assert!(c.api_key.is_none());
}

// ===========================================================================
// 6. Builder
// ===========================================================================

#[test]
fn builder_produces_correct_config() {
    let c = VlmConfig::builder()
        .endpoint("https://custom.api/v1")
        .model("custom-model")
        .api_key("my-key")
        .max_tokens(8192)
        .temperature(0.5)
        .prompt_override("Extract text.")
        .timeout_seconds(120)
        .build();

    assert_eq!(c.endpoint, "https://custom.api/v1");
    assert_eq!(c.model, "custom-model");
    assert_eq!(c.api_key.as_deref(), Some("my-key"));
    assert_eq!(c.max_tokens, 8192);
    assert_eq!(c.temperature, 0.5);
    assert_eq!(c.prompt_override.as_deref(), Some("Extract text."));
    assert_eq!(c.timeout_seconds, 120);
}

// ===========================================================================
// 7. Supported formats
// ===========================================================================

#[test]
fn supported_formats_match_spec() {
    let p = VlmOcrProvider::new(VlmConfig::ollama("m")).unwrap();
    let fmts = p.supported_formats();
    assert!(fmts.contains(&ImageFormat::Png));
    assert!(fmts.contains(&ImageFormat::Jpeg));
    assert!(fmts.contains(&ImageFormat::Tiff));
    assert!(!fmts.contains(&ImageFormat::Raw));
}

// ===========================================================================
// 8. Adapter auto-detection
// ===========================================================================

#[test]
fn auto_detect_anthropic_endpoint() {
    let c = VlmConfig::anthropic("key", "claude-sonnet-4-20250514");
    let p = VlmOcrProvider::new(c).unwrap();
    assert_eq!(p.engine_type(), OcrEngine::Vlm);
}

#[test]
fn auto_detect_ollama_endpoint() {
    let c = VlmConfig::ollama("llava");
    let p = VlmOcrProvider::new(c).unwrap();
    assert_eq!(p.engine_type(), OcrEngine::Vlm);
}

#[test]
fn auto_detect_openai_for_generic_endpoint() {
    let c = VlmConfig::builder()
        .endpoint("https://my-custom-api.com/v1")
        .model("some-model")
        .api_key("key")
        .build();
    let p = VlmOcrProvider::new(c).unwrap();
    assert_eq!(p.engine_type(), OcrEngine::Vlm);
}
