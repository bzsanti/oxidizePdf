# PDF MacRoman regression reference

Table fixed before the correction, from PDFBox 3.0.5 commit
804cc824f1a19bcce85a3d0d60a8f10e98188e52 and Adobe AGL commit
4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6; cross-checked with ISO 32000-1
Annex D.2. Copyright Apache Software Foundation and Adobe; licenses adjacent.
Table SHA-256: 4949d9ceb741e1b8c5024a00195c1271f8338fb9bb5cdd1d99cefb06d638e4da.
Columns: byte, glyph name, Unicode scalar. Undefined entries are marked '-'.
Tests require U+FFFD for undefined codes as explicit extraction recovery policy.
The table is not generated from the implementation. Mac OS Roman differs at
0xDB and includes symbols unassigned in the PDF encoding.
