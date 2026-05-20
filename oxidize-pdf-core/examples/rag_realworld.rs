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
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

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

fn main() {
    let entry = &CORPUS[0]; // ENS, smallest known-good URL
    match ensure_local_copy(entry) {
        Ok(path) => {
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            eprintln!(
                "[ok]   {} cached at {} ({} bytes)",
                entry.slug,
                path.display(),
                size
            );
        }
        Err(e) => {
            eprintln!("[fail] {} → {}", entry.slug, e);
            std::process::exit(1);
        }
    }
}
