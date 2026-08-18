# Issue #498 diagnostic matrix

Select and copy only the value in the right column of each row.

| Visible value | Expected copied value | Mechanism |
|---|---|---|
| `VISUAL_A` | `INLINE_ASCII` | Marked-content `/ActualText` only |
| `VISUAL_B` | `STRUCT_ASCII` | Structure-element `/ActualText` only |
| `VISUAL_C` | `2⁴⁰ E = mc² Aⁿ⁺¹B` | Both locations, Unicode |

Interpretation:

- If A works but B does not, the engine only reads marked-content properties.
- If B works but A does not, the engine only reads the structure tree.
- If A and B work but C does not, Unicode or font-coverage handling is at fault.
- If all three copy their visible `VISUAL_*` value, the engine ignores
  `/ActualText` during selection/copy.

Regenerate with:

```sh
cargo run -p oxidize-pdf --example generate_issue_498_diagnostic_fixture
```
