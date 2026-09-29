# Predictor decoding

FlateDecode and LZWDecode apply predictor parameters after decompression.
Successful decompression does not imply successful predictor decoding: invalid
rows, row filter bytes or parameters return `ParseError::StreamDecodeError`
from both `decode_stream` and `decode_stream_with_limit`. This applies to strict,
tolerant and skip-error parsing options. These byte-only APIs do not silently
recover predictor failures or return untransformed samples as decoded output.

Supported predictor values are 1 (identity, also the default) and 10–15 (PNG).
TIFF predictor 2 is not implemented and is now an explicit error, as are other
unknown values. Predictor parameters are interpreted only for Flate and LZW.

For PNG predictors, Columns and Colors must be positive integers representable
on the target platform (both default to 1). A null dictionary value uses the
corresponding default, just as an absent entry does. BitsPerComponent must be 1, 2, 4, 8
or 16 (default 8). Pixel and row arithmetic is checked before predictor output
allocation; each row must contain one filter byte plus its complete sample
payload. Filter bytes 0–4 select None, Sub, Up, Average and Paeth. Packed samples
and multibyte components retain their byte-level PNG semantics. Empty decoded
input remains valid with valid parameters. Identity does not interpret PNG
geometry parameters.

Absent/null DecodeParms and null entries in its array mean no parameters.
Use the document-aware
`PdfDocument::decode_stream` or `decode_stream_with_limit` to resolve indirect
DecodeParms, including references within a parameter array, as supported by
#514. The low-level byte APIs cannot resolve document references themselves; their
existing parameter-container handling is unchanged. Document-aware APIs reject
malformed, missing or circular parameter references.

Existing decompression limits remain in force, including the bounded API's
limit on intermediate filter output before predictor row bytes are removed.
Predictor processing does not override a decompression limit. Flate recovery
itself is tracked separately under #637; this change does not claim to fix it.

Compatibility: callers that relied on malformed or unsupported predictors
returning raw samples will now receive errors. They must handle those errors
explicitly. No TIFF implementation or inferred predictor recovery is introduced.

Null handling follows the [Adobe dictionary object contract](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdflsdk/apireference/COS_Layer/CosDict.html).
