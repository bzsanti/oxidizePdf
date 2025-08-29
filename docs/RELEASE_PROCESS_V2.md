# Release Process v2.0

## Overview

The new release process ensures that releases **only happen after successful PR merges to main** with all CI checks passing.

## 🔒 Release Conditions

The release workflow will **only execute** when:

1. ✅ **Tag push to main branch** - A version tag (e.g., `v1.2.0`) is pushed
2. ✅ **Tag is on main branch** - The tagged commit must be on the main branch
3. ✅ **Came from merged PR** - The commit must be the result of a merged pull request
4. ✅ **All CI checks pass** - Every CI check must be green before release

## 📋 Step-by-Step Release Process

### 1. Development Phase
- Work on `develop_santi` branch
- Implement features, fix bugs, etc.
- Ensure all tests pass locally

### 2. Pre-Release Preparation
- Update version numbers in `Cargo.toml` files
- Update `CHANGELOG.md` with release notes
- Update documentation if needed
- Commit all changes to `develop_santi`

### 3. Create Pull Request
```bash
# Create PR from develop_santi to main
gh pr create --base main --title "Release v1.2.0" --body "Release changes for v1.2.0"
```

### 4. PR Review and Merge
- Wait for all CI checks to pass ✅
- Review changes
- **Merge the PR** (this is crucial!)

### 5. Tag the Release
After PR is merged to main:
```bash
# Checkout main and pull latest
git checkout main
git pull origin main

# Tag the release
git tag v1.2.0
git push origin v1.2.0
```

### 6. Automated Release
- GitHub Actions will automatically:
  - ✅ Verify release conditions
  - ✅ Check all CI status 
  - 📦 Build packages
  - 🚀 Publish to crates.io
  - 📝 Create GitHub Release

## 🚨 Important Notes

### What WON'T Trigger Release:
- Direct pushes to main without PR
- Tags not on main branch
- Tags on commits from non-merged PRs
- Any failing CI checks

### Emergency Releases:
For emergency releases, you can push directly to main with a tag, but this should be avoided.

### Failed Releases:
If a release fails:
1. Fix the issue in `develop_santi`
2. Create new PR to main
3. Merge PR
4. Create new tag (increment version)

## 🔍 Verification Commands

Check if a commit came from a merged PR:
```bash
# Get PR info for a commit
gh api "repos/OWNER/REPO/commits/COMMIT_SHA/pulls"

# Check PR status
gh pr view PR_NUMBER --json state
```

Check if tag is on main:
```bash
git merge-base --is-ancestor TAG_COMMIT origin/main
```

## 🛠 Workflow Configuration

The release workflow is configured in `.github/workflows/release.yml` with these key jobs:

1. **`verify-release-conditions`** - Validates all release requirements
2. **`check-ci-status`** - Ensures all CI checks are passing  
3. **`release`** - Performs the actual release process

## 📊 Benefits of New Process

- **No accidental releases** - Multiple validation layers
- **Guaranteed CI coverage** - All checks must pass
- **Proper PR workflow** - Encourages code review
- **Clear audit trail** - Every release linked to a specific PR
- **Fail-safe mechanisms** - Multiple points where invalid releases are blocked

## 🔧 Troubleshooting

### "Release blocked: Tag is not on main branch"
- Ensure you tagged the commit that was merged to main
- Check: `git log --oneline main` to see recent commits

### "Release blocked: No associated PR found"  
- The commit wasn't created by merging a PR
- Either create PR first, or push directly to main for emergency

### "Release blocked: CI checks failed"
- Fix failing tests in develop branch
- Create new PR with fixes
- Merge and tag again

---

This process ensures **high quality, safe releases** that only happen after proper review and testing.