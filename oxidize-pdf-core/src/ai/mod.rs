//! AI/ML integration utilities for PDF processing
//!
//! This module provides tools for integrating PDF documents into AI/ML pipelines,
//! including document chunking for RAG (Retrieval Augmented Generation), LLM-optimized
//! formats, and vector store preparation.

pub mod chunking;

pub use chunking::{ChunkMetadata, ChunkPosition, DocumentChunk, DocumentChunker};
