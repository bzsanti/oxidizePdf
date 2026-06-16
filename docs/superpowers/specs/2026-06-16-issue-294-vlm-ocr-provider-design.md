# VLM OCR Provider — Design Spec (#294)

## 1. Goal

Implement `OcrProvider` for Vision Language Models. A single `VlmOcrProvider` struct
delegates HTTP serialization to an internal `ApiAdapter` trait with three built-in
adapters: OpenAI-compatible, Anthropic, and Ollama. Feature-gated under `ocr-vlm`.

## 2. Architecture

```
VlmOcrProvider (pub, implements OcrProvider)
  ├── config: VlmConfig
  ├── adapter: Box<dyn ApiAdapter + Send + Sync>
  └── runtime: tokio::runtime::Runtime   (created once in ::new(), reused)
```

### 2.1 VlmConfig (pub)

```rust
pub struct VlmConfig {
    pub endpoint: String,           // base URL (e.g. "https://api.openai.com/v1")
    pub model: String,              // e.g. "gpt-4o", "claude-sonnet-4-20250514"
    pub api_key: Option<String>,    // None = fail-safe error if adapter requires it
    pub max_tokens: u32,            // default 4096
    pub temperature: f32,           // default 0.0 (deterministic OCR)
    pub prompt_override: Option<String>,
    pub timeout_seconds: u32,       // default 60, overridden by OcrOptions.timeout_seconds
}
```

Builder pattern via `VlmConfigBuilder` (standard Rust builder, no derive macro).

Constructors for common setups:

```rust
VlmConfig::openai(api_key: impl Into<String>, model: impl Into<String>) -> Self
VlmConfig::anthropic(api_key: impl Into<String>, model: impl Into<String>) -> Self
VlmConfig::ollama(model: impl Into<String>) -> Self  // endpoint defaults to localhost:11434
```

### 2.2 ApiAdapter (pub(crate) trait)

```rust
pub(crate) trait ApiAdapter: Send + Sync {
    fn build_request(
        &self,
        client: &reqwest::Client,
        config: &VlmConfig,
        image_base64: &str,
        mime_type: &str,
        prompt: &str,
    ) -> reqwest::RequestBuilder;

    fn parse_response(&self, body: &serde_json::Value) -> OcrResult<String>;

    fn requires_api_key(&self) -> bool;
}
```

Three implementations:

- **OpenAiAdapter**: `POST {endpoint}/chat/completions`. Body:
  `{"model": M, "max_tokens": N, "temperature": T, "messages": [{"role":"user","content":[{"type":"image_url","image_url":{"url":"data:{mime};base64,{b64}"}},{"type":"text","text":PROMPT}]}]}`.
  Response: `choices[0].message.content`. `requires_api_key() = true`.

- **AnthropicAdapter**: `POST {endpoint}/v1/messages`. Headers: `x-api-key`, `anthropic-version: 2023-06-01`.
  Body: `{"model": M, "max_tokens": N, "temperature": T, "messages": [{"role":"user","content":[{"type":"image","source":{"type":"base64","media_type":MIME,"data":B64}},{"type":"text","text":PROMPT}]}]}`.
  Response: `content[0].text`. `requires_api_key() = true`.

- **OllamaAdapter**: `POST {endpoint}/api/chat`. Body:
  `{"model": M, "messages": [{"role":"user","content":PROMPT,"images":[B64]}], "options":{"temperature":T}}`.
  Response: `message.content`. `requires_api_key() = false`.

### 2.3 VlmOcrProvider (pub, implements OcrProvider)

```rust
pub struct VlmOcrProvider {
    config: VlmConfig,
    client: reqwest::Client,
    adapter: Box<dyn ApiAdapter>,
    runtime: tokio::runtime::Runtime,
}
```

Constructor:

```rust
impl VlmOcrProvider {
    pub fn new(config: VlmConfig) -> OcrResult<Self>
    // Builds tokio runtime (current_thread, single worker).
    // Validates: if adapter.requires_api_key() && config.api_key.is_none()
    //   → OcrError::AuthenticationError (fail-safe at construction, not at first call).
    // Builds reqwest::Client with timeout from config.

    pub fn with_adapter(config: VlmConfig, adapter: Box<dyn ApiAdapter>) -> OcrResult<Self>
    // For custom/third-party adapters and testing.
}
```

### 2.4 OcrProvider implementation

**`process_image`**:

1. Detect MIME from magic bytes (PNG `\x89PNG`, JPEG `\xFF\xD8\xFF`, TIFF `II`/`MM`).
   Unknown → `OcrError::UnsupportedImageFormat`.
2. Base64-encode image bytes.
3. Resolve prompt: `config.prompt_override` if set, else default OCR prompt.
4. Build request via `adapter.build_request(...)`.
5. `runtime.block_on(client.execute(request))`.
6. Map HTTP errors: 401/403 → `AuthenticationError`, 429 → `RateLimitExceeded`,
   timeout → `NetworkError`, other 4xx/5xx → `ProcessingFailed`.
7. Parse JSON body via `adapter.parse_response(...)`.
8. Build `OcrProcessingResult` with heuristic confidence (section 3).

**`process_page`**: delegates to default (calls `process_image`).

**`process_image_regions`**: delegates to default (calls `process_image` per region).

**`supported_formats`**: PNG, JPEG, TIFF, WebP.

**`engine_name`**: `format!("vlm:{}", self.config.model)`.

**`engine_type`**: `OcrEngine::Vlm`.

**`validate_image_data`**: magic-byte check only.

## 3. Confidence heuristic

VLMs don't return OCR confidence scores. Estimate from the response text:

