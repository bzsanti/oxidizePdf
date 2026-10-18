# Issue #722 structural controls

`issue_722_tiny.pdf` is the exact 335-byte, one-page, hand-assembled PDF
published in issue #722. It was decoded from that issue's base64, independently
of the oxidize-pdf writer. `qpdf --check` accepts it.

`issue_722_object_stream.pdf` was produced with qpdf 11.9.0:

```
qpdf --object-streams=generate issue_722_tiny.pdf issue_722_object_stream.pdf
qpdf --check issue_722_object_stream.pdf
```

It provides a positive control for xref streams and compressed structural objects.
These files do not claim PDF/A or complete ISO conformance. Negative controls
mutate same-length references/types/counts in the tiny file, preserving xref offsets;
large invalid controls add padding without adding any structure.

`issue_722_nested.pdf` is hand-assembled with two leaf pages, one nested under
an intermediate Pages node. Offsets were calculated from the emitted bytes;
qpdf accepts it. `issue_722_missing_pages.pdf` is independently assembled with
a catalog and leaf referring to absent object 113. qpdf returns exit 3 with null-object warnings. It exercises
the general reader's existing synthetic recovery (#720), which verification
must not treat as evidence. Neither fixture uses the library writer.
