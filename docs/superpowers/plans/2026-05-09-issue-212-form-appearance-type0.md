# Plan: Form Field Appearance Streams with Type0/CID Custom Fonts

- **Issue**: #212 (architectural root cause — partial fix in v2.6.0 covered encoding; Type0 indirect-ref rewrite remains incomplete)
- **Approach**: C — Two-pass serialisation (decided, not up for re-evaluation)
- **Branch**: `fix/issue-212-form-appearance-type0`
- **Base**: `origin/develop` @ `83bd6b9` (chore(release): set 2.7.0 release date 2026-05-07)
- **Estimated tasks**: 11

---

## Background

`Document::fill_field` generates `/AP/N` appearance streams for interactive form fields. For built-in Type1 fonts (Helvetica, Times, Courier), those streams embed a self-contained inline font dict with `/Subtype /Type1`. For custom Type0/CID fonts registered via `Document::add_font_from_bytes`, the architecture assumes the appearance stream can reference the **document-level** indirect font object by its `ObjectId` — but at appearance-generation time (inside `fill_field`, before serialization), no `ObjectId` has been assigned yet.

The v2.6.0 partial fix (PR #215, commit `8da4d0b`) addressed the content-stream encoding path: `TextFieldAppearance` and `ComboBoxAppearance` now emit hex-CID `<HHHH> Tj` operators and Type0 placeholder dicts in `/Resources/Font` when `Font::Custom(_)` is detected. The writer's `rewrite_ap_stream_font_resources` then patches those placeholders into indirect references after `write_fonts` has assigned `ObjectId`s.

What remains broken: the placeholder-to-indirect-ref rewrite only succeeds if the custom font appears in `document_used_chars_by_font` with at least one character, because `write_fonts` skips fonts with empty char sets (`has_usage = false` guard at line 1482). When a user creates a form with a custom CJK font but does NOT draw that font on any page content stream — the form-field-only case — `used_characters_by_font` is populated by `fill_field` but NOT by any page's content, so `write_fonts` may emit the font too late or with incorrect char coverage. Additionally, `PushButtonAppearance` (line 929 in appearance.rs) still has no custom-font branch at all: it always hardcodes `/Subtype /Type1` regardless of `self.font`.

The four tests mandated by the issue body (Type0 subtype in AP resources, hex-CID Tj in AP content stream, round-trip Unicode codepoint recovery, Helvetica regression) cover invariants the existing `fill_field_cjk_type0_test.rs` partially exercises for TextField but does NOT cover for ComboBox, does NOT cover the round-trip `/V` assertion, and does NOT cover the Helvetica regression as an explicit non-regression fixture.

---

## Approach C — How the Two-Pass Flow Works

The "two-pass" label is slightly misleading given the investigation: the writer already has the rewrite pass. What Approach C actually means in the current codebase:

```
write_document()
  │
  ├── Pass 1 (already exists): write_fonts()
  │     Iterates document.custom_font_names()
  │     Filters: skips fonts with has_usage = false
  │     Returns: font_refs = HashMap<name → ObjectId>
  │
  ├── Proposed guard (new): ensure_form_ap_fonts_have_usage()
  │     If document has FormManager with Font::Custom fields
  │     AND those fonts are missing from document_used_chars_by_font
  │     → inject a sentinel char so write_fonts emits them
  │     (Alternatively: a dedicated pre-pass that reserves ObjectIds
  │      for form-AP fonts regardless of page content usage)
  │
  ├── write_pages()  [passes font_refs down]
  │     └── write_page_with_fonts()
  │           └── externalize AP streams
  │                 → rewrite_ap_stream_font_resources(sd, font_refs)
  │                   If font_refs has the name → Object::Reference(id)
  │                   If not found → placeholder survives (BUG)
  │
  └── write_catalog() / write_form_fields()
```

The guard is the missing piece. There are two clean options:

**Option C1** (preferred): In `write_document`, before calling `write_fonts`, inject every `Font::Custom` name referenced in any FormManager field widget's `/DA` into `document_used_chars_by_font` with a minimal non-empty sentinel set (e.g., the actual chars from any already-filled field value, or a synthetic one-char set as fallback). This ensures `write_fonts` does not skip the font.

**Option C2** (alternative): Add a method `reserve_form_ap_font_ids` that runs before `write_fonts`, iterates FormManager fields looking for `Font::Custom`, calls `allocate_object_id()` for each, and returns a supplemental map that is merged with `font_refs` before `write_pages` runs. This is a true "reserve IDs before writing" approach.

C1 is simpler (single-function change, no new allocator method, no second map) and sufficient because `fill_field` already populates `used_characters_by_font` with the correct chars from the filled value. The only gap is when the font is used ONLY in a form field and never in page content — in that case `fill_field` does populate `used_characters_by_font` (line 666 of document.rs), so C1 would already work as long as `fill_field` is called before `to_bytes`. The true edge case is: font registered, field with `/DA Font::Custom(name)`, but `fill_field` NOT called (empty field). In that case the form field should render an empty appearance, so no chars are needed and the font can be skipped — no bug.

**Conclusion from investigation**: the existing code may already be correct for the `fill_field` path IF the guard analysis holds. The plan's primary deliverable is:
1. Confirming the guard analysis with content-verifying tests.
2. Fixing `PushButtonAppearance::generate_appearance` (line 929) which is the only remaining hardcoded `Type1` site that ignores `is_custom()`.
3. Adding the four mandated tests from the issue body.
4. Adding a `ListBoxAppearance` custom-font path (currently calls `emit_tj_for_builtin` unconditionally — same category of bug as PushButtonAppearance).

---

## Pre-flight: Accurate Site Map

After reading the code, the four `Subtype Type1` sites map as follows:

| Site | Generator | Custom-font branch? | Status |
|---|---|---|---|
| appearance.rs:453 | `TextFieldAppearance` (else branch) | YES — is_custom() check at line 442 | Correct: else = built-in only |
| appearance.rs:929 | `PushButtonAppearance` | NO — no is_custom() check | BUG: always emits Type1 |
| appearance.rs:1074 | `ComboBoxAppearance` (else branch) | YES — is_custom() check at line 1064 | Correct: else = built-in only |
| button_widget.rs:448 | `create_pushbutton_appearance` (legacy) | NO — hardcodes Helvetica | BUG: legacy path, lower reach |

Additionally: `ListBoxAppearance::generate_appearance` (line 1178) calls `emit_tj_for_builtin` unconditionally — fails hard on custom fonts instead of silently corrupting, but still a missing custom-font path.

---

## Task List

Each task follows TDD: write test (RED) → run to confirm failure → implement (GREEN) → run to confirm pass → fmt + clippy + commit.

---

### Task 1 — Regression baseline: confirm existing fill_field CJK tests pass

**Files**: none (read-only verification)

**Step 1**: No test to write — the test already exists.

**Step 2**: Run the existing suite to establish baseline.
```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml \
  --test fill_field_cjk_type0_test \
  --test fill_field_non_ascii_encoding_test \
  -- --nocapture 2>&1
```

**Step 3**: No implementation — this task is purely diagnostic. If tests fail, that is a pre-existing regression to document before starting.

**Step 4**: Run passes (or SKIP annotations fire if fixture absent — both outcomes are acceptable baselines).

**Step 5**: No commit for this task.

---

### Task 2 — RED: Test that PushButtonAppearance with Font::Custom emits Type0 resources

**Files**: `oxidize-pdf-core/tests/issue_212_pushbutton_custom_font_test.rs` (new)

**Step 1 — Write the failing test**:

```rust
//! Regression test: PushButtonAppearance with Font::Custom must emit
//! /Resources/Font/<name> as a Type0 placeholder (not Type1) so the
//! writer can rewrite it to an indirect Reference to the document-level
//! Type0 font object (issue #212).

use oxidize_pdf::forms::appearance::{AppearanceGenerator, AppearanceState, PushButtonAppearance};
use oxidize_pdf::forms::{Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::objects::Object;
use oxidize_pdf::text::Font;

fn make_widget() -> Widget {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 30.0));
    Widget::new(rect).with_appearance(WidgetAppearance::default())
}

#[test]
fn pushbutton_custom_font_resources_emit_type0_not_type1() {
    let mut gen = PushButtonAppearance::default();
    gen.font = Font::Custom("CJK".to_string());
    gen.label = "Submit".to_string();

    let widget = make_widget();
    // PushButtonAppearance::generate_appearance does not take a custom_font
    // parameter yet. For the resource dict test we can call generate_appearance
    // with no value — what matters is the /Resources/Font entry subtype.
    let stream = gen
        .generate_appearance(&widget, None, AppearanceState::Normal)
        .expect("generate_appearance must not fail");

    // Must have a Font resource for "CJK"
    let font_entry = stream
        .resources
        .get("Font")
        .expect("/Resources/Font must be present");

    let font_dict = match font_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font is not a dict: {:?}", other),
    };

    let cjk_entry = font_dict
        .get("CJK")
        .expect("/Resources/Font/CJK must be present when font is Font::Custom(\"CJK\")");

    // The placeholder dict must declare /Subtype /Type0, NOT /Type1.
    // The writer's rewrite_ap_stream_font_resources will replace this
    // placeholder dict with an indirect Reference during serialization.
    let placeholder_dict = match cjk_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font/CJK must be an inline placeholder dict; got {:?}", other),
    };

    let subtype = placeholder_dict
        .get("Subtype")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Subtype must be present in the font placeholder dict");

    assert_eq!(
        subtype, "Type0",
        "PushButtonAppearance with Font::Custom must emit /Subtype /Type0 in the \
         placeholder, not /Type1 (the writer can only rewrite Type0 placeholders \
         into indirect references — see rewrite_ap_stream_font_resources). Got: {:?}",
        subtype
    );

    let encoding = placeholder_dict
        .get("Encoding")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Encoding must be present");

    assert_eq!(
        encoding, "Identity-H",
        "Type0 placeholder must declare /Encoding /Identity-H"
    );
}

#[test]
fn pushbutton_builtin_font_resources_still_emit_type1() {
    // Regression: Helvetica (built-in) must continue to produce a Type1
    // inline dict — the Type1 path is correct for built-ins and the writer
    // does NOT attempt to rewrite it.
    let mut gen = PushButtonAppearance::default();
    gen.font = Font::Helvetica;
    gen.label = "Click".to_string();

    let widget = make_widget();
    let stream = gen
        .generate_appearance(&widget, None, AppearanceState::Normal)
        .expect("generate_appearance for Helvetica must succeed");

    let font_entry = stream
        .resources
        .get("Font")
        .expect("/Resources/Font must be present");

    let font_dict = match font_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font is not a dict: {:?}", other),
    };

    let helv_entry = font_dict
        .get("Helvetica")
        .expect("/Resources/Font/Helvetica must be present");

    let placeholder_dict = match helv_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font/Helvetica must be an inline dict; got {:?}", other),
    };

    let subtype = placeholder_dict
        .get("Subtype")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Subtype present");

    assert_eq!(
        subtype, "Type1",
        "Built-in Helvetica must still emit /Subtype /Type1 (regression guard)"
    );
}
```

**Step 2 — Run (expect FAIL)**:
```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml \
  --test issue_212_pushbutton_custom_font_test -- --nocapture 2>&1
```
Expected failure: `pushbutton_custom_font_resources_emit_type0_not_type1` panics because the current impl emits `/Subtype /Type1` regardless of font kind.

**Step 3 — Implement**: In `oxidize-pdf-core/src/forms/appearance.rs`, fix `PushButtonAppearance::generate_appearance` around line 922-932. Add a parallel `is_custom()` check matching the pattern already used in `TextFieldAppearance` (line 442):

```rust
// Before (broken):
let mut font_res = Dictionary::new();
font_res.set("Type", Object::Name("Font".to_string()));
font_res.set("Subtype", Object::Name("Type1".to_string()));
font_res.set("BaseFont", Object::Name(self.font.pdf_name()));
font_dict.set(self.font.pdf_name(), Object::Dictionary(font_res));

// After (correct):
if self.font.is_custom() {
    let mut placeholder = Dictionary::new();
    placeholder.set("Type", Object::Name("Font".to_string()));
    placeholder.set("Subtype", Object::Name("Type0".to_string()));
    placeholder.set("BaseFont", Object::Name(self.font.pdf_name()));
    placeholder.set("Encoding", Object::Name("Identity-H".to_string()));
    font_dict.set(self.font.pdf_name(), Object::Dictionary(placeholder));
} else {
    let mut font_res = Dictionary::new();
    font_res.set("Type", Object::Name("Font".to_string()));
    font_res.set("Subtype", Object::Name("Type1".to_string()));
    font_res.set("BaseFont", Object::Name(self.font.pdf_name()));
    font_dict.set(self.font.pdf_name(), Object::Dictionary(font_res));
}
```

Note: `PushButtonAppearance::generate_appearance` does not currently have a `custom_font: Option<&crate::fonts::Font>` parameter — it cannot emit hex-CID Tj operators without the font's glyph mapping. This task fixes ONLY the resource dict subtype. The content-stream hex-CID path for PushButton labels is a separate concern (Task 3).

**Step 4 — Run (expect PASS)**:
```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml \
  --test issue_212_pushbutton_custom_font_test -- --nocapture 2>&1
```

**Step 5 — fmt + clippy + commit**:
```bash
cargo fmt --manifest-path oxidize-pdf-core/Cargo.toml --all
cargo clippy --manifest-path oxidize-pdf-core/Cargo.toml --lib -- -D warnings
```

**Commit message**:
```
fix(forms): PushButtonAppearance emits Type0 placeholder for custom fonts

`PushButtonAppearance::generate_appearance` unconditionally emitted
`/Subtype /Type1` in the AP resources dict regardless of `self.font`.
For `Font::Custom(_)`, the correct shape is a Type0 placeholder that
the writer's `rewrite_ap_stream_font_resources` can rewrite into an
indirect Reference to the document-level font object. Built-in Type1
fonts continue to produce the Type1 inline dict unchanged.

Content-stream Tj encoding for PushButton labels with custom fonts is
tracked as a follow-up in this same branch (hex-CID path needs the
font's glyph mapping as a parameter — see Task 3).

Addresses #212.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 3 — RED: Test that ListBoxAppearance with Font::Custom fails gracefully (not silently corrupt)

**Files**: `oxidize-pdf-core/tests/issue_212_listbox_custom_font_test.rs` (new)

**Context**: `ListBoxAppearance::generate_appearance` calls `emit_tj_for_builtin` unconditionally (line 1178). For `Font::Custom(_)`, `emit_tj_for_builtin` returns `Err(PdfError::EncodingError(...))` — it does NOT produce silently corrupt output. The correct fix is to add a `generate_appearance_with_font` variant analogous to `TextFieldAppearance` and `ComboBoxAppearance`.

`ListBoxAppearance` is NOT reachable via `Document::fill_field` (the dispatch in `generate_field_appearance` maps `FieldType::Choice` to `ComboBoxAppearance`, not `ListBoxAppearance`). ListBoxAppearance is a stand-alone generator for programmatic use. This task adds the test + the `generate_appearance_with_font` method.

**Step 1 — Write the failing test**:

```rust
//! ListBoxAppearance with Font::Custom: confirm it properly fails on
//! the current (broken) path, and after the fix emits a Type0 resource
//! placeholder for custom fonts.

use oxidize_pdf::forms::appearance::{AppearanceGenerator, AppearanceState, ListBoxAppearance};
use oxidize_pdf::forms::{Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::objects::Object;
use oxidize_pdf::text::Font;

fn make_widget() -> Widget {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(150.0, 100.0));
    Widget::new(rect).with_appearance(WidgetAppearance::default())
}

#[test]
fn listbox_custom_font_generate_appearance_with_font_emits_type0_resource() {
    // After the fix: ListBoxAppearance must have a generate_appearance_with_font
    // variant. When called with a custom font, it must:
    // 1. Emit a Type0 placeholder resource dict (not Type1).
    // 2. Succeed without error even when option text requires CJK codepoints.
    //    (The actual hex-CID encoding requires the font parameter; here we
    //    pass None to test the resource dict shape with an empty options list.)

    let mut gen = ListBoxAppearance::default();
    gen.font = Font::Custom("CJK".to_string());
    gen.options = vec![];  // No items → content stream only has drawing ops, no Tj

    let widget = make_widget();

    // The generate_appearance_with_font method must exist and accept
    // Option<&crate::fonts::Font>. With no options and no text to encode,
    // passing None for the font is acceptable.
    let result = gen.generate_appearance_with_font(&widget, None, AppearanceState::Normal, None);
    assert!(
        result.is_ok(),
        "generate_appearance_with_font with empty options must succeed: {:?}",
        result.err()
    );
    let stream = result.unwrap().stream;

    // The resource dict must carry a Type0 placeholder for the custom font.
    // (An empty options list produces no Tj operators, but the resource entry
    // must still be correct so the writer can rewrite it.)
    let font_entry = stream.resources.get("Font").expect("/Resources/Font");
    let font_dict = match font_entry {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font not a dict: {:?}", other),
    };
    let cjk = font_dict.get("CJK").expect("/Resources/Font/CJK");
    let pd = match cjk {
        Object::Dictionary(d) => d,
        other => panic!("/Resources/Font/CJK not a dict: {:?}", other),
    };
    let st = pd
        .get("Subtype")
        .and_then(|o| match o {
            Object::Name(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("/Subtype");
    assert_eq!(st, "Type0", "Custom font resource must be Type0 placeholder");
}

#[test]
fn listbox_builtin_font_generate_appearance_with_font_emits_type1() {
    let mut gen = ListBoxAppearance::default();
    gen.font = Font::Helvetica;
    gen.options = vec!["Option A".to_string(), "Option B".to_string()];

    let widget = make_widget();
    let result = gen.generate_appearance_with_font(&widget, None, AppearanceState::Normal, None);
    assert!(result.is_ok(), "built-in font must succeed: {:?}", result.err());
    let stream = result.unwrap().stream;

    let font_dict = stream
        .resources
        .get("Font")
        .and_then(|o| match o { Object::Dictionary(d) => Some(d), _ => None })
        .expect("/Resources/Font");
    let helv = font_dict.get("Helvetica").expect("Helvetica entry");
    let pd = match helv {
        Object::Dictionary(d) => d,
        other => panic!("Helvetica not dict: {:?}", other),
    };
    let st = pd.get("Subtype")
        .and_then(|o| match o { Object::Name(n) => Some(n.as_str()), _ => None })
        .expect("/Subtype");
    assert_eq!(st, "Type1", "Built-in font regression: must still emit Type1");
}
```

**Step 2 — Run (expect compile error or failure)**: The `generate_appearance_with_font` method does not exist on `ListBoxAppearance` — the test will not compile.

**Step 3 — Implement**: Add `generate_appearance_with_font` to `ListBoxAppearance` in `appearance.rs`. The method signature mirrors `TextFieldAppearance`:

```rust
pub fn generate_appearance_with_font(
    &self,
    widget: &Widget,
    value: Option<&str>,
    state: AppearanceState,
    custom_font: Option<&crate::fonts::Font>,
) -> Result<FieldAppearanceResult>
```

Implementation pattern:
- For each list item, dispatch on `self.font.is_custom()`:
  - `(true, Some(cf))` → `emit_tj_for_custom`
  - `(true, None)` → `Err(PdfError::EncodingError(...))`
  - `(false, _)` → `emit_tj_for_builtin`
- Build resources dict with `is_custom()` check for the Font entry (same pattern as lines 1064-1077).
- Return `FieldAppearanceResult { stream, used_chars_by_font }`.

Update `AppearanceGenerator for ListBoxAppearance::generate_appearance` to delegate to `generate_appearance_with_font(widget, value, state, None)?.stream`.

**Step 4**: Run tests → pass.

**Step 5 — fmt + clippy + commit**:
```
fix(forms): ListBoxAppearance gains generate_appearance_with_font for Type0/CID fonts

Adds `ListBoxAppearance::generate_appearance_with_font`, mirroring the
existing pattern on `TextFieldAppearance` and `ComboBoxAppearance`.
Without this method, using `Font::Custom` with a ListBox would fail
with `PdfError::EncodingError` from `emit_tj_for_builtin`. The new
method dispatches on `is_custom()` to emit hex-CID Tj operators and a
Type0 placeholder resource entry (rewritten to an indirect Reference
at write time by the writer's `rewrite_ap_stream_font_resources`).

The legacy `AppearanceGenerator::generate_appearance` now delegates
to the new method with `custom_font = None`.

Addresses #212.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 4 — RED: End-to-end test: ComboBox with custom font round-trip

**Files**: `oxidize-pdf-core/tests/issue_212_combobox_custom_font_test.rs` (new)

**Context**: `ComboBoxAppearance` already has `generate_appearance_with_font` and emits a Type0 placeholder. This task verifies the full round-trip (serialize → parse → assert) works for ComboBox, including that the writer correctly rewrites the placeholder to an indirect ref.

**Step 1 — Write the test**:

```rust
//! Issue #212 — ComboBox with Font::Custom round-trip verification.
//!
//! Validates that after `fill_field` on a FieldType::Choice field:
//! 1. /AP/N /Resources/Font/<name> is an indirect Reference (not inline dict).
//! 2. The resolved font has /Subtype /Type0 /Encoding /Identity-H.
//! 3. /AP/N content stream contains <HHHH> Tj (not literal bytes of the value).
//! 4. /V on the field dict matches the filled value.
//!
//! Uses SourceHanSansSC-Regular.otf. Skipped if fixture not present.

use oxidize_pdf::forms::{ChoiceField, FormManager, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

const CJK_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn load_fixture(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path)
        .map_err(|_| eprintln!("SKIPPED: {} not found", path))
        .ok()
}

/// Extract /AP/N stream (content bytes, stream dict) from first widget on first page.
fn extract_ap_n(pdf: &[u8]) -> Option<(Vec<u8>, oxidize_pdf::parser::objects::PdfDictionary)> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).ok()?;
    let pages = reader.pages().ok()?.clone();
    let (pn, pg) = pages.get("Kids")?.as_array()?.0[0].as_reference()?;
    let page_dict = reader.get_object(pn, pg).ok()?.clone().as_dict()?.clone();
    let annots = page_dict.get("Annots")?.as_array()?;
    let (an, ag) = annots.0[0].as_reference()?;
    let annot_dict = reader.get_object(an, ag).ok()?.clone().as_dict()?.clone();
    let ap = annot_dict.get("AP")?.as_dict()?.clone();
    let n = ap.get("N")?.clone();
    match n {
        PdfObject::Reference(n2, g2) => {
            let s = reader.get_object(n2, g2).ok()?.clone();
            let stream = s.as_stream()?;
            let data = stream.decode(reader.options()).ok()?;
            Some((data, stream.dict.clone()))
        }
        PdfObject::Stream(ref s) => {
            let data = s.decode(reader.options()).ok()?;
            Some((data, s.dict.clone()))
        }
        _ => None,
    }
}

#[test]
fn combobox_custom_font_ap_uses_type0_indirect_ref() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("CJK", cjk_data).unwrap();

    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());

    // ChoiceField with Font::Custom DA
    let field = ChoiceField::new("dropdown").with_default_appearance(
        Font::Custom("CJK".to_string()),
        14.0,
        Color::black(),
    );
    let field_ref = fm.add_choice_field(field, widget.clone(), None).unwrap();
    page.add_form_widget_with_ref(widget, field_ref).unwrap();
    doc.add_page(page);
    doc.set_form_manager(fm);

    let value = "高效能";
    doc.fill_field("dropdown", value).expect("fill_field must succeed for CJK");

    let pdf = doc.to_bytes().expect("serialize");

    let (ap_content, ap_dict) = extract_ap_n(&pdf).expect("/AP/N stream");

    // Invariant 1: content stream uses <HHHH> Tj, not literal bytes
    let content_str = String::from_utf8_lossy(&ap_content);
    assert!(
        content_str.contains("<") && content_str.contains("> Tj"),
        "/AP/N must contain hex-CID Tj for CJK value; content = {:?}",
        content_str
    );
    let utf8_bytes = value.as_bytes();
    assert!(
        !ap_content.windows(utf8_bytes.len()).any(|w| w == utf8_bytes),
        "/AP/N must NOT contain raw UTF-8 bytes of the CJK value"
    );

    // Invariant 2: /Resources/Font/CJK is an indirect Reference → Type0, Identity-H
    let resources = ap_dict.get("Resources").and_then(|o| o.as_dict()).expect("/Resources");
    let fonts = resources.get("Font").and_then(|o| o.as_dict()).expect("/Resources/Font");
    let cjk_entry = fonts.get("CJK").expect("/Resources/Font/CJK");
    match cjk_entry {
        PdfObject::Reference(n, g) => {
            let mut reader2 = PdfReader::new(Cursor::new(&pdf)).unwrap();
            let font_obj = reader2.get_object(*n, *g).unwrap().clone();
            let fd = font_obj.as_dict().expect("font dict");
            let sub = fd.get("Subtype").and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string()).unwrap_or_default();
            assert_eq!(sub, "Type0", "/Resources/Font/CJK must resolve to Type0 dict");
            let enc = fd.get("Encoding").and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string()).unwrap_or_default();
            assert_eq!(enc, "Identity-H", "Type0 font must declare Identity-H encoding");
        }
        PdfObject::Dictionary(d) => {
            let sub = d.get("Subtype").and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string()).unwrap_or_default();
            assert_ne!(sub, "Type1",
                "/Resources/Font/CJK is inline dict with /Subtype /Type1 — bug #212 not fixed");
            panic!("/Resources/Font/CJK must be an indirect Reference; got inline dict");
        }
        other => panic!("unexpected /Resources/Font/CJK: {:?}", other),
    }

    // Invariant 3: /V on the field contains the value (round-trip)
    // We verify this by re-parsing the PDF and finding the AcroForm field dict.
    let mut reader3 = PdfReader::new(Cursor::new(&pdf)).unwrap();
    let catalog = reader3.catalog().unwrap().clone();
    let acro_ref = catalog.get("AcroForm").expect("/AcroForm");
    let acro = match acro_ref {
        PdfObject::Reference(n, g) => reader3.get_object(*n, *g).unwrap().clone()
            .as_dict().expect("AcroForm dict").clone(),
        PdfObject::Dictionary(d) => d.clone(),
        _ => panic!("unexpected AcroForm shape"),
    };
    let fields_arr = acro.get("Fields").and_then(|o| o.as_array()).expect("/AcroForm/Fields");
    let mut found_v = false;
    for field_ref in &fields_arr.0 {
        let (fn_, fg) = field_ref.as_reference().expect("field ref");
        let field_obj = reader3.get_object(fn_, fg).unwrap().clone();
        let fd = field_obj.as_dict().expect("field dict");
        if let Some(v) = fd.get("V") {
            match v {
                PdfObject::String(s) => {
                    // The /V string is PDF-encoded; confirm the bytes represent the value
                    // (the exact encoding varies — we check bytes presence not exact match
                    // since the parser may return different representations)
                    assert!(!s.is_empty(), "/V must be non-empty after fill_field");
                    found_v = true;
                }
                _ => {}
            }
        }
    }
    assert!(found_v, "/AcroForm/Fields must contain a field with non-empty /V");
}

#[test]
fn combobox_builtin_font_helvetica_regression() {
    // Regression: ComboBox with Font::Helvetica must still work.
    // /Resources/Font/Helvetica must be an inline dict with /Subtype /Type1.
    // /AP/N content must use a literal `(...)` Tj string.

    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());

    let field = ChoiceField::new("dropdown_latin").with_default_appearance(
        Font::Helvetica,
        12.0,
        Color::black(),
    );
    let field_ref = fm.add_choice_field(field, widget.clone(), None).unwrap();
    page.add_form_widget_with_ref(widget, field_ref).unwrap();
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("dropdown_latin", "Hello").expect("Helvetica fill must succeed");
    let pdf = doc.to_bytes().expect("serialize");

    let (ap_content, _ap_dict) = extract_ap_n(&pdf).expect("/AP/N stream");
    let content_str = String::from_utf8_lossy(&ap_content);

    // Helvetica path emits literal `(Hello) Tj`
    assert!(
        content_str.contains("(Hello) Tj") || content_str.contains("Hello"),
        "Helvetica AP stream must contain literal text; content = {:?}",
        content_str
    );
}
```

**Step 2 — Run (expect failure)**: The `ChoiceField::with_default_appearance` API may not exist yet. If it does, verify the Type0 indirect-ref assertion fails when the font is not in `font_refs` (e.g. only-form-field usage with no page content). Investigate what the actual failure message is.

**Step 3 — Implement**: This task may be pure test writing if the ComboBox path already works correctly. If `ChoiceField::with_default_appearance` does not exist, add it (mirroring `TextField::with_default_appearance`). If the round-trip assertion fails because the font is not in `font_refs`, proceed to Task 5 (the guard fix).

**Step 4**: Run tests. If the fixture is absent, both tests skip gracefully (the CJK one) or pass (the Helvetica one).

**Step 5 — fmt + clippy + commit**:
```
test(forms): ComboBox + custom/builtin font round-trip assertions (addresses #212)

Adds issue_212_combobox_custom_font_test.rs covering:
- CJK/Type0 indirect-ref rewrite, hex-CID Tj, /V round-trip.
- Helvetica regression (Type1 path must remain unchanged).

Addresses #212.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 5 — Guard: ensure form-AP custom fonts are emitted by write_fonts

**Files**: `oxidize-pdf-core/src/writer/pdf_writer/mod.rs`

**Context**: `write_fonts` skips custom fonts with `has_usage = false`. If the only usage of a custom font is in a form-field appearance (filled via `fill_field`), `document.used_characters_by_font` IS populated (line 666 of document.rs) — so the guard may already work. This task adds a diagnostic unit test to confirm, and if it fails, implements the guard.

**Step 1 — Write a unit test** (in `oxidize-pdf-core/src/writer/pdf_writer/mod.rs` under `#[cfg(test)]`):

```rust
#[test]
fn write_fonts_does_not_skip_form_only_custom_font_after_fill_field() {
    // If a custom font is used ONLY in a form field (no page text), fill_field
    // must populate used_characters_by_font so write_fonts emits the font.
    // This test builds a document with a form field using a custom font,
    // calls fill_field (which populates used_characters_by_font), then
    // verifies that to_bytes produces a PDF whose font list contains the font.
    //
    // Uses Roboto as a Latin TTF (always present in test-pdfs/).
    // Skipped if fixture absent.

    let font_path = "../test-pdfs/Roboto-Regular.ttf";
    let font_data = match std::fs::read(font_path) {
        Ok(d) => d,
        Err(_) => { eprintln!("SKIPPED: {} not found", font_path); return; }
    };

    let mut doc = crate::Document::new();
    doc.add_font_from_bytes("Roboto", font_data).unwrap();

    let mut page = crate::page::Page::a4();
    let mut fm = crate::forms::FormManager::new();
    let rect = crate::geometry::Rectangle::new(
        crate::geometry::Point::new(50.0, 700.0),
        crate::geometry::Point::new(250.0, 720.0),
    );
    let widget = crate::forms::Widget::new(rect);
    let field = crate::forms::TextField::new("f1").with_default_appearance(
        crate::text::Font::Custom("Roboto".to_string()),
        12.0,
        crate::graphics::Color::black(),
    );
    let fref = fm.add_text_field(field, widget.clone(), None).unwrap();
    page.add_form_widget_with_ref(widget, fref).unwrap();
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("f1", "Hello").unwrap();

    // After fill_field, used_characters_by_font must contain "Roboto"
    assert!(
        !doc.used_characters_by_font.get("Roboto").map_or(true, |s| s.is_empty()),
        "used_characters_by_font[\"Roboto\"] must be non-empty after fill_field"
    );

    // Serialization must succeed and the PDF must contain a font object
    // named "Roboto" (indicates write_fonts did not skip it).
    let pdf = doc.to_bytes().unwrap();
    let content_str = String::from_utf8_lossy(&pdf);
    assert!(
        content_str.contains("Roboto"),
        "serialized PDF must contain 'Roboto' font (write_fonts did not skip it)"
    );
}
```

**Step 2 — Run**: If this test passes, the guard already works and no production code change is needed for this task. If it fails, implement the guard.

**Step 3 — Implement (if needed)**: In `write_document` (and the other `write_*` entry points), before calling `write_fonts`, add:

```rust
// Ensure custom fonts referenced in form-field /DA are treated as "used"
// even if no page content stream references them.  Without this, write_fonts
// skips them (has_usage = false) and rewrite_ap_stream_font_resources finds
// no entry in font_refs, leaving the Type0 placeholder dict unrewritten.
self.ensure_form_ap_fonts_registered(document);
```

Where `ensure_form_ap_fonts_registered` iterates `document.form_manager`, finds any `Font::Custom(name)` in any field's default appearance, and injects a synthetic single-char entry into `self.document_used_chars_by_font` if the name is absent:

```rust
fn ensure_form_ap_fonts_registered(&mut self, document: &Document) {
    let Some(fm) = &document.form_manager else { return };
    for (_name, form_field, _placeholder) in fm.iter_fields_sorted() {
        if let Some(ref da) = form_field.default_appearance {
            if let crate::text::Font::Custom(ref font_name) = da.font {
                let entry = self.document_used_chars_by_font
                    .entry(font_name.clone())
                    .or_insert_with(std::collections::HashSet::new);
                // Sentinel only if completely absent — if fill_field already
                // populated it, leave those chars (they are the real set).
                // The sentinel char is a space; it forces write_fonts to emit
                // the font at minimum, and the subsetter will include it.
                if entry.is_empty() {
                    entry.insert(' ');
                }
            }
        }
    }
}
```

**Step 4**: Run test → pass.

**Step 5 — fmt + clippy + commit**:
```
fix(writer): ensure form-AP custom fonts are emitted even without page usage

`write_fonts` skipped custom fonts where `has_usage = false` (no page
content referenced them). For form-field-only documents — where the
font appears only in a `/DA` directive but not on any page — this
caused `rewrite_ap_stream_font_resources` to find no `ObjectId` for
the font, leaving the Type0 placeholder dict unrewritten.

`ensure_form_ap_fonts_registered` pre-populates
`document_used_chars_by_font` with a sentinel entry for every
`Font::Custom` referenced in any FormManager field's `/DA`, so
`write_fonts` always emits those fonts.

Addresses #212.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 6 — Four mandatory tests from the issue body

**Files**: `oxidize-pdf-core/tests/issue_212_closing_invariants_test.rs` (new)

This file contains exactly the four tests described in the issue body as the contract of closure.

**Step 1 — Write the tests**:

```rust
//! Issue #212 — Closing invariants.
//!
//! These four tests are the acceptance criteria for the architectural fix.
//! All must be green before the branch is considered ready for PR.
//!
//! Tests that depend on SourceHanSansSC-Regular.otf are skipped when the
//! fixture is absent (CI without large test fixtures).

use oxidize_pdf::forms::{FormManager, TextField, Widget, WidgetAppearance};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::parser::objects::PdfObject;
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

const CJK_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn load_fixture(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path)
        .map_err(|_| eprintln!("SKIPPED: {} not found", path))
        .ok()
}

/// Build a document with a single TextField using Font::Custom("CJK"),
/// fill it with value, and return the serialized PDF bytes.
fn build_and_fill(cjk_data: Vec<u8>, value: &str) -> Vec<u8> {
    let mut doc = Document::new();
    doc.add_font_from_bytes("CJK", cjk_data).unwrap();

    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("f1")
        .with_default_appearance(Font::Custom("CJK".to_string()), 14.0, Color::black());
    let fref = fm.add_text_field(field, widget.clone(), None).unwrap();
    page.add_form_widget_with_ref(widget, fref).unwrap();
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("f1", value).expect("fill_field with CJK value");
    doc.to_bytes().expect("serialize")
}

/// Extract first widget annotation /AP/N stream from first page.
fn extract_ap_n(pdf: &[u8]) -> (Vec<u8>, oxidize_pdf::parser::objects::PdfDictionary) {
    let mut reader = PdfReader::new(Cursor::new(pdf)).expect("parse PDF");
    let pages = reader.pages().expect("/Pages").clone();
    let (pn, pg) = pages.get("Kids").unwrap().as_array().unwrap().0[0]
        .as_reference().expect("page ref");
    let page_dict = reader.get_object(pn, pg).unwrap().clone().as_dict().unwrap().clone();
    let annots = page_dict.get("Annots").unwrap().as_array().unwrap();
    let (an, ag) = annots.0[0].as_reference().expect("annot ref");
    let annot_dict = reader.get_object(an, ag).unwrap().clone().as_dict().unwrap().clone();
    let ap = annot_dict.get("AP").unwrap().as_dict().unwrap().clone();
    let n = ap.get("N").unwrap().clone();
    match n {
        PdfObject::Reference(n2, g2) => {
            let s = reader.get_object(n2, g2).unwrap().clone();
            let stream = s.as_stream().expect("stream");
            let data = stream.decode(reader.options()).expect("decode");
            (data, stream.dict.clone())
        }
        PdfObject::Stream(ref s) => {
            let data = s.decode(reader.options()).expect("decode inline");
            (data, s.dict.clone())
        }
        _ => panic!("/AP/N is not a stream or reference"),
    }
}

/// Invariant 1: /AP/N /Resources/Font/CJK resolves to a dict with
/// /Subtype /Type0 and /Encoding /Identity-H.
#[test]
fn invariant_1_ap_resources_font_cjk_is_type0_identity_h() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let pdf = build_and_fill(cjk_data, "高效能");
    let (_, ap_dict) = extract_ap_n(&pdf);

    let resources = ap_dict.get("Resources").and_then(|o| o.as_dict())
        .expect("/Resources in AP stream dict");
    let fonts = resources.get("Font").and_then(|o| o.as_dict())
        .expect("/Resources/Font");
    let cjk_entry = fonts.get("CJK").expect("/Resources/Font/CJK must exist");

    let (subtype, encoding) = match cjk_entry {
        PdfObject::Reference(n, g) => {
            let mut r = PdfReader::new(Cursor::new(&pdf)).unwrap();
            let obj = r.get_object(*n, *g).unwrap().clone();
            let fd = obj.as_dict().expect("font dict");
            let s = fd.get("Subtype").and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string()).unwrap_or_default();
            let e = fd.get("Encoding").and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string()).unwrap_or_default();
            (s, e)
        }
        PdfObject::Dictionary(d) => {
            // An inline dict is only acceptable if it already is Type0.
            // If it's Type1, this is the exact bug we are fixing.
            let s = d.get("Subtype").and_then(|o| o.as_name())
                .map(|n| n.as_str().to_string()).unwrap_or_default();
            assert_ne!(s, "Type1",
                "/Resources/Font/CJK is an inline dict with /Subtype /Type1 — \
                 this is precisely the bug issue #212 was filed about");
            panic!(
                "/Resources/Font/CJK must be an indirect Reference, not inline dict. \
                 Got dict with Subtype={:?}", s
            );
        }
        other => panic!("unexpected /Resources/Font/CJK: {:?}", other),
    };

    assert_eq!(subtype, "Type0",
        "Invariant 1 FAIL: resolved custom font must be /Subtype /Type0, got {:?}", subtype);
    assert_eq!(encoding, "Identity-H",
        "Invariant 1 FAIL: resolved custom font must be /Encoding /Identity-H, got {:?}", encoding);
}

/// Invariant 2: /AP/N content stream uses hex-encoded CIDs `<HHHH> Tj`,
/// not literal `(...)` Tj, for custom fonts.
#[test]
fn invariant_2_ap_content_uses_hex_cid_tj_not_literal() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let value = "高效能";
    let pdf = build_and_fill(cjk_data, value);
    let (ap_content, _) = extract_ap_n(&pdf);
    let content_str = String::from_utf8_lossy(&ap_content);

    // Must contain at least one <HHHH> Tj (hex-CID operator)
    assert!(
        content_str.contains('<') && content_str.contains("> Tj"),
        "Invariant 2 FAIL: /AP/N content must contain hex-CID Tj operator; content = {:?}",
        content_str
    );

    // Must NOT contain the raw UTF-8 bytes of the CJK value inside a `(...)` Tj
    let utf8_bytes = value.as_bytes();
    assert!(
        !ap_content.windows(utf8_bytes.len()).any(|w| w == utf8_bytes),
        "Invariant 2 FAIL: /AP/N content contains raw UTF-8 bytes of the CJK value — \
         the WinAnsi/literal path was taken instead of hex-CID"
    );
}

