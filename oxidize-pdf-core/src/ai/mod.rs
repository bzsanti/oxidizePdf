//! AI/ML integration utilities for PDF processing
//!
//! This module provides tools for integrating PDF documents into AI/ML pipelines,
//! including document chunking for RAG (Retrieval Augmented Generation), LLM-optimized
//! formats, and vector store preparation.

pub mod chunking;
pub mod formats;

pub use chunking::{ChunkMetadata, ChunkPosition, DocumentChunk, DocumentChunker};
pub use formats::{ContextualFormat, DocumentMetadata, MarkdownExporter, MarkdownOptions};

#[cfg(feature = "semantic")]
pub use formats::{JsonExporter, JsonOptions};
