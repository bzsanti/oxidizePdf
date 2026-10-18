# Effective memory and streaming options

These contracts are in unreleased `develop` after core 5.4.2 (issue #758).
No published version containing them is claimed yet. Bridges must adopt the
subsequent published release, record its version/checksum and run their own
binding acceptance tests; a git dependency is not the release handoff.

## `memory::MemoryOptions` and `LazyDocument`

`LazyDocument::configuration()` returns an immutable view of requested options,
effective values and typed `UnsupportedMemoryOption` entries. Presets retain
their original requested values. Unsupported requests remain visible rather than
being reported as active modes. Mapping and incremental stream-content support
inside this adapter remain pending capabilities under core #710. The separate
low-level `MemoryMappedFile` API is not activated by these options.

| Field | Unit and default | Actual consumer / effective behavior |
| --- | --- | --- |
| `lazy_loading` | bool, true | False parses all indexed in-use objects during construction and propagates parse errors; true defers unrelated objects. Both index page dictionaries. Eager objects remain in parser caches. Stream decoding/text extraction is not eager. |
| `cache_size` | objects, 1000 | Adapter LRU capacity. Zero disables it; capacity allocates incrementally as objects arrive. Does not control parser caches. |
| `buffer_size` | bytes, 65536 | Replaces the reader I/O buffer, preserving logical position, parsing options and existing parser caches. Zero normalizes to one; values above `isize::MAX` return `InvalidOperation` before buffer allocation. The reader's prior header/xref parsing is not replayed. Underlying reads may bypass a small buffer; this is not a maximum read size. |
| `memory_mapping` | bool, true | Unsupported by LazyDocument, effective false; a true request appears in `unsupported`. No mapped regions are opened. |
| `mmap_threshold` | bytes, 10485760 | Inactive because mapping is unsupported; requested value retained, effective `usize::MAX` sentinel. No file-size threshold enables mapping. |
| `streaming` | bool, true | Unsupported by LazyDocument, effective false; a true request appears in `unsupported`. Parser objects/stream payloads are materialized. Use `streaming::StreamingDocument` for page-at-a-time content. |

`small_file()` requests `(false, false, 0, false, 16384, usize::MAX)` and
`large_file()` requests `(true, true, 5000, true, 262144, 1048576)` in field order
`lazy_loading, memory_mapping, cache_size, streaming, buffer_size, mmap_threshold`.

`memory_stats().cached_objects` measures current adapter retention. Hit/miss
counters concern adapter requests (page requests count as misses), not OS reads.
`allocated_bytes` contains only explicitly recorded allocations; `lazy_loads`
and `mapped_regions` are not instrumented allocation measurements. None measures
RSS or promises a global memory cap. Clearing/evicting the adapter cache leaves
parser-owned and caller-owned objects intact. `MemoryManager` alone manages the
adapter cache/counters; it does not open or configure a document.

## `streaming::StreamingOptions` and `StreamingDocument`

`configuration()` exposes requested and effective values; `cached_pages()` and
`memory_usage()` expose retained adapter payloads. This type is **different from**
`memory::stream_processor::StreamingOptions`, which is also re-exported at the
crate root. Always use the namespace matching the consumer.

| Field | Unit and default | Actual consumer / effective behavior |
| --- | --- | --- |
| `buffer_size` | bytes, 262144 | Outer `BufReader` around the supplied seekable input. Zero normalizes to one; >`isize::MAX` is rejected before buffer allocation. The parser also has its own buffer, and large reads can bypass the outer buffer. |
| `page_cache_size` | pages, 3 | Upper bound on retained page payloads, oldest-first eviction. Zero disables retention. |
| `max_content_stream_size` | decoded bytes per page, 10485760 | Combined page content budget, including inserted newlines between streams. Applies to compressed input after decoding and to uncompressed content. Zero accepts empty content only. Errors do not advance the current page. |
| `progressive_hints` | bool, true | Reserved/unsupported. Effective false; true requests set `progressive_hints_unsupported`. |
| `memory_limit` | retained content+text bytes, 104857600 | Adapter cache budget; oversized pages are returned without retention. Zero prevents retention of nonempty payloads. Eviction/clear does not invalidate caller-held pages. |

`minimal_memory()` requests `(65536, 1, 1048576, false, 10485760)` and
`fast_processing()` requests `(1048576, 10, 52428800, true, 524288000)` in the
field order above. Unsupported hints are reported for all presets requesting them.

The decoded-content limit is not a process allocation limit. Parser caches,
encoded source streams, text extraction and caller-retained pages are outside
the retained-cache budget. A page is materialized before the retention decision;
`memory_limit` must not be described as peak memory. These APIs require seekable
inputs and do not provide network/progressive rendering.