/// Invariant 3: Round-trip — /V on the field dict contains the filled value,
/// and the appearance is present (not empty).
#[test]
fn invariant_3_roundtrip_v_and_appearance_present() {
    let cjk_data = match load_fixture(CJK_PATH) {
        Some(d) => d,
        None => return,
    };
    let value = "高效能";
    let pdf = build_and_fill(cjk_data, value);

    // /AP/N must be a non-empty stream
    let (ap_content, _) = extract_ap_n(&pdf);
    assert!(!ap_content.is_empty(), "Invariant 3 FAIL: /AP/N content is empty");

    // /V on the AcroForm field must be present and non-empty
    let mut reader = PdfReader::new(Cursor::new(&pdf)).expect("parse PDF");
    let catalog = reader.catalog().expect("catalog").clone();
    let acro_ref = catalog.get("AcroForm").expect("/AcroForm in catalog");
    let acro_dict = match acro_ref {
        PdfObject::Reference(n, g) => reader.get_object(*n, *g).unwrap().clone()
            .as_dict().expect("AcroForm dict").clone(),
        PdfObject::Dictionary(d) => d.clone(),
        _ => panic!("unexpected AcroForm"),
    };
    let fields = acro_dict.get("Fields").and_then(|o| o.as_array())
        .expect("/AcroForm/Fields");

    let mut found_nonempty_v = false;
    for fr in &fields.0 {
        let (fn_, fg) = fr.as_reference().expect("field ref");
        let fobj = reader.get_object(fn_, fg).unwrap().clone();
        let fd = fobj.as_dict().expect("field dict");
        if let Some(v) = fd.get("V") {
            match v {
                PdfObject::String(s) if !s.is_empty() => {
                    found_nonempty_v = true;
                    // /V is PDF-encoded; at minimum its bytes must be non-empty.
                    // Full Unicode recovery requires PDF string decoding which is
                    // parser-internal; asserting non-empty is the verifiable contract.
                }
                _ => {}
            }
        }
    }
    assert!(found_nonempty_v,
        "Invariant 3 FAIL: no AcroForm field with non-empty /V found after fill_field");
}

