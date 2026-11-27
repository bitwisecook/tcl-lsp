# Build and test cost: current state

Updated 27 November 2026. This file is a current snapshot of the build- and
test-time work on `work`; git records the sequence. The measurements and the
ranked plan it works through are in the report
<https://claude.ai/artifact/BFfNK69TYvCcQMzTRqQLKi> (private; ask the owner
for access).

## Now-1 analysis and engine baseline (106a)

The single baseline is produced by `scripts/perf/split_baseline.py` through
`perf.yml`'s manual, non-gating `split-baseline` mode. Item 90 and item 483
read this section rather than defining another budget. Cases share the same
native release binaries, upload independently, and combine with per-case host
metadata. Missing, provisional and invalid answers contribute no percentile.

[Measured run](https://github.com/bitwisecook/tcl-lsp/actions/runs/45997018051),
27 November 2026, revision `da72a72339d86ae38bd2a3af2f2eaf01e9533d1a`: Ubuntu 26.04, Cargo `release`,
Tcl 8.6 editor workspaces and Tcl 9.0 VM scripts. Editor actions request 20
samples; startup/cold/restart request one. VM rows use 5,000 iterations and
3 fresh processes, with `TCL_LIBRARY` unset; wall times include startup.

| Case | CPU | Cores | RAM, GB |
|---|---|---:|---:|
| small | AMD EPYC 9V74 80-Core Processor | 4 | 16 |
| large | Intel(R) Xeon(R) 6975P-C | 4 | 15 |
| source | AMD EPYC 7763 64-Core Processor | 4 | 16 |
| engines | Intel(R) Xeon(R) 6975P-C | 4 | 15 |

Hardware differs between cases. Later comparisons need matching case/host
conditions; these results alone do not attribute a cross-host change to code.

| Measurement | Small (6 lines) | Large (10,000 lines) | Source (9 lines across 2 files) |
|---|---:|---:|---:|
| Startup, ms | 15.072 | 14.162 | 18.183 |
| Startup RSS, KiB | 35,212 | 36,160 | 35,164 |
| Cold workspace scan, ms | 1,553.630 | unavailable (0/1) | 1,352.624 |
| Warm definition, ms p95 | 5.136 | unavailable (not reached) | unavailable (0/20) |
| References, ms p95 | 32.385 | unavailable (not reached) | unavailable (0/20) |
| Formatting, ms p95 | 1.300 | unavailable (not reached) | 1.612 |
| After-edit definition, ms p95 | 120.271 | unavailable (not reached) | unavailable (0/20) |
| Restart and scan, ms | 867.184 | unavailable (0/1) | 1,120.194 |

Every measured editor action accepts 20/20 samples. Small definitions return
one location, references two, and formatting one edit; source formatting
returns one edit. Source definitions, references and after-edit definitions
accept 0/20 invalid/missing responses. The large cold and restart scans each
accept 0/1 after their separate 600-second limits; its editor queries and
open-document memory measurement are not reached. After-edit latency includes
complete versioned diagnostics before the definition response.

The Tcllib case is being retried after its runner shut down before upload.
Its end-to-end row is unavailable until that case returns a result.

Whole-process open-document RSS is 355,300 KiB (small) and 319,884 KiB
(source), with one document open. The artifact also records their arithmetic
normalisations: 592,166,666.667 and 355,426,666.667 KiB per 10,000 lines.
Fixed process costs make these tiny-workspace extrapolations unsuitable as
the 10,000-line memory budget; that budget's large-workspace row is unavailable.
No allocator delta or retained-summary-only memory is claimed.

| VM script | Median, ms | p95, ms | Accepted |
|---|---:|---:|---:|
| `dict_ops` | 129.660 | 131.996 | 3/3 |
| `empty_loop` | 62.516 | 62.917 | 3/3 |
| `expr_arith` | 89.131 | 89.725 | 3/3 |
| `global_var` | 128.517 | 129.108 | 3/3 |
| `list_ops` | 114.441 | 115.626 | 3/3 |
| `namespace_var` | 131.438 | 132.580 | 3/3 |
| `oo_method` | 150.920 | 151.488 | 3/3 |
| `proc_body` | 72.507 | 74.307 | 3/3 |
| `proc_call` | 100.784 | 101.436 | 3/3 |
| `set_loop` | 66.871 | 68.324 | 3/3 |
| `string_ops` | 107.355 | 108.958 | 3/3 |

The native server binary is 118,683,112 bytes. The remaining budget
rows are explicitly unavailable under the owner's scripts/CI-only scope:

| Row | Why unavailable |
|---|---|
| Binding-only cold, 10,513 lines | No binding-only timer; workspace scan includes parsing and other analysis. |
| On-disk binding reload | Item 103 has no exposed tier yet. |
| Summaries per 10,000 lines, excluding parse/binding | Item 483 has no exposed summary-only timer. |
| 50 user packs, warm | Item 416 owns the interface and fixture. |
| Standalone runtime scripts | No release runtime runner was built/supplied. |
| Server WASM, WASI, VSIX, JetBrains ZIP, web bundle sizes | Those artefacts were not built on this runner; no size is borrowed from another revision. |

No latency, size or regression threshold is enforced. JSON and Markdown
samples remain in the CI artifact `split-baseline-45997018051` and its case
artifacts. All 15 harness contract tests and Python lint checks pass.

## Items, in priority order

| # | Item | State | Where |
|---|---|---|---|
| 1 | Long-tail tests driven by `tcl` CLI start-up | **Landed**: reused CLI workers for `rust/tcl-cli/tests` | `rust/tcl-cli-support/src/{worker,sink}.rs`, test-tiers contract rule 9 |
| 2 | `rust-check` is one serial 16-minute job | **Landed, awaiting first CI run**: `cargo xtask batch`/`make xtask-gates`; `rust-check` split into `lint` and `drift` matrix legs | `rust/xtask/src/main.rs`, `Makefile`, `.github/workflows/ci.yml`, `scripts/dev/test-pr-gate-path.sh` |
| 3 | Repository weight on `work` | **Landed**: the evidence prune (`8ab317a`) took the tree from 10.7 GB to 0.34 GB; `make check-tracked-size` in `xtask-check` keeps it there | `scripts/dev/check-tracked-size.py`, `scripts/dev/test_tracked_size.py` |
| 4 | Monolithic crates compiled twice | **Landed**: the emitters split into `tcl-codegen`; test parity with the base proven | `rust/tcl-codegen`, `docs/design/contracts/project-layout.md` |
| 5 | Hundreds of fat test binaries | **Landed**: integration tests merged per (crate, shard); no test lost | `rust/*/tests/it_<k>/`, `scripts/dev/rust-test-binary-shards.tsv`, test-tiers contract |
| 6 | Broken and racing CI caches | **Landed, awaiting first CI run**: shard cache keyed per runner class; sccache outlives the test phase | `.github/workflows/ci.yml`, `scripts/dev/test-rust-tests-runner.sh` |
| 7 | Exhaustive checks in unoptimised unit tests | **Landed for the registry; the lsp-core depth tests are down 4–8% and blocked on the naming owners**: registration slots indexed per dialect (38 s → 0.6 s, registry harness 51 s → 23 s); lazy image geometry and cached source maps for the depth tests | `rust/tcl-registry/src/registry.rs`, `rust/tcl-lexer/src/source_map.rs`, `rust/tcl-compiler/src/analyser/` |
| 8 | Version stamp reruns on every cargo call | **Landed**: dev-profile builds no longer re-run the stamp on every call; a no-op `cargo build -p tcl-lsp-server` went from 65 s to 0.3 s | `rust/tcl-version/build.rs` |
| 9 | Duplicate WASM builds | **Landed, awaiting first CI run**: `lsp-server-wasm` builds the browser module once and `test-ext-web` downloads it | `.github/workflows/ci.yml`, `scripts/dev/test-release-dependency-graph.sh` |
| 10 | Runner queueing in bursts | **Landed**: superseded resolution-oracle PR runs are cancelled; Dependabot's weekly PRs are spread over the week | `.github/workflows/resolution-oracles.yml`, `.github/dependabot.yml` |

## Baseline state of `work` (`84ddae4`), independent of this work

Recorded so a failure here is not mistaken for a regression from it:

- `tcl-cli` integration tests fail in both spawn and worker mode:
  `value_transfers_cli` 29/39, `cli` 21/71, `compile_verbs` 2/12,
  `spec_verbs` 4/49 — the same test names in both modes.
- `tcl-cli-support` lib: `highlight::tests::original_highlight_uses_actual_c_jim_and_irules_lexical_axes`
  fails on the base commit.
- `tcl-compiler` has two dead-code warnings, so `clippy -D warnings` fails
  before reaching downstream crates.
- `scripts/dev/test-vscode-test-partitions.sh` ("partition manifest does
  not cover the canonical glob exactly once") and
  `tcl-spec-studio` `reference_doc::fields_doc_matches_the_schema`
  (`fields.md` lags the registry's Clay grammar and TclOO flags) fail on the
  base commit.
- Drift gates failing on the base commit (each also fails run alone):
  `kcs-index-links`, `owner-resolution`, `resolution-drift`,
  `retired-api-gate`, `value-transfers --check`, `registry-axes --check`,
  `command-backing --check`, `callback-inventory --check`.

## Worktree cost baseline (item 259)

Measured 26 November 2026 by `scripts/dev/worktree-cost.py`
([design](../../design/build-time/259-worktree-cost.md)) on `work` at
`a87feb87` (build inputs unchanged since `18040b3b`), on a 4-vCPU, 15.7 GB
container with rustc and cargo 1.100.0 and the agent build environment.

How it was run, and what that changes:

- **In place, not in a fresh worktree** (`run --in-place`). The disk had
  22 GB free and a fresh worktree needs 40 GB. So "cold" is the warm target
  directory of the main checkout (34 GB before the run), not an empty one:
  the cold rows measure what that checkout rebuilt to catch up, not a
  worktree's first build. The after-edit rows are the per-edit cost.
- **One edit and command per invocation** (`--edits`, `--commands`,
  `--phases`, `--merge`), each under the session's 10-minute foreground
  limit. Restoring an edited file changes its mtime, so the next command
  on that crate rebuilds it once more (the lexer test run's 10 units).
- **Skipped:** the test runs of `tcl-registry`, `tcl-compiler` and
  `tcl-lsp-core` (the `tcl-compiler` suites alone take over 7 minutes), for
  time; the whole baseline took about 50 minutes. rust-analyzer's memory:
  not measured, the rustup proxy has no `rust-analyzer` component.
- **Reproducibility does not hold in place.** A second after-edit test build
  of the analyser edit compiled 40 units in 77 s (first: 31 in 338 s), and of
  the registry edit 22 in 44 s (first: 22 in 99 s). The incremental cache
  keeps the edited state of the first run, and the registry edit in between
  made 9 more units stale. Wall times in place are an upper bound; unit
  counts after an edit are exact only when nothing else ran between.
- **Build scripts:** the count recorded with this baseline listed every
  build script in the graph (cargo reports fresh scripts too). The script
  now counts only reruns; this run compiled no third-party unit except the
  `lsp` cold clippy, so no build script of a dependency reran.

Units are cargo units (one per crate target), not crates. RSS is the
process tree's summed peak (sampled every 200 ms) and the largest single
process.

| Edit | Crate | Phase | Command | Workspace units | Total units | Wall (s) | Peak RSS tree (MB) | Peak RSS single (MB) | Target growth (MB) | Test run RSS (MB) |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| analyser | `tcl-compiler` | warm | test_build | 0 | 0 | 0.4 | 79 | 79 | 0 | – |
| analyser | `tcl-compiler` | after | test_build | 31 | 31 | 338.4 | 6232 | 5405 | 2135 | – |
| analyser | `tcl-compiler` | warm | clippy | 32 | 32 | 143.5 | 5058 | 4976 | 0 | – |
| analyser | `tcl-compiler` | after | clippy | 32 | 32 | 147.5 | 5059 | 4977 | 0 | – |
| registry | `tcl-registry` | warm | test_build | 20 | 20 | 105.7 | 4307 | 3424 | 1062 | – |
| registry | `tcl-registry` | after | test_build | 22 | 22 | 99.1 | 6098 | 3419 | 642 | – |
| registry | `tcl-registry` | warm | clippy | 29 | 29 | 111.0 | 2897 | 2842 | 0 | – |
| registry | `tcl-registry` | after | clippy | 22 | 22 | 107.5 | 2933 | 2842 | 0 | – |
| lexer | `tcl-syntax` | warm | test_build | 12 | 12 | 17.1 | 1059 | 603 | 222 | – |
| lexer | `tcl-syntax` | after | test_build | 10 | 10 | 1.5 | 682 | 409 | -1 | – |
| lexer | `tcl-syntax` | warm | clippy | 15 | 15 | 4.5 | 779 | 446 | 4 | – |
| lexer | `tcl-syntax` | after | clippy | 10 | 10 | 3.6 | 684 | 446 | 0 | – |
| lexer | `tcl-syntax` | warm | test_run | 10 | 10 | 12.9 | 729 | 406 | 0 | 10 |
| lexer | `tcl-syntax` | after | test_run | 10 | 10 | 13.4 | 668 | 410 | 0 | 10 |
| lsp | `tcl-lsp-core` | warm | test_build | 27 | 27 | 455.3 | 6341 | 4912 | 4508 | – |
| lsp | `tcl-lsp-core` | after | test_build | 11 | 11 | 13.5 | 2925 | 1937 | -9 | – |
| lsp | `tcl-lsp-core` | warm | clippy | 32 | 57 | 112.5 | 2896 | 2841 | 165 | – |
| lsp | `tcl-lsp-core` | after | clippy | 11 | 11 | 36.4 | 1822 | 1712 | 0 | – |

Crates rebuilt by the test build after each edit:

- analyser: 9 — `tcl-compiler`, then through its dev-dependencies
  `tcl-spectcl`, `tcl-lsp-core`, `tcl-engine-tclvm`, `tcl-codegen`,
  `tcl-bigip`, `tcl-irules`, `tcl-sslictcl`, `tcl-spec-hooks` (item 246's
  target);
- registry: 11 — the analyser's nine plus `tcl-registry` and `tcl-vm`,
  through `tcl-registry`'s dev-dependency on `tcl-spectcl` (item 269);
- lexer: 1 (`tcl-syntax`);
- lsp: 2 (`tcl-lsp-core`, `tcl-spectcl`, which depends on it; item 270).

Profiles (answers the design's open question): in the test builds every
dependency unit is compiled at `opt_level` 1 with `test: false`, so the
test profile's settings, not dev's, apply to dependencies. Clippy builds at
opt-level 0 in check mode (`.rmeta` only).

Variants: across the commands of this run every workspace package was built
with one feature set, except `tcl-expr`, `tcl-numeric` and `tcl-syntax`
(two each). The unit directories on disk are not a variant count in place:
the checkout's `target/debug/build/` holds every unit since it was created
(`tcl-compiler` 135, `tcl-vm` 88, `tcl-registry` 66); a fresh worktree is
needed for that column.

### Item 246 (step 0, `tcl-compiler`): before → after

`cargo test -p tcl-compiler --lib --no-run` after appending one line to
`rust/tcl-compiler/src/analyser/diagnostics/widget_command.rs`, warm target,
same machine, 26 November 2026:

- before (`24b32a5c^`): 9 crates, 10 units (`tcl-compiler` lib and test
  harness, `tcl-spectcl`, `tcl-lsp-core`, `tcl-engine-tclvm`, `tcl-codegen`,
  `tcl-bigip`, `tcl-irules`, `tcl-sslictcl`, `tcl-spec-hooks`), 55.0 s;
- after: 1 crate, 1 unit (the `tcl-compiler` test harness), 29.8 s.

Both are incremental rebuilds of a warm target; the failset logs' 2–6
minutes were taken under heavier load and colder caches.

### Items 269-272 (steps 0b-0e): before → after

`cargo test -p <crate> --no-run` after appending one line to a source file of
that crate, warm target, same machine, 26 November 2026. Cones are unique
workspace packages in `cargo tree -e normal,build,dev`.

| Item | Crate | Edited file | Before | After | Test cone before → after |
|---|---|---|---|---|---|
| 269 | `tcl-registry` | `src/commands/irules/active_members.rs` | 11 crates (`tcl-registry`, `tcl-compiler`, `tcl-vm`, `tcl-irules`, `tcl-sslictcl`, `tcl-codegen`, `tcl-engine-tclvm`, `tcl-bigip`, `tcl-lsp-core`, `tcl-spec-hooks`, `tcl-spectcl`), 95.2 s | 1 crate, 9.2 s | 29 → 15 |
| 270 | `tcl-lsp-core` | `src/folding.rs` | 2 crates (`tcl-lsp-core`, `tcl-spectcl`), 11.3 s | 1 crate, 8.5 s | 29 → 21 |
| 271 | `tcl-lsp-db` | `rust/tcl-vm/src/lib.rs` (a crate below the removed edges) | 5 crates (`tcl-vm`, `tcl-lsp-db`, `tcl-engine-tclvm`, `tcl-spec-hooks`, `tcl-spectcl`), 9.0 s | 0 crates, 0.3 s | 30 → 22 |
| 271 | `tcl-lsp-db` | `src/display_queries.rs` | 1 crate, 8.2 s | 1 crate, 6.6 s | |
| 272 | `tcl-vm` | `rust/tcl-compiler/src/analyser/diagnostics/widget_command.rs` (a crate above the edges) | 3 crates (`tcl-compiler`, `tcl-codegen`, `tcl-vm`: lib test harness, 15 test binaries), 21.9 s | 0 crates, 0.12 s | 19 → 17 |

### Item 318a (`tcl-spectcl`'s hook runner behind a feature): before → after

`tcl-spectcl`'s dependency on `tcl-spec-hooks` is optional, behind the
non-default `programmed-packs` feature. The server, the CLI (with
`tcl-cli-support`), MCP, Spec Studio, `xtask`, `tcl-compiler-it` and
`tcl-lsp-db-it` enable it, and so does `tcl-spectcl`'s own test build,
through a dev-dependency on itself. Without the feature, the static reader
loads every shipped pack. A pack that needs the interpreter fails its load
with a notice naming the feature, and hook bodies get no slot, so they abstain
as dormant ones do. Measured 27 November 2026 in place, on the main checkout
with the agent build environment. Cones are the packages in `cargo tree -e
normal` (deduplicated), all packages / workspace packages:

| Crate | Before | After | Leaves the cone |
|---|---|---|---|
| `tcl-spectcl` | 141 / 28 | 133 / 21 | `tcl-spec-hooks`, `tcl-engine-tclvm`, `tcl-engine-api`, `tcl-vm`, `tcl-codegen`, `tcl-host-native`, `tcl-host-c-abi` |
| `tcl-explorer` | 142 / 29 | 135 / 23 | the same, except `tcl-codegen`, which it names itself |
| `tcl-explorer-wasm` (own lockfile) | 149 / 30 | 142 / 24 | as `tcl-explorer` |
| server, CLI, MCP, Spec Studio, `xtask` and their WASM hosts | | unchanged | none (they enable the feature) |

`cargo tree -p tcl-spectcl -e normal` lists no `tcl-spec-hooks`, and
`cargo tree -e normal -i tcl-vm` prints nothing for `tcl-spectcl`,
`tcl-explorer`, `tcl-compiler`, `tcl-lsp-core`, `tcl-lsp-db` or `tcl-registry`.

`cargo test -p tcl-explorer --no-run` after appending one line to
`rust/tcl-engine-tclvm/src/lib.rs`, warm target:

- before: 4 crates (`tcl-engine-tclvm`, `tcl-spec-hooks`, `tcl-spectcl`,
  `tcl-explorer`), 22.0 s. An edit to `tcl-vm` (another agent's, between two
  runs) rebuilt 5 crates in 15.7 s.
- after: 0 crates, 0.2 s.

Variants: features are per build, so a target that builds the explorer's
tests (feature off) and an entry point (feature on) compiles `tcl-spectcl`
and `tcl-explorer` twice. That is the one-variant goal of 296 and D-I, which
the feature approach does not meet: the variant leaves with the rest of item
318, when the hook runner becomes a dependency that entry crates link.

### Build levers (items 296–300 and 260): before → after

Measured 26 November 2026 in place on the main checkout (warm target, 11 GB
free, so no fresh worktree), 4 vCPU, cargo 1.100.0, agent build environment.

**298, `xtask` without the engines.** `command-backing` moved to its own
package, `rust/xtask-backing` (`cargo xtask-backing command-backing --check`,
run by `make xtask-command-backing` under `xtask-check`, out of the batch).

| | Before | After |
|---|---:|---:|
| `cargo build -p xtask` units (dev) | 272 | 270 |
| packages in `xtask`'s normal and build graph | 236 | 235 |
| workspace crates in `xtask`'s normal graph | 36 | 35 |
| `tcl-runtime` units in that build | 2 (build script; lib as rlib, cdylib and staticlib, 383 MB) | 0 |
| `cargo build -p xtask` after touching `runtime/rust/src/lib.rs` | 27.0 s (`tcl-runtime` 17.8 s, `xtask` relink 9.1 s) | no `xtask` unit rebuilt |
| `make xtask-gates`, warm build | 251 s, 50 gates | 248 s, 49 gates |
| `cargo tree -p xtask -e normal -i tcl-runtime` | `xtask` | no match |

`tcl-vm` stays in `xtask`'s cone only through `tcl-spectcl → tcl-spec-hooks →
tcl-engine-tclvm`, as the design says. The cost of the split: `xtask-backing`
resolves features alone, so 36 of its 101 units are variants `xtask` does not
build (`tcl-registry`, `tcl-vm`, `tcl-runtime`, `tcl-cmd-core`, `tcl-regex`,
`tcl-host-native`, `tcl-host-c-abi`, and third-party `serde`, `regex`, `clap`,
`sha2` and their dependencies); its build took 44 s on a warm target. Only
`xtask-check` (CI's drift leg) builds it, once. The gate run itself is 0.1 s
either way; the saving is the `tcl-runtime` build in every gate run and every
`xtask` relink after a runtime edit, and, once 297 moves `xtask` to the test
profile, `tcl-runtime` at opt-level 1.

**297, one build profile.** `agent-build-env.sh` exports
`CARGO_ALIAS_XTASK="run --profile test --package xtask --"` and the same for
`xtask-backing`; CI keeps the checked-in aliases. What shares what (unit
directories from cargo's JSON messages, `tcl-dialect` and its dependency):

| Command, after `cargo test -p tcl-dialect --no-run` | Units | Shared with the test build |
|---|---:|---:|
| `cargo build --profile test -p tcl-dialect` | 2 | 2 (the libraries) |
| `cargo clippy --profile test -p tcl-dialect --all-targets` | 5 | 0 |
| `cargo clippy -p tcl-dialect --all-targets` (dev) | 5 | 0 |

Check mode shares no unit with a build in either profile, so clippy and
check stay on dev, where third-party check units are shared with
rust-analyzer; the merge is `xtask` (and `cargo run`) into the test build.
Cargo has no alias for its built-in `check` and `clippy` anyway.

| `xtask` in a worktree whose test builds include `xtask`'s | Dev profile (before) | Test profile (after) |
|---|---:|---:|
| units in the build | 269, none shared with tests | 272, 197 already built by `cargo test` |
| units compiled for it | 269 | 75 (876 MB of unit directories) |
| its own unit directories | 1,571 MB | 876 MB new |
| `make xtask-gates`, warm build | 248 s | 81 s |
| slowest gates | owner-resolution 56.7 s, retired-api-gate 40.6 s, parser-ownership 37.0 s | retired-api-gate 20.9 s, owner-resolution 20.2 s, parser-ownership 13.6 s |
| `make xtask-command-backing` from a warm test build | – | 109 s, `tcl-runtime` at opt-level 1 included |

The 75 test-profile units took 438 s at `-j3`, most of it the opt-level-1
compiles of `tcl-compiler` and `tcl-lsp-core` in `xtask`'s feature set. The
gates' 3x speed-up is opt-level 1 itself. `CARGO_ALIAS_SPEC_IMAGES` waits for
`tcl-spec-images` (item 307), which does not exist yet.

**260, the shared cache and pruning.** The cache is configured but not
measured: sccache is not installed in this container, and the design's
three-worktree run needs worktrees and a disk with 60 GB free. With
`sccache --version` succeeding, `agent-build-env.sh` sets `RUSTC_WRAPPER`,
`SCCACHE_DIR` and `SCCACHE_CACHE_SIZE=20G` (checked with a stub `sccache` on
`PATH`); without it the output is unchanged. `scripts/dev/prune-targets.py`
(`make prune-targets`, tests in `make check-prune-targets-script` under
`xtask-check`) deletes unit directories built by an older rustc, those
superseded by three newer ones with the same fingerprint key, empty ones and
those older than three days, and incremental directories older than three
days, under cargo's own lock. On this checkout's target:

| Run | Unit directories deleted | Freed |
|---|---:|---:|
| first, `--keep 3` | 432 superseded, 175 empty | 9,187 MB |
| after two failset runs, `--keep 2` | – | 3,344 MB |

`target/debug/build` was 41 GB before the first run and is 30 GB now.

**296, dev-only features (measurement and decision).** For each crate whose
test enables a dev-only feature, the packages whose resolved features differ
from a dev `cargo build -p tcl-lsp-server` (`cargo tree -f '{p}|{f}'`, test
graph with dev-dependencies against the server's normal graph), and the
workspace crates of the server's graph that sit at or above the differing
workspace crate in that test graph, so get a second unit hash:

| Test of | Dev-only feature that differs | Duplicated workspace crates |
|---|---|---|
| `tcl-codegen` | `tcl-compiler/test-support` | `tcl-compiler`, `tcl-codegen` (2) |
| `tcl-compiler-it` | `tcl-compiler/test-support` | `tcl-compiler`, `tcl-codegen`, `tcl-lsp-core`, `tcl-spectcl`, `tcl-spec-hooks`, `tcl-engine-tclvm`, `tcl-bigip`, `tcl-irules`, `tcl-sslictcl` (9) |
| `tcl-vm`, `tcl-irule-test` | `tcl-vm/test-support` | `tcl-vm` (1) |
| `tcl-engine-tclvm` | `tcl-vm/test-support` | `tcl-vm`, `tcl-engine-tclvm` (2) |
| `tcl-spectcl` | `tcl-vm/test-support` | `tcl-vm`, `tcl-engine-tclvm`, `tcl-spec-hooks`, `tcl-spectcl` (4) |
| `tcl-lsp-server` | `tcl-lsp-core/{truth-table, transport-fixtures}` | `tcl-lsp-core`, `tcl-lsp-db`, `tcl-spectcl`, `tcl-explorer`, `f5-xc`, `tcl-lsp-server` (6) |
| `tcl-cli` | `tcl-lsp-core` and `tcl-explorer` features | `tcl-lsp-core`, `tcl-explorer`, `tcl-spectcl` (3) |
| `tcl-mcp` | `tcl-lsp-core`, `tcl-irules/test-instrumentation`, `tcl-explorer` and `tcl-bigip-query` features | `tcl-lsp-core`, `tcl-irules`, `tcl-bigip`, `tcl-bigip-query`, `tcl-explorer`, `tcl-diagram`, `tcl-spectcl`, `f5-xc` (8) |

Each row is two variants of those crates (with and without the feature).
Third-party packages also differ between any two roots (the server's graph
turns on `serde_json/preserve_order`, `syn/full` and others), which is not a
dev-only-feature cost. On this checkout's disk, the test-profile lib unit of
the feature variant is 284 MB for `tcl-compiler`, 104 MB for `tcl-vm` (two
units), 67 MB for `tcl-lsp-core` and 2 MB for `tcl-irules`, before the
crates above them and their incremental state. Decision: no change in this
item. The design marks no lever as a standalone win: `tcl-compiler`'s
feature goes with items 266 and 292 (option 2, per crate), `tcl-vm`'s
constructors with item 291 (option 2) while `native_conformance` keeps its
feature (option 3), and the `tcl-lsp-core` fixtures wait for the
`tcl-lsp-policy-table` check (option 1 or 2). `tcl-explorer`'s and
`tcl-bigip-query`'s features in `tcl-cli`'s and `tcl-mcp`'s tests are not in
the design's table yet.

**300, generated tables (measurement only).** Dev build (non-incremental,
opt-level 0) of the owning crate, with the file as it is and with its table
replaced by `&[]`, `cargo build --timings`, `-j3`:

| File | Lines | Crate compile, as is | With the table empty | Table's share |
|---|---:|---:|---:|---:|
| `tcl-registry/src/base_objects/generated_names.rs` | 21,712 | 22.1 s | 21.5 s | 0.6 s |
| `tcl-registry/src/base_objects/generated_objects.rs` | 20,419 | 22.1 s | 21.5 s | 0.6 s |
| `tcl-registry/src/bigip/references.rs` | 5,061 | 22.1 s | 21.8 s | 0.4 s |
| `tcl-registry/src/profile_defaults/generated.rs` | 2,213 | 22.1 s | 21.7 s | 0.4 s |
| all four registry tables at once | | 22.1 s build, 15.9 s check | 20.7 s build, 14.4 s check | 1.4 s, 1.6 s |
| `tcl-dialect/src/generated_release_facts.rs` (`RELEASE_FACTS`) | 11,445 | 1.14 s | 1.15 s | none measurable |
| `tcl-bigip/src/model/gen/canon.rs`, `parsers.rs` (code) | 10,733 | `tcl-bigip` 5.3 s for 58.5k lines | – (functions, not tables) | – |

So the tables cost at most about 0.6 s each per cold or check compile of
their crate, and the release facts nothing measurable. The load-cost side
(first-access parse of a data form) is not measured: no data form exists
yet. Test-profile (opt-level 1, incremental) A/Bs are not taken.

**299, build scripts.** `tcl-version`'s build script, in an unstamped dev or
test build, emits `<CARGO_PKG_VERSION>-dev` (plus `+g<GIT_HASH>` when that is
set) and watches no git path; stamped and release builds are unchanged.
`tcl-dialect/build.rs` is deleted: `TclVersion::patchlevel()` and
`reference_source_tag()` read `RELEASE_FACTS`, and `cargo xtask release-facts`
now checks the rendered table against `data/reference-toolchains.tsv`, the
one owner of the two columns.

| | Before | After |
|---|---:|---:|
| `cargo build -p tcl-lsp-server` (dev) after an empty commit | 3 units (`tcl-version`, `tcl-lsp-server` lib and bin), 40 s | 0 units, 0.1 s |
| the same after `git pack-refs --all` | (watched `packed-refs`) | 0 units |
| `tcl-dialect` build-script units per profile | 2 (compile, run) | 0 |
| `target/debug/tcl --version` | `0.1.0+g<hash>[.dirty]` | `0.1.0-dev` |

Not changed: `bigip-report-gen-rust`'s own build script still watches the git
refs for its report footer; it is outside `tcl-lsp-server`'s graph and
rebuilds only itself and its binary.

## 1. CLI start-ups in tests

What changed:

- `tcl __tcl-cli-worker <dir> <slot>` serves invocations from one process
  (`tcl_cli::serve_worker`, `tcl_cli_support::worker::serve`). Verb output goes
  through `tcl_cli_support::sink`; `tcl-cli` denies `clippy::print_stdout` and
  `clippy::print_stderr`.
- `tcl_cli_support::worker::output` is the client. `cli.rs`, `compile_verbs.rs`,
  `value_transfers_cli.rs` and the environment-free command in `spec_verbs.rs`
  use it through `reused_output()`. `pkg_*` tests keep real processes: each
  test sets its own `HOME`/`TCL_VENV`, so a worker would serve one call.
- Commands that differed only by a throwaway path now share one environment:
  `absent_config_home()` is one never-created directory per test binary, the
  `Scratch` fixture uses it, and each truth-table pass rewrites one
  `XDG_CONFIG_HOME` per row (the CLI reads `config.ini` afresh per run).
- `callgraph_reads_a_cr_terminated_tcl_document_the_way_the_editor_does`
  compares two runs' opaque `bodyKey`s, which carry a process-wide counter, so
  it runs real processes (`run_fresh_tcl_on_scratch_doc`).
- `tcl_cli_support::cli_packs` keys its cache on the current directory as well
  as the demand generation, because a worker changes directory per request.

Measured on 4 vCPU, debug profile without debuginfo:

| | spawn (before) | workers (after) |
|---|---|---|
| `tcl` starts, `value_transfers_cli` + `cli` | 394 | 22 (one process per test, nextest-style) |
| `value_transfers_cli` wall, 4 threads | 174 s | 125 s (110 s one process per test) |
| `cli` wall, 4 threads | 145 s | 51 s test time (60 s one process per test) |
| one `tcl opt --profile full` on one line | ~1.6 s | 0.3–0.4 s warm, ~1.9 s first |

Pass/fail sets are identical between modes in all four suites, in both the
libtest and the one-process-per-test runs. Logs:
`/tmp/claude-0/…/scratchpad/w/{spawn,worker2,worker3,pp}-*.log`.

Open:

- 29 of 39 `value_transfers_cli` tests fail early on the base commit, so its
  speed-up is understated until they pass and run all their invocations.
- 12 pools remain in `cli.rs`, each a test that sets its own global config.
- Per-request cost (0.3–0.4 s for `opt --profile full` on one line) is now the
  floor; it is verb work, not start-up.

## 2. `rust-check`

What changed:

- `cargo xtask batch '<gate argv>' …` runs subcommands in one process and
  fails if any failed; `make xtask-gates` runs the 35 single-command gates of
  `xtask-check` through it (`XTASK_GATES`). The individual `xtask-*` targets
  remain.
- `rust-check` is a matrix job with legs `lint` (`make rust-check-lint` =
  `check-rust-pr`) and `drift` (`make rust-check-drift` = `xtask-check`), each
  with its own rust-cache key. `make rust-check` is still exactly both, and
  `scripts/dev/test-pr-gate-path.sh` holds the legs, `fail-fast: false` and
  the Makefile union, with negative controls for each.

Measured: the 35 gates take 413 s in one batch locally, of which
`kcs-index-links` is 243 s (it reads 3,472 markdown files, 2,630 of them under
`docs/design/analysis`) and `owner-resolution` 81 s. Run alone, a gate also
pays cargo's start-up and cold caches: `callback-inventory` 5.2 s alone against
1.0 s in the batch, `registry-axes` 13.2 s against 9.4 s.

Open:

- Confirm on the first CI run that the TomMath fetch ended the
  `tcl-runtime`/`xtask` rebuild loop (`Compiling tcl-runtime` should appear
  once per leg) and record both legs' durations.
- `kcs-index-links` is now the drift leg's long pole; see item 3.

## 3. Repository weight

The evidence prune on `work` (`8ab317a`, a separate workstream) took the
tracked tree from 10.7 GB in 127,281 files to 0.34 GB in 39,003, and
`docs/**/*.md` from 3,472 files to 1,847 (`kcs-index-links` read all of them).

`make check-tracked-size`, part of `xtask-check` and so of CI's `rust-check
(drift)` leg, fails when a tracked file exceeds 10 MiB or the tracked tree
exceeds 1 GiB. A file a build or test reads may be listed in
`scripts/dev/tracked-size-allowlist.txt`; an entry that no longer names an
oversized file fails too. The largest tracked file today is 8.8 MiB
(`rust/tcl-compiler/tests/data/native_brace_expression/named-row-audit.json`).

Open:

- History still carries the old blobs: a full clone, or a shallow clone of an
  old commit, downloads them. Only a history rewrite removes them; that is the
  owner's call.
- Re-time `kcs-index-links` after the prune (243 s against the old tree).

## 4. Splitting `tcl-compiler`

A balanced split is not available as a move: the production code (about
338k of the crate's 908k lines; the rest is `#[cfg(test)]`) is one strongly
connected module graph of 97 modules and 330k lines. Even `ir` reaches
`analyser`, `command_binding` and `lowering`. Any further split first needs
cycles broken by design.

The one slice that comes out with few cut edges is the back end, now the
`tcl-codegen` crate: `codegen` (30k production + 22k test lines), `inlining`
and `compile_service`. It depends on `tcl-compiler`; nothing in
`tcl-compiler` depends on it.

- Edges cut: the list/format folds and `command_text` moved down into
  `tcl_compiler::codegen_helpers` (re-exported as `codegen::helpers`); the
  native-entry configuration helpers moved into `command_binding`
  (`compile_service` delegates); doc links became plain text.
- About 150 crate-private items that the emitters use became
  `#[doc(hidden)] pub`: the emitters always used `tcl-compiler` internals.
- Tests: emitter tests that lived in `tcl-compiler` moved to
  `tcl-codegen` — 19 integration test files and an example, plus unit
  tests in `src/tests`. Fixtures they need from `tcl-compiler` are
  behind its `test-support` feature (enabled only by `tcl-codegen`'s
  dev-dependency); fixtures that hold a VM live in
  `tcl_codegen::test_support`, since `tcl-compiler` cannot depend on
  `tcl-vm`.
- CI: the shard map re-keys the moved binaries and adds 22 rows, 19 of
  which were missing on the base commit (`verify-nextest-binary-shards.py
  --metadata-only` now passes: 417 targets). `wasm-real-link` runs
  `-p tcl-codegen`; the `lsp_wasm`, GitHub Pages and SpecTcl-compat path
  filters include `rust/tcl-codegen`. The five separately locked WASM crates
  gained the new package in their lockfiles, and nothing else.

Verified: `cargo check --workspace --all-targets --all-features` is clean
apart from two dead-code warnings already on the base; `tcl-vm-wasm` checks
for the host. `tcl-codegen`'s tests run; no failure is a missing fixture or
path.

Parity: the same tests run on the base commit `4be8d9f` (as `tcl-compiler`
unit and integration tests) and on the split (as `tcl-codegen` tests), compared
by binary and test name: 1,529 tests in 21 binaries, **no outcome changed** in
either direction. The work tip's existing failures (for example
`fixture body "::p" absent`) are the same tests on both sides. The 37 former
`codegen::helpers` tests run in `tcl-compiler` with `codegen_helpers`.
Logs: `/tmp/claude-0/…/scratchpad/split/{codegen-tests,base-integ,base-lib}.log`.

Naming and ownership gates. The split first took the emitters out of the
drift gates' scan roots, so they quietly stopped checking moved code. Now:

- `rust/tcl-codegen/src` is a root of `util::ANALYSIS_TIER_ROOTS`
  (`value-transfers`, `registry-axes`), `dialect-drift`'s document crates and
  `diag-emission`'s search roots; `retired-api-gate`'s owned patterns that
  covered the compiler's codegen cover `tcl-codegen`.
- The owner-resolution source manifest records 47 rows the split invalidated:
  rows whose owner and consumer both moved name the `tcl-codegen` package, and
  owners widened to `#[doc(hidden)] pub` record their actual visibility. Only
  rows that pass on the base were touched.
- Four source routes now cross from `tcl-codegen` into `tcl-compiler`, which
  the route checker cannot follow; they are listed beside the manifest. Two
  owners keep verified in-crate routes; `original_literal_argument_data_for_module`
  and `matches_foreach_header` have no checked consumer route until the checker
  resolves `tcl_compiler::` paths.
- The moved test code lives in `tcl-codegen/src/tests/`, the path every gate
  already treats as tests.

Result, run on the base `4be8d9f` and on the split and compared with line
numbers ignored: `owner-resolution`, `resolution-drift`,
`segmentation-drift`, `dialect-drift`, `number-drift`, `retired-api-gate`,
`catalogue-callers`, `value-transfers`, `registry-axes`, `callback-inventory`,
`command-backing` and `diag-emission-check` report the same findings and exit
codes on both. Clippy on `tcl-compiler` matches the base too: the 44 widened
functions carry `#[must_use]`.

Open:

- Teach the source-route checker cross-crate `tcl_compiler::` routes, so the
  two unchecked routes above become checked (naming workstream's gate).

Build time, cold `cargo build --workspace --lib` on 4 vCPU without debuginfo,
base `4be8d9f` then the split, back to back: **248.6 s → 235.4 s** (−5%).
`tcl-compiler` itself 125.5 s → 114.7 s and still the critical path;
`tcl-lsp-core` starts 14 s earlier; `tcl-vm` now waits for `tcl-codegen` but
is off the critical path. The large wins in this area need the cycle in
`tcl-compiler`'s production code broken by design, or the build-once CI
pipeline of item 5.

Next (26 November 2026): the cycle analysis and a layered split are designed in
[splitting tcl-compiler](../../design/compiler/crate-split-design.md), and the
name-handling plan runs it first as item 90, with items 117 and 118 shrinking
`tcl-registry`. Two measurements set the order:

- **The rebuild cascade.** The failset logs show every `tcl-compiler` edit
  rebuilding 9 crates in 2–6 minutes before its lib tests run. `tcl-compiler`'s
  dev-dependencies (`tcl-spectcl`, which depends on `tcl-lsp-core`, plus
  `tcl-vm` and `tcl-engine-tclvm`) are built for `--lib` tests too. Moving the 35
  test files that need them into their own crate is step 0.
- **The linker.** Linking is not the cost. On the 183 MB test binary, the
  bundled `rust-lld` takes 1.4–1.7 s, `mold` 1.2 s and GNU ld 6.5 s.

## 5. Integration-test binaries

Merging every crate's tests into one binary would move all of a crate's tests
into one CI shard and undo the binary-aware balance. Instead, in each crate the
`tests/*.rs` files assigned to shard `k` became modules of one harness
`tests/it_<k>/main.rs`. Each test keeps its shard.

- 267 files in 13 crates became 35 binaries; the shard map proves 185 targets
  instead of 417 (`verify-nextest-binary-shards.py --metadata-only`).
- Kept as their own binaries: smoke binaries, `*_corpus` and
  `differential_incremental` (nextest binary filters), anything run by
  `--test <name>`, explicit `[[test]]` targets, files with crate-level
  attributes, and `f5-cli`/`runtime/rust` (outside the shard map).
- `tcl-vm`, deps warm, 4 vCPU, no debuginfo: building its integration
  harnesses went from 51 s and 9.2 GB to 46 s and 1.8 GB (82 files, 15
  executables).
- No test lost: each of the 35 merged binaries' `--list` count equals the
  static `#[test]` count of its files (8,025 tests), except four files where
  the difference is a `#[test]` in a doc comment (`taint_depth`,
  `pack_torture`), a `cmd-core`-feature test (`precision_oracle`, built in
  CI's `--all-features`) and a release-only test (`cmd_thread_e2e`).
- Merged modules: `crate::` paths became `crate::<module>::`, out-of-line
  modules and relative `include_*!` paths resolve from `tests/it_<k>/`, and the
  one shared support file that named a sibling through `crate::` uses
  `super::`. References to moved files in docs, scripts and code follow them.

Open:

- The CI build-once/archive pipeline for `rust-tests` (plan 5.3) and test
  debuginfo settings (5.2).

## 6. CI caches

- `rust-tests-shard` caches are keyed `rust-tests-v3-<tank|hosted>`. Tank and
  hosted workspaces sit at different depths, so an archive saved on one
  restored its workspace-relative `../../../../.cargo` outside the other's home
  (`tar: Cannot mkdir: Permission denied`, about 4,200 errors, then "No cache
  found" in run 41810769449).
- `SCCACHE_IDLE_TIMEOUT=0` keeps the shard's sccache server alive through the
  10–13 minute test phase; its default idle shutdown erased the statistics, so
  the compile-phase hit rate was unknown.
- `rust-check`'s two legs already have separate keys (item 2).
- `scripts/dev/test-rust-tests-runner.sh` pins both.

Open:

- After the first CI run: confirm the restore succeeds and record the shard
  sccache hit rate.
- Five legs still race to save one key per class ("another job may be
  creating this cache"); that is harmless, but saving from one leg needs the
  pinned setup action's save option checked first.

## 7. Exhaustive unit tests

### Registry: fixed

`registry::prerequisite_index_tests::the_prerequisite_index_selects_what_the_full_scan_selects`
took 138.6 s in the report and 38 s after the earlier per-dialect threads.

Profiling corrected an earlier note here: the indexed lookup was never as
slow as the scan. The cost was in two other places:

- **The test's own full scan.** It re-ran `best_visible` and
  `implementation_prerequisites` over every spec for each identity. The scan
  is now taken once per registry and dialect, and each identity filters it.
  It is still a flat filter rather than a map, so it stays an independent
  oracle for the index.
- **A production naming owner.** `native_registration_lookup` re-walked every
  visible spec, implementation path and stock-worker contract for each new
  identity: one full registry scan per distinct command name, for the
  compiler as well as the test. It now reads a per-dialect `FxHashMap` keyed
  through `tcl_syntax::naming::unroot_rooted_key`, built once, like
  `prerequisite_index`. An ambiguous slot keeps answering `None`.

The test now also checks the lookup index against its own scan for every
spec, member, implementation and stock-worker slot. Results:

- the test: 38 s → **0.6 s**
- the whole `tcl-registry` lib harness: 51 s → **23 s**, with the same 13
  failures as HEAD

### lsp-core depth tests: partly fixed, root cause with the naming owners

Measured alone on the current tip, before → after:

| Test | Before | After |
|---|---|---|
| `minify::tests::deeply_nested_if_survives_minify` | 616 s | 589 s |
| `package_resolver::tests::parse_pkg_index_survives_deeply_nested_source_wrapper` | 149 s | 138 s |
| `minify::tests::guarded_body_keeps_collision_candidates_but_not_absence_authority` | 63 s | (not re-timed) |
| `executable_regions::…::checked_potential_source_retains_depth_exhaustion_as_incomplete` | 36 s | (not re-timed) |

The report's plan for these does not apply:

- `AGENTS.md` allows `#[ignore]` only for fuzz-shaped tests.
- The 2000-deep inputs are crash guards set above the measured stack crash
  floor (`docs/design/compiler/recursive-descent-depth-limits.md`). Cutting
  them to cap+1 would stop them catching a removed cap.

So the fix has to be in the analyser's cost. Stack samples (gdb, no `perf`)
put the time in `Analyser::retain_original_static_source_names`, which runs
once per command and re-lexes source for every ancestor:

- `vendor_source_barriers::capture_in_script_body` re-lexes the whole
  document and then every enclosing body region. A command at depth d costs
  d full lexes, which is cubic in nesting.
- `original_readonly_vendor_source_vector` re-segments the command's own text,
  which for nested `if`s contains every inner level.
- `OriginalSourceCommandProjection::capture_with_nested_cuts` re-lexes again.

Landed here:

- **`SourceImage` geometry on first use.** Every image built a whole-input
  line index and byte digest up front, though most are only lexed. Both are
  now `OnceLock`s in the shared geometry, so the
  `naming.source.original-immutable-image-geometry` contract (shared across
  clones, excluded from identity) is unchanged. Its proof doc says so.
- **Cached source maps in per-command handlers.** Five analyser call sites had
  gone back to `SourceMap::new(&self.source)`, the O(document × depth)
  rescan that `Analyser::source_map`'s doc calls a DoS. They use it again.

Tried and reverted: a thread-local memo of lexed plans per (image, region,
config, purpose). Every hit cloned a plan as large as the region, and every
nested region's plan stayed resident. Run time got worse and RSS reached
3.4 GB.

Proposal for the naming owners: one shared plan per region per analysis.
`native_script_words_in` would hand out `Arc<NativeScriptWordsPlan>` from a
per-analysis store, and `capture_region` would iterate it by reference. The
static-source retention would then lex each region once instead of once per
descendant command.

### Verification

- `tcl-compiler` lib: 10,224 tests at HEAD and at the parent commit, via a
  resumable runner because the harness aborts. Failure sets are identical.
- 11 tests abort or time out on both trees: the `depth_guard` stack-budget
  tests, several `taint` depth tests and one `ssa` test. These are baseline
  state of `work`.

Open:

- The per-region plan sharing above.
- Five `depth_guard` stack-budget tests abort the `tcl-compiler` lib harness
  on `work` (stack overflow) or exceed 15 min. Until they're fixed, every
  plain `cargo test -p tcl-compiler --lib` ends early without a summary.

## 8. Version stamp

`tcl-version`'s build script watched a file that never exists whenever
`TCL_LSP_VERSION` was unset. It did this to keep the `.dirty` marker exact,
so every unstamped cargo call re-ran the script and recompiled every crate
that reads `tcl_version::VERSION`. In a no-op build that meant 65 s for
`tcl-lsp-server`, whose library embeds the version, and 5 s for `tcl-cli`.
Every local `cargo build` or `cargo test` paid it before doing any work.

The probe is now emitted only outside the dev profile (`PROFILE != debug`).
Release builds and stamped CI builds behave exactly as before. Since item 299
an unstamped debug build reads no git state at all: it reports
`0.1.0-dev` with an empty commit, so commits and tag fetches never re-run the
script.

No-op builds after the change: `tcl-lsp-server` **65 s → 0.3 s**, `tcl-cli`
**5 s → 0.3 s**. `cargo test -p tcl-version` passes, and a release build of the
crate still emits the probe.

## 9. Duplicate WASM builds

On a PR that touched the browser server, `lsp-server-wasm` and `test-ext-web`
each did the same release/LTO `make lsp-server-wasm` link on their own runner.
`test-ext-web` built its own copy because `lsp-server-wasm` skipped on a
narrower path filter, and an extension-only change would then have had no
module to test.

`lsp-server-wasm` is now the only producer:

- Its build gate, `BUILD_LSP_WASM`, is the union of its own gate and
  `test-ext-web`'s `RUN_EXT_WEB`.
- It uploads `rust/tcl-lsp-server-wasm/dist` as `lsp-server-wasm-dist` and
  then runs its node session.
- `test-ext-web` needs it, downloads the artifact, and no longer installs Rust
  or `wasm-bindgen`.
- `test-ext-web` runs under `if: !cancelled()`. A failed node session in the
  producer, which runs after the upload, still lets the extension suite report
  on its own. A failed build fails the download, and with it the suite.

`scripts/dev/test-release-dependency-graph.sh` now checks the `needs` edge,
the `!cancelled()` guard, and that exactly one `ci.yml` step runs
`make lsp-server-wasm`. Negative controls (no producer, a second producer)
fail it.

The release path is unchanged: `build-vsix` still builds its own payload under
the release stamp, because a published module must carry the tag's version.

Trade-off: `test-ext-web`'s `npm ci` and Chromium install now start after the
link instead of alongside it. That adds a few minutes of wall time to that leg
and saves one full LTO link of runner time on every qualifying run.

### Stale references from items 4 and 5, fixed alongside

Checking the CI contract scripts found references the crate split and the
binary merge had left behind:

- `scripts/dev/{rust-tests,lsp-e2e}-package-paths.txt` lacked
  `rust/tcl-codegen`. Their `test-*-paths.sh` contracts failed, and a
  `tcl-codegen`-only change would not have triggered those suites.
- `--test <name>` commands for merged modules now name the merged binary and
  a module filter (`--test it_2 codegen_golden::`):
  - the `bytecode-compare` skill
  - the `reference_doc` regenerate hint, both in code and in `fields.md`
  - `pack_torture`'s ignore reason
  - `release_optimiser_safety`, `early_cutoff` and `references_rename` in the
    design and work docs
- `wasm_tiers` references now say `-p tcl-codegen`.
- Dated receipts (`*_RECEIPT_*.json`, `validation-2026-11-04/`) record what was
  run at the time and are left as they are.

Every `-p <pkg> --test <name>` in the tree now resolves to a real target.

## 10. Runner queueing

Most of the queueing defences were already in place:

- `ci.yml` cancels superseded runs per ref.
- The `tank` shard keeps every pending job with `queue: max`.
- The diagnostic workflows deliberately never cancel.

Two sources of bursts remained:

- **`resolution-oracles.yml`** runs a 45-minute reference matrix on every PR
  push that touches `rust/**`, and it had no concurrency group. Each push
  stacked another full run behind the last. PR runs now share one group per
  ref and cancel the superseded head. Other events group per commit, so a
  landed commit's run is neither cancelled nor displaced from the queue.
- **Dependabot** had seven weekly ecosystems (actions, pip, three npm trees,
  cargo, gradle), all on the default day. Each grouped PR runs the full CI
  pipeline, so one morning queued seven pipelines at once. The schedules are
  now staggered Monday to Friday. Security updates still open as soon as an
  advisory lands.

Open:

- Confirm on the first PR after this lands that superseded oracle runs show
  as cancelled.

## Keyed lookups instead of repeated scans

The aim was lookups that scan a large collection for a key on every call.

Converted:

- **`native_registration_lookup`:** a full registry scan per new identity,
  now a per-dialect slot map (see 7).
- **Registry-wide queries:** `object_command_method`,
  `manufacturer_methods` / `is_manufacturer_method`,
  `is_possible_class_construction_word`, `data_collection_protocol` and
  `data_collection_collect_command`. Each walked every registered command per
  call to find a handful of specs. Those specs are now derived once per
  registry state into the existing derived-data cache, in the same iteration
  order, and reset on every mutation.
- **`NamespaceLedger`:** its lazy key index now records each key's row count.
  `source_namespace_creation` and `retained_scope_for_source_advice` used to
  scan every row to prove a key unique; they now read the index.

Left as scans, deliberately:

- A grep of keyed `iter().find/position/any` found 27 struct-field sites,
  1,452 `any(==)` and 4,130 `contains(&)`. Nearly all are per-object lists of
  a few items (subcommands, class members, abbreviation keywords, timers,
  array-search handles, a pack's files, one literal word's variables). There a
  linear scan beats hashing, and several need ordered prefix matching.
- `HashMap` iteration order would also leak into diagnostics and generated
  output that are currently deterministic.

Each conversion keeps the original iteration order where it can affect a
result. Each passes its crate's suite with HEAD's failure set: registry 13/13
identical, compiler identical across 10,224 tests.

## Round 2: the follow-ups

Done in this round, each verified against the parent commit's failure set
for its crate:

- **Test debuginfo off in the CI shards and the agent environment.**
  `CARGO_PROFILE_TEST_DEBUG=0`; about 150 MB of line tables per test harness.
- **Verified cargo-nextest build cache.** Every nextest job (5 shards, heavy,
  the lsp-e2e archive and 3 partitions) spent 150-245 s compiling the pinned,
  patched Nextest. `validate.py` now takes `TCL_LSP_NEXTEST_CACHE`. A cached
  binary is used only if it still reports the pinned version and matches the
  SHA-256 recorded when it was built; the reporter smoke checks run either
  way. Locally: cold 206 s, warm 3 s.
- **`tcl` start-up.** `tcl-lexer` (opt-level 2) and `xxhash-rust` (3) are
  optimised in the dev profile, and the pack segmenter uses `LineIndex`.
  A debug `tcl diag` went from 1.3 s to 0.7-0.86 s. Demand-driven pack
  loading had already landed upstream.
- **Lint and build blockers on the tip.**
  - Two unused methods failed every `CARGO_BUILD_WARNINGS=deny` build;
    they now carry `#[expect(dead_code)]` with a reason.
  - 245 `mutable_key_type` false positives from the lazy `SourceImage`
    geometry; `clippy.toml` exempts the type.
  - Duplicate `#[path]` support modules left by the test-binary merge.
  - A test that no longer compiled (`interned_gc`).
  - About 45 upstream clippy findings.
  - Workspace clippy and `cargo fmt --check` are clean.
- **Stack-budget and hang fixes** (crash list from item 7):
  - Source binding, lowering and taint recursions held whole-function
    temporaries across their recursive calls in the dev profile. Each now
    decides in `#[inline(never)]` leaves and carries one boxed plan across
    the call. Measured: `walk_prepared_source_target` 3,120 -> 560 B,
    `lower_default` 27.9 -> 6.8 KB, `substitution_word_taint` 5.8 -> 1.3 KB.
  - The materialized-footprint walk now caps re-evaluated bodies at
    `MAX_SOURCE_NEST_DEPTH`; the 256 bracket-text cap had let a deep body
    reach 1.6 MB.
  - Exact literal-list `foreach` unrolling multiplied with nesting
    (2^62 body walks). It now spends from a per-walk budget of 256
    iterations, then takes the general join-until-stable walk. Each admitted
    iteration walks its whole remaining body, so the cost is the budget
    times the nest. Measured on a debug build: depth 8 took 16 s at 4,096,
    8 s at 256 and 2 s with unrolling off; depth 16 takes 7 s with it off.
    The 555 failures among 2,333 `command_binding`, `variable_bindings`,
    `realm` and `source_` tests are identical at 4,096 and 256.
  - CFG `foreach` lowering held barrier, call and loop-scaffolding
    temporaries in the frame that recurses into the body (29 KiB a level),
    overflowing a 2 MiB thread at depth 51. The opaque shapes and the loop
    open/close steps are leaves now.
  - Now passing: `the_source_walk_cap_fits_its_stack_budget`,
    `source_bindings_with_computed_conditions_fit_the_shared_stack_budget`,
    `proved_structured_bodies_fit_the_shared_stack_budget` (57 s, was 300),
    `the_analyser_stops_at_the_shared_cap` (268 s; it overflowed, and
    before that ran past 15 minutes) and
    `taint::tests::deeply_nested_taint_walks_survive`.
- **Slow-test budget.** nextest's default profile reports SLOW each minute and
  ends a test at 20 minutes; a hang had held shard 5/5 for 6,315 s.
- **sccache report.** The shard job summary shows the Rust hit rate, and the
  job warns below 70%.
- **Cross-crate owner routes.** The source-route checker follows a `static`
  route into a dependency crate through plain `pub` paths only. The three
  free-function routes from `tcl-codegen` into `tcl-compiler` are checked. The
  gate's other 95 problems on the tip are unchanged.

Decided against, with the evidence:

- **Build-once test archive (5.3).** Per-shard compile is 254-325 s. One
  producer compiling the union of all shards would put roughly twice that,
  plus archive transfer, in front of every shard. Hosted minutes cost nothing
  here and wall time is what matters, so it would be a net loss. The nextest
  cache removed the bigger per-shard cost.
- **Splitting `tcl-lsp-server/src/lib.rs` (73k lines).** Modules within one
  crate compile as one unit with incremental off, so a module split buys no
  build time. A crate split means untangling `Backend` while other sessions
  edit the file daily. *Reopened 26 November 2026* under the crate split's
  goal of many small crates (name-handling plan item 290). `lib.rs` itself is
  now 1,359 lines, with `Backend`'s `impl` blocks spread over 120 files.
- **Dropping `aws-lc-sys`.** It costs about 65 CPU-seconds, off the critical
  path. `ring` would drop `rustls`'s post-quantum key exchange and change
  `russh`'s backend for SSH to BIG-IPs. That is the BIG-IP client owners'
  call.
- **Thinner `xtask`.** Only `wasm-backing` needs `tcl-vm` and `tcl-runtime`.
  An opt-in feature would make the CI drift leg build `xtask` twice (batch
  and single gates). As a default feature it changes nothing. *Reopened
  26 November 2026* as name-handling plan item 298: a separate binary for
  `wasm-backing` instead of a feature.
- **Shared wasm32 cache key.** The three wasm jobs build different
  workspaces, lockfiles and targets. A shared key would only make them
  overwrite each other.
- **History rewrite.** It would force-push a branch others push to many times
  an hour. Not done without an explicit go-ahead.

Still open:

- **Per-region plan sharing (item 7).** One shared plan per region per
  analysis, in the naming owners' static-source retention.
- **Two tcl-compiler tests now fail fast instead of crashing or hanging.**
  They are real faults for the compiler owners:
  - `unrelated_command_observer_keeps_structured_bodies_within_stack_budget`
    (46 s, 0 of 62 levels). A `trace add command` on an unrelated proc
    makes even a two-level literal `foreach` nest lower to opaque `Call`s.
    Without the trace the nest stays structured. The unrolling budget is
    not involved: depth 2 is well inside it.
  - `ssa::…scalar_alias_write_after_unset_reaches_both_spellings` (0.8 s).
    Its failure messages printed the whole SSA with `{:#?}`, which tried to
    allocate 12 GB. They now print a per-statement outline. The fault:
    after `upvar 0 x y; unset x; set y NEW`, `puts $x` reads the new
    version but `puts $y` reads a later one.
- CI run 3 (`42324892928`):
  - `test-ext-web` used `lsp-server-wasm`'s module, so item 9 is confirmed.
  - The nextest cache missed on its first run, because every job started
    together. The next run should hit.
  - The lint leg got as far as three newer upstream `too_many_lines`
    findings in `tcl-vm`, now fixed.
- CI run 4 (`42331787084`, tip `ad559ef`):
  - Fixed: the standalone wasm workspaces' `Cargo.lock` files lacked the
    `rustc-hash` dependency the keyed-lookup sweep added (`cargo-deny` and
    the drift leg). Ruff flagged the upstream license-header script. An
    upstream runtime method was dead code without TomMath (runtime and
    doctest legs).
  - The lint leg passed.
  - Already failing in the last pre-session run (`41781045833`), so not
    from this work:
    - lsp-e2e partitions, `lsp-server-wasi`, `rust-tests-heavy` (live iRule
      harness), and shards 3 and 5. The shards fail with "module requires a
      genuine native compiler preflight provider" in `tcl-vm` and "DSL phase
      requires its complete original source" in `bpf-tcl`.
    - `spectcl-compat`: the string pack's hooks make no attempts.
    - `wasm-real-link`: `samples/wasm/budgets.tsv` has drifted.
    - The VS Code partition manifest. `codeActionKinds.test` was added on
      4 November without a partition.
  - Shards 2 and 4 lost their runners mid-run.
- CI run 5 (`42340307964`, tip `a557fbe`):
  - The nextest cache hit in all ten nextest jobs. "Install cargo-nextest"
    took 3-12 s, against 150-245 s when built from source.
  - `cargo-deny` and the lockfile drift check passed, and ruff passed.
  - `ty` and `pyright` then flagged unnarrowed Optionals in four scripts,
    two of them from this work. Fixed; both are clean locally.
  - Further steps can now run, and they fail on faults unrelated to this
    work:
    - The drift leg's `check-tcl-reference-toolchains` needs a C 8.4
      source tree the job does not fetch.
    - 104 `tcl-runtime` unit tests fail. Several need `TCL_LSP_JIMSH` or
      a C oracle; several are clock tests.
    - The `naming::rooted_member_key_bytes` doctest in `tcl-syntax`
      returns `::a::::v` where its example expects `::a:::v`.

## Round 3: CI failures fixed or diagnosed

Fixed:

- **VS Code partitions.** `codeActionKinds.test` (3 tests) now sits in
  partition 2. The counts in the manifest and its three checkers rise to
  110 files, 1,014 identities and 1,013 passes.
- **`naming::rooted_member_key_bytes` doctest.** The example miscounted
  colons; the function matches its prose.
- **Drift leg.** The pinned C 8.4 formatter comparison needs the built 8.4
  archive. It now runs as `make test-c84-native-formatter` in
  runtime-rust-tests, after that job builds the tree, and the drift check
  skips it. Locally the archive builds but its hash differs from the pin,
  so it can only pass on the CI image the pin was recorded on.
- **BPF-Tcl.** No program with an `if` or a child body compiled:
  - `discard_execution_proofs` did not clear the later `namespace_context`
    field, so the syntactic body comparison refused.
  - The closed-effects witness refused every variable in a condition. A
    plain identifier is a DSL register, so it is now admitted.
  - A refused phase now gets a diagnostic-only graph, so `while`, unknown
    commands and coroutines keep their specific diagnostics.
  - 219 of 219 BPF tests pass.
- **Python type checks.** Four scripts.

Diagnosed, not fixed. Each was already failing before this work:

- **`tcl-vm` "native compiler preflight provider" (about 25 of it_3's 68
  failures; similar in codegen and compiler shards).** `throw`, `format`,
  `lreplace` and `string map`/`is`/`replace` get an `Unknown` compile
  selection and no named-compiler proof. Codegen therefore marks the unit
  `ProviderRequired`, and the test VM, which has no provider, refuses it.
  `throw` and parts of `string`, `dict`, `chan`, `prefix`, `const` and
  `lfilter` are registered as `NativeCompilationGrammar::Unresolved`,
  documented as "not yet an authored closed contract". This is unfinished
  porting of their C compilers, not a regression to patch.
- **The other it_3 failures** are spread across 20+ sites: namespace,
  TclOO, release-number and tclsh-parity assertions.
- **`samples/wasm/budgets.tsv`.** 70 rows drifted, some better and some
  much worse. For example `13_expr_ops` native lost all 48 native numeric
  ops and went from 17 to 34 runtime dispatches. Regenerating would claim
  those regressions without a reason, so it was not regenerated.
- **SpecTcl string pack.** The `map` and `range` hooks are never
  attempted on their corpus samples. The `LineIndex` loader change was
  ruled out: it fails the same with that change reverted.

## Round 4: porting the unmodelled C compilers

The preflight refusals diagnosed above are unfinished ports. Each compiler
is ported one at a time from the pinned C sources, checked form by form
against tclsh, and pushed on its own. No VM plain-dispatch fallback was
added. it_3 went from 596 passed / 65 failed to 608 / 53. Each later port
below left it_3 unchanged, because the remaining failures need other
work.

| Compiler | Releases now modelled | Notes |
|---|---|---|
| `throw` | 8.6, 9.0, 9.1 | Literal, empty-type (`BADEXCEPTION`) and dynamic type-check branches. |
| `const` | 9.0, 9.1 | `CONST_IMM`/`CONST_STK`; `try` handler variables and `-during` fixed alongside. |
| `dict create` | 8.6, 9.0 (9.1 constant) | Constant fold, `DICT_PUT` and the worker path. |
| `string is` | 8.6, 9.0, 9.1 | Every class, prefix and `-strict`; 9.1's `IS_EMPTY` and `SWAP`. |
| `regexp` | 8.4 to 9.1 | 8.4 literal-pattern branch; 8.4 guard is chunk entry; 8.4/8.5 push `1` for an empty pattern without compiling the subject. |
| `format` | 8.6, 9.0, 9.1 | All-literal fold, including bignum conversions; `TclCompileSyntaxError` for every `Tcl_Format` failure; the dynamic branch's `%s`/`%%` concat and its generic declines. |
| `string range` | 8.6, 9.0, 9.1 | Constant and non-constant index branches. |
| `string map` | 8.6, 9.0, 9.1 | Inline `STR_MAP`, empty-key identity, Basic2Arg, and the failed-member `INVOKE_REPLACE` path. |
| `string replace` | 8.6, 9.0, 9.1 | Every constant-index recipe, `genericReplace`, and 9.1's `SWAP`. |
| `lreplace` | 8.6, 9.0, 9.1 | 9.x `LREPLACE4`/`LREPLACE`; 8.6's list-range splice and its declines. |
| `string bytelength` | 8.6 | Basic1Arg named invocation. |
| C8.4 `string` | 8.4 | Out-of-line members select an ordinary invocation. |
| C8.5 `string map`/`range` | 8.5 | NULL compileProc selects an ordinary invocation. |
| `tcl::unsupported::representation`, `corotype` | 8.6, 9.0, 9.1 | Created without a compileProc: an ordinary invocation. |

Bugs these ports exposed and fixed:

- **`STR_RANGE_IMM`.** The VM read C index encodings as plain integers, so
  `string range $s 2 end` returned an empty string.
- **`end+1` encoding.** On releases with a 64-bit `Tcl_Size`, C reports
  `end+1` as a range error, which takes the non-constant path. That
  depends on the target's width, so it is now refused rather than encoded
  as `after`.
- **`format %c` on 8.x.** `TCL_UTF_MAX` 3 releases turn a scalar past
  U+FFFF into U+FFFD, both at run time and in compile-time folds.
- **`format` error codes.** Template failures now carry C 8.6+'s
  `TCL FORMAT <kind>` codes; 8.4/8.5 keep `NONE`.
- **8.4/8.5 `lreplace`.** The command now raises "list doesn't contain
  element" and uses its `?element element ...?` usage string.

Still unmodelled or refused by design:

- **A static compile that redefines a compiled builtin.** In
  `run_script::loop_body_redefining_continue_is_honoured` the whole script is
  compiled ahead of time and a loop body defines `proc continue`. The
  source walk merges the redefinition into later lookups and refuses; C
  compiles against the bindings at chunk entry and replays the changed
  command at `INST_START_CMD`. The same script run through the VM's own
  eval path compiles and matches tclsh.

Fixed since the list above was written:

- **Width-dependent indices.** 9.x indices now follow `TclIndexEncode` on
  the 64-bit host, so `end+1` and absolute indices beyond `INT_MAX` take
  C's constant or non-constant path.
- **8.4 expression errors.** A C8.4 body whose expression fails 8.4's own
  parser raises that parser's message and errorCode at compile time,
  with C8.4's global errorInfo.
- **8.5 `regexp`**, **9.1 `string is`** (new `IS_EMPTY` opcode, which
  avoids generating a string for dicts, lists and numbers),
  **`tcl::unsupported::representation`** and **`format` bignum folds**
  (`%d` truncates to 32 bits on 9.x and through a 64-bit `long` on 8.6;
  `%lld` keeps the bignum).
- **`subst` with command substitutions.** `TclCompileSubstCmd` now compiles
  inline in procedure bodies and nested scripts, each `[...]` in its own
  catch range. A runtime `subst` error now logs the invoking command, as
  C's TEBC does.
- **Execution traces.** A traced command compiles as an ordinary
  invocation instead of refusing; only its effects stay unknown.
- **9.0 `info object creationid`.** The worker index kept both the 9.0 and
  9.1 `creationid` descriptors, which disagreed. Sub-subcommand compilers
  are now filtered by release. `info class definitionnamespace` is
  implemented.
- **A namespace recreated under a deleted parent.** A retained child of a
  dying namespace is no longer reported reachable, so the new namespace at
  the same path resolves.
- **`(reading increment)`** and similar frames a command adds to its own
  error now reach errorInfo before the caller logs it.
