//! Complete example of building a PDF renderer using oxidize-pdf
//!
//! This example demonstrates how to use the parser API to build
//! a basic PDF renderer that processes content streams and extracts
//! rendering information.

use oxidize_pdf::parser::content::{ContentOperation, ContentParser, TextElement};
use oxidize_pdf::parser::{PdfDocument, PdfReader};

/// Graphics state for rendering
#[derive(Debug, Clone)]
struct GraphicsState {
    // Transformation matrix
    ctm: [f64; 6],
    // Text state
    text_matrix: [f64; 6],
    text_line_matrix: [f64; 6],
    font_name: Option<String>,
    font_size: f64,
    // Graphics state
    line_width: f64,
    stroke_color: (f64, f64, f64),
    fill_color: (f64, f64, f64),
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text_line_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            font_name: None,
            font_size: 12.0,
            line_width: 1.0,
            stroke_color: (0.0, 0.0, 0.0),
            fill_color: (0.0, 0.0, 0.0),
        }
    }
}

/// Simple PDF renderer that processes content streams
struct PdfRenderer {
    state: GraphicsState,
    state_stack: Vec<GraphicsState>,
    current_path: Vec<PathCommand>,
    rendered_items: Vec<RenderItem>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PathCommand {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    CurveTo(f64, f64, f64, f64, f64, f64),
    ClosePath,
}

#[derive(Debug)]
enum RenderItem {
    Text {
        x: f64,
        y: f64,
        text: String,
        font: String,
        size: f64,
    },
    Path {
        commands: Vec<PathCommand>,
        stroke: Option<(f64, f64, f64)>,
        fill: Option<(f64, f64, f64)>,
        line_width: f64,
    },
    Image {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        xobject_name: String,
    },
}

impl PdfRenderer {
    fn new() -> Self {
        Self {
            state: GraphicsState::default(),
            state_stack: Vec::new(),
            current_path: Vec::new(),
            rendered_items: Vec::new(),
        }
    }

    fn render_page(
        &mut self,
        document: &PdfDocument<std::fs::File>,
        page_idx: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let page = document.get_page(page_idx)?;

        println!(
            "Rendering page {} ({}x{} points)",
            page_idx + 1,
            page.width(),
            page.height()
        );

        // Get page resources
        let resources = page.get_resources();
        if let Some(res) = resources {
            self.analyze_resources(res, document)?;
        }

        // Get content streams
        let streams = page.content_streams_with_document(document)?;

        // Process each content stream
        for (stream_idx, stream) in streams.iter().enumerate() {
            println!("Processing content stream {stream_idx}");
            let operations = ContentParser::parse(stream)?;
            self.process_operations(operations)?;
        }

        // Output rendered items
        println!("\nRendered items:");
        for item in &self.rendered_items {
            match item {
                RenderItem::Text {
                    x,
                    y,
                    text,
                    font,
                    size,
                } => {
                    println!("Text at ({x:.2}, {y:.2}): '{text}' [Font: {font}, Size: {size}]");
                }
                RenderItem::Path {
                    commands,
                    stroke,
                    fill,
                    line_width,
                } => {
                    println!("Path with {} commands", commands.len());
                    if let Some((r, g, b)) = stroke {
                        println!("  Stroke: RGB({r:.2}, {g:.2}, {b:.2}), Width: {line_width}");
                    }
                    if let Some((r, g, b)) = fill {
                        println!("  Fill: RGB({r:.2}, {g:.2}, {b:.2})");
                    }
                }
                RenderItem::Image {
                    x,
                    y,
                    width,
                    height,
                    xobject_name,
                } => {
                    println!("Image '{xobject_name}' at ({x:.2}, {y:.2}) size {width}x{height}");
                }
            }
        }

        Ok(())
    }