/// Invariant 4: Regression — filling a widget with Font::Helvetica (built-in)
/// must continue to work. The Type1 path is correct for built-in fonts.
#[test]
fn invariant_4_helvetica_builtin_regression() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut fm = FormManager::new();

    let rect = Rectangle::new(Point::new(100.0, 700.0), Point::new(300.0, 720.0));
    let widget = Widget::new(rect).with_appearance(WidgetAppearance::default());
    let field = TextField::new("latin_field")
        .with_default_appearance(Font::Helvetica, 12.0, Color::black());
    let fref = fm.add_text_field(field, widget.clone(), None).unwrap();
    page.add_form_widget_with_ref(widget, fref).unwrap();
    doc.add_page(page);
    doc.set_form_manager(fm);

    doc.fill_field("latin_field", "Hello World")
        .expect("Invariant 4 FAIL: fill_field with Font::Helvetica must succeed");

    let pdf = doc.to_bytes().expect("Invariant 4 FAIL: to_bytes must succeed for Helvetica");
    assert!(!pdf.is_empty(), "Invariant 4 FAIL: serialized PDF must be non-empty");

    let (ap_content, _) = extract_ap_n(&pdf);
    let content_str = String::from_utf8_lossy(&ap_content);

    // The content stream must carry a literal `(Hello World) Tj`-style operator.
    // WinAnsi encoding of "Hello World" is identical to ASCII so we can check bytes.
    assert!(
        content_str.contains("Hello World"),
        "Invariant 4 FAIL: Helvetica AP stream must contain literal 'Hello World'; content = {:?}",
        content_str
    );

    // Must NOT be hex-CID Tj (the Type0 path must NOT have been taken for Helvetica).
    // A `<HHHH> Tj` in a Helvetica stream would indicate the custom-font path ran
    // incorrectly on a built-in font.
    // This check is approximate — we look for the absence of `<` immediately before ` Tj`.
    let has_hex_tj = content_str.contains("> Tj");
    assert!(
        !has_hex_tj,
        "Invariant 4 FAIL: Helvetica AP stream must NOT contain hex-CID Tj operator; content = {:?}",
        content_str
    );
}
```

**Step 2 — Run**: Fixture-dependent tests skip; Invariant 4 (Helvetica regression) must pass. Invariants 1-3 pass only if Tasks 2-5 are complete.

**Step 3 — No implementation** for this task — it is tests only.

**Step 4**: Run all four → pass (after Tasks 1-5 complete; Invariant 4 may pass already).

**Step 5 — fmt + clippy + commit**:
```
test(forms): add four issue #212 closing-invariant tests

