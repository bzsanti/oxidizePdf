use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub pages: Option<Vec<PageSpec>>,
    pub page_template: Option<PageTemplate>,
    pub page_counts: Option<Vec<usize>>,
    pub expected_metrics: ExpectedMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSpec {
    pub page_number: u32,
    pub width: f64,
    pub height: f64,
    pub content: Vec<ContentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTemplate {
    pub width: f64,
    pub height: f64,
    pub content: Vec<ContentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "text")]
    Text {
        text: String,
        x: f64,
        y: f64,
        font: String,
        font_size: f64,
        max_width: Option<f64>,
    },
    #[serde(rename = "rectangle")]
    Rectangle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill_color: Option<String>,
        stroke_color: Option<String>,
    },
    #[serde(rename = "table")]
    Table {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rows: usize,
        columns: usize,
        data: Vec<Vec<String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedMetrics {
    pub min_generation_time_ms: Option<u64>,
    pub max_file_size_kb: Option<u64>,
    pub pages_count: Option<usize>,
    pub pages_per_second_min: Option<u64>,
    pub memory_growth_linear: Option<bool>,
    pub file_size_per_page_kb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub library_name: String,
    pub test_case: String,
    pub page_count: usize,
    pub generation_time_ms: u64,
    pub file_size_bytes: u64,
    pub memory_usage_mb: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub name: String,
    pub description: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub results: Vec<BenchmarkResult>,
    pub summary: BenchmarkSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub total_tests: usize,
    pub successful_tests: usize,
    pub failed_tests: usize,
    pub average_time_ms: f64,
    pub fastest_library: String,
    pub smallest_files_library: String,
}

pub fn load_test_case(path: &str) -> Result<TestCase, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let test_case: TestCase = serde_json::from_str(&content)?;
    Ok(test_case)
}

pub fn measure_time<F, R>(f: F) -> (Duration, R)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (duration, result)
}

pub fn get_file_size(path: &str) -> Result<u64, std::io::Error> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

pub fn format_duration(duration: Duration) -> String {
    format!("{:.2}ms", duration.as_secs_f64() * 1000.0)
}

pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    
    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_test_case() {
        // This test would load actual test case files
        // For now, just test the basic structure
        let test_case = TestCase {
            name: "test".to_string(),
            description: "Test case".to_string(),
            pages: None,
            page_template: None,
            page_counts: Some(vec![1, 10, 100]),
            expected_metrics: ExpectedMetrics {
                min_generation_time_ms: Some(1),
                max_file_size_kb: Some(100),
                pages_count: None,
                pages_per_second_min: Some(50),
                memory_growth_linear: Some(true),
                file_size_per_page_kb: Some(2),
            },
        };
        
        assert_eq!(test_case.name, "test");
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(512), "512 bytes");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
    }
}