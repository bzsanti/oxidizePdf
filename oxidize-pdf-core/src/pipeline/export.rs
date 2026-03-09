use crate::pipeline::Element;

/// Configuration for element-aware markdown export.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Include Header and Footer elements in output (default: false).
    pub include_headers_footers: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            include_headers_footers: false,
        }
    }
}

/// Exports a slice of [`Element`]s to Markdown format.
#[derive(Debug, Clone, Default)]
pub struct ElementMarkdownExporter {
    pub config: ExportConfig,
}

impl ElementMarkdownExporter {
    pub fn new(config: ExportConfig) -> Self {
        Self { config }
    }

    /// Export elements to a Markdown string.
    pub fn export(&self, elements: &[Element]) -> String {
        if elements.is_empty() {
            return String::new();
        }
        let mut parts: Vec<String> = Vec::new();
        for element in elements {
            if let Some(md) = self.element_to_markdown(element) {
                parts.push(md);
            }
        }
        parts.join("\n\n")
    }

    fn element_to_markdown(&self, element: &Element) -> Option<String> {
        match element {
            Element::Title(d) => Some(format!("# {}", d.text.trim())),
            Element::Paragraph(d) => Some(d.text.trim().to_string()),
            Element::ListItem(d) => Some(format!("- {}", d.text.trim())),
            Element::KeyValue(kv) => Some(format!("**{}**: {}", kv.key.trim(), kv.value.trim())),
            Element::CodeBlock(d) => Some(format!("```\n{}\n```", d.text.trim())),
            Element::Image(img) => {
                let alt = img.alt_text.as_deref().unwrap_or("");
                Some(format!("![{}]()", alt))
            }
            Element::Table(t) => Some(table_to_markdown(&t.rows)),
            Element::Header(_) | Element::Footer(_) => {
                if self.config.include_headers_footers {
                    Some(element.display_text())
                } else {
                    None
                }
            }
        }
    }
}

fn table_to_markdown(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    lines.push(format!("| {} |", rows[0].join(" | ")));
    let sep: Vec<&str> = vec!["---"; rows[0].len()];
    lines.push(format!("| {} |", sep.join(" | ")));
    for row in &rows[1..] {
        lines.push(format!("| {} |", row.join(" | ")));
    }
    lines.join("\n")
}
