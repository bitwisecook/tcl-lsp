# Build and test cost: current state

Updated 5 November 2026. This file is a current snapshot of the build- and
test-time work on `work`; git records the sequence. The measurements and the
ranked plan it works through are in the report
<https://claude.ai/artifact/BFfNK69TYvCcQMzTRqQLKi> (private; ask the owner
for access).

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
Release builds and stamped CI builds behave exactly as before. A debug build
still re-stamps when HEAD, the current branch ref, tags or `packed-refs`
change. Its `.dirty` marker can lag behind an uncommitted edit until the next
such change. A debug binary is never shipped, so the lag is acceptable.

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
    (2^62 body walks). It now spends from a per-walk budget of 4,096
    iterations, then takes the general join-until-stable walk.
  - Now passing: `the_source_walk_cap_fits_its_stack_budget`,
    `source_bindings_with_computed_conditions_fit_the_shared_stack_budget`,
    `proved_structured_bodies_fit_the_shared_stack_budget` (about 300 s) and
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
  edit the file daily.
- **Dropping `aws-lc-sys`.** It costs about 65 CPU-seconds, off the critical
  path. `ring` would drop `rustls`'s post-quantum key exchange and change
  `russh`'s backend for SSH to BIG-IPs. That is the BIG-IP client owners'
  call.
- **Thinner `xtask`.** Only `wasm-backing` needs `tcl-vm` and `tcl-runtime`.
  An opt-in feature would make the CI drift leg build `xtask` twice (batch
  and single gates). As a default feature it changes nothing.
- **Shared wasm32 cache key.** The three wasm jobs build different
  workspaces, lockfiles and targets. A shared key would only make them
  overwrite each other.
- **History rewrite.** It would force-push a branch others push to many times
  an hour. Not done without an explicit go-ahead.

Still open:

- **Per-region plan sharing (item 7).** One shared plan per region per
  analysis, in the naming owners' static-source retention.
- **Command-trace observers and nested loops.** Eight nested two-element
  `foreach` loops under a `trace add command` observer exhaust 14 GB.
  `unrelated_command_observer_keeps_structured_bodies_within_stack_budget`
  (now an assertion failure) and `ssa::…scalar_alias_write_after_unset…`
  (OOM-killed) come from this.
- **`the_analyser_stops_at_the_shared_cap`** still exceeds 15 minutes.
- CI run 3 (`42324892928`):
  - `test-ext-web` used `lsp-server-wasm`'s module, so item 9 is confirmed.
  - The nextest cache missed on its first run, because every job started
    together. The next run should hit.
  - The lint leg got as far as three newer upstream `too_many_lines`
    findings in `tcl-vm`, now fixed.

