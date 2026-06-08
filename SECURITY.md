# Security Policy

## Reporting a Vulnerability

Please report security vulnerabilities **privately** — do not open a public issue.

- Use GitHub's [private vulnerability reporting](https://github.com/bzsanti/oxidizePdf/security/advisories/new), or
- Email **santiago.fernandez@belowzero.tech**.

Include a description, reproduction steps (ideally a minimal crate or input PDF), the affected version, and the impact you observed.

You can expect an initial response within 5 business days. Confirmed vulnerabilities are addressed in a patch release on crates.io, and you will be credited unless you ask to remain anonymous.

## Supported Versions

Security fixes are applied to the latest released version on [crates.io](https://crates.io/crates/oxidize-pdf). Upgrade to the latest release before reporting.

`oxidize-pdf` is the pure-Rust PDF engine that backs the Python (`oxidize-pdf`), .NET (`OxidizePdf.NET`), and WASM bindings. Vulnerabilities fixed here are propagated to the bindings in a coordinated release.

## Scope

Because this library parses untrusted PDF input, parser-level issues are in scope: out-of-bounds reads, unbounded memory/CPU on crafted input (decompression bombs, malformed cross-reference tables), and panics reachable from untrusted documents. Report these even if they are "only" a denial of service.
