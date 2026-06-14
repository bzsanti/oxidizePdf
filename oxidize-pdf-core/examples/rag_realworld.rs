//! Real-world RAG ingestion example
//!
//! Downloads five real government and academic PDFs, runs the default
//! `rag_chunks()` pipeline on each, and writes RAG-ready JSONL to `./out/`.
//!
//! Run with:
//!   cargo run --example rag_realworld
//!
//! Output:
//!   - ./corpus_cache/<sha1>.pdf  (downloaded PDFs, kept across runs)
//!   - ./out/<slug>.jsonl         (one chunk per line, RAG-ready)
//!
//! Exit code: 0 if every document succeeded; N if N documents failed;
//! 2 if a fatal error occurred (filesystem, etc.).

use std::fs;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::RagChunk;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct CorpusEntry {
    slug: &'static str,
    name: &'static str,
    url: &'static str,
    country: &'static str,
    language: &'static str,
}

const CORPUS: &[CorpusEntry] = &[
    CorpusEntry {
        slug: "ens",
        name: "BOE Real Decreto 311/2022 (Esquema Nacional de Seguridad)",
        url: "https://www.boe.es/boe/dias/2022/05/04/pdfs/BOE-A-2022-7191.pdf",
        country: "ES",
        language: "es",
    },
    CorpusEntry {
        slug: "boe-sumario",
        name: "BOE sumario diario (2025-01-15)",
        url: "https://www.boe.es/boe/dias/2025/01/15/pdfs/BOE-S-2025-13.pdf",
        country: "ES",
        language: "es",
    },
    CorpusEntry {
        slug: "higgs",
        name: "ATLAS Collaboration — Higgs boson observation (Phys. Lett. B 716, 2012)",
        url: "https://arxiv.org/pdf/1207.7214",
        country: "CERN",
        language: "en",
    },
    CorpusEntry {
        slug: "bsi-tr-02102",
        name: "BSI TR-02102 — Kryptographische Verfahren (German master)",
        url: "https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR02102/BSI-TR-02102.pdf?__blob=publicationFile&v=15",
        country: "DE",
        language: "de",
    },
    CorpusEntry {
        slug: "ncsc-caf",
        name: "NCSC Cyber Assessment Framework v4.0",
        url: "https://www.ncsc.gov.uk/sites/default/files/documents/NCSC-Cyber-Assessment-Framework-4.0.pdf",
        country: "UK",
        language: "en",
    },
];

const CACHE_DIR: &str = "corpus_cache";
#[allow(dead_code)]
const OUT_DIR: &str = "out";
const DOWNLOAD_TIMEOUT_SECS: u64 = 30;

fn cache_path(url: &str) -> PathBuf {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(url.as_bytes());
    let digest = h.finalize();
    let hex: String = digest
        .iter()
        .take(8)
        .map(|b| format!("{:02x}", b))
        .collect();
    PathBuf::from(CACHE_DIR).join(format!("{}.pdf", hex))
}

