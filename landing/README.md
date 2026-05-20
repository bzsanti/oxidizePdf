# landing/

Source for the public landing page at `https://bzsanti.github.io/oxidizePdf/`.

## What this is

A single `index.html` with embedded CSS — no build tooling, no Jekyll, no npm. Edit the HTML, push, GitHub Pages publishes.

## How it gets served

Two viable setups; pick one when enabling GitHub Pages.

**Option 1 — main branch + `/landing` folder.** Requires GitHub Pages to support arbitrary subfolders, which it currently does NOT (Pages only supports root or `/docs` on the source branch). So this option is OFF the table unless we move into `/docs`, which would clash with developer docs already there.

**Option 2 — dedicated `gh-pages` branch (recommended).** Treat this `landing/` folder as the source-of-truth. To deploy:

```bash
# One-time setup — create an orphan gh-pages branch with only the landing
git switch --orphan gh-pages
git rm -rf .
cp landing/index.html .
git add index.html
git commit -m "Initial landing page"
git push -u origin gh-pages

# In repo settings: Settings -> Pages -> Source = gh-pages branch, / (root)
```

To update the landing later, edit `landing/index.html` on a regular branch (and PR through develop → main as usual), then republish to `gh-pages`:

```bash
git checkout gh-pages
git checkout main -- landing/index.html
mv -f landing/index.html index.html
git add index.html && git commit -m "Update landing"
git push
```

A small GitHub Actions workflow can automate the republish if it becomes tedious — not necessary for a single-file site.

## What it claims and what it doesn't

Every claim on the landing maps to verified state in the repo:

- "99.3% parse success on 9,000+ real-world PDFs" — README/CHANGELOG.
- "3,000-4,000 pages/sec generation" — README.
- "Pure Rust, zero C dependencies" — `Cargo.toml`, no system deps required.
- "MIT licensed" — `LICENSE`.
- Python bindings on PyPI — `oxidize-pdf` 0.5.x (also ships the `oxidize-mcp` entry point for the MCP server).
- .NET bindings on NuGet — `OxidizePdf.NET` 0.9.x.
- LlamaIndex reader on PyPI — `llama-index-readers-oxidize-pdf` 0.1.x. Published per current LlamaIndex policy (independent PyPI publication under the `llama-index-readers-*` namespace; PRs adding new integration packages to `run-llama/llama_index` are auto-closed by policy). Feature-request issue #21437 open in `run-llama/llama_index` since 2026-04-21 as the canonical announcement channel.
- MCP server registered at `io.github.bzsanti/oxidize-pdf-mcp` in the MCP Registry.
- Claude Code plugin marketplace — `bzsanti/oxidize-pdf-integrations`, 6 skills + 1 agent.

Things deliberately NOT claimed because they are not yet verifiable:

- No "trusted by X" / logo wall — 0 public reverse deps on crates.io at the time of writing.
- No "production ready" badge — there's no public stability commitment doc yet.
- No specific benchmark numbers vs competitors — those need a public benchmark first.
- No mention of `langchain-oxidize-pdf` or `haystack-oxidize-pdf` — code exists in the integrations repo but neither is published to PyPI yet. Add them once published.
- No "Featured on LlamaHub" badge or claim — `llamahub.ai` only indexes plugins that live in the `run-llama/llama_index` monorepo (via its `/api/plugins` endpoint, last refreshed 2026-02-12; the source repo `run-llama/llama-hub` has been archived since 2024-03-01). The current LlamaIndex contribution policy explicitly forbids adding new integration packages to the monorepo (`CONTRIBUTING.md`: "PRs that add a new pyproject.toml will be automatically closed"), so plugins published per current policy — including ours — cannot appear on llamahub.ai. Claiming a LlamaHub presence would be inaccurate.