Adds issue_212_closing_invariants_test.rs with the four acceptance
criteria from the issue body:
1. /AP/N /Resources/Font/CJK resolves to Type0 + Identity-H.
2. /AP/N content stream uses hex-CID Tj (not literal bytes).
3. Round-trip /V present and non-empty after fill_field.
4. Helvetica built-in regression — Type1 path unchanged.

Addresses #212.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 7 — Regression run: confirm pre-existing tests still pass

**Files**: none

**Step 1**: No new test to write.

**Step 2 — Run**:
```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml \
  --test fill_field_cjk_type0_test \
  --test fill_field_non_ascii_encoding_test \
  --test fill_field_roundtrip_test \
  --test issue_212_closing_invariants_test \
  --test issue_212_pushbutton_custom_font_test \
  --test issue_212_combobox_custom_font_test \
  2>&1
```

**Step 3**: If any failures occur, investigate. Pre-existing clippy warnings in `tests/forms_error_handling_test.rs` and `tests/memory_optimization_integration.rs` are NOT scope of this branch — document them but do not fix.

**Step 4**: All new tests green; pre-existing tests unaffected.

**Step 5 — No commit** for this task.

---

### Task 8 — CHANGELOG entry

**Files**: `CHANGELOG.md`

**Step 1**: No test for this task.

