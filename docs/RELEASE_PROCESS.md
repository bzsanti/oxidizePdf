# Release Process Guidelines

## ⚠️ Critical Rules

### 1. **NEVER create a release tag without green CI**
- All tests must pass on ALL platforms (Ubuntu, macOS, Windows)
- All clippy warnings must be resolved
- Code coverage must meet minimum thresholds

### 2. **Always follow the correct order**
1. ✅ Ensure CI is completely green
2. ✅ Create and merge PR to main branch
3. ✅ Create tag from main branch (not develop branches)
4. ✅ GitHub Actions handles the rest automatically

### 3. **Pre-release Checklist**
- [ ] Run `cargo test --workspace` locally
- [ ] Run `cargo clippy --workspace -- -D warnings` locally
- [ ] All CI checks pass on the target branch
- [ ] Version numbers are updated in all Cargo.toml files
- [ ] CHANGELOG.md is updated with release notes
- [ ] ISO_COMPLIANCE_REPORT.md is current

## 🔄 Correct Release Workflow

### Step 1: Prepare Release Branch
```bash
# Work on develop_santi or feature branch
git checkout develop_santi
git pull origin develop_santi

# Ensure all tests pass
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Fix any warnings/errors found
```

### Step 2: Create PR to Main
```bash
# Create PR to main
gh pr create --base main --head develop_santi \
  --title "feat: prepare release v1.1.10" \
  --body "Preparing release with improvements and fixes"

# Wait for CI to be completely green
# Get approval and merge to main
```

### Step 3: Create Release Tag from Main
```bash
# Switch to main and pull latest
git checkout main
git pull origin main

# Create and push tag (this triggers release)
git tag v1.1.10
git push origin v1.1.10
```

### Step 4: Monitor Release Process
- Watch GitHub Actions release workflow
- Verify crates.io publication
- Check that GitHub Release is created
- Approve the automated merge PR back to main

## 🚨 Emergency Procedures

### If Release Fails Mid-Process

1. **DO NOT panic or create additional tags**
2. Check the specific failure in GitHub Actions
3. If crates.io publication succeeded but PR failed, manually merge
4. If crates.io publication failed, fix the issue and retry with patch version

### If Bad Version is Published to crates.io

1. **Minor issues**: Create immediate patch release (e.g., v1.1.9 → v1.1.10)
2. **Major issues**: Consider yanking (rare, last resort)
3. Document the issue in GitHub Release notes

## 🛠️ Release Workflow Improvements (v1.1.9+)

The release workflow now includes:

- ✅ **CI Status Verification**: Checks all CI before releasing
- ✅ **Robust PR Creation**: Better handling of file conflicts
- ✅ **Clear Documentation**: Release notes explain what happened
- ✅ **Error Handling**: Better failure modes and recovery

## 📋 Version Update Process

When preparing a release, update versions in:
- `Cargo.toml` (workspace version)
- `oxidize-pdf-cli/Cargo.toml`
- `oxidize-pdf-api/Cargo.toml`
- Update `ISO_COMPLIANCE_REPORT.md` if compliance changed
- Update `CHANGELOG.md` with new features/fixes

## 🔍 Post-Release Verification

After successful release:
1. ✅ Verify `cargo search oxidize-pdf` shows new version
2. ✅ Check GitHub Release is properly formatted
3. ✅ Confirm CI is green on main branch
4. ✅ Test installation: `cargo install oxidize-pdf-cli --version X.X.X`

## ❌ What NOT to Do

- ❌ Never use `cargo-release` locally
- ❌ Never create tags from develop branches
- ❌ Never release without green CI
- ❌ Never bypass the automated workflow
- ❌ Never create manual releases on GitHub
- ❌ Never ignore failing tests "because they pass locally"

## 📞 Emergency Contacts

If the release process is broken:
- Check GitHub Actions logs first
- Review this document
- Check recent commits for any workflow changes
- If needed, manually create PR to main after successful crates.io publication

---

*Last updated: August 2025*
*This document reflects lessons learned from v1.1.9 release issues*