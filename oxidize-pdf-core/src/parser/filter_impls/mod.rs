//! PDF stream filter implementations
//!
//! This module contains implementations of various PDF stream filters
//! according to ISO 32000-1:2008 Section 7.4
//!
//! ## JBIG2 Decoding Infrastructure
//!
//! The JBIG2 decoder (ITU-T T.88) is implemented across multiple submodules:
//! - `mq_coder` - MQ arithmetic entropy coder (Section 7)
//! - `bitstream` - Bitstream reader for variable-length codes (Section 6.2)
//! - `huffman` - Standard Huffman tables B.1-B.15 (Annex B)
//! - `jbig2` - Main decoder and segment parsing

pub mod bitstream;
pub mod ccitt;
pub mod dct;
pub mod huffman;
pub mod jbig2;
pub mod mq_coder;

pub use ccitt::decode_ccitt;
pub use dct::{decode_dct, parse_jpeg_info, JpegColorSpace, JpegInfo};
pub use jbig2::decode_jbig2;
