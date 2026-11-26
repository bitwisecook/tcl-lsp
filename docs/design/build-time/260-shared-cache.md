# A compiler cache shared across worktrees

Status: plan item 260, a build-time lever beside item 90. It builds on the
baseline from [item 259](259-worktree-cost.md), which measures the per-worktree
cost this item shares. Item 299's build-script changes
([levers](296-300-build-levers.md#299-build-scripts)) remove one source of
cache misses. Item 297's profile choice decides how many variants the cache
holds.

## Problem

Every worktree compiles the whole dependency stack into its own target
directory. That is the rule, and it is right: AGENTS.md forbids a shared
`CARGO_TARGET_DIR`. Cargo gives a workspace member the same unit hash in every
checkout, so two worktrees building at once race on the same outputs
([KCS](../../kcs/kcs-issue-parallel-worktree-builds-serve-stale-artefacts.md)).

What that costs today (this checkout, 26 November 2026):

| Part of `target/debug` | Size |
|---|---|
| `build/` for the 389 third-party packages (name and version) the workspace resolves | 3.2 GiB |
| `build/` for the 57 workspace packages | 18.4 GiB (`tcl-compiler` 5.6, `tcl-registry` 2.8, `tcl-runtime` 2.1, `xtask` 2.1, `tcl-lsp-core` 1.7, `tcl-vm` 1.3) |
| `incremental/` | 11 GiB |

Those figures are for a target directory that has grown over many sessions.
Item 259 gives the cold, single-session figures. The shape is the point:

- **Third-party crates are about a tenth of the disk.** They are identical in
  every worktree and are the safe part to share.
- **Workspace crates and their incremental state are the rest.** The edited
  crates among them cannot be shared, because each worktree's sources differ.
  The unedited ones could be shared in principle. In practice the test profile
  compiles them incrementally, and incremental output is not cacheable (below).

### What the installed sccache does

CI installs sccache **v0.18.0** through `mozilla-actions/sccache-action@v0.0.11`
(`.github/workflows/ci.yml:613–615`, also at 1427, 1796 and 2441). It sets
`RUSTC_WRAPPER=sccache` only when setup succeeded, and `CARGO_INCREMENTAL: 0`
globally (`ci.yml:58`). It is not installed in agent containers. The v0.18.0
`docs/Rust.md` says:

- "`rustc`'s incremental compilation needs to be disabled";
- "Crates that invoke the system linker cannot be cached. Examples are `bin`,
  `dylib`, `cdylib`, and `proc-macro` crates";
- `SCCACHE_BASEDIRS` is documented in v0.18.0's `Configuration.md`, but only
  for C and C++ (preprocessed source and compiler arguments). Nothing says it
  applies to rustc.

sccache's `main` branch documents more: Rust basedirs that also strip
the working directory, the `CARGO_*` variables and the values read with `env!`,
`OUT_DIR` included. It also documents `SCCACHE_ALLOW_INCREMENTAL=cache` and
`SCCACHE_BASEDIRS_VERBATIM_ENV_DEPS`. GitHub search on 26 November 2026 lists
the related pull requests as open, for example "rust: normalize cache keys with
SCCACHE_BASEDIRS" (#2813) and "rust: key crates alike in every cargo target
directory" (#3071). Treat these as unreleased until a tagged release's docs
carry them.

So, with v0.18.0 across two worktrees of this repository:

| Unit | Shared? | Why |
|---|---|---|
| third-party library, no build script | likely | sources sit in the shared `CARGO_HOME`, and its `CARGO_*` values are the same in every worktree |
| third-party library whose source `include!`s from `OUT_DIR` | likely not | `OUT_DIR` is under each worktree's target directory and is read with `env!` |
| proc-macro, build script binary, any `bin` | never | they invoke the linker (25 proc-macro packages in the workspace graph) |
| workspace crate, test profile | never | incremental (`CARGO_PROFILE_TEST_INCREMENTAL=true`, `agent-build-env.sh:112`) |
| workspace crate, dev profile (check, clippy, `cargo xtask`) | no | not incremental (`agent-build-env.sh:113`), but `CARGO_MANIFEST_DIR` is an absolute, per-worktree `CARGO_*` value |

The CI comment at `ci.yml:1417–1419` still says "the pinned v0.17 Rust
backend"; the pin is v0.18.0 (see Corrections).

## Design

### The cache

One sccache local disk cache per machine, used by every agent worktree:

- `SCCACHE_DIR=$HOME/.cache/sccache`;
- `SCCACHE_CACHE_SIZE=20G`;
- `RUSTC_WRAPPER=sccache`.

`agent-build-env.sh` sets these only when `sccache --version` succeeds, as CI
does. Correctness never depends on it.

It shares what v0.18.0 can share: third-party libraries. It does not change the
workspace profile settings, so nothing in the fingerprint changes. An agent
with and without the cache builds the same units, which keeps AGENTS.md's
rule against differing environments intact.

### What it cannot cache, and the target

The edited crate's cone is rebuilt in each worktree, by design: its sources
differ. The unedited workspace crates below the edit are rebuilt too, because
they are incremental. Two terms make the target precise:

- for an edit *e*, **C(e)** is the edited crate plus every workspace crate
  above it in the measured build;
- **B(e)** is the rest of that build.

Targets, measured by item 259's script with a warm cache:

1. **After an edit, the rebuild is exactly C(e).** Rebuilt units ÷ |C(e)| = 1.0.
   There are no extra variants (items 296, 297) and no build-script reruns
   (item 299).
2. **Third-party hit rate in a fresh worktree is at least 90%** of cacheable
   units. Proc macros and build-script binaries are counted separately as
   "not cacheable".
3. **The rebuilt share of the cone is |C(e)| ÷ |B(e) ∪ C(e)|, at or below 25%
   for every edit after the split.** Today it is far above that for the
   registry edit, where C(e) is most of the stack. Item 259 records the actual
   values; this target is checked when item 264 and step 12 (item 257) land.

### Alternatives considered

- **A shared target directory for third-party crates only.** Cargo has one
  target (or build) directory per invocation and no way to place some units
  elsewhere. Pointing two worktrees at one directory shares the workspace units
  as well, which is the KCS failure. Rejected.
- **Cargo's `build-dir`** (`build.build-dir`, `CARGO_BUILD_BUILD_DIR`;
  templates `{workspace-root}`, `{cargo-cache-home}`, `{workspace-path-hash}`).
  It moves intermediates out of the checkout, but
  `{workspace-path-hash}` keeps one directory per worktree, so nothing is
  shared. It is useful for pruning, since all build state then sits under one
  root, and it costs nothing. Adopt it only if the pruning script needs it.
  Cargo's `-Z fine-grain-locking`, `-Z build-dir-new-layout` and `-Z gc` are
  nightly-only on cargo 1.100.
- **`--remap-path-prefix`.** It makes outputs independent of the checkout path.
  It does not make the cache key independent, because the flag's own value
  names the worktree. Setting it means setting `RUSTFLAGS`, which changes every
  unit's fingerprint and which AGENTS.md forbids outside `agent-build-env.sh`.
  Cargo's `trim-paths` profile setting would be the right form, and it is
  unstable. Rejected for now.
- **sccache with Rust basedirs and `SCCACHE_ALLOW_INCREMENTAL=cache`**, once
  released. This could share unedited workspace crates even in the test
  profile. It needs `SCCACHE_BASEDIRS_VERBATIM_ENV_DEPS=CARGO_MANIFEST_DIR,CARGO_MANIFEST_PATH`.
  Without it, a crate that bakes `env!("CARGO_MANIFEST_DIR")` into its output
  can take another worktree's path from the cache. 138 files in `rust/` use
  that macro, 18 outside tests, examples and benches, for example
  `tcl-spectcl/src/bundled.rs`, `tcl-spectcl/src/discovery.rs` and
  `tcl-vm/src/cmd_info/native_jim.rs`. A hit like that is the KCS failure in a
  new form. Revisit when a tagged release documents it.
- **Turning incremental off for crates a worktree does not edit** (a
  `[profile.test.package.<name>] incremental = false` override; the Cargo book
  lists only `panic`, `lto` and `rpath` as non-overridable). Those crates would
  then be cacheable, but with v0.18.0 they still miss across worktrees on
  `CARGO_MANIFEST_DIR`. Re-evaluate together with the basedirs release.

### Pruning policy

`scripts/dev/prune-targets.py` runs at session start (after `agent-build-env.sh`)
and on demand. Its rules:

| What | When | Rule |
|---|---|---|
| a worktree's whole `target/` | the worktree is removed | `scripts/dev/prune-targets.py --removed <path>`. The script also sweeps `target/` directories under paths that `git worktree list --porcelain` no longer lists |
| unit directories `target/debug/build/<pkg>/<hash>/` | newest file older than 3 days | delete. This dir's `tcl-compiler` holds 110 unit directories and `tcl-vm` 85, far more than the live variants |
| `target/debug/incremental/<crate>-<hash>/` | no unit directory still names that crate hash, or older than 3 days | delete |
| test executables under each unit directory | not the newest per test target | delete |
| a target directory used by a build that hit ENOSPC | always | delete whole (AGENTS.md: results after ENOSPC are suspect) |
| the sccache directory | over `SCCACHE_CACHE_SIZE` | sccache's own LRU |

The script prints what it would delete with `--dry-run`, never touches a
directory while a `cargo` process holds `target/debug/.cargo-lock`, and has
unit tests on a fake target tree (`scripts/dev/test_prune_targets.py`).

### Open questions

- What is the measured cross-worktree hit rate for third-party crates with
  v0.18.0? Item 259's script run in a second worktree with a warm cache answers
  it. If it is below 90%, the misses' crate names show whether `OUT_DIR` is
  the cause.
- Does the cache save enough to justify a wrapper process per rustc call? Compare
  cold-build wall time with and without it, on the same machine, back to back.
- Does `CARGO_PROFILE_TEST_INCREMENTAL` affect the unit hash or only the
  fingerprint? If only the fingerprint, a later per-package override costs one
  rebuild, not a second variant. The unit directories before and after flipping
  it show which.

## Work list

1. Install sccache v0.18.0 in the session-start hook, the same pin as CI.
2. `agent-build-env.sh`: the three variables above, guarded by `sccache --version`.
   Its `--check` mode prints them.
3. `scripts/dev/prune-targets.py` and `scripts/dev/test_prune_targets.py`.
4. Correct the stale v0.17 comment in `ci.yml`.
5. Measure: three worktrees, each on a different isolated edit from item 259's
   table, warm cache. Record the total disk, the hit rate and target 1's ratio
   in [build-test-cost](../../work/build-test-cost/current-state.md).
6. Record the configuration in AGENTS.md's build environment paragraph.

## Verification

Done when (item 260's own):

- three worktrees on different isolated edits, with a warm shared cache, use less
  total disk than one worktree does today (target, incremental and cache
  together);
- the rebuild after an edit is exactly C(e), and the rebuilt share is at or below
  its target where the split has landed;
- the pruning policy runs from `scripts/dev/`, and its tests pass;
- the configuration is in `agent-build-env.sh` and AGENTS.md.

Measurements: per worktree, the cone and rebuild count from item 259, cache hits
and misses (`sccache --show-stats`), and disk before and after pruning.

## Risks

- **Three worktrees may not beat one today on disk alone.** Third-party
  sharing saves about 3 GiB a worktree, and the bulk is workspace state. The
  done-when may only pass after items 296 and 297 cut variants and step 0 cuts
  dev cones. The measurement says which; the item stays open until then.
- **A wrong hit is a wrong answer.** Hold the basedirs features back until
  released, and never enable them without the verbatim `CARGO_MANIFEST_DIR` rule.
- **Pruning a live build.** Pruning takes cargo's lock and skips locked targets.

## Corrections to the overview

- `.github/workflows/ci.yml:1417–1419` says the pinned sccache is v0.17; every
  pin is v0.18.0.
- The overview's "sccache does not cache incrementally compiled crates" holds for
  v0.18.0. Unreleased sccache documents opt-in incremental caching, so the
  limit may lift. Path normalisation, not incremental, is then what decides
  sharing.
