# analysis-performance: pause handoff, 18 November 2026

The working tree was clean at the pause: everything finished is on `work`.
Two pieces of work are parked:

- The per-body wrapper realm, held at
  `docs/work/name-handling/patches/handover-analysis-performance-body-wrapper-realm.patch`
  (landed earlier as docs).
- The flake hunt. It has no code; its partial measurement is below.

There are no patches under `pause-2026-11-18/patches/`, because nothing
uncommitted existed.

## (a) Landed today (17–18 Nov), on `work`

### #2543: server memory growth under pack churn

| Commit | What it does |
| --- | --- |
| `ac289446` | Per-document lowered-module memo `lowered_document_module(file, ModuleLoweringKey)`. tcl-compiler `ModuleLoweringInputs` is built by an exhaustive destructure, and `LoweredModule` is new. |
| `43d017e0` | Routes `compilation_unit` and `proc_taint_solve` through the memo. The context generation in the key is compared by value (`same_context_generation`, `ResolvedAnalysisInput::same_by_generation`). Adds the evaluator generation and overlay epoch to the key. |
| `8febb23f` | Retained-module table. Salsa's interned GC was reclaiming `ModuleLoweringKey` within its 3-revision window; found by the `server` issue session. |
| `2a026629` | The table becomes a per-database salsa singleton input (`RetainedModules`) and evicts the LRU entry instead of clearing. |
| `a78bcdc0`, `261f4c35`, `3fff2d03` (ids before rebase) | The `server` session's three _2543 tests, cherry-picked with its authorship. |

### Other landed work

| Commit | What it does |
| --- | --- |
| `d69a82ae` (rebased id) | `object_handle_facts` report-less comparison is now module-relative (aliases.tcl two-analysis equality; routed from class-facts). |
| `1170e4ae` | Phi budget tests (ledger rows 23 and 24) re-pointed at `phi_undef_trace_steps()`, ≤ 2.2× per doubling. |
| `f3d891bc` | Shared-prefix skip in `retain_possible_effects`. Phi 60 stages: 15.9 s → 11.6 s. |

### Nesting

| Commit | What it does |
| --- | --- |
| `5ac0b9e5` | Witness, red. |
| `be6f1362` | Exact memo of `capture_in_script_body`; barrier steps down about 40×. |
| `1a3a7cd6` | Clippy fix to `be6f1362`. I pushed `be6f1362` before clippy had finished. |
| `51110dc3` | Witness holds the ruled quadratic ceiling (≤ 4.0×), with the tclsh transcript in its comment. `deeply_nested_bodies_survive_when_scanning` comment rewritten. |
| `a25d0d7e` | `outstanding/analysis-performance-nesting-constant.md`: measured shares and three proposals in other owners' code. |

### Docs and ledger

- The per-body wrapper held-patch updates.
- Ledger rows for every test above.

## (b) In flight

1. **Parallel-only flake hunt.** Coordinator queue item. The tests are
   `command_binding::source_callable_static_tests::static_formals_update_one_retained_slot_across_actual_calls`
   and
   `command_binding::source_command_world::tests::original_opaque_jim_callable_name_does_not_gate_ascii_static_capture`.
   Measured at `2a026629`, opt-level 1, test binary directly:
   - Each passes alone.
   - Each passes 3/3 with its own module at `--test-threads 1` and at 8.
   - Both fail 3/3 when the whole `command_binding::` filter runs with
     `--test-threads 1`.
   - So the failure is deterministic and order-dependent: a process-wide (or
     thread-local, since one thread runs them all) cache keyed without some
     input.

   Next step: bisect the `command_binding::` test list with
   `--test-threads 1`, run before the failing test, to find the poisoning
   neighbour. Then name the cache and its missing key component. Suspects:
   the thread_local caches in `command_binding/original_command_table.rs`
   (lines 34, 116, 291), `conditional_body.rs:1144`,
   `incomplete_body_metadata.rs:56`, `source_point.rs:135`,
   `command_observers.rs:29`, and my own `BODY_BARRIERS` and
   `REGION_PLANS` in `registry_invocation/vendor_source_barriers.rs`.
   Rule mine out first: run with `be6f1362` reverted. Ledger rows have been
   added (FLAKY, open).
2. **Per-body incremental analysis.** The held patch
   `handover-analysis-performance-body-wrapper-realm.patch` applies with
   `git apply` on work as of 18 Nov morning. It may need a rebase.
   - `PROBE_WRAP=1` turns it on.
   - Residue: the two security parity tests,
     `source_paths::...per_item_value_parity` and
     `source_patterns::...per_item_parity`.
   - Remaining after that: the method wrapper; removing the env switches;
     landing one issuer per commit with ids sent to analyser-binding; the
     tcl-lsp-db memo tests; then telling server-e2e-db.

## (c) Queue, in order, with why

