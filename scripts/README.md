# oxidize-pdf Scripts

This directory contains automation scripts for the oxidize-pdf project.

## Available Scripts

### verify-examples.sh
Compile gate for all examples plus an output gate for the RAG showcase.

```bash
./scripts/verify-examples.sh          # compile all examples (--all-features)
./scripts/verify-examples.sh --run    # also run rag_realworld if corpus_cache present
```

`cargo build --all` (CI) builds workspace default targets only — examples are
not default targets, so a broken example slips through without this. Wired into
CI (`examples` job) and `release.sh`.

### release.sh
Automated release script using cargo-release.

```bash
./scripts/release.sh [patch|minor|major]
```

Features:
- Runs tests and clippy checks
- Updates version numbers
- Updates CHANGELOG
- Creates git tag
- Pushes to GitHub
- Publishes to crates.io

### bump-version.sh
Manual version bump script (for when you need more control).

```bash
./scripts/bump-version.sh [patch|minor|major]
```

### commit-helper.sh
Interactive helper for creating conventional commits.

```bash
./scripts/commit-helper.sh
```

Helps create commits following the conventional commit format:
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `chore:` Maintenance tasks
- etc.

## Release Process

1. **Development**: Work on features/fixes in `development` branch
2. **Prepare Release**: 
   ```bash
   ./scripts/release.sh patch  # or minor/major
   ```
3. **Create PR**: Create pull request from `development` to `main`
4. **Merge & Tag**: After merging, the GitHub Action will:
   - Run tests
   - Build binaries
   - Publish to crates.io
   - Create GitHub release

## Versioning Strategy

We follow [Semantic Versioning](https://semver.org/):

- **PATCH** (0.0.x): Bug fixes, minor updates
- **MINOR** (0.x.0): New features, backward compatible
- **MAJOR** (x.0.0): Breaking changes

## Conventional Commits

Commit messages should follow the format:
```
type(scope): description

[optional body]

[optional footer(s)]
```

Types that affect versioning:
- `feat:` → Minor version bump
- `fix:` → Patch version bump
- `feat!:` or `BREAKING CHANGE:` → Major version bump