# Issue #498 ActualText interoperability fixture

Regenerate the adjacent PDF from the workspace root:

```sh
cargo run -p oxidize-pdf --example generate_issue_498_interop_fixture
```

Open `issue_498_actual_text_interop.pdf`, select the visible formula, copy it,
and paste into a plain-text editor. The expected text is:

```text
2⁴⁰ E = mc² Aⁿ⁺¹B
```

Record the application name, version, operating-system version, copied text,
and result for both:

- Apple Preview on macOS.
- One independent graphical PDF engine.

The automated regression test verifies the underlying `/ActualText`, MCID,
`/StructParents`, `/ParentTree`, and page-reference relationships. This manual
check verifies the viewer behavior that an internal parser cannot establish.
Viewer support is not guaranteed: a viewer that ignores `/ActualText` during
selection will copy the painted fallback glyphs despite a valid structure tree.
