const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const api = require('./pkg-node/wasm_encryption_regression.js');
assert.equal(api.direct_iv().length, 16);
for (const [algorithm, name] of ['rc4', 'aes128', 'aes256'].entries()) {
  const pdf = api.authoring_case(algorithm);
  assert.ok(pdf.length > 100);
  if (process.env.PDF_OUTPUT_DIR) {
    fs.writeFileSync(path.join(process.env.PDF_OUTPUT_DIR, `${name}.pdf`), pdf);
  }
}
console.log('PASS: Node IV and RC4/AES128/AES256 R5 contracts');