1. **#2543 heap residue**, the server's top memory issue. At `8febb23f`,
   40 documents × 200 rounds, the salsa counts are flat, but the trimmed
   heap still grows about 2.7 MB a round over rounds 100→200. The `server`
   issue session is attributing it with heap snapshots and was re-measuring
   `2a026629` (40- and 183-document runs) at the pause.

   | state | CfgContext | FnLatticeKey | trimmed heap at round 200 | heap per round, rounds 100→200 |
   | --- | --- | --- | --- | --- |
   | before (0f09b12e era) | 316 (at round 60) | 803 (at round 60) | 4.04 GB | 9.5 MB |
   | ac289446 | 219 | 515 | 2.88 GB | 4.8 MB |
   | 8febb23f | 64 (flat) | 166 (flat) | 2.45 GB | 2.7 MB |

   Lattice-key counts are at round 200 unless marked. 43d017e0 was
   202 / 432 / 3.16 GB.
2. **Flake hunt** (b1). It is the same family as #2543 (state shared
   between analyses), so it may be a correctness fault in the long-lived
   server.
3. **tcl-lsp-db integration tests that take minutes or never finish.**
   server-e2e-db is naming them; I haven't received the list.
4. **Nesting constant on real code.** Depth 400 takes 5.9 s per
   `find_enclosing_when_event` against tclsh's 19 ms at depth 1000. The
   proposals are in `outstanding/analysis-performance-nesting-constant.md`.
   They need owner rulings first, so measure on real tcllib/iRules code
   before acting.
5. **`edit_session_memory_growth_plateaus` rewrite**: count-based, ruled
   18 Nov, not done. See (e).
6. Per-body incremental analysis (b2), then the structure-only N=600 target.

## (d) Waiting on, and waited on by

- Waiting on the `server` issue session (`session_017u5g3HDHiPN1CimB7QiVSF`):
  the measurements on `2a026629` and the heap-residue attribution.
- Waiting on the coordinator: owner names, or permission for me to take,
  the three nesting-constant proposals.
- Waiting on server-e2e-db: the list of slow or non-finishing tcl-lsp-db
  tests.
- Others are waiting on me for: the flake hunt finding (coordinator); the
  per-body landing (server-e2e-db, analyser-binding).

## (e) Rulings not yet carried out

1. `edit_session_memory_growth_plateaus` (ledger REWRITE):
   - First check that the per-quarter interned counts are deterministic
     (same edit script twice in one process).
   - If they are, assert counts against (largest per-revision live count) ×
     (GC window + 1), with the window read from where the server sets it.
   - Fix the stale comment ("byte-identical q2=q3=q4=0").
   - Say whether the late rise is revision-size variation or never-reused
     slots.
2. The flake hunt: name the cache and its missing key, and route the fix to
   that cache's owner.

## (f) Last measured counts

- tcl-compiler, full run at `be6f1362` (with and without the memo):
  13160 run, 1894 failed, identical.
  Command: `cargo nextest run -p tcl-compiler --no-fail-fast`.
- tcl-lsp-db at `2a026629`, oracles required: 256 run, 188 passed,
  68 failed. Without the commit: 254 run, same 68 failures.
  Command: `cargo nextest run -p tcl-lsp-db --no-fail-fast`.
- Barrier witness at depth 50/100/200: 6915 / 18977 / 55877 steps.
- `find_enclosing_when_event` per call, opt-level 1, at `be6f1362`: depth
  200 → 2.8 s, 400 → 5.9 s, 800 → 14.4 s.
- Phi 60 stages, opt-level 1: analyse + unit + checks = 10.3 s. Scales about
  2.8× per doubling (VariableCellTable join, dataflow's area).
- Gates: `make xtask-gates` 39/39 at `2a026629`.
- Environment for every measurement:
  - `source scripts/dev/agent-build-env.sh`
  - `CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`
  - Oracles: `TCL_LSP_JIMSH=tmp/jimtcl-oracle/jimsh`, from
    `scripts/dev/ensure-jim-oracle.sh`, and
    `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1`.

## (g) What a successor would get wrong

- **Disk.** It fills repeatedly. First `rm -rf /tmp/tcl-spectcl-cache-test-*`
  (leaked per test process), then `target/debug/incremental`, then stale
  `target/debug/build/*/<hash>` directories (keep the newest two). Never
  print full nextest failure output to a file: panic messages dump whole
  units and reached 1.2 GB per file. Use
  `--failure-output never --success-output never`.
- **`cargo fmt -p tcl-compiler` reformats other sessions' unformatted
  files.** Use `rustfmt --edition 2024 <your files>`.
- **Run clippy and let it finish before any push.** Don't chain the push
  after a grep of clippy output.
- **An empty `PROBE_WRAP=` still counts as set** (`var_os`). Unset it.
- **`ModuleLoweringInputs` equality.** The context generation compares by
  value, not by Arc; `fresh_document_input` reassembles it per build under
  an overlay. Adding a lowering input means adding it to the struct, and
  the exhaustive destructure makes compilation fail until you do. Keep the
  each-input test in step.
- **`RetainedModules` exists only for databases built through
  `TclDatabase::with_storage`** (`Default`, `with_event_logger`). Literal
  `TclDatabase { storage }` test databases retain nothing. That is
  deliberate and exact.
- **Restoring a cached fallback lattice** (`restored_source_proofs`) is
  less precise than a fresh build by design, because
  `PointResolveContexts::restore_source_tokens` marks argument points
  unknown. Do not memoise the fallback tier by rebinding; that was ruled
  out on 18 Nov.
- **Interned slot counts** (`IngredientInfo::count`) are a high-water mark
  inside salsa's 3-revision GC window, not a live count.
