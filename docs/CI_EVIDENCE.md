# CI evidence and release protocol

Issue: [#790](https://github.com/bzsanti/oxidizePdf/issues/790).

A merge SHA alone is not a reason to rerun the same full suite. Validation jobs
retain short-lived, content-addressed receipts and resolve their current inputs
before looking for a matching receipt. The job still appears on the new commit;
its summary links to the original run when validation is reused.

## Inputs and authority

`scripts/ci_evidence.py` fingerprints the complete Git tree, job/matrix identity,
resolved `rustc -Vv` and Cargo versions, Python runtime, hosted runner ImageOS,
ImageVersion and architecture, build flags, and any declared extra tool versions.
The full tree includes lockfiles, features, scripts, workflow definitions and
pinned action implementations. Compiler aliases are not identities: the resolved
compiler is. Coverage also includes llvm-cov and Poppler versions.

The API must confirm the same repository, workflow path and source tree, a
completed successful run, a successful exact job, and a successful final
`Record validation evidence` step. Fork runs, missing jobs, failed or cancelled
runs and different source trees cannot provide receipts. A PR's tested merge
tree must also equal its source head tree for later reuse; otherwise it runs
again. Checks on unrelated commits cannot authorize a release.

Receipts last at most 24 hours from both run start and artifact creation, and are
retained for two days for inspection. A hit never republishes a receipt, so reuse
cannot extend its lifetime. API errors, missing image identity, deleted artifacts,
changed inputs and exhausted pagination all fall back to actual validation.
Artifacts are receipts, not executable caches: their names carry the fingerprint;
GitHub's run/job metadata and the identical producer workflow establish provenance.
The API's artifact-name filter is documented in the
[GitHub REST reference](https://docs.github.com/en/rest/actions/artifacts#list-artifacts-for-a-repository).

Reused jobs: the three full-test platforms (including both Linux SPI profiles),
12 text-contract variants, native product checks, OmniDocBench contracts,
optimized release tests, and optional coverage. Every compilation in a reused
full-test job uses locked resolution. Examples, MSRV and SemVer keep running;
SemVer depends on registry state and branch policy. Interoperability and corpus
jobs retain live external-tool/corpus validation rather than asserting that
external inputs are fixed. T2/T3 and their differential gates also run on release PRs and main pushes;
the weekly T4/T5/T6 schedule remains unchanged.

## Normal release

1. Open the preparation PR against `develop`. Complete CI on its final tree.
   This first run executes validation and records receipts.
2. Merge into `develop`. Push checks resolve the current inputs and reuse each
   equivalent job. A PR from the same repository/source branch cancels an older
   in-progress push/PR of that branch. Forks have distinct concurrency groups.
   Pushes themselves do not cancel an in-progress validation.
3. Open the release PR against `main` (only release PRs target `main`). Equivalent
   jobs reuse evidence. `Optimized release tests` runs `cargo test --locked --all
   --release`: this profile has different optimization/overflow behavior from
   the debug suite and is deliberately preserved, now before publication.
4. Merge the reviewed release into `main`. The main push resolves current inputs
   again and normally reuses both debug and optimized receipts. Wait for all
   required workflows in `scripts/ci_required_jobs.json`, including every matrix
   member and the T2/T3 corpus job. Missing, skipped, failed, cancelled or stale required validation blocks
   publication. No branch protections/rulesets were configured at inventory time;
   the explicit release policy does not assume they exist or modify them.
5. Push the matching version tag. Release runs only on version tags, with serial
   non-cancelling concurrency per tag. It verifies a merged release PR to main,
   main ancestry and manifest/tag equality. The gate enumerates complete job lists
   for successful current-main push runs, with a 30-minute bounded wait. The tag
   checks the optimized receipt again after resolving its own compiler/image;
   missing or changed inputs run the optimized suite before publishing.
6. Recheck the candidate CI after build/package validation, immediately before
   publication. Failed or pending scheduled/manual runs of required workflows on
   the same main commit also block publication, even when push runs are green.
   Resolve and rerun the failed execution successfully, or validate a corrected
   commit. Keep locked publication and registry checks.
   An exact-version 404 permits publication; other lookup errors block it. Verify
   the registry version, non-yanked state and SHA256 against the local `.crate`
   before creating the GitHub Release. Existing versions must match too.

Coverage now runs as an optional job within CI on the actual tested commit;
there is no separate `workflow_run` chain launching another full suite on merges.
Wait for the source CI run to complete if you want its receipts reused. During
rollout, the old default-branch coverage workflow can still fire until main
contains this change. No default-branch settings are modified by this PR.

## Fallback and diagnosis

Read `Find equivalent validation` and the job summary. A miss is a normal path;
there is no unconditional skip option. A rerun of the same workflow run does not
reuse its own artifacts. Re-run a failed/missing **main push run** before tagging;
manual CI dispatch can exercise a branch (including the optimized profile), but
is not a substitute for the release gate's current-main push provenance. Never
move an already published tag to work around a failure. Workflow/toolchain,
source or lockfile changes require new evidence automatically.

Update the required-job policy when changing mandatory workflow coverage. Keep
optional coverage out of the required-job matrix; never ignore a known failed
scheduled run of a required workflow on the release candidate.
Run `python3 -m unittest discover -s scripts -p 'test_ci_*.py' -v` and actionlint
when changing this protocol. Test absent evidence, platform omissions, pagination,
changed inputs, expiry, forks, cancellation and registry/tag identity.

## Cost evidence

The 5.5.0 inventory covers 26 runs at preparation head `4cdd5f88`, develop
`3a622522`, and main `55284b48`: **591.01 raw runner minutes** (not wall-clock
latency or billing multipliers). It excludes successful historical coverage runs
whose workflow_run API head SHA points at the old default branch.

Measured steps now eligible for reuse in the three repeated rounds total
**288.36 minutes**, projecting **302.65 minutes plus evidence-lookup overhead**
for the same selected runs with equivalent inputs. This is a counterfactual,
not a measured new production release. It excludes additional savings from
concurrency and coverage deduplication and retains the optimized test cost once.
Runner/toolchain changes or missing evidence reduce the saving by design.
Actual GitHub reuse and timing must be recorded separately during validation.
