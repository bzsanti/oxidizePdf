# Docs7 and Context7 maintenance

The user-facing source is `docs/site/`. It serves both Docs7 and Context7, so edits have one source of truth. These introductory pages target 5.5.5; they do not replace the full API reference or imply complete bridge parity.

## Preview

Requires Node.js 20.19 or newer. From the repository root:

```bash
npx @upstash/docs7@0.1.12 dev docs/site --host 127.0.0.1 --port 4000
```

The first run downloads the renderer. Open the URL printed by the CLI. Verify every page, navigation, code block, keyboard focus, and light/dark appearance before publishing.

## Publish Docs7

Prefer the GitHub integration: in the Context7 teamspace's Docs7 tab, add this repository, select production branch `main` and directory `docs/site`. Only publish after the documentation reaches main through the project's release process. Work PRs target `develop`.

Alternatively, authenticate with `npx @upstash/docs7@0.1.12 login --no-browser`, then publish the reviewed directory with `npx @upstash/docs7@0.1.12 deploy docs/site`. Local CLI publication can precede GitHub integration; it publishes exactly the local pages, so verify their release scope first. Keep the generated `docs/site/.docs7/project.json`: it identifies the site for subsequent deployments. Never commit credentials. Once connected to GitHub, use GitHub pushes rather than CLI deploys.

Record the actual URL returned by Docs7 after publication. Do not guess a docs7.io hostname or add a live-site badge before checking it.

## Published site and Context7 index

Published site: https://oxidize-pdf-for-rust.docs7.io
Context7 library: `/llmstxt/oxidize-pdf-for-rust_docs7_io_llms_txt`
Source: https://oxidize-pdf-for-rust.docs7.io/llms.txt

The site was published through the CLI on 2026-11-01. Its non-secret `docs/site/.docs7/project.json` is retained for subsequent deployments. Production search returns the quickstart successfully. The llms.txt source was registered with Context7's public API, and a context query returned the documented 5.5.5 installation and usage examples. This live index uses the website, not the repository parser configuration.

After changing the published guides, refresh this library in Context7 and query the changed example to confirm the new content. Preserve the library ID rather than submitting a duplicate source.

## Optional repository index

The root `context7.json` prepares a separate repository-based index. It does not control the live llms.txt index above. If adding that source later:

1. After `context7.json` and the pages reach `main`, open https://context7.com/add-library and submit https://github.com/bzsanti/oxidizePdf.
2. Check parse status and logs, resolve the returned library ID, and query an example from `quickstart.mdx`.
3. Check the result uses 5.5.5 and this binding's API, rather than another language's API.
4. Record the returned ID and verified public site URL before adding direct README links.

The parser is pinned to `main`; unfinished `develop` APIs must not appear as released functionality. Context7 indexes English documentation. Our `folders` selects only `docs/site`, while `excludeFiles` enumerates root documentation because root Markdown is included automatically. Keep this list current when adding a root Markdown/RST/TXT/notebook file. Exact filenames are required; file glob patterns are not supported. Excluding README also avoids importing its legacy sections.

Do not add source-code extensions to `includeExtensions`: code examples belong inside the selected documentation. Reports, evidence, planning notes, release notes and contributor instructions are intentionally outside the index.

## Update a release

Update the package version and examples in the pages and `context7.json` together. Execute the examples against the published package, validate both JSON configurations using their declared schemas, preview all pages, then publish. Request a Context7 refresh and verify the resulting query before claiming the new docs are indexed. Do not add an automatic refresh workflow without configuring and validating its authentication.

## Presentation

Use a documentation sidebar organized around installation, examples and limits. The orange accent derives from the core project's existing landing page; the darker primary improves contrast on light backgrounds. Use Docs7 typography and spacing for reading technical prose. No marketing cards, fabricated statistics, custom illustrations or additional animation are introduced.

## Local validation (2026-11-01)

- Both configurations pass the official JSON schemas using Python jsonschema Draft 7, including URI format checks. Navigation matches the four MDX pages; local links resolve; tracked root documentation is explicitly excluded from indexing.
- The quickstart was executed against the published package 5.5.5. Rust generated a PDF and extracted its greeting.
- Docs7 CLI 0.1.12 / renderer 0.1.63 served all four pages with HTTP 200. Chromium headless checked desktop/mobile navigation, both themes, code copying, Markdown copying and keyboard focus. No page overflow at 390px; no JavaScript page errors during the page sweep.
- The local renderer returns HTTP 404 for `/api/search?q=Install`; production returns HTTP 200 with the quickstart in its results. The preview limitation is resolved by checking the published site.
- Authentication, publication and indexing completed on 2026-11-01. Context7 returned the expected package version and language-specific code from this site's quickstart. Credentials remain outside the repository.

## Official references

- https://context7.com/docs/docs7/quickstart
- https://context7.com/docs/docs7/cli
- https://context7.com/docs/library-owners
- https://context7.com/docs/adding-libraries