```rust
fn estimate_confidence(text: &str, image_bytes: &[u8]) -> f64 {
    if text.trim().is_empty() { return 0.0; }

    let total = text.chars().count() as f64;
    let alpha_num = text.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).count() as f64;
    let ratio = alpha_num / total;  // high ratio = clean text

    // Penalize very short text relative to image size
    let image_kb = image_bytes.len() as f64 / 1024.0;
    let chars_per_kb = total / image_kb.max(1.0);
    let length_factor = (chars_per_kb / 10.0).min(1.0);  // expect ~10+ chars per KB of image

    let raw = ratio * 0.7 + length_factor * 0.3;
    raw.clamp(0.05, 0.95)  // never 0 (got text) or 1 (no ground truth)
}
```

## 4. Default OCR prompt

```
Extract all visible text from this image exactly as it appears.
Preserve the original layout, line breaks, and reading order.
Output only the extracted text with no commentary, labels, or formatting.
If no text is visible, respond with an empty string.
```

Stored as `const DEFAULT_VLM_OCR_PROMPT: &str` in the module.

## 5. OcrEngine extension

Add variant to `OcrEngine`:

```rust
pub enum OcrEngine {
    Mock,
    Tesseract,
    Azure,
    Aws,
    GoogleCloud,
    Vlm,   // ← new
}
```

With `name() → "VLM"`, `supports_format() → JPEG | PNG | TIFF | WebP`.

## 6. Feature gate and dependencies

```toml
# Cargo.toml (oxidize-pdf-core)
[dependencies]
reqwest = { workspace = true, features = ["json"], optional = true }
tokio = { workspace = true, features = ["rt"], optional = true }
base64 = { workspace = true, optional = true }

[features]
ocr-vlm = ["dep:reqwest", "dep:tokio", "dep:base64", "dep:serde_json"]
ocr-full = ["ocr-tesseract", "ocr-vlm"]
```

Module gate:

```rust
// text/mod.rs
#[cfg(feature = "ocr-vlm")]
pub mod vlm_provider;
```

File: `oxidize-pdf-core/src/text/vlm_provider.rs` (single file, adapters as inner structs).

## 7. Public API surface

Exports from `oxidize_pdf::text` (gated `ocr-vlm`):

- `VlmOcrProvider` — the provider
- `VlmConfig` — configuration
- `VlmConfigBuilder` — builder

NOT exported (pub(crate)):
- `ApiAdapter` trait
- `OpenAiAdapter`, `AnthropicAdapter`, `OllamaAdapter`

`OcrEngine::Vlm` is always visible (the enum is not feature-gated).

## 8. Error handling

- **No API key when required**: `OcrError::AuthenticationError` at construction (`::new()`).
- **HTTP 401/403**: `OcrError::AuthenticationError`.
- **HTTP 429**: `OcrError::RateLimitExceeded`.
- **Timeout**: `OcrError::NetworkError`.
- **Non-2xx**: `OcrError::ProcessingFailed` with status code and body excerpt.
- **Malformed JSON response**: `OcrError::ProcessingFailed`.
- **Unrecognized image format**: `OcrError::UnsupportedImageFormat`.

No retries. The caller decides retry policy (consistent with Tesseract provider behavior).

## 9. Testing strategy

Mock at the `ApiAdapter` trait level. A `MockApiAdapter` returns canned JSON
without HTTP calls. Inject via `VlmOcrProvider::with_adapter()`.

Tests (content-verifying, no smoke):

1. **OpenAI adapter**: build_request produces correct JSON shape; parse_response
   extracts `choices[0].message.content`.
2. **Anthropic adapter**: same for Anthropic JSON shape; header `x-api-key` present.
3. **Ollama adapter**: no API key required; correct JSON shape.
4. **VlmOcrProvider integration**: mock adapter → `process_image` returns exact
   expected text, confidence within expected range, engine_name = `vlm:{model}`.
5. **Fail-safe**: `VlmConfig` without api_key + OpenAI adapter → `AuthenticationError`
   at construction.
6. **Error mapping**: mock adapter returning error status codes → correct `OcrError` variants.
7. **Confidence heuristic**: unit tests with known inputs → expected confidence ranges.
8. **MIME detection**: PNG/JPEG/TIFF/WebP magic bytes → correct MIME; unknown → error.
9. **Prompt override**: custom prompt reaches the adapter's build_request.

## 10. Files changed

| File | Change |
|------|--------|
| `oxidize-pdf-core/src/text/vlm_provider.rs` | NEW — VlmOcrProvider, VlmConfig, adapters |
| `oxidize-pdf-core/src/text/ocr/mod.rs` | Add `OcrEngine::Vlm` variant + match arms |
| `oxidize-pdf-core/src/text/mod.rs` | `#[cfg(feature = "ocr-vlm")] pub mod vlm_provider;` + re-exports |
| `oxidize-pdf-core/src/lib.rs` | Re-export VlmOcrProvider, VlmConfig, VlmConfigBuilder |
| `oxidize-pdf-core/Cargo.toml` | Add optional deps + `ocr-vlm` feature + update `ocr-full` |
| `oxidize-pdf-core/tests/vlm_ocr_test.rs` | NEW — integration tests (9 test cases) |
| `.github/workflows/ci.yml` | Add `ocr-vlm` to feature matrix |

## 11. Not in scope

- Streaming responses (overkill for OCR text extraction).
- Retry/backoff policy (caller's responsibility).
- Image preprocessing before sending to VLM (the VLM handles raw images).
- `OcrTextFragment` position data from VLM (VLMs return plain text, not bounding boxes;
  `fragments` will be a single fragment covering the full text).
- Azure/AWS/GoogleCloud adapters (existing enum variants; separate issue if needed).
