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
//! - `generic_region` - Generic region decoder (Section 6.2)
//! - `symbol_dict` - Symbol dictionary decoder (Section 6.5)
//! - `text_region` - Text region decoder (Section 6.4)
//! - `page_buffer` - Page buffer manager (Section 6.3)
//! - `halftone_region` - Halftone/pattern dictionary decoder (Sections 6.6, 6.7)
//! - `jbig2` - Main decoder and segment parsing

pub mod bitstream;
pub mod ccitt;
pub mod dct;
pub mod generic_region;
pub mod halftone_region;
pub mod huffman;
pub mod jbig2;
pub mod mq_coder;
pub mod page_buffer;
pub mod symbol_dict;
pub mod text_region;

pub use ccitt::decode_ccitt;
pub use dct::{decode_dct, parse_jpeg_info, JpegColorSpace, JpegInfo};
pub use jbig2::decode_jbig2;