#[derive(Debug)]
enum FetchError {
    Http(String),
    Io(std::io::Error),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Http(msg) => write!(f, "http error: {}", msg),
            FetchError::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl From<std::io::Error> for FetchError {
    fn from(e: std::io::Error) -> Self {
        FetchError::Io(e)
    }
}

/// Returns Ok(path) on success. If the file is already cached, no HTTP request
/// is made. Network errors and non-2xx responses become `FetchError::Http`.
fn ensure_local_copy(entry: &CorpusEntry) -> Result<PathBuf, FetchError> {
    fs::create_dir_all(CACHE_DIR)?;
    let path = cache_path(entry.url);
    if path.exists() {
        return Ok(path);
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build();
    let resp = agent
        .get(entry.url)
        .call()
        .map_err(|e| FetchError::Http(e.to_string()))?;
    let status = resp.status();
    if !(200..300).contains(&status) {
        return Err(FetchError::Http(format!("status {}", status)));
    }
    let mut reader = resp.into_reader();
    let tmp = path.with_extension("pdf.partial");
    {
        let mut out = fs::File::create(&tmp)?;
        std::io::copy(&mut reader, &mut out)?;
        out.flush()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(path)
}

#[derive(Debug)]
enum RunError {
    Fetch(FetchError),
    Parse(String),
    Empty,
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Fetch(e) => write!(f, "{}", e),
            RunError::Parse(s) => write!(f, "parse error: {}", s),
            RunError::Empty => write!(f, "produced 0 valid chunks"),
        }
    }
}

impl From<FetchError> for RunError {
    fn from(e: FetchError) -> Self {
        RunError::Fetch(e)
    }
}

struct DocStats {
    chunks: usize,
    avg_tokens: usize,
    oversized: usize,
    headings: usize,
}

fn run_one(entry: &CorpusEntry) -> Result<(Vec<RagChunk>, DocStats), RunError> {
    let path = ensure_local_copy(entry)?;
    let reader = PdfReader::open(&path).map_err(|e| RunError::Parse(e.to_string()))?;
    let doc = PdfDocument::new(reader);
    let chunks = doc
        .rag_chunks()
        .map_err(|e| RunError::Parse(e.to_string()))?;
    let non_empty: Vec<RagChunk> = chunks
        .into_iter()
        .filter(|c| !c.text.trim().is_empty())
        .collect();
    if non_empty.is_empty() {
        return Err(RunError::Empty);
    }
    let total_tokens: usize = non_empty.iter().map(|c| c.token_estimate).sum();
    let stats = DocStats {
        chunks: non_empty.len(),
        avg_tokens: total_tokens / non_empty.len(),
        oversized: non_empty.iter().filter(|c| c.is_oversized).count(),
        headings: non_empty
            .iter()
            .filter(|c| c.heading_context.is_some())
            .count(),
    };
    Ok((non_empty, stats))
}

/// Serialize a single chunk to the canonical JSONL line shape.
/// Pub so the integration test can call it; lives in the example for showcase clarity.
pub fn jsonl_line(
    entry_slug: &str,
    entry_name: &str,
    entry_country: &str,
    entry_language: &str,
    entry_url: &str,
    chunk: &RagChunk,
) -> String {
    let m = &chunk.metadata;
    // Citation anchor: where in the source PDF this chunk lives (the RAG
    // coordinate-fidelity differentiator).
    let page_regions: Vec<_> = m
        .page_regions
        .iter()
        .map(|r| {
            json!({
                "page": r.page,
                "x": r.bbox.x,
                "y": r.bbox.y,
                "width": r.bbox.width,
                "height": r.bbox.height,
            })
        })
        .collect();
    let value = json!({
        "id": format!("{}-{:04}", entry_slug, chunk.chunk_index),
        "text": chunk.text,
        "metadata": {
            "source_url": entry_url,
            "document_name": entry_name,
            "country": entry_country,
            "language": entry_language,
            "page_numbers": chunk.page_numbers,
            "heading_context": chunk.heading_context,
            "heading_path": m.heading_path,
            "element_types": chunk.element_types,
            "token_estimate": chunk.token_estimate,
            "is_oversized": chunk.is_oversized,
            "chunk_id": m.chunk_id,
            "prev_chunk_id": m.prev_chunk_id,
            "next_chunk_id": m.next_chunk_id,
            "page_span": m.page_span,
            "page_regions": page_regions,
            "char_count": m.char_count,
            "word_count": m.word_count,
            "content_types": {
                "has_table": m.content_types.has_table,
                "has_list": m.content_types.has_list,
                "has_code": m.content_types.has_code,
                "heading_only": m.content_types.heading_only,
            },
            "table_rows": m.table_rows,
            "table_cols": m.table_cols,
        }
    });
    value.to_string()
}

fn write_jsonl(entry: &CorpusEntry, chunks: &[RagChunk]) -> std::io::Result<PathBuf> {
    fs::create_dir_all(OUT_DIR)?;
    let path = PathBuf::from(OUT_DIR).join(format!("{}.jsonl", entry.slug));
    let file = fs::File::create(&path)?;
    let mut w = BufWriter::new(file);
    for chunk in chunks {
        let line = jsonl_line(
            entry.slug,
            entry.name,
            entry.country,
            entry.language,
            entry.url,
            chunk,
        );
        writeln!(w, "{}", line)?;
    }
    w.flush()?;
    Ok(path)
}

fn main() -> std::process::ExitCode {
    let mut failed = 0usize;
    let mut total_chunks = 0usize;

    for entry in CORPUS {
        match run_one(entry) {
            Ok((chunks, stats)) => {
                total_chunks += stats.chunks;
                match write_jsonl(entry, &chunks) {
                    Ok(out_path) => {
                        eprintln!(
                            "[ok]   {:<13} → {} chunks   ~{} tok/avg   {} oversized   {} headings   {}",
                            entry.slug, stats.chunks, stats.avg_tokens, stats.oversized, stats.headings,
                            out_path.display()
                        );
                    }
                    Err(e) => {
                        failed += 1;
                        eprintln!("[fail] {:<13} → io error writing jsonl: {}", entry.slug, e);
                    }
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("[fail] {:<13} → {}", entry.slug, e);
            }
        }
    }

    let ok = CORPUS.len() - failed;
    if failed == 0 {
        eprintln!(
            "\n{}/{} documents processed successfully · {} total chunks",
            ok,
            CORPUS.len(),
            total_chunks
        );
        std::process::ExitCode::SUCCESS
    } else {
        eprintln!(
            "\n{}/{} documents processed ({} failed) · exit {}",
            ok,
            CORPUS.len(),
            failed,
            failed
        );
        std::process::ExitCode::from(failed.min(255) as u8)
    }
}
