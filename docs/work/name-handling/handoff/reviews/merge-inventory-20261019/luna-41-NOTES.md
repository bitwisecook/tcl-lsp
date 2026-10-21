# Luna-41 — LSP crates: measurement, root causes, fixes, clippy

Agent: Luna-41 (LSP). Owned paths: `rust/tcl-lsp-core`, `rust/tcl-lsp-db`,
`rust/tcl-lsp-server` (+ tests/data), plus one contract paragraph in
`docs/design/analysis/name-resolution-proofs/original-method-candidates.md`.
I committed nothing; the coordinator's WIP snapshots (6afd791002 … da22e0cb25 and later)
carry my edits as they stood. `luna-41.patch` = `git diff f3aed2a40e -- <my paths>`
(everything of mine after the BIG-IP commit); the hunks of mine that went *into*
f3aed2a40e are listed under "Changes" (tcl-lsp-server/src/lib.rs).

Run conditions: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0`,
`TCL_LSP_REQUIRE_TCLSH=9.0,8.6`, `TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`, one
target at a time, `--test-threads` 1–3, logs reduced to test/panic lines, binaries
deleted after use. The tree was shared with Sol-48/49, compiler-names and Luna-42, so
every number is "HEAD + working tree at that moment"; compiler fixes landing in parallel
move numbers between runs (e.g. the `my`-in-`switch` and math-function-in-`[expr]` tests
started passing without an LSP change).

## 1. Measurement

### tcl-lsp-core lib (3,485 tests; CI 5201 has no lsp-core lib rows — shard 2 missing)

Before (binary built ~13:40 from 363a9f89de-era tree, per module):
**2,623 ok / 785 fail / 77 not run** (the not-run are 3 stack-overflow aborts + the 74
`definition::` tests after the abort). 549 of the 785 failures are tests whose function
name also exists on origin/rust (approximation of "regressed vs rust"); biggest modules:
rename 68, refactor 67, workspace_index 63, references 59, minify 44, semantic_tokens 38,
hover 32, definition 27, completion 21, rename_safety 18, code_lens 17, code_actions 16,
call_hierarchy 15. Per-test status: `scratchpad/luna41/corelib-before-status.txt`.

Stack-overflow aborts (lib, all compiler lowering recursion — routed):
`graphs::tests::deeply_nested_namespaces_survive_symbol_graph`,
`definition::tests::deeply_nested_namespaces_survive_scope_extent_and_alias_scans`,
`refactor::inline_variable::tests::deeply_nested_namespaces_survive_scope_walk`,
`inlay_hints::tests::deeply_nested_namespaces_survive_type_hints` (80 nested
`namespace eval`: lower_namespace_eval ↔ lower_body per-level frame exceeds the 24 KiB
budget of depth_guard.rs), and
`executable_regions::checked_potential_source_tests::checked_potential_source_retains_depth_exhaustion_as_incomplete`
(derived `Clone` of a cap-deep `if` IR in `Lowerer::extract_oo_methods_pass`,
lowering/mod.rs `let top_level = self.module.top_level.clone();`).

After (partial — the re-run was stopped when the disk fell to 1.3 GB and I had to delete
the binary): 9 modules re-run (bigip, call_hierarchy, caller_frame, code_actions,
code_lens, completion, config_ini, declaration, declaration_outline): **no pass→fail**,
counts identical to before. The remaining modules are not re-measured after my edits.

### Integration targets (tcl-lsp-core), CI 5201 comparison

| target | CI 5201 fail | before my edits (this tree) | now | fixed since CI | still failing from CI | new vs CI |
|---|---|---|---|---|---|---|
| name_resolution | 31 | 46/75 (29 fail) | 65/77 (12 fail) | 19 | 12 | 0 |
| references_rename | (not in CI lists) | 59/98 | 70/98 | – | – | – |
| force_import_shadow_consumers | 9 | 1/10 | 2/10 | 1 | 8 | 0 |
| hover_completion | 6 | 50/56 | 50/56 | 0 | 6 | 0 |
| hover_residual | 9 | 27/36 | 27/36 | 1 | 8 | 1 (`residual_hover_var_without_registry_omits_intrep`, fixed afterwards — not yet re-run) |
| references_residual | (not in CI lists) | 11/23 (coordinator) | 16/23 | – | – | – |

name_resolution "now" was taken after the classmethod correction, E and the iRules
correction were partly in; a final re-run with every edit is in
`scratchpad/luna41/run-it-name_resolution-final.log` if it completed.

Not measured (disk/CPU): the other 28 lsp-core integration targets, tcl-lsp-db
integration targets, tcl-lsp-server lib/e2e/smoke.

### tcl-lsp-db lib (200 tests; CI 5201: 37 lib + 2 interned_gc)

Before: 191 ran (120 ok / 66 fail+ignored) then the process aborted with
"panic in a destructor during cleanup" at `value_transfer_parity::a_rename_invalidates_every_summary`.
Coordinator's guarded group: `original_optimiser_memo_transport_tests` 2/5 → fixed the
pass→fail loss (see below). Not re-run in full after my edits.

### Decisions of 2026-10-18T16:00 — implemented on HEAD?

| decision | status |
|---|---|
| DB921 merged unit | implemented (aff99dc7e3, admitted e3e85d990f) |
| survey option a | implemented (aff99dc7e3: package_target_survey first-match bodies) |
| forced-import offsets | only `a_forced_import_before_the_call_does_not_shadow_a_later_call_site` moved onto the head word; the sibling tests in definition.rs (`a_forced_import_shadows_the_local_command`, `a_redefinition_ends_a_forced_import_shadow`, partly-observable `after_last(.., "    helper\n")`) still query after the newline — not done |
| test-only lexical helpers deleted | done (none of original_data_word/script_of/command_source/bracketed_body/parse_vsatisfies/is_tcl_version_subject/is_requirement_word/Guard::TclSatisfies remain in reachability) |
| Server102/103 snapshot consistency | implemented (workspace-symbol captured snapshot, #1854 tests in tcl-lsp-server) |
| Server108 cached analysis | NOT implemented: `symbol()` → `capture_workspace_symbol_analysis` → `capture_fresh_source_analysis` still analyses each non-Salsa document fresh per request; no LRU (decision 2026-10-19T11:00 closed-document cache) |

Fresh-interpreter entry, world-after-load and realm-aware iRules: on the compiler side
(Sol-49 / compiler agents); the LSP-visible effects are listed under the clusters.

## 2. Root-cause clusters

LSP-side, fixed (see Changes):
- L1 classmethod fixtures under tcl8.6 (13 name_resolution tests) — test inputs, decision 2026-10-17T20:30Z.
- L2 Tcl 9 classmethod through a subclass (`SubFactory make`) — class-delegate chain (decision via coordinator, C Tcl fidelity rule).
- L3 unsafe new names bypassed the shape gate on the original rename path.
- L4 quoted-only variable names (`set {$n} 1`) renamed instead of refused (gate ordered after the variable tier).
- L5 `$x` read edits emitted as `x`→`y`; rust tests expect whole-token `$y`.
- L6 unsound partial rename of class-declared variables (R3) — now refused.
- L7 call-hierarchy original items said "Source declaration" instead of `(N params)` / `KIND of CLASS (N params)`.
- L8 hover intrep/taint shown without a caller registry.
- L9 browser (wasm32-unknown-unknown) workspace scan found 0 files: std `Path::is_absolute` is false for every path on that target; `constructed_file_uri` then called `fs::canonicalize` and dropped every store path (origin/rust had a browser fallback). Same trap in source_graph's path admission.
- L10 DB test inputs that contradict the fresh-interpreter decision (2 tests).
- L11 iRules bare call to a rule proc (BIG-IP probe).

Compiler-side (sent to the coordinator with repros; routed to compiler-names / compiler):
- C1 invocations inside command substitutions lose `original_lookup`/`original_name_input`
  (`puts [greet]`): find-references/rename/lens miss every bracketed call; name operands
  (`info body greet`, `trace add execution`) carry no lookup.
- C2 completed command world withheld when any command substitution exists
  (`original_completed_root_state` None) → every original rename refuses
  `unknown-completed-command-world`.
- C3 commands inside a directly called proc body lose `original_lookup` → rename refuses
  `incomplete-original-command-coverage` (empty body works).
- C4 self-dispatch (`my m`, `next`, `nextto`) inside nested control bodies of a method
  missing from the typed tape (`if` still open; `switch` fixed in-tree meanwhile).
- C5 math functions inside `[expr {..}]` nested in another command (fixed in-tree meanwhile).
- C6 `namespace import ::a::Factory; Factory make`: invocation has no `original_lookup`, so nothing follows the alias.
- C7 itcl/snit definers do not publish the class command / `::Class::proc` into the authored command world (snit `Factory make` not even an invocation).
- C8 class-declared variable not unified across methods (R3 refusal is the LSP stop-gap).
- C9 W210 missing for `$missing` inside `[file join $dir $missing]` in pkgIndex.tcl.
- C10 lowering stack overflows (5 lib tests above).
- C11 CRLF method bodies trip the lexer's `scan_var_ref` debug assertion
  (`Analyser::analyse(crlf class with method body)`, folding/document_symbols tests).
- C12 `derived_hash_with_manual_eq` deny in tcl-compiler/src/ir.rs:1090 blocks clippy on all dependents (checked with `-A`).
- C13 run_all_checks returns nothing for logical-source entries (`set x [gets stdin]; subst $x` → no T100; IRULE1005/3001/5002, S100 missing) — code_actions diagnostic_context tests.

Needs a decision / larger LSP work (not done):
- force_import_shadow_consumers (8): the tests expect the workspace export oracle to decide
  a `-force` import, while the fresh-interpreter decision runs MAIN alone (the sibling
  export is not a proven source predecessor) → local proc survives. Conflict of decisions.
- code_lens method counts "at least N" (17): the readonly member census is Partial for any
  object-command dispatch by contract (original-method-candidates.md); decision
  2026-10-17T20:30 says "Complete wins once a sound census exists" — no census yet.
- hover/completion presentation: "built-in" / "operation" markers and missing
  summary/synopsis for registry hovers in the test registry; completion of proc params.
- WASM: pack-property hover empty, 1 notice when priming shipped packs; WASI: diagnostics
  never arrive for a document that `source`s a sibling, exits time out — needs a wasip1
  build + wasmtime to reproduce (no disk).
- Server timeouts (`workspace_symbol_caps_its_answer`,
  `startup_index_and_factory_reindex_keep_checked_document_roles`): need the cached
  per-document analysis (Server108 decision), not implemented.
- `document_links::tests::a_chained_computed_source_path_resolves_775` stays red by user
  decision (2026-10-17T20:30Z, "136 stays fail-closed").

## 3. Changes (all compile; cargo fmt clean)

tcl-lsp-core
- `src/method_symbol.rs`: class-object lookup (`class_object_metadata_candidate`,
  `delegate_chain_may_apply`, `class_delegate_order`): Registry classmethod delegate
  members first along the superclass-only linearisation, then the class object's own
  non-delegate members; 8.x has no delegate members. Removed redundant `#[must_use]` (3).