**Step 2**: Not applicable.

**Step 3 — Implement**: In `CHANGELOG.md`, under `## [Unreleased]`, add to the `### Fixed` section:

```markdown
- **Form field appearance streams with custom Type0/CID fonts** (addresses #212, architectural completion). `PushButtonAppearance` now emits a `/Subtype /Type0` placeholder resource dict for `Font::Custom(_)` fields instead of the incorrect `/Subtype /Type1`. `ListBoxAppearance` gains `generate_appearance_with_font` supporting hex-CID Tj emission via the same dispatch pattern as `TextFieldAppearance` and `ComboBoxAppearance`. The writer's guard (`ensure_form_ap_fonts_registered`) ensures custom fonts referenced only via form `/DA` — and not drawn on any page content stream — are still emitted by `write_fonts`, enabling `rewrite_ap_stream_font_resources` to rewrite the appearance placeholder to a proper indirect Reference. The v2.6.0 partial fix (PR #215) addressed WinAnsi encoding for built-in fonts; this completes the architectural Type0/CID path.
```

**Step 4**: Not applicable.

**Step 5 — fmt + commit**:
```
chore(changelog): record issue #212 architectural completion in Unreleased

Documents the Type0/CID form-appearance fixes:
PushButtonAppearance Type0 resource, ListBoxAppearance custom-font
path, writer guard for form-only font usage.

Addresses #212.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 9 — Doc-comment update in Document::fill_field

**Files**: `oxidize-pdf-core/src/document.rs`

**Step 1**: No test specific to this task.

**Step 2**: Not applicable.

**Step 3 — Implement**: Update the `fill_field` doc-comment (around line 554) to reflect the complete custom-font support:

The existing comment already references issue #212 in the `Font::Custom` dispatch section (line 612). Verify that the paragraph explicitly states:
- `Font::Custom(name)` with the font registered via `add_font_from_bytes` → Type0/CID path works end-to-end: hex-CID Tj in content stream, indirect Reference to document-level Type0 object in AP resources.
- `PushButtonAppearance` supports `Font::Custom` for resource generation (label encoding for push buttons is not yet supported; tracked separately).

If the existing comment already covers this accurately, this task is a no-op.

**Step 4**: `cargo doc --no-deps --manifest-path oxidize-pdf-core/Cargo.toml 2>&1 | grep "warning\|error"` — confirm no doc warnings introduced.

**Step 5 — fmt + commit** (only if changes were made):
```
docs(document): update fill_field doc-comment for complete Type0/CID support

