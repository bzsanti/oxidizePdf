# Type1 inert procedure regression (#674)

Original project PFB fixture `../type1/pfb-full-intrinsic.pdf`, with
`/Unused { /Encoding StandardEncoding def } def` inserted in clear FontFile
before its actual `/Encoding`, updating Length1 and PDF stream/xref lengths.
No procedure is invoked. The executable encoding remains byte65=B, byte66=A.
Original outlines are MIT licensed as the repository.

SHA256: `df09f636276b1f83ff0e03a6f6272ae02e5827e38210b997394009c42164758f`.

Qpdf accepts the structure, FontTools reads Encoding65/66 as B,A and MuPDF
extracts BA. Poppler extracts AB; this disagreement is preserved explicitly.
Independent reader evidence: `docs/reports/2026-10-04-issue-666-qr-evidence/`.
The regression must use the executed static definition, not scan an unused body.
