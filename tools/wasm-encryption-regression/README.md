# WASM encryption regression (#711)

The native integration test and this external WASM consumer share behavioral
assertions in `oxidize-pdf-core/tests/support/issue_711_contract.rs`: actual PDF
serialization, dictionary algorithms/permissions, both passwords, wrong-password
retry and recovered plaintext. This package explicitly selects the JavaScript
entropy backends required by `wasm32-unknown-unknown`.

From the repository root:

```sh
wasm-pack build tools/wasm-encryption-regression --target web --dev --no-opt --locked
node tools/wasm-encryption-regression/browser_test.cjs
wasm-pack build tools/wasm-encryption-regression --target nodejs --out-dir pkg-node --dev --no-opt --locked
node tools/wasm-encryption-regression/node_test.cjs
```

`CHROME_BIN` selects a Chrome/Chromium executable. The runner starts an ephemeral
loopback HTTP server, uses an isolated temporary browser profile and closes both
when done. `PDF_OUTPUT_DIR` optionally captures Node-generated PDFs for qpdf.
`generate_iv()` retains its infallible signature and panics on entropy failure;
`try_generate_iv()` and the document writer return explicit errors instead.