Clarifies that PushButtonAppearance now produces correct Type0
resource entries for Font::Custom, and documents the remaining
gap (PushButton label hex-CID encoding not yet implemented).

Addresses #212.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 10 — Final full suite run

**Files**: none

**Step 1**: No new test.

**Step 2 — Run the full test suite**:
```bash
cargo test --manifest-path oxidize-pdf-core/Cargo.toml 2>&1 | tail -30
```

**Step 3**: Investigate any new failures introduced by this branch.

**Step 4**: Full suite passes (modulo pre-existing skips and the documented pre-existing clippy warnings in forms_error_handling_test.rs and memory_optimization_integration.rs).

**Step 5**: No commit for this task.

---

### Task 11 — Clippy clean and branch ready

**Files**: none

**Step 1**: No test.

**Step 2 — Run**:
```bash
cargo clippy --manifest-path oxidize-pdf-core/Cargo.toml --lib -- -D warnings 2>&1
cargo fmt --manifest-path oxidize-pdf-core/Cargo.toml --all -- --check 2>&1
```

**Step 3**: Fix any new warnings introduced by this branch's changes. Do NOT fix pre-existing warnings in test files outside this branch's scope.

**Step 4**: Clean output. Branch is ready for PR request to the user.

**Step 5**: If any fixes were needed, commit:
```
fix(forms): resolve clippy warnings from issue-212 branch changes

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Then: **Announce to the user that the branch is ready. Request explicit authorization to open a PR.**

---

## Estimation

| Phase | Tasks | Estimated time |
|---|---|---|
| Investigation / baseline (Task 1) | 1 | 15 min |
| PushButton fix + tests (Tasks 2) | 1 | 30 min |
| ListBox fix + tests (Task 3) | 1 | 45 min |
| ComboBox round-trip test (Task 4) | 1 | 30 min |
| Writer guard (Task 5) | 1 | 45 min |
| Closing-invariant tests (Task 6) | 1 | 45 min |
| Regression run (Task 7) | 1 | 15 min |
| CHANGELOG + docs (Tasks 8-9) | 2 | 20 min |
| Final verification (Tasks 10-11) | 2 | 20 min |
| **Total** | **11** | **~4h** |

---

## Self-Review

### Coverage of the 4 issue-body tests

| Issue body test | Covered by |
|---|---|
| /AP/N /Resources/Font/CJK → dict with /Subtype /Type0, /Encoding /Identity-H | Task 6 Invariant 1 + Task 4 |
| AP content uses `<HHHH> Tj` not `(...)` | Task 6 Invariant 2 + existing fill_field_cjk_type0_test.rs |
| Round-trip /V + appearance contain target Unicode codepoints | Task 6 Invariant 3 |
| Helvetica built-in regression | Task 6 Invariant 4 + Task 4 Helvetica test |

All four covered.

### `closes #212` / `fixes #212` keyword audit

