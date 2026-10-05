# F09 CID CFF selection fixtures

Original outlines under the repository MIT license. Generate with pinned FontTools4.60.1:

`PYTHONPATH=target/issue666-fonttools python3 tools/generate_text_cff_selection_contracts.py OUTPUT`

Eight CFF1 programs and32 PDFs cross FDSelect0/3, full/subset, private/Adobe-Japan1 collections, H/V and explicit Unicode/fallback. The manifest freezes hashes and independent expectations. Unknown private Unicode is a recovery policy; CID/GID/Font DICT and raster are checked separately. CI is deferred to the overall #666 closure by user instruction.
