# LSP agent (after Luna-41) — measurements, fixes, routed causes

Owned paths: `rust/tcl-lsp-core`, `rust/tcl-lsp-db`, `rust/tcl-lsp-server` (not the BIG-IP
modules). I committed nothing; the coordinator's WIP snapshots carry my edits (last seen in
`56893780b9` and later). Run conditions: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0`, `TCL_LSP_REQUIRE_TCLSH=9.0,8.6`,
`TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`, `--test-threads=2`, each test executable deleted
after its run. Numbers are "HEAD + shared working tree at that moment"; compiler agents landed
changes between runs. Temporary probes (`tests/zz_lsp_agent_probe.rs`) were deleted after use.

## 1. Integration targets (tcl-lsp-core) — before / after

origin/rust passes all of these (validation row 20: 239/239 incl. name_resolution 75/75 on
`fcb7927f02`).

| target | Luna-41 hand-back | first run here (nr0/h1) | latest run | remaining failures (owner) |
|---|---|---|---|---|
| name_resolution | 69/77 | 68/77 | **69/77** (p5) | itcl ×4 (compiler: in-namespace class join + calls inside class-member bodies), `fn_to_tp_an_explicit_namespace_import_is_followed` (C6, compiler-names), `tn_object_command_dispatch_is_scoped_to_its_creation_namespace` (compiler-names, §3a), `irules_switch_glob_body_is_followed` (iRules `proc` invocation, §3d), `pkgindex_dir::fp_other_undefined_var_in_pkgindex_still_fires` (C9 W210) |
| hover_completion | 50/56 | 52/56 | **55/56** (p5) → 56/56 expected with the past-end-of-line fix (pending re-run) | — |
| hover_residual | 27/36 | 28/36 | **33/36** (p4) | option word / 2× regexp pattern with an undefined `$var` operand (compiler, §3c) |
| references_residual | 16/23 | 16/23 | **21/23** (p5) | `document_highlights_dedup_keeps_write_over_read_on_collision` (synthetic `AnalysisResult::default()` + injected reporting map — the original-context contract refuses it; needs a decision), `references_external_obj_method_inside_proc_body_subst` (tclsh contradicts the fixture: `$d` is unset inside `proc run`; needs a decision) |
| references_rename | 70/98 | — | **71/98** (p4) | ~15 refuse `unknown-completed-command-world` (`puts [greet]`, compiler R1 not effective on this tree, §3b), class/object-variable renames refuse (§3b), itcl rename ×2 (in-namespace join), others listed in `scratchpad/lsp/out-p4-references_rename.log` |
| force_import_shadow_consumers | 2/10 | 2/10 | **6/10** (p5, ABSTAIN decision) | 4 not-shadowed halves that never passed on work: inline-proc / code-action offer no local `{args}` body, prepare_rename None, rename refused `unknown-completed-command-world` |
| call_hierarchy | (7/7 row 20a) | — | **7/7** | — |
| inlay_hints | (9/9 row 20) | — | **9/9** | — |

## 2. Library targets and server

| target | origin/rust | here (latest) | notes |
|---|---|---|---|
| tcl-lsp-server lib (840 tests, one process per test, 400 s cap) | all pass (row 20) | **694 ok / 140 FAILED / 6 TIMEOUT** (srv4, tree of ~11:05Z, before the N05/N15 edits) | whole-binary run aborts after ~221 tests (vm-runtime TLS destructor, §3e) so it was run per test. Failure table: `lsp/server-lib-srv4-failures.txt` (first panic line per test); status: `lsp/server-lib-srv4-status.txt`. 10 are `Elapsed` waits in the publication/worker tests and 4 of the 6 timeouts are analysis-cost bound (§3f); most of the rest are cross-document/source-rehoming expectations that the fresh-interpreter contract no longer reports (`[]` references/definitions for sourced or imported declarations, missing workspace subtype rows, lens counts). Not classified per owner in this session. `workspace_trust_comes_from_the_client_and_never_from_a_setting` was not measured (runner artifact: binary already deleted). |
| tcl-lsp-core lib | all pass | PENDING (see §6) | |
| tcl-lsp-db lib | all pass | PENDING | |
| tcl-lsp-server e2e | | not run (disk) | |
| clippy (my crates, `--all-targets`) | | 152 (Luna-41 baseline 1,059 → 152); not re-measured | |

## 3. Root causes routed to other owners (evidence: `lsp/probe-results.txt`)

- **a. compiler-names** — an object created by a relative name inside `namespace eval ::a` is published at root (`::a`'s `rex` → `::rex`; `::b`'s → none) → `tn_object_command_dispatch_is_scoped_to_its_creation_namespace`. C6 (`namespace import ::a::Factory; Factory make`), C9 (W210 in pkgIndex `[file join $dir $missing]`).
- **b. compiler-names (R1)** — rename refuses `unknown-completed-command-world` for `proc greet {} {…}; puts [greet]` (~15 references_rename tests, force_import unshadowed rename); `incomplete-original-command-coverage` for a class split across two `namespace eval ::a` blocks; class `variable n` rename "original command unavailable". The triage report (`reviews/naming-migration-review-20261022-triage.md`) lists N04–N07 as "refused, pending R1" for the same reason.
- **c. compiler** — the registry source schema is withdrawn when an operand reads an undefined variable (`regexp {plainword} $line`, `lsearch -exact $list $needle`) → 3 hover_residual tests. itcl classes declared inside `namespace eval ::app` do not join contract D; class-proc calls inside class-member bodies → 4 itcl name_resolution + 2 references_rename.
- **d. compiler (iRules realm)** — a top-level `proc hit` in f5-irules: the invocation lacks a resolved reference / original lookup → `irules_switch_glob_body_is_followed`.
- **e. vm-runtime** — tokio worker thread exit drops a thread-local PackHookHost → tcl_vm `Value` drop → `value::native_retirement::release` touches already-destroyed TLS → panic in destructor → SIGABRT of the whole tcl-lsp-server lib binary. Needs `try_with` in the retirement path.
- **f. perf agent / compiler** — Analyser cost quadratic in command count (§4 numbers) and `tcl_spectcl::bundled::load_embedded` 65–84 s. Causes both 180 s server tests (`workspace_symbol_caps_its_answer`, `startup_index_and_factory_reindex_keep_checked_document_roles`), all WASI row-17 failures and the folder-add "hang" (host `/usr/share/tcltk/tk8.6/tclIndex`, 253 lines, analysed per scan; gdb samples in `lsp/folder-add-hang-bt.txt`). Timeouts were not tuned (coordinator instruction).


## 4. WASI host failures (validation row 17) — diagnosis

No wasip1 build was made (a release LTO wasip1 build of the whole stack needs well over the
shared ~2 GB). Diagnosis from code plus native measurements:

**Mechanism (confirmed in code).** `tcl_lsp_server::rt::spawn_blocking` on wasip1 runs the
closure *inline* on the only thread (`rt.rs`, `mod wasi`), and `rust/tcl-lsp-server-wasi/src/driver.rs`
only reads stdin / routes `exit` between task polls. Every analysis and the spec-pack load
therefore block the driver for their whole duration; nothing is lost, it is just late.

**Measured cost on this tree (native, debug test profile, `scratchpad/lsp/out-p4-zz_lsp_agent_probe.log`).**
- `Analyser::analyse("proc p {a} {return $a}\n" + "p 1\n"×n)`: n=10 3.1 s, 20 5.9 s, 40 17.7 s,
  80 66.0 s — quadratic; n=500 did not finish in 15 min. The e2e `exit`+stdin-close scenario
  opens exactly this shape with n=4000.
- 41 empty procs: 8.5 s per analysis.
- `tcl_spectcl::bundled::load_embedded()`: 84 s cold, 65 s warm (second call, cache redirected).

**Each failed check explained.**
- `exit` + stdin close ×12 timeouts: `didOpen busy.tcl` → inline analysis of 4000 calls; the
  driver never gets back to `route()` within 30 s.
- idle session, exit without shutdown → timeout: after the 10 s configuration deadline the
  server loads spec packs inline (`reload_spec_packs` → `spawn_blocking` →
  `load_discovered_configuration_with_studio_sources_and_inputs_in`), longer than the 30 s
  exit budget.
- main session, APP `publishDiagnostics` missing within 60 s, then BROKEN arrives 6 s later:
  APP's didOpen queues behind the post-`initialized` pack load + workspace scan; BROKEN is
  opened after they finish.
- go-to-definition / hover into `lib.tcl` empty: issued while the workspace scan / source
  rehoming is still pending behind the same work (no index row for `lib.tcl` yet).
- cache scenario: the cache directory is checked right after the `initialised` log line, i.e.
  before the (inline, slow) pack load wrote it; "analysis still answers" then fails exactly
  like the APP publish above.

**Owner.** Compiler (analysis cost superlinear in command count) and vm-runtime/tcl-spectcl
(pack load), routed to the coordinator; a dedicated perf agent was started. Server108
(below) only removes *repeated* analyses of an unchanged revision.

**Exact verification steps once disk allows (~2.5 GB free for the wasip1 target dir):**
1. `bash rust/tcl-lsp-server-wasi/build-wasi.sh` (or `CARGO_PROFILE_RELEASE_LTO=false` to save
   time/disk), then `node rust/tcl-lsp-server-wasi/test/e2e.mjs`.
2. Natively first, cheaper: time `Analyser::analyse` on the busy fixture for n=100/200 and
   `bundled::load_embedded()` in `--release`; if analysis is not ~linear the WASI checks
   cannot pass whatever the host does.
3. After the perf fix: rerun step 1; expect all 26 checks. If only the cache check still
   fails, wait for the cache directory with `until(...)` instead of checking once (test-side
   race, needs a decision since it changes the harness).

## 5. Fixes landed (all uncommitted by me; carried by the WIP snapshots up to `ddb21ae4db`)

**Server108 — cached per-document analysis.** `rust/tcl-lsp-server/src/fresh_analysis_cache.rs`
(new): bounded LRU (512 entries, 64 MiB of source) keyed on uri, exact text, dialect, path
admission, analyser-inputs epoch, class-factory generation and the configured analyser inputs
(disabled checks, non-ASCII mode, extra commands, pack key, resource). `fresh_analysis_seed`
reuses a hit only after the same currency checks as a fresh seed; sidecar invalidation clears it.
Unit test `fresh_analysis_is_reused_only_for_an_unchanged_input_revision`. It removes repeated
analyses of an unchanged revision; it does not address the first analysis (§3f).

**Forced-import ABSTAIN (user decision 2026-10-22T07:18Z).** `definition::program_forced_import_abstains`
(program oracle says a forced import shadows the call, the document alone does not) is consulted
by definition, hover, references, signature help, inlay hints, call-hierarchy prepare, inline
proc, rename and prepare-rename. 8 force_import_shadow_consumers assertions changed from the
shadowed direction to abstention, each citing the decision. Sibling tests place their cursor on
the call head word (decision 2026-10-18T16:00Z, helper `last_call_head`).

**Review N05 — rename edits ⊆ references.** `rename_in_program` refuses with
`unreferenced-edit` any edit that find-references from the same cursor (same program oracle,
declarations included) does not report, and with `force-import-shadow-conditional` any edit on
an abstained call. Regressions: `references_rename::n05_rename_never_edits_an_occurrence_references_does_not_report`
(before/after `-force`, `forget`, re-import, redefinition) and
`n05_an_accepted_rename_keeps_the_program_output` (tclsh 9.0 outputs `A`, `B\nA`, `B`, `C`
before and after); cross-file: `force_import_shadow_consumers::rename_and_references_from_the_local_declaration_agree_on_the_unproven_call`.

**Review N15 — minifier name compaction refused.** `minify/original.rs`:
`const LEXICAL_NAMING_PASSES: bool = false`; the lexical ("tcl") path now refuses Compact /
Aggressive renaming exactly like the native path (`MissingAlphaRenameContract`, plus alias /
semantic / keyword refusals for Aggressive) and runs syntax compaction only. Every public entry
(`minify_tcl_compact`, `minify_tcl_aggressive[_with]`, `minify_with_profile`,
`minify_with_analysis`, the server's minifyDocument) goes through this gate. `compact_names`,
`lexical_compact` and `lexical_aggressive` stay compiled but unreachable — **delete them when
the transactional naming planner lands.** Regression `minify_residual::n15_compact_and_aggressive_refuse_unsafe_renaming_and_keep_the_output`
(tcl / tcl8.6 / tcl9.0 × Compact / Aggressive, isolated). **Release notes must state the
temporary restriction** (compact/aggressive minify no longer shorten names). The list of lib
tests whose expectation changed is in §6.

**Host-install decision 2026-10-19T11:00Z.** `test_backend_over` presets an empty discovered
installation survey (comment cites the decision); new pinned-fixture test
`scan_workspace_folders_surveys_a_pinned_discovered_installation`; per-thread plain-data memo
of tclIndex surveys (`package_resolver/original_names.rs::original_index_entries`, ≤64 rows,
≤4 MiB, existence rechecked per call) + unit test. (A first static `Mutex<Arc<AnalysisResult>>`
memo overflowed the `Sync` proof — E0275 — for other agents' builds and was reverted within
minutes.)

**Provider fixes (hover / completion / references / name resolution).** Contract D itcl
`Class::proc` dispatch (`method_symbol.rs` `Target::SourceClassProcCall`); class
superclass/mixin operand references; declaration-name end-of-word cursor tolerance;
constructor/destructor keyword hover; built-in vs package command hover labels and scoped
operation cards; built-in completion and intrep hover lines only when the caller supplies a
registry; scoped-body operation completion; 8.x namespace→global variable completion
(`namespace_var_global_fallback`); `$` completion at a past-end-of-line cursor; highlights of an
undeclared variable are empty.

**Narrow documented test corrections (coordinator rulings).**
- (A) `references_residual::document_highlights_dedup_keeps_write_over_read_on_collision`: the
  synthetic `AnalysisResult::default()` + injected reporting map is refused by the
  original-context contract; replaced by a real source collision `set x 1\nincr x\n` (cursor
  1:5, one Write highlight).
- (B) `references_external_obj_method_inside_proc_body_subst`: tclsh shows `$d` unset inside
  `proc run`; fixture gains `global d` and a new negative test
  `…without_global_is_not_a_site` asserts no reference without it.
- classmethod fixtures in hover_residual / references_residual run under tcl9.0 (decision
  2026-10-17T20:30Z: `classmethod` is 9.0+).