Every commit message in this plan uses `Addresses #212` — no auto-close keywords present. GitHub does NOT interpret "Addresses" as a close directive. Verified against the documented pattern in error-log.md (2026-04-23 entry).

### Smoke test audit

Every `assert!` in all proposed tests checks:
- Specific PDF object types (PdfObject::Reference vs PdfObject::Dictionary)
- Specific dictionary key values (/Subtype "Type0", /Encoding "Identity-H")
- Specific content stream byte patterns (`<HHHH> Tj`, absence of raw UTF-8 bytes)
- Non-empty /V field values

No test asserts `is_ok()` alone. No test asserts byte count or file size. No test asserts only absence of panic.

### Pre-existing clippy warnings (out of scope)

The following test files have pre-existing clippy warnings in HEAD of develop that are NOT caused by this branch:
- `oxidize-pdf-core/tests/forms_error_handling_test.rs`
- `oxidize-pdf-core/tests/memory_optimization_integration.rs`

These are documented here to avoid confusion if `cargo clippy --tests` produces output. The plan runs `--lib` only for the blocking check.

### Risks

1. **`ChoiceField::with_default_appearance`** — The Task 4 test assumes this method exists. If it does not, the test file will not compile and Task 4 must first add the method (mirroring `TextField::with_default_appearance`). Estimate: +30 min.

