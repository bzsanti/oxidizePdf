//! Explicit, bounded recovery. Byte-only APIs never opt into this path.
use super::*;

/// Why Flate bytes cannot be treated as a verified zlib stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FlateRecoveryKind {
    /// DEFLATE ended, but the zlib wrapper/checksum was not verified.
    Unverified,
    /// Only a prefix was decoded; DEFLATE did not reach its end.
    Incomplete,
}

/// A filter whose output required recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRecovery {
    /// Zero-based position in the stream's Filter array.
    pub filter_index: usize,
    /// Integrity/completion limitation on the returned bytes.
    pub kind: FlateRecoveryKind,
    /// Error from normal zlib decoding.
    pub error: String,
}

/// Stream bytes accompanied by every Flate recovery performed on them.
/// Empty diagnostics mean no Flate recovery, not validation of the entire PDF.
#[derive(Debug)]
pub struct RecoveredStream {
    /// Decoded bytes. Recovered bytes must not be assumed trustworthy or complete.
    pub data: Vec<u8>,
    /// Nonempty whenever any filter needed recovery.
    pub diagnostics: Vec<FilterRecovery>,
}

/// Classification allowing a consumer to omit damaged Flate content explicitly
/// without swallowing resource limits or unrelated decoding errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StreamRecoveryErrorKind {
    /// No usable Flate bytes could be recovered. A consumer may report an omission.
    InvalidFlate,
    /// A Flate size or compression-ratio limit was reached. Never eligible for fallback.
    ResourceLimit,
    /// Another filter, predictor, dictionary, or configuration error.
    Other,
}

/// A failed recovery attempt, with the underlying error preserved.
#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct StreamRecoveryError {
    /// Determines whether omitting this stream is an allowed recovery action.
    pub kind: StreamRecoveryErrorKind,
    /// Original structured decoding error.
    #[source]
    pub source: ParseError,
}
impl StreamRecoveryError {
    pub(crate) fn other(source: ParseError) -> Self {
        Self {
            kind: StreamRecoveryErrorKind::Other,
            source,
        }
    }
}

/// Explicitly recover damaged Flate streams, enforcing an output bound.
///
/// Callers must inspect diagnostics; recovered bytes have no integrity guarantee.
/// Exhausted decoding returns an error, never fabricated empty bytes. The normal
/// decoding APIs remain strict regardless of ParseOptions recovery flags.
pub fn decode_stream_with_recovery(
    data: &[u8],
    dict: &PdfDictionary,
    options: &ParseOptions,
    max_bytes: usize,
) -> Result<RecoveredStream, StreamRecoveryError> {
    let max_bytes = max_bytes.min(MAX_DECOMPRESSED_SIZE);
    let filters = match dict.get("Filter") {
        Some(PdfObject::Name(n)) => vec![n.as_str()],
        Some(PdfObject::Array(a)) => {
            a.0.iter()
                .map(|o| {
                    o.as_name().map(|n| n.as_str()).ok_or_else(|| {
                        StreamRecoveryError::other(bounded_decode_syntax("Invalid filter in array"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        None => {
            return copy_with_limit(data, max_bytes)
                .map(|data| RecoveredStream {
                    data,
                    diagnostics: Vec::new(),
                })
                .map_err(StreamRecoveryError::other)
        }
        _ => {
            return Err(StreamRecoveryError::other(bounded_decode_syntax(
                "Invalid Filter type",
            )))
        }
    };
    let mut result: Option<Vec<u8>> = None;
    let mut diagnostics = Vec::new();
    for (index, name) in filters.iter().enumerate() {
        let filter = Filter::from_name(name).ok_or_else(|| {
            StreamRecoveryError::other(bounded_decode_syntax("Unknown stream filter"))
        })?;
        let input = result.as_deref().unwrap_or(data);
        let params = get_filter_params(dict.get("DecodeParms"), index);
        let decoded = if matches!(filter, Filter::FlateDecode) {
            let (decoded, recovery) = recover_flate(input, max_bytes)?;
            if let Some((kind, error)) = recovery {
                diagnostics.push(FilterRecovery {
                    filter_index: index,
                    kind,
                    error,
                });
            }
            apply_declared_predictor(decoded, params).map_err(StreamRecoveryError::other)?
        } else {
            // Reuse the bounded implementation and predictor contract of other
            // filters. A failure here must never be skipped as damaged Flate.
            let mut single = PdfDictionary::new();
            single.insert(
                "Filter".into(),
                PdfObject::Name(super::super::PdfName::new((*name).into())),
            );
            if let Some(params) = params {
                single.insert("DecodeParms".into(), PdfObject::Dictionary(params.clone()));
            }
            decode_stream_with_limit(input, &single, options, max_bytes)
                .map_err(StreamRecoveryError::other)?
        };
        result = Some(decoded);
    }
    let data = match result {
        Some(bytes) => bytes,
        None => copy_with_limit(data, max_bytes).map_err(StreamRecoveryError::other)?,
    };
    Ok(RecoveredStream { data, diagnostics })
}

#[cfg(feature = "compression")]
fn recover_flate(
    data: &[u8],
    max_bytes: usize,
) -> Result<(Vec<u8>, Option<(FlateRecoveryKind, String)>), StreamRecoveryError> {
    let first = match inflate_bounded(data, max_bytes, true) {
        Ok(bytes) => return Ok((bytes, None)),
        Err(e) => e,
    };
    if first.resource_limit {
        return Err(StreamRecoveryError {
            kind: StreamRecoveryErrorKind::ResourceLimit,
            source: first.error,
        });
    }
    let original = first.error;
    // Do not hold two expanded buffers while retrying. Raw DEFLATE starts at
    // the same payload offset for a valid zlib header, and can expose a prefix.
    drop(first.partial);
    let has_header = data.len() >= 2
        && data[0] & 15 == 8
        && data[0] >> 4 <= 7
        && u16::from_be_bytes([data[0], data[1]]) % 31 == 0;
    if has_header && data[1] & 32 != 0 {
        // Preset dictionaries are unsupported; do not interpret DICTID as data.
        return Err(StreamRecoveryError {
            kind: StreamRecoveryErrorKind::InvalidFlate,
            source: original,
        });
    }
    let raw = if has_header { &data[2..] } else { data };
    match inflate_bounded(raw, max_bytes, false) {
        Ok(bytes) => Ok((
            bytes,
            Some((FlateRecoveryKind::Unverified, original.to_string())),
        )),
        Err(e) if e.resource_limit => Err(StreamRecoveryError {
            kind: StreamRecoveryErrorKind::ResourceLimit,
            source: e.error,
        }),
        Err(e) if !e.partial.is_empty() => Ok((
            e.partial,
            Some((FlateRecoveryKind::Incomplete, original.to_string())),
        )),
        Err(_) => Err(StreamRecoveryError {
            kind: StreamRecoveryErrorKind::InvalidFlate,
            source: original,
        }),
    }
}
#[cfg(not(feature = "compression"))]
fn recover_flate(
    _data: &[u8],
    _max_bytes: usize,
) -> Result<(Vec<u8>, Option<(FlateRecoveryKind, String)>), StreamRecoveryError> {
    Err(StreamRecoveryError::other(ParseError::StreamDecodeError(
        "FlateDecode requires the compression feature".into(),
    )))
}
