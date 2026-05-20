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

use std::path::PathBuf;

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

fn main() {
    eprintln!("rag_realworld: corpus has {} documents", CORPUS.len());
    for entry in CORPUS {
        eprintln!("  - {} ({}) → {}", entry.slug, entry.country, entry.url);
    }
}