- `src/rename.rs`: shape gate (`is_safe_symbol_name`) moved ahead of the variable and
  original tiers; `literal_name_variable_rename_refusal` run before the variable tier;
  new `class_variable_rename_refusal` (R3).
- `src/variable_symbol/rename.rs`: a plain `$name` read edit covers `$name` → `$new`.
- `src/original_call_hierarchy.rs`: `original_item_detail` (lexical detail format).
- `src/hover.rs`: intrep/taint annotations only with a caller-supplied registry.
- `src/source_graph.rs`: `host_absolute` (rooted = absolute in a browser worker).
- `src/completion.rs`: removed a stale orphan doc comment.
- `src/original_command_rename.rs`, `src/original_member_rename.rs`: test-trace binding
  read in non-test builds.
- `src/code_actions/spec_notice.rs`: reasoned `naive_bytecount` allow.
- `src/lib.rs` + 2 examples: `#![allow(clippy::too_many_lines)]` with reason.
- `tests/name_resolution.rs`: `classmethod_refs_at` (tcl9.0) for 13 classmethod tests,
  per-test tclsh citation; controls `tcl86_rejects_classmethod_so_nothing_resolves`,
  `a_superclass_self_method_is_not_inherited_by_the_subclass_object`;
  `irules_switch_glob_body_is_followed` expects `[0]` (BIG-IP probe resolution-2286
  `ordinary_literal`: `undefined procedure: same`).
