//! High Complexity PDF Benchmark - Technical Manual Simulation
//!
//! This benchmark creates a complex technical document with:
//! - Table of contents with internal links
//! - Technical diagrams (shapes, lines, arrows)
//! - Code blocks with syntax highlighting
//! - Complex tables with merged cells simulation
//! - Marginal notes and callouts
//! - Cross-references and annotations
//! - Multiple text formatting styles
//!
//! Expected performance: 100-300 pages/second (high complexity)

use oxidize_pdf::{Color, Document, Font, Page, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(100)
    } else {
        100
    };

    let start_time = Instant::now();
    let mut doc = Document::new();
    doc.set_title("Technical Manual - High Complexity Benchmark");
    doc.set_author("oxidize-pdf Performance Test");
    doc.set_subject("Software Architecture Documentation");

    let chapters = [
        "System Architecture",
        "API Documentation",
        "Database Design",
        "Security Protocols",
        "Deployment Guide",
        "User Interface",
        "Performance Tuning",
        "Monitoring",
        "Testing Strategy",
    ];
    let code_languages = [
        "Rust",
        "Python",
        "SQL",
        "TypeScript",
        "YAML",
        "JavaScript",
        "Bash",
        "JSON",
        "XML",
    ];

    for page_num in 0..page_count {
        let mut page = Page::a4();
        let mut y_pos = 780.0;
        let chapter = chapters[page_num % chapters.len()];
        let section = (page_num / 5) + 1;

        // === TECHNICAL HEADER ===
        // Header background
        page.graphics()
            .set_fill_color(Color::rgb(0.1, 0.1, 0.2))
            .rectangle(0.0, y_pos - 35.0, 595.0, 40.0)
            .fill();

        // Logo placeholder (hexagon shape)
        let logo_x = 30.0;
        let logo_y = y_pos - 15.0;
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.7, 0.3))
            .move_to(logo_x, logo_y - 8.0)
            .line_to(logo_x + 6.0, logo_y - 12.0)
            .line_to(logo_x + 18.0, logo_y - 12.0)
            .line_to(logo_x + 24.0, logo_y - 8.0)
            .line_to(logo_x + 18.0, logo_y - 4.0)
            .line_to(logo_x + 6.0, logo_y - 4.0)
            .close_path()
            .fill();

        page.text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_fill_color(Color::white())
            .at(70.0, y_pos - 20.0)
            .write("TechDocs Pro v2.1")?;

        // Chapter and page info
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .set_fill_color(Color::rgb(0.7, 0.7, 0.7))
            .at(300.0, y_pos - 15.0)
            .write(&format!("Chapter {}: {}", section, chapter))?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .set_fill_color(Color::rgb(0.6, 0.6, 0.6))
            .at(480.0, y_pos - 15.0)
            .write(&format!("Page {}", page_num + 1))?;

        y_pos -= 50.0;

        // === BREADCRUMB NAVIGATION ===
        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.5, 0.5, 0.5))
            .at(50.0, y_pos)
            .write(&format!(
                "Home > Documentation > {} > Section {}.{}",
                chapter,
                section,
                (page_num % 5) + 1
            ))?;

        y_pos -= 25.0;

        // === SECTION TITLE WITH DECORATIVE ELEMENTS ===
        page.text()
            .set_font(Font::HelveticaBold, 18.0)
            .set_fill_color(Color::rgb(0.1, 0.1, 0.4))
            .at(50.0, y_pos)
            .write(&format!(
                "{}.{} Implementation Details",
                section,
                (page_num % 5) + 1
            ))?;

        // Decorative underline with gradient simulation
        for i in 0..5 {
            let alpha = 0.8 - (i as f64 * 0.15);
            page.graphics()
                .set_stroke_color(Color::rgb(0.1 * alpha, 0.1 * alpha, 0.4 * alpha))
                .set_line_width(2.0 - (i as f64 * 0.3))
                .move_to(50.0, y_pos - 8.0 - (i as f64))
                .line_to(400.0, y_pos - 8.0 - (i as f64))
                .stroke();
        }

        y_pos -= 40.0;

        // === ENHANCED TECHNICAL DIAGRAM (varies per page) ===
        if page_num % 3 == 0 {
            let diagram_type = (page_num / 3) % 3;
            let diagram_title = match diagram_type {
                0 => "System Architecture - Network Topology",
                1 => "Data Flow Pipeline - Processing Stages",
                _ => "Microservices Communication Graph",
            };

            page.text()
                .set_font(Font::HelveticaBold, 12.0)
                .at(50.0, y_pos)
                .write(&format!("{} (Page {})", diagram_title, page_num + 1))?;

            y_pos -= 30.0;

            // Diagram background with grid
            page.graphics()
                .set_fill_color(Color::rgb(0.95, 0.96, 0.98))
                .rectangle(50.0, y_pos - 110.0, 495.0, 105.0)
                .fill();

            // Grid pattern
            for gx in 0..10 {
                let x = 50.0 + (gx as f64 * 49.5);
                page.graphics()
                    .set_stroke_color(Color::rgb(0.88, 0.89, 0.92))
                    .set_line_width(0.3)
                    .move_to(x, y_pos - 110.0)
                    .line_to(x, y_pos - 5.0)
                    .stroke();
            }

            let diagram_start_x = 70.0;
            let diagram_start_y = y_pos;

            // Main system box (unique per page with shadow effect)
            let core_name = match page_num % 9 {
                0 => "Core Engine",
                1 => "API Gateway",
                2 => "Data Processor",
                3 => "Auth Service",
                4 => "Cache Manager",
                5 => "Load Balancer",
                6 => "Message Broker",
                7 => "Event Stream",
                _ => "Service Mesh",
            };

            // Shadow
            page.graphics()
                .set_fill_color(Color::rgb(0.7, 0.7, 0.75))
                .rectangle(diagram_start_x + 3.0, diagram_start_y - 63.0, 120.0, 50.0)
                .fill();

            // Main box with gradient
            for layer in 0..5 {
                let layer_h = 10.0;
                let layer_y = diagram_start_y - 60.0 + (layer as f64 * layer_h);
                let alpha = 0.85 + (layer as f64 * 0.03);
                page.graphics()
                    .set_fill_color(Color::rgb(0.85 * alpha, 0.9 * alpha, 1.0 * alpha))
                    .rectangle(diagram_start_x, layer_y, 120.0, layer_h)
                    .fill();
            }

            page.graphics()
                .set_stroke_color(Color::rgb(0.2, 0.3, 0.7))
                .set_line_width(2.5)
                .rectangle(diagram_start_x, diagram_start_y - 60.0, 120.0, 50.0)
                .stroke();

            page.text()
                .set_font(Font::HelveticaBold, 10.0)
                .set_fill_color(Color::rgb(0.1, 0.1, 0.5))
                .at(diagram_start_x + 10.0, diagram_start_y - 35.0)
                .write(core_name)?;

            // Status indicator
            let status_color = match (page_num * 7) % 3 {
                0 => Color::rgb(0.2, 0.8, 0.2), // Green
                1 => Color::rgb(1.0, 0.8, 0.0), // Yellow
                _ => Color::rgb(0.9, 0.3, 0.3), // Red
            };
            page.graphics()
                .set_fill_color(status_color)
                .circle(diagram_start_x + 110.0, diagram_start_y - 15.0, 4.0)
                .fill();

            // Connected components (unique layout per page)
            let component_names = [
                "PostgreSQL",
                "Redis",
                "RabbitMQ",
                "ElasticSearch",
                "Prometheus",
                "Grafana",
                "Vault",
                "Consul",
                "Kafka",
                "Cassandra",
                "MongoDB",
                "Neo4j",
            ];

            let num_components = 5 + (page_num % 3);
            for i in 0..num_components {
                let angle = (i as f64 / num_components as f64) * std::f64::consts::PI * 1.5 - 0.5;
                let radius = 180.0 + ((page_num * (i + 1)) % 30) as f64;
                let comp_x = diagram_start_x + 60.0 + angle.cos() * radius;
                let comp_y = diagram_start_y - 35.0 - angle.sin() * (radius * 0.4);
                let comp_idx = (page_num * 3 + i * 5) % component_names.len();

                // Component shadow
                page.graphics()
                    .set_fill_color(Color::rgb(0.75, 0.75, 0.78))
                    .rectangle(comp_x + 2.0, comp_y - 27.0, 70.0, 30.0)
                    .fill();

                // Component box with varying colors
                let color_shift = (page_num as f64 * 0.07 + i as f64 * 0.13) % 0.4;
                page.graphics()
                    .set_fill_color(Color::rgb(
                        1.0 - color_shift * 0.3,
                        0.92 - color_shift * 0.2,
                        0.85 - color_shift * 0.15,
                    ))
                    .rectangle(comp_x, comp_y - 25.0, 70.0, 30.0)
                    .fill();

                page.graphics()
                    .set_stroke_color(Color::rgb(0.8, 0.5, 0.2))
                    .set_line_width(1.5)
                    .rectangle(comp_x, comp_y - 25.0, 70.0, 30.0)
                    .stroke();

                page.text()
                    .set_font(Font::HelveticaBold, 7.0)
                    .set_fill_color(Color::rgb(0.3, 0.2, 0.1))
                    .at(comp_x + 5.0, comp_y - 12.0)
                    .write(component_names[comp_idx])?;

                // Connection line with curve
                let mid_x = (diagram_start_x + 120.0 + comp_x) / 2.0;
                let mid_y = (diagram_start_y - 35.0 + comp_y - 10.0) / 2.0
                    + ((page_num * i) % 20) as f64
                    - 10.0;

                // Bezier-like curve using multiple line segments
                let segments = 8;
                for s in 0..segments {
                    let t = s as f64 / segments as f64;
                    let t_next = (s + 1) as f64 / segments as f64;

                    let x1 = (1.0 - t).powi(2) * (diagram_start_x + 120.0)
                        + 2.0 * (1.0 - t) * t * mid_x
                        + t.powi(2) * comp_x;
                    let y1 = (1.0 - t).powi(2) * (diagram_start_y - 35.0)
                        + 2.0 * (1.0 - t) * t * mid_y
                        + t.powi(2) * (comp_y - 10.0);

                    let x2 = (1.0 - t_next).powi(2) * (diagram_start_x + 120.0)
                        + 2.0 * (1.0 - t_next) * t_next * mid_x
                        + t_next.powi(2) * comp_x;
                    let y2 = (1.0 - t_next).powi(2) * (diagram_start_y - 35.0)
                        + 2.0 * (1.0 - t_next) * t_next * mid_y
                        + t_next.powi(2) * (comp_y - 10.0);

                    page.graphics()
                        .set_stroke_color(Color::rgb(0.45, 0.45, 0.55))
                        .set_line_width(1.2)
                        .move_to(x1, y1)
                        .line_to(x2, y2)
                        .stroke();
                }

                // Arrow head
                page.graphics()
                    .set_fill_color(Color::rgb(0.4, 0.4, 0.5))
                    .move_to(comp_x - 5.0, comp_y - 10.0)
                    .line_to(comp_x - 10.0, comp_y - 7.0)
                    .line_to(comp_x - 10.0, comp_y - 13.0)
                    .close_path()
                    .fill();

                // Data rate label (unique per page)
                let rate = 100 + (page_num * (i + 1) * 17) % 900;
                page.text()
                    .set_font(Font::Helvetica, 6.0)
                    .set_fill_color(Color::rgb(0.5, 0.5, 0.6))
                    .at(mid_x - 15.0, mid_y - 3.0)
                    .write(&format!("{}req/s", rate))?;
            }

            y_pos -= 115.0;
        }

        // === CODE BLOCK WITH SYNTAX HIGHLIGHTING (unique per page) ===
        let language_idx = (page_num * 3) % code_languages.len();
        let language = code_languages[language_idx];

        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(50.0, y_pos)
            .write(&format!("Code Example - {} Implementation", language))?;

        y_pos -= 20.0;

        // Code block background (more visible)
        page.graphics()
            .set_fill_color(Color::rgb(0.15, 0.15, 0.2))
            .rectangle(50.0, y_pos - 120.0, 495.0, 115.0)
            .fill();

        // Line numbers background
        page.graphics()
            .set_fill_color(Color::rgb(0.2, 0.2, 0.25))
            .rectangle(50.0, y_pos - 120.0, 30.0, 115.0)
            .fill();

        // Code content with syntax highlighting simulation (unique per page)
        let code_lines = match language {
            "Rust" => {
                let func_name = match page_num % 5 {
                    0 => "process_request",
                    1 => "handle_connection",
                    2 => "parse_data",
                    3 => "validate_input",
                    _ => "execute_task",
                };
                let struct_name = match page_num % 4 {
                    0 => "RequestProcessor",
                    1 => "ConnectionHandler",
                    2 => "DataParser",
                    _ => "TaskExecutor",
                };
                vec![
                    format!("fn {}(req: Request) -> Result<Response> {{", func_name),
                    format!("    let mut processor = {}::new();", struct_name),
                    format!("    processor.validate_headers(&req.headers)?;"),
                    format!("    let session_id = {};", page_num * 1000 + 42),
                    format!("    match req.method() {{"),
                    format!("        Method::GET => handle_get_request(req),"),
                    format!("        Method::POST => handle_post_request(req),"),
                    format!("        _ => Err(ErrorKind::UnsupportedMethod),"),
                    format!("    }}"),
                    format!("}}"),
                ]
            }
            "Python" => {
                let class_name = match page_num % 4 {
                    0 => "DataProcessor",
                    1 => "RequestHandler",
                    2 => "EventProcessor",
                    _ => "TaskManager",
                };
                let cache_size = 500 + (page_num * 100);
                vec![
                    format!("class {}:", class_name),
                    format!("    def __init__(self, config: Config):"),
                    format!("        self.config = config"),
                    format!("        self.cache = LRUCache(maxsize={})", cache_size),
                    format!("        self.session_id = {}", page_num + 1000),
                    format!("    async def process_data(self, data: List[Dict]):"),
                    format!("        results = []"),
                    format!("        for item in data:"),
                    format!("            processed = await self._process_item(item)"),
                    format!("            results.append(processed)"),
                ]
            }
            _ => {
                let timeout = 5000 + (page_num * 500);
                let retry_count = 3 + (page_num % 5);
                vec![
                    format!("interface ProcessorConfig{} {{", page_num + 1),
                    format!("  timeout: {};", timeout),
                    format!("  retryCount: {};", retry_count),
                    format!("  enableCache: {};", page_num % 2 == 0),
                    format!("  sessionId: {};", page_num * 100),
                    format!("}}"),
                    format!(""),
                    format!("export class DataProcessor{} {{", page_num + 1),
                    format!(
                        "  constructor(private config: ProcessorConfig{}) {{}}",
                        page_num + 1
                    ),
                    format!("  async process(data: unknown[]): Promise<Result[]> {{"),
                ]
            }
        };

        for (i, line) in code_lines.iter().enumerate() {
            let line_y = y_pos - 15.0 - (i as f64 * 12.0);

            // Line number
            page.text()
                .set_font(Font::Courier, 8.0)
                .set_fill_color(Color::rgb(0.6, 0.6, 0.6))
                .at(55.0, line_y)
                .write(&format!("{:2}", i + 1))?;

            // Code with basic syntax highlighting (improved legibility)
            if line.contains("fn ") || line.contains("class ") || line.contains("interface ") {
                page.text()
                    .set_font(Font::CourierBold, 9.0)
                    .set_fill_color(Color::rgb(0.9, 0.6, 1.0))
                    .at(85.0, line_y)
                    .write(line)?;
            } else if line.contains("//") || line.contains("#") {
                page.text()
                    .set_font(Font::CourierOblique, 9.0)
                    .set_fill_color(Color::rgb(0.7, 0.9, 0.7))
                    .at(85.0, line_y)
                    .write(line)?;
            } else {
                page.text()
                    .set_font(Font::Courier, 9.0)
                    .set_fill_color(Color::rgb(0.95, 0.95, 0.95))
                    .at(85.0, line_y)
                    .write(line)?;
            }
        }

        y_pos -= 140.0;

        // === COMPLEX TABLE WITH SIMULATED MERGED CELLS ===
        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(50.0, y_pos)
            .write("API Endpoint Configuration")?;

        y_pos -= 25.0;

        // Table structure (unique data per page)
        let api_endpoints = [
            "/api/v1/users",
            "/api/v1/data",
            "/api/v1/reports",
            "/api/v1/analytics",
            "/api/v1/sessions",
            "/api/v1/files",
            "/api/v1/auth",
            "/api/v1/metrics",
            "/api/v1/logs",
        ];
        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];
        let auth_types = ["JWT", "API Key", "OAuth2", "Basic", "Bearer"];
        let statuses = ["Active", "Beta", "Deprecated", "Maintenance"];

        // Pre-generate strings to avoid borrow checker issues
        let mut rate_limits = Vec::new();
        let mut response_times = Vec::new();

        for i in 0..4 {
            rate_limits.push(format!(
                "{}{}min",
                10 + (page_num * 5) + (i * 15),
                if i % 2 == 0 { "/" } else { " per " }
            ));
            response_times.push(format!(
                "< {}{}s",
                if i == 0 {
                    50 + (page_num * 10)
                } else {
                    100 + (page_num * 50) + (i * 200)
                },
                if i == 0 { "m" } else { "" }
            ));
        }

        let mut table_data = vec![vec![
            "Endpoint",
            "Method",
            "Auth",
            "Rate Limit",
            "Response Time",
            "Status",
        ]];

        // Generate unique table rows per page
        for i in 0..4 {
            let endpoint_idx = (page_num * 3 + i) % api_endpoints.len();
            let method_idx = (page_num + i * 2) % methods.len();
            let auth_idx = (page_num * 2 + i) % auth_types.len();
            let status_idx = (page_num + i * 3) % statuses.len();

            table_data.push(vec![
                api_endpoints[endpoint_idx],
                methods[method_idx],
                auth_types[auth_idx],
                &rate_limits[i],
                &response_times[i],
                statuses[status_idx],
            ]);
        }

        for (row_idx, row_data) in table_data.iter().enumerate() {
            let row_y = y_pos - (row_idx as f64 * 20.0);

            if row_idx == 0 {
                // Header row
                page.graphics()
                    .set_fill_color(Color::rgb(0.2, 0.2, 0.4))
                    .rectangle(50.0, row_y - 15.0, 495.0, 18.0)
                    .fill();
            } else {
                // Data rows with alternating colors
                let bg_color = if row_idx % 2 == 0 {
                    Color::rgb(0.98, 0.98, 0.98)
                } else {
                    Color::white()
                };
                page.graphics()
                    .set_fill_color(bg_color)
                    .rectangle(50.0, row_y - 15.0, 495.0, 18.0)
                    .fill();
            }

            // Cell borders
            page.graphics()
                .set_stroke_color(Color::rgb(0.7, 0.7, 0.7))
                .set_line_width(0.5)
                .rectangle(50.0, row_y - 15.0, 495.0, 18.0)
                .stroke();

            let col_widths = [120.0, 80.0, 80.0, 80.0, 90.0, 65.0];
            let mut col_x = 50.0;

            for (col_idx, (cell_data, col_width)) in
                row_data.iter().zip(col_widths.iter()).enumerate()
            {
                let text_color = if row_idx == 0 {
                    Color::white()
                } else if col_idx == 5 && *cell_data == "Deprecated" {
                    Color::rgb(0.8, 0.3, 0.3)
                } else if col_idx == 5 && *cell_data == "Beta" {
                    Color::rgb(0.8, 0.6, 0.1)
                } else {
                    Color::black()
                };

                let font = if row_idx == 0 {
                    Font::HelveticaBold
                } else {
                    Font::Helvetica
                };

                page.text()
                    .set_font(font, 9.0)
                    .set_fill_color(text_color)
                    .at(col_x + 5.0, row_y - 8.0)
                    .write(cell_data)?;

                col_x += col_width;
            }
        }

        y_pos -= 120.0;

        // === MARGINAL NOTES ===
        if page_num % 4 == 1 {
            // Note callout box
            page.graphics()
                .set_fill_color(Color::rgb(1.0, 0.98, 0.9))
                .set_stroke_color(Color::rgb(1.0, 0.8, 0.4))
                .set_line_width(2.0)
                .rectangle(350.0, y_pos - 60.0, 180.0, 55.0)
                .fill();

            // Note icon (exclamation mark)
            page.graphics()
                .set_fill_color(Color::rgb(1.0, 0.6, 0.0))
                .circle(365.0, y_pos - 32.0, 8.0)
                .fill();

            page.text()
                .set_font(Font::HelveticaBold, 12.0)
                .set_fill_color(Color::white())
                .at(361.0, y_pos - 28.0)
                .write("!")?;

            page.text()
                .set_font(Font::HelveticaBold, 10.0)
                .set_fill_color(Color::rgb(0.8, 0.4, 0.0))
                .at(380.0, y_pos - 20.0)
                .write("Important Note")?;

            page.text()
                .set_font(Font::Helvetica, 8.0)
                .at(355.0, y_pos - 35.0)
                .write("This configuration requires")?;

            page.text()
                .set_font(Font::Helvetica, 8.0)
                .at(355.0, y_pos - 45.0)
                .write("special permissions. See")?;

            page.text()
                .set_font(Font::HelveticaOblique, 8.0)
                .set_fill_color(Color::rgb(0.0, 0.4, 0.8))
                .at(355.0, y_pos - 55.0)
                .write("Section 3.2 for details.")?;
        }

        // === FOOTER WITH REFERENCES ===
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.8, 0.8))
            .set_line_width(0.5)
            .move_to(50.0, 60.0)
            .line_to(545.0, 60.0)
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.5, 0.5, 0.5))
            .at(50.0, 45.0)
            .write(&format!(
                "Technical Manual v2.1 | {} | Last updated: Sept 2025",
                chapter
            ))?;

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.5, 0.5, 0.5))
            .at(400.0, 45.0)
            .write(&format!(
                "Page {} of {} | Ref: TM-{:03}",
                page_num + 1,
                page_count,
                page_num + 1
            ))?;

        // Version info
        page.text()
            .set_font(Font::Helvetica, 7.0)
            .set_fill_color(Color::rgb(0.6, 0.6, 0.6))
            .at(50.0, 25.0)
            .write("This document contains proprietary information. Distribution restricted.")?;

        doc.add_page(page);
    }

    let generation_time = start_time.elapsed();

    // Separate write timing
    let write_start = Instant::now();
    doc.save("examples/results/high_complexity_benchmark.pdf")?;
    let write_time = write_start.elapsed();

    let total_time = start_time.elapsed();

    // Output for parsing by benchmark scripts
    println!("PAGES={}", page_count);
    println!("GENERATION_MS={}", generation_time.as_millis());
    println!("WRITE_MS={}", write_time.as_millis());
    println!("TOTAL_MS={}", total_time.as_millis());
    println!(
        "PAGES_PER_SEC={:.2}",
        page_count as f64 / total_time.as_secs_f64()
    );
    println!("COMPLEXITY=HIGH");

    println!("\n📊 High Complexity Benchmark Results:");
    println!("  📄 Pages: {}", page_count);
    println!("  ⚡ Generation: {}ms", generation_time.as_millis());
    println!("  💾 Write: {}ms", write_time.as_millis());
    println!("  🕐 Total: {}ms", total_time.as_millis());
    println!(
        "  📈 Performance: {:.1} pages/second",
        page_count as f64 / total_time.as_secs_f64()
    );
    println!("  📋 Content: Technical diagrams + code blocks + complex tables per page");

    Ok(())
}