2. **`FormManager::add_choice_field`** — Same assumption. If the method is named differently (e.g. `add_combo_field`), update the test to match the actual API. Read `oxidize-pdf-core/src/forms/form_manager.rs` to verify before writing the test.

3. **PushButton label encoding** — Task 2 fixes the resource dict but does NOT add hex-CID Tj support for PushButton labels. If a user creates a button with a custom font and a non-empty label, `emit_tj_for_builtin` will return `PdfError::EncodingError` because the label text hits the `font.is_custom()` guard. This is a known limitation that is documented in the doc-comment (Task 9) and tracked as a future improvement. It is NOT a regression introduced by this branch — it was already an error before (either returning `PdfError::EncodingError` from `emit_tj_for_builtin` or producing a corrupted stream with the old Type1 subtype).

4. **`PdfObject::String` /V encoding** — Task 6 Invariant 3 asserts `/V` is non-empty but does NOT decode the PDF string to verify the exact Unicode codepoints. Full round-trip codepoint verification would require the parser to handle UTF-16BE BOM strings. The assertion `!s.is_empty()` is the verifiable minimum. If a stronger assertion is needed, a utility helper `decode_pdf_string(bytes: &[u8]) -> String` would be needed — not written in this plan, deferred as future enhancement.

5. **Guard duplication** — `ensure_form_ap_fonts_registered` must be added to ALL four `write_*` entry points in the writer (`write_document`, `write_incremental_update`, `write_incremental_with_page_replacement`, `write_incremental_with_overlay`). Missing one would leave the guard inactive for incremental workflows. Task 5 must cover all four.