- Mechanical clippy fixes across ~80 files (closures, collapsible ifs, doc backticks,
  let-else, struct-literal defaults in tests, `Type::default()`, `clone_from`, …);
  7 `redundant_closure_for_method_calls` allows where the method path is not general
  enough over the schema borrow's lifetime.
- `docs/design/analysis/name-resolution-proofs/original-method-candidates.md`: one
  paragraph on the Tcl 9 class-delegate chain with tclsh 9.0.4/9.1.0 probes.

tcl-lsp-db
- `src/lib.rs`: `body_optimiser_memo_retains_the_actual_optional_worker_dialect` takes its
  body from `ir_module.procedures` (fresh interpreter installs `proc sample`);
  `source_optimisation_certificates_retain_unknown_world_hazards` builds its unit with an
  explicit unknown entry (`for_supplied_source` + `unknown_entry = true`); assertions
  unchanged (decision 2026-10-19T14:20Z). `#![allow(clippy::too_many_lines)]`.
- `src/project_{call,source,token}_inputs.rs`: reasoned `wildcard_imports` allow.

tcl-lsp-server
- In f3aed2a40e (BIG-IP commit) — mine: `DocumentSourceUnavailable::ForeignReceipt.actual: Box<Uri>`
  (+ its constructor) → result_large_err 207→0; `Box::pin` around
  `compute_recovery_analysis`, `compute_base_analysis` (lib.rs + current_diagnostics.rs ×3
  + 2 tests), `compute_deep_diagnostic_report`, `run_bounded` → large_futures 124→0.