    fn analyze_resources(
        &self,
        resources: &oxidize_pdf::parser::PdfDictionary,
        document: &PdfDocument<std::fs::File>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Analyze fonts
        if let Some(fonts) = resources.get("Font").and_then(|f| f.as_dict()) {
            println!("\nFonts in resources:");
            for (name, font_ref) in &fonts.0 {
                println!("  {} -> {:?}", name.as_str(), font_ref);

                // Resolve font object
                if let Ok(font_obj) = document.resolve(font_ref) {
                    if let Some(font_dict) = font_obj.as_dict() {
                        if let Some(base_font) = font_dict.get("BaseFont").and_then(|b| b.as_name())
                        {
                            println!("    BaseFont: {}", base_font.as_str());
                        }
                        if let Some(subtype) = font_dict.get("Subtype").and_then(|s| s.as_name()) {
                            println!("    Subtype: {}", subtype.as_str());
                        }
                    }
                }
            }
        }

        // Analyze XObjects
        if let Some(xobjects) = resources.get("XObject").and_then(|x| x.as_dict()) {
            println!("\nXObjects in resources:");
            for (name, xobj_ref) in &xobjects.0 {
                println!("  {} -> {:?}", name.as_str(), xobj_ref);

                // Resolve XObject
                if let Ok(xobj) = document.resolve(xobj_ref) {
                    if let Some(xobj_stream) = xobj.as_stream() {
                        if let Some(subtype) =
                            xobj_stream.dict.get("Subtype").and_then(|s| s.as_name())
                        {
                            println!("    Subtype: {}", subtype.as_str());

                            if subtype.as_str() == "Image" {
                                let width = xobj_stream
                                    .dict
                                    .get("Width")
                                    .and_then(|w| w.as_integer())
                                    .unwrap_or(0);
                                let height = xobj_stream
                                    .dict
                                    .get("Height")
                                    .and_then(|h| h.as_integer())
                                    .unwrap_or(0);
                                println!("    Image dimensions: {width}x{height}");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn process_operations(
        &mut self,
        operations: Vec<ContentOperation>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for op in operations {
            match op {
                // Graphics state
                ContentOperation::SaveGraphicsState => {
                    self.state_stack.push(self.state.clone());
                }
                ContentOperation::RestoreGraphicsState => {
                    if let Some(saved) = self.state_stack.pop() {
                        self.state = saved;
                    }
                }

                // Transform matrix
                ContentOperation::SetTransformMatrix(a, b, c, d, e, f) => {
                    let transform = [a as f64, b as f64, c as f64, d as f64, e as f64, f as f64];
                    self.state.ctm = multiply_matrices(&self.state.ctm, &transform);
                }

                // Path construction
                ContentOperation::MoveTo(x, y) => {
                    let (tx, ty) = transform_point(x as f64, y as f64, &self.state.ctm);
                    self.current_path.push(PathCommand::MoveTo(tx, ty));
                }
                ContentOperation::LineTo(x, y) => {
                    let (tx, ty) = transform_point(x as f64, y as f64, &self.state.ctm);
                    self.current_path.push(PathCommand::LineTo(tx, ty));
                }
                ContentOperation::CurveTo(x1, y1, x2, y2, x3, y3) => {
                    let (tx1, ty1) = transform_point(x1 as f64, y1 as f64, &self.state.ctm);
                    let (tx2, ty2) = transform_point(x2 as f64, y2 as f64, &self.state.ctm);
                    let (tx3, ty3) = transform_point(x3 as f64, y3 as f64, &self.state.ctm);
                    self.current_path
                        .push(PathCommand::CurveTo(tx1, ty1, tx2, ty2, tx3, ty3));
                }
                ContentOperation::ClosePath => {
                    self.current_path.push(PathCommand::ClosePath);
                }
                ContentOperation::Rectangle(x, y, w, h) => {
                    let (tx, ty) = transform_point(x as f64, y as f64, &self.state.ctm);
                    self.current_path.push(PathCommand::MoveTo(tx, ty));
                    self.current_path
                        .push(PathCommand::LineTo(tx + w as f64, ty));
                    self.current_path
                        .push(PathCommand::LineTo(tx + w as f64, ty + h as f64));
                    self.current_path
                        .push(PathCommand::LineTo(tx, ty + h as f64));
                    self.current_path.push(PathCommand::ClosePath);
                }

                // Path painting
                ContentOperation::Stroke => {
                    if !self.current_path.is_empty() {
                        self.rendered_items.push(RenderItem::Path {
                            commands: self.current_path.clone(),
                            stroke: Some(self.state.stroke_color),
                            fill: None,
                            line_width: self.state.line_width,
                        });
                        self.current_path.clear();
                    }
                }
                ContentOperation::Fill => {
                    if !self.current_path.is_empty() {
                        self.rendered_items.push(RenderItem::Path {
                            commands: self.current_path.clone(),
                            stroke: None,
                            fill: Some(self.state.fill_color),
                            line_width: self.state.line_width,
                        });
                        self.current_path.clear();
                    }
                }
                ContentOperation::FillStroke => {
                    if !self.current_path.is_empty() {
                        self.rendered_items.push(RenderItem::Path {
                            commands: self.current_path.clone(),
                            stroke: Some(self.state.stroke_color),
                            fill: Some(self.state.fill_color),
                            line_width: self.state.line_width,
                        });
                        self.current_path.clear();
                    }
                }
                ContentOperation::EndPath => {
                    self.current_path.clear();
                }

                // Color
                ContentOperation::SetStrokingRGB(r, g, b) => {
                    self.state.stroke_color = (r as f64, g as f64, b as f64);
                }
                ContentOperation::SetNonStrokingRGB(r, g, b) => {
                    self.state.fill_color = (r as f64, g as f64, b as f64);
                }
                ContentOperation::SetLineWidth(w) => {
                    self.state.line_width = w as f64;
                }

                // Text
                ContentOperation::BeginText => {
                    self.state.text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    self.state.text_line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                }
                ContentOperation::SetFont(name, size) => {
                    self.state.font_name = Some(name);
                    self.state.font_size = size as f64;
                }
                ContentOperation::SetTextMatrix(a, b, c, d, e, f) => {
                    self.state.text_matrix =
                        [a as f64, b as f64, c as f64, d as f64, e as f64, f as f64];
                    self.state.text_line_matrix = self.state.text_matrix;
                }
                ContentOperation::MoveText(tx, ty) => {
                    let translation = [1.0, 0.0, 0.0, 1.0, tx as f64, ty as f64];
                    self.state.text_matrix =
                        multiply_matrices(&translation, &self.state.text_line_matrix);
                    self.state.text_line_matrix = self.state.text_matrix;
                }
                ContentOperation::ShowText(text_bytes) => {
                    let text = String::from_utf8_lossy(&text_bytes).to_string();
                    let (x, y) = transform_point(0.0, 0.0, &self.state.text_matrix);

                    self.rendered_items.push(RenderItem::Text {
                        x,
                        y,
                        text: text.clone(),
                        font: self
                            .state
                            .font_name
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string()),
                        size: self.state.font_size,
                    });

                    // Update text matrix for next text
                    let text_width = text.len() as f64 * self.state.font_size * 0.5; // Approximate
                    let advance = [1.0, 0.0, 0.0, 1.0, text_width, 0.0];
                    self.state.text_matrix = multiply_matrices(&advance, &self.state.text_matrix);
                }
                ContentOperation::ShowTextArray(elements) => {
                    for element in elements {
                        match element {
                            TextElement::Text(text_bytes) => {
                                let text = String::from_utf8_lossy(&text_bytes).to_string();
                                let (x, y) = transform_point(0.0, 0.0, &self.state.text_matrix);

                                self.rendered_items.push(RenderItem::Text {
                                    x,
                                    y,
                                    text: text.clone(),
                                    font: self
                                        .state
                                        .font_name
                                        .clone()
                                        .unwrap_or_else(|| "Unknown".to_string()),
                                    size: self.state.font_size,
                                });

                                // Update text matrix
                                let text_width = text.len() as f64 * self.state.font_size * 0.5;
                                let advance = [1.0, 0.0, 0.0, 1.0, text_width, 0.0];
                                self.state.text_matrix =
                                    multiply_matrices(&advance, &self.state.text_matrix);
                            }
                            TextElement::Spacing(adjustment) => {
                                // Adjust text position
                                let tx = -(adjustment as f64) / 1000.0 * self.state.font_size;
                                let advance = [1.0, 0.0, 0.0, 1.0, tx, 0.0];
                                self.state.text_matrix =
                                    multiply_matrices(&advance, &self.state.text_matrix);
                            }
                        }
                    }
                }

                // XObject
                ContentOperation::PaintXObject(name) => {
                    // In a real renderer, we would resolve and render the XObject
                    let (x, y) = transform_point(0.0, 0.0, &self.state.ctm);
                    self.rendered_items.push(RenderItem::Image {
                        x,
                        y,
                        width: 100.0, // Would get from XObject
                        height: 100.0,
                        xobject_name: name,
                    });
                }

                _ => {
                    // Other operations not handled in this example
                }
            }
        }

        Ok(())
    }
}

// Matrix operations
fn multiply_matrices(a: &[f64; 6], b: &[f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn transform_point(x: f64, y: f64, matrix: &[f64; 6]) -> (f64, f64) {
    let tx = matrix[0] * x + matrix[2] * y + matrix[4];
    let ty = matrix[1] * x + matrix[3] * y + matrix[5];
    (tx, ty)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pdf-file>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];

    // Open PDF document
    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    // Get document information
    let page_count = document.page_count()?;
    let metadata = document.metadata()?;

    println!("PDF Document: {pdf_path}");
    println!("Version: {}", document.version()?);
    println!("Pages: {page_count}");
    if let Some(title) = &metadata.title {
        println!("Title: {title}");
    }
    if let Some(author) = &metadata.author {
        println!("Author: {author}");
    }
    println!();

    // Create renderer
    let mut renderer = PdfRenderer::new();

    // Render first page (or all pages)
    renderer.render_page(&document, 0)?;

    Ok(())
}