- After it: `constructed_file_uri` rooted-path-on-browser fix (L9); 19 expanded
  `match … { Some(__source_value) => …, None => return … }` rewritten as let-else;
  `#![allow(clippy::too_many_lines)]`; reasoned `wildcard_imports` allow on 8 split-out
  module files (their nested test modules share the parent import set — explicit lists
  broke 208 references; the coordinator's owner-resolution request to make
  document_transform/current_diagnostics/studio_source explicit is therefore NOT done);
  `Default::default()` call sites qualified.

Tests that passed before my edits and fail after: none observed in what I re-ran
(name_resolution, references_rename, force_import_shadow_consumers, hover_completion,
hover_residual, references_residual, 9 lib modules). Not re-run after the last batch of
clippy/R3/hover/browser edits: the rest of the lsp-core lib, tcl-lsp-db, tcl-lsp-server.

## 4. Clippy (`-p tcl-lsp-core -p tcl-lsp-db -p tcl-lsp-server --all-targets`)

1,059 findings at start → **152** (core 100, server 38, db 14; 10 of them in BIG-IP-owned
files left to Luna-42). result_large_err 207→0, large_futures 124→0, too_many_lines
151→0 (crate allow; of the 151 functions ~91 do not exist on origin/rust, ~24 grew on
work, ~37 were already long — heuristic, list in `scratchpad/luna41/too_many_lines.txt`;
follow-up debt). Remaining: cast_possible_truncation 33, items_after_statements 30,
similar_names 16, needless_pass_by_value 11, too_many_arguments 9, large_enum_variant 8,
large_stack_arrays 7, type_complexity 6, misc 32.
