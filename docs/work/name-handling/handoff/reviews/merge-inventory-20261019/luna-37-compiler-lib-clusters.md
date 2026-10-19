# Luna-37: tcl-compiler lib test failures grouped by root cause (merge inventory 2026-10-19)

Reviewer: Luna-37 (read-only). Branch `work` at `acfc74477`. Base for comparison: `origin/rust` `fcb7927f0`, where CI is green.

## Input and method

- Log: `scratchpad/w/inv2/tcl-compiler__lib__tcl_compiler.log`. The run was killed at test 2705 of 10444 (`summary.jsonl` gives `rc=-9` after 2037 s). It recorded **754 failures**: 750 FAIL and 4 TIMEOUT at 180 s.
- Each failing test body was compared with its `origin/rust` version:
  - **479** are byte-identical to `origin/rust` (whitespace normalised). These are true regressions.
  - **119** were modified on `work`.
  - **156** are new on `work`. Some of these must have passed when they were written, so part of the damage happened on the branch itself. The clone is shallow, so I could not bisect.
- Reproduction used the saved ELF, one test at a time. That was about 30 runs in total: gdb line breakpoints (the binary has line tables only, so locals cannot be printed), the built-in test traces `TCL_LSP_TRACE_SOURCE_PHASES`, `TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS` and `TCL_LSP_TRACE_ORIGINAL_FUNCTION_LATTICE`, and gdb-attach stack sampling for the slow tests.
- The classification heuristics live in the scratchpad (`luna37/cluster.py`). Counts are approximate where noted.

## Summary table

Clusters are ranked by how many failing tests they explain.

| # | Cluster | ≈ members | Class | Smallest fix |
|---|---|---|---|---|
| C1 | Whole-document analysis (`Analyser::analyse`, the LSP diagnostic report) builds its CompilationUnit with an **unknown runtime entry** for core-Tcl and iRules inputs. Every command purpose is refused, so `proc`, `if`, `while`, `switch` and the rest are lowered as generic calls. The result is zero procedures, no structured CFG, and no CFG/SSA, bounds, shimmer, taint or iRules cell diagnostics. | **≈440 directly** (392 core-Tcl and 46 iRules tests in the analyse bucket); bucket upper bound 560 | (a) product regression on real source | Product: whole-document entry = fresh-authoring contract |
| C2 | The dynamic-name barrier goes opaque even when procedures *are* lowered. Commands without a proven original word vector (deferred body and expression `[..]` substitutions) mark every name as possibly written, which suppresses W210/W211/W220 and the I230 existence folds. | 54 primary; also gates C1 tests once C1 is fixed | (a) partly; (b) for the legacy fixture ingress | Product: give expression substitutions their original site. Then move `codes_for` onto `analyse` |
| C3 | Object-handle and constant-`$cmd` dispatch facts are not settled. W307 fires where suppression is expected; W308 and E001 are missing; `$cmd` heads are not resolved. This shows on the permissive `tcl` input too, where C1 does not apply. | ≈105 overlap (≈40 not explained by C1) | (a) probable; not root-caused | Re-run after C1, then triage |
| C4 | Tests drive internal handlers with synthetic tokens (`dispatch_tokens`, `dispatch_words`, `esc_tok`, `a.handle_*`, `created_by`, `upvar_link_of`). There is no original source, so they fail closed by design. | 45 | (b) | Narrow test correction: drive `analyse` on the equivalent source |
| C5 | New "original metadata" API tests (`param_traits`, `original_members`, `oo`, `interp_visibility`, `original_definer`, `class_hierarchy`) | 64 (34 new) | probably downstream of C1/C6; not verified | Re-run after C1/C6 |
| C6 | Standalone CompilationUnit builders (`build_for*`) now lower `proc` into a read-only `::original-authored-body#N` body unit. No named procedure is created, so `unit.function("::f")` returns None. | 20 (18 new) | (d) API/contract change | Product decision: also publish the named procedure (tclsh defines `::f`) |
| C7 | The `per_item` incremental fast path is never taken for ordinary bodies. It also has a quadratic re-tokenisation cost that is behind the 30-minute tests. | 7 failures plus the 2 slow tests | (c) and (a) | Product: narrow the fallback predicate; memoise original token materialisation |
| C8 | Timeouts. Two of them come from a test memo key built with `format!("{:?}", req.body)` over the new, very large `ir::Script` Debug output. | 4 | (c) | Narrow test correction (stable key) |

The bucket counts in the table add up to more than 754 because C2 and C3 overlap other buckets. The disjoint split in `cluster.py` is: C1 bucket 560, C2 54, C4 45, C5 64, C6 20, C7 7, C8 4. C3 sits inside the C1 bucket.

---

## C1 — Whole-document CompilationUnit is built with an unknown runtime entry (≈440 to 560 tests)

### Minimal Tcl input

```tcl
proc f {} { puts $u }
```

- `fp::sanity::baseline_read_before_set_fires_w210` under tcl8.6 emits **[]**.
- `origin/rust` emits W210.
- tclsh agrees with `origin/rust`: calling `f` raises `can't read "u": no such variable`, so W210 is a true positive.

The same pattern loses W240 on this top-level loop (`bounds_checks::tests::w240_and_w241_read_the_branch_fact`):

```tcl
set n 0
while {$n} {puts x}
```

It also loses IRULE6001 on `when RULE_INIT { set greeting hi }`.

### Mechanism (each step reproduced)

1. `Analyser::emit_cfg_ssa_diagnostics` (`rust/tcl-compiler/src/analyser/diagnostics.rs:362-452`) is new compared with `origin/rust`, which simply built the unit (`origin/rust` `diagnostics.rs:325-365`). It now calls `CompilationUnit::build_with_analysis_input(source, options, self.source_analysis_entry.as_deref(), &input)`.
   - `source_analysis_entry` is `None` for every `Analyser::new()` that has no explicit `.with_source_analysis_entry(..)`.
   - The concept does not exist on `origin/rust` (`analyser/state.rs` there has 0 hits).
   - gdb confirms the new guards at lines 363-387 all pass. The unit is built (line 441).
2. `compilation_unit.rs:3737-3749` documents the behaviour: "With no supplied entry, matching Logical and hosted inputs retain their source advice. Runtime history stays unknown for other supplied inputs."
   - The LSP builds the same thing explicitly. `SourceAnalysisEntry::for_supplied_source` (`command_binding/source_analysis_entry.rs:431-448`) falls back to `unknown_entry: true` whenever the input is neither Logical nor hosted.
   - That covers every native-protocol dialect, i.e. tcl8.4 to tcl9.1. `ResolvedAnalysisInput::has_logical_source_name_context` (`analyser/input.rs:86-96`) is false for them.
   - The contract row in `docs/design/contracts/naming-consumer-matrix.md:605` says the same: "Native or unsupported axes retain unknown runtime history".
3. `ModuleCommandBindings::retained_original_source_purpose` (`command_binding/origin_inventory.rs:1616-1617`) returns `None` on `baseline.unknown_entry`.
   - gdb hit counts: 41 of 41 purpose queries returned there in the W210 baseline test, 101 in the iRules test, 42 in `switch_unpaired_pattern_fires_e005`.
   - `original_invocation_source_purpose` (origin_inventory.rs:69-117) and `original_source_advice_purpose` (`declaration_layout.rs:2993-3027`) therefore refuse.
4. `Lowerer::logical_structured_invocation` (`lowering/mod.rs:2876-2930`) returns None, and `try_lower_logical_procedure_declaration` (`lowering/mod.rs:3577-3592`) refuses. As a result `lower_command` takes `return Some(self.lower_default_boxed(..))` at **`lowering/mod.rs:3922-3923`** for every command (gdb shows both `proc` and `while` here).
5. The unit ends up with `procedures=0` and no structured control flow. Trace from the W210 baseline test: `DIAGNOSTIC_CFG_EMISSION_PHASE ... procedures=0 methods=0 body_units=1`.
6. Every diagnostic that rides the CFG/SSA/lattice is therefore absent. In the logs this shows as "emitted: []": W210, W211, W213, W214, W220, W230, W232, W233, W240, W241, W242, I230, I231, O1xx, the iRules cell codes 4001, 4003, 6001 and 5001, plus switch E005 and the loop/arm facts.

The iRules inputs are hosted rather than native, but they behave the same way inside event bodies. `lower_when` (`lowering/mod.rs:4658`) still creates the event procedure (`procedures=1`), but `set` inside it hits `origin_inventory.rs:1617` 101 times and is lowered generically at `lowering/mod.rs:3923`.

### Classification

(a) Product regression on real source, and it reaches the LSP. `tcl-lsp-core/src/diagnostic_report.rs:285-303` builds `for_supplied_source` and then `build_with_analysis_input` for every document, so a live tcl8.x/9.x document loses every CFG/SSA-backed diagnostic.

For a document run by `tclsh file.tcl`, C Tcl starts from a fresh interpreter: `proc`, `set` and `while` are the builtins. Treating the whole document's prior history as unknown contradicts the semantics that decide these answers.

### Members

About 392 core-dialect and 46 iRules tests in the analyse bucket. The modules most affected are:

- `diagnostics::tests` (≈150 of 206 in the bucket)
- `fp::rbs` 66, `fp::sh` 35, `fp::obj` 24, `fp::ds` 19, `fp::sty` 18, `fp::opt` 12, `fp::tnt` 12, `fp::inj` 9, `fp::bnd` 8
- `bounds_checks` 13, `irules_event_checks` 23, `did_you_mean_variables` 5

Examples:

- `analyser::diagnostics::fp::sanity::baseline_read_before_set_fires_w210`
- `analyser::diagnostics::fp::ds::fp_ds_01_genuine_dead_store_still_fires`
- `analyser::bounds_checks::tests::w240_and_w241_read_the_branch_fact`
- `analyser::irules_event_checks::tests::irule6001_fires_for_implicit_global_in_rule_init`
- `analyser::diagnostics::tests::switch_unpaired_pattern_fires_e005`

About 73 of the 560 tests in the bucket use the permissive `tcl` profile. That profile counts as Logical and is **not** unknown-entry: gdb shows the purpose queries pass line 1619. Those tests belong to C3 or other causes.

### Smallest fix direction

**Product fix.** A whole-document request with no explicit entry should get the *fresh-authoring* entry, not unknown history.

- Code already exists for that contract:
  - `SourceCommandBindings::analyse` (`command_binding.rs:6887-6911`: "Interpret a fresh authoring entry using the selected registry profile").
  - `OriginalInvocationSourcePurpose::from_authored_simulation_policy` (origin_inventory.rs:44-50), fed by `InvocationDialect::authored_name_policy` (`tcl-registry/src/native_string_materialization.rs:100+`).
- The test suite already *states* this contract: `diagnostics/tests.rs:7885-7886` says "The convenience driver selects its fresh authoring contract. An unknown incoming interpreter is an explicit, distinct entry in this control."
- Concrete change: in `for_supplied_source` (and `build_with_analysis_input` / `emit_cfg_ssa_diagnostics` when `entry` is None), set `execution_name_policy` to the dialect's authored-simulation policy instead of `unknown_entry: true`.
- Keep `unknown_entry` for *explicitly* unknown entries. `info_exists_actual_folds_do_not_borrow_declaration_or_missing_entry` (`diagnostics/tests.rs:7880-7911`) pins that, and the fail-closed guard is preserved.
- This changes the `naming-consumer-matrix.md:605` row, so the contract owner needs to sign off.
- Re-run the inventory before triaging C2, C3 and C5. Many of their members are expected to move.

---

## C2 — Dynamic-name barrier opaque for commands without a proven original word vector (54 primary)

### Mechanism

`dynamic_names::scan_call` (`rust/tcl-compiler/src/dynamic_names.rs:788-845`) unions `DynamicNameBarrier::OPAQUE_SCRIPT` in three cases:

- when `commands.metadata` is None (line 797-798);
- for a non-standalone CFG, when `binding.original_lexer_config_for_tokens(tokens)` is None or differs from the CFG config (lines 800-809);
- when no invocation resolves and the possible-name residual is unknown (lines 838-845).

The resulting `fu.dynamic_names.writes` sends `emit_read_before_set_diagnostics` down the early path at **`analyser/diagnostics/dataflow.rs:1712-1735`**. That suppresses chain-based W210. The same barrier clobbers SCCP existence through `sccp.rs:3089` → `ExistenceRun::new` (`sccp.rs:1937`), which kills I230.

### Evidence

- **`info_exists_control_still_flags_w210`** goes through the test-only `emit_cfg_ssa_diagnostics_standalone_for_test`, i.e. `build_with_context_registry` (`diagnostics.rs:455-482`).
  - The procedure *is* lowered (`procedures=1`).
  - gdb hits `dataflow.rs:1713` and `dynamic_names.rs:809/844`. W210 is not emitted for `proc f {} { puts $u }`.
- **`info_exists_folds_false_for_never_defined_local`** has a captured *native* entry: `native_existence_result`, `diagnostics/tests.rs:7816-7836`.
  - Purposes resolve to Execution, and the source `proc f {a} { if {[info exists b]} { puts hi } }; f VALUE` lowers fine.
  - But `scan_call` still goes opaque inside `FunctionUnit::build_procedure_in_module` (5 hits at line 809).
  - The child-guard trace shows `ORIGINAL_WORD_CONFIG stage=declaration_site offset=Some(17) passed=false` for `[info exists b]`. That is the command substitution *inside the `if` expression*: it has no written-word or compiler-word vector and no invocation site (`command_binding/original_name_value.rs:1091-1180`).
  - So the I230 fold is lost even under a native entry. On `origin/rust` this test, without the native-entry wrapper, expected I230.

### Classification

- (a) for the expression-substitution site gap, which shows on real source with a native entry.
- (b) for the `codes_for` fixture ingress. `emit_cfg_ssa_diagnostics_standalone_for_test` is documented as "Explicit legacy fixture ingress without a supplied source analysis".

### Members

54 tests: `codes_for`, `w210_codes`, `w210_fires_for` and the `native_existence_*` helpers, nearly all in `diagnostics::tests`.

Examples:

- `analyser::diagnostics::tests::info_exists_control_still_flags_w210`
- `analyser::diagnostics::tests::emit_cfg_ssa_diagnostics_w210_read_before_set`
- `analyser::diagnostics::tests::info_exists_folds_false_for_never_defined_local`
- `analyser::diagnostics::tests::info_exists_folds_true_for_parameter`
- `analyser::diagnostics::tests::emit_cfg_ssa_diagnostics_w210_read_after_unset`

### Fix direction

- **Product:** retain the original written-word site for expression-embedded command substitutions and deferred-body commands, so that `original_lexer_config_for_tokens` succeeds. The opaque guard for *genuinely* unproven tokens stays.
- **Narrow test correction:** once C1 lands, re-route `codes_for` through `Analyser::analyse`. Keep every assertion; they are tclsh truths, e.g. `info exists` of a never-set local is 0.

---

## C3 — Object-handle and constant-`$cmd` dispatch facts not settled (≈105 overlapping, ≈40 independent of C1)

### Evidence

`diagnostics::tests::const_cmd_head_resolves_through_a_pure_copy_chain_m7` uses the `tcl` profile, which is Logical, so it is not C1:

```tcl
proc target {} {}
set x target
set cmd $x
$cmd
```

The procedure is lowered (`procedures=1`), but the indirect `$cmd` invocation is never resolved to `::target`. The guard trace shows `retained.unknown passed=false` 93 times and `structure.declaration_advice passed=false` 37 times.

Related failures:

- `analyse_w307_suppressed_for_my_self_dispatch`: W307 now fires.
- `analyse_w308_*`: W307 is emitted instead of W308.
- `tp_e001_bare_*`: E001 is missing.
- `*_945`: may-targets are absent.

### Classification

(a) probable. These are real-source inputs whose answers are fixed by tclsh. For example, `$cmd` with `cmd=target` dispatches `::target`.

### Root cause

**Not root-caused in this review.** The suspects are the constant-dispatch and value-provenance settlement in `emit_var_command_diagnostics` and the constant-dispatch emitter called from `emit_cfg_ssa_diagnostics_with_cu` (`diagnostics.rs:651-700`), together with `produce_object_handle_facts` (`diagnostics.rs:505-520`).

### Members

About 105 test names or messages mention W307, W308, E001, const dispatch, factory or object. 102 of them sit in the C1 bucket, and about 40 of those use `tcl` or other Logical inputs.

Examples:

- `analyser::diagnostics::tests::const_cmd_head_resolves_through_a_pure_copy_chain_m7`
- `analyser::diagnostics::tests::analyse_w307_suppressed_for_my_self_dispatch`
- `analyser::diagnostics::tests::tp_e001_bare_command_substitution_head`
- `analyser::diagnostics::fp::obj::fp_obj_09_const_string_multi_dispatch_fires_w307`

### Fix direction

Re-run after C1, then root-cause the remainder individually. These are product fixes; the original assertions stay.

---

## C4 — Internal entry points driven with synthetic tokens (45)

### Mechanism

These tests call handlers directly with no original source:

- `dispatch_tokens` and `dispatch_words` with `esc_tok`/`str_tok` (`analyser/handlers.rs` test module);
- `a.handle_oo_objdefine(..)` and `a.handle_interp_alias(..)`;
- the `created_by` and `upvar_link_of` helpers;
- `commands::tests::process_*`.

Every original-source proof (site, word vector, purpose) is absent, so the new fail-closed consumers refuse by design. For example, `handle_set_defines_variable` (`handlers.rs:13891-13903`) no longer records `x`.

### Classification

(b).

### Members

45: `handlers::tests` 39, `commands::tests` 3, and one each in `diagnostics::tests`, `oo::tests` and `original_definer::tests`.

Examples:

- `analyser::handlers::tests::handle_set_defines_variable`
- `analyser::handlers::tests::handle_incr_with_amount`
- `analyser::handlers::tests::handle_oo_objdefine_records_bare_name`
- `analyser::handlers::tests::interp_create_double_dash_ends_flag_parsing`
- `analyser::commands::tests::process_set_defines_variable`

### Fix direction

Narrow, documented test correction. Drive each case through `Analyser::analyse` on the equivalent real source line, e.g. `set x 1`, `upvar #0 counter c` or `interp create -safe -- z`, and keep the assertions.

Check one expectation against tclsh: `handle_interp_alias_records_canonical_form` now yields `::set` where `set` is expected. tclsh's `interp alias {} myset` reports the target as written (`set`), so keep the assertion and fix the recorder.

---

## C5 — New "original metadata" API tests (64, of which 34 new)

### Members

`param_traits` 28, `original_members` 9, `oo` 8, `interp_visibility` 5, `original_definer` 3, plus `class_hierarchy` and `class_lattice`.

Examples:

- `analyser::param_traits::tests::regsub_records_dynamic_name_local` ("no traits for out")
- `analyser::original_members::tests::original_literal_loop_members_keep_readonly_list_children_and_worker_words`
- `analyser::oo::tests::original_unknown_operation_advice_refuses_missing_foreign_changed_and_dynamic_source`
- `analyser::class_hierarchy::tests::bare_superclass_links_via_unique_tail_across_namespaces`

### Evidence

- The `param_traits` helper `infer_owned` (`param_traits.rs:2047-2079`, new on `work`) builds `realm::document_realm_bindings` from real source under the tcl9.0 profile with no entry.
- The failure is consistent with the early return at `param_traits.rs:90-96` (`env.matches_source` / `env.matches_parameters`). That path does not go through `retained_original_source_purpose` (gdb shows 0 hits there).

### Classification

Unverified. Probably downstream of the same entry/ownership refusal (C1/C6).

### Fix direction

Re-run after C1 and C6, then triage what remains.

---

## C6 — Standalone CompilationUnit builders no longer publish named procedures (20, of which 18 new)

### Mechanism

`build_for`, `build_for_dialect` and `build_for_profile` (`compilation_unit.rs:3460-3585`) build in Standalone authoring mode (`allows_standalone_registry_metadata`, `lowering/mod.rs:5396-5403`).

`lower_command` takes `try_lower_authored_procedure_declaration` first (`lowering/mod.rs:3878-3885`; gdb hits line 3884 in the `build_for_profile` path). That function (`lowering/authored_source_bodies.rs:12-76`) retains the body as a **read-only body unit** named `::original-authored-body#N` with `ProcedureBodyKind::OriginalDeclaration`, and returns `lower_default` for the `proc` call.

As a result:

- `cu.procedures` is empty and `CompilationUnit::functions()` (`compilation_unit.rs:5062-5066`) contains only the top level;
- `unit.function("::f")` returns None, which is the panic `called Option::unwrap() on None` at `analyser/diagnostics/helpers.rs:2922`;
- the `run_all_checks` half of `fp::codes` (shimmer and taint on `build_for_profile`) loses proc-level findings. `fp_sh_09_untraced_variable_still_fires` gets []; this part is the likely cause but was not stepped through.

### Classification

(d) Contract change in the standalone builders. tclsh semantics are clear: `proc f` defines `::f`.

### Members

20, mostly `diagnostics::helpers::exact_cell_diagnostic_tests` (12).

Examples:

- `analyser::diagnostics::helpers::exact_cell_diagnostic_tests::destruction_versions_do_not_kill_a_prior_read_or_supply_a_stored_value`
- `analyser::diagnostics::security::source_patterns::tests::*`
- `analyser::diagnostics::fp::bnd::fp_bnd_01_loop_index_past_append_slot_fires`

### Fix direction

Product: also retain the named `Procedure` (or expose the authored body unit through `function(qname)`) when the standalone authoring purpose is selected. Most of these tests are new on `work` and presumably passed when written, so this is a branch-internal regression.

---

## C7 — `per_item` fast path never taken, and quadratic token re-materialisation (7 failures plus 2 slow tests)

### (i) The fallback predicate is too broad

`fill_deferred_bodies` (`analyser/per_item.rs:773-776`) falls back with `PerItemFallback::OriginalSourceOwnership` whenever `fragment_needs_original_source_owner` (`per_item.rs:1674-1720`) is true. That predicate is true when any invocation carries `original_lookup`, `original_name_input` or `resolved_definition`, which is essentially every command in an owned body.

So `fast_path(..)` (`per_item.rs:2619-2624`) fails its `gate == None` assertion. Members:

- `a_body_defined_hash_zero_global_survives_the_graft`
- `a_caller_frame_upvar_defines_no_global_cell`
- `a_body_defined_global_cell_survives_the_graft`
- `same_file_arity_e003_matches_across_proc_body_call`
- `param_list_with_backslash_continuation_no_spurious_w215`

Class (a) for the LSP: incremental analysis always degrades to full analysis. The test is unchanged from `origin/rust`.

### (ii) Where the time goes in `ordinary_large_body_stays_on_the_fast_path` and `many_small_bodies_totalling_over_the_threshold_stay_fast`

Both pass on `origin/rust`. One run, single-threaded, 175 s timeout, with `TCL_LSP_TRACE_SOURCE_PHASES` and three gdb stack samples:

- **Shell walk:** the single `proc big {..1000 × set x 1..}` command (12 KB) spends **43.9 s** inside `process_command`. Per-stage timings:

  | Stage | Seconds |
  |---|---|
  | original-names | 2.2 |
  | scoped-bodies | 8.4 |
  | variable-receivers | 2.3 |
  | direct-invocations | 2.4 |
  | callback | 3.5 |
  | namespace-role-facts | 2.3 |
  | nested-diagnostics | 2.4 |
  | procedure-resolution | 4.7 |
  | expression-diagnostics | 2.4 |
  | dispatch-diagnostics | 9.7 |
  | handlers | 3.6 |

- **Isolated body pass:** 1000 × `set x 1` at about 46 ms each, **46.4 s** in total.
- **Then the C7(i) fallback** starts a *fresh full analyse* (another ≈90 s). After that the test runs its own `Analyser::new().analyse` (another ≈90 s). The total is about 4.5 min single-threaded, and it exceeds 30 min when four inventory workers contend for the CPUs.

All three stack samples show the same hot path:

```
process_command (analyser/commands.rs:667/679/713)
 → source_registry_words / original_registry_words_for_tokens (registry_invocation/source_structure.rs:1492/1783/956/973/1049)
 → selected_registry_words_in_original_image (source_structure.rs:1068)
 → CommandBindingRealm::stamp_original_tokens (realm.rs:421)
 → SourceInvocationBinding::original_recorded_command (command_binding/original_name_value.rs:1032-1049)
     builds SourceMap::from_image(image) and CommandTokens::from_segmented anew
 → WordExpr::from_word → original_word_span → original_run_end (word_expr.rs:205)
     builds *another* SourceMap::new(source) → LineIndex::from_bytes over the whole text, per word
```

- `stamp_original_tokens` loops over **every** `tokens.nested_bindings` (`realm.rs:419-424`; for `proc big` that is all 1000 body commands).
- Each iteration rebuilds a full-image `SourceMap` and line index and re-tokenises.
- This happens on every registry-word query, and `process_command` issues hundreds of those per command. The cost is O(queries × nested commands × image size).
- `original_run_end` and the realm stamping do not exist on `origin/rust`.

`many_small_bodies...` has the same path. There are fewer nested bindings, but the image is about 300 KB, so each `SourceMap::from_image` costs 25× more.

Class (c).

### Fix direction (product)

1. Memoise the original recorded command and tokens per binding, or skip re-stamping nested bindings that are already stamped.
2. Pass the existing `SourceMap` into `original_run_end` instead of constructing `SourceMap::new` per word (`word_expr.rs:205`).
3. Narrow `fragment_needs_original_source_owner` so that ordinary resolved builtins do not force fallback, or graft their original lookups.

---

## C8 — Timeouts (4)

### `memoized_compilation_unit_shift_correctness` and `memoized_compilation_unit_diagnostics_match_whole_file`

These are small inputs. A gdb sample after 30 s and after 45 s shows the time is spent in `core::fmt` formatting `HashMap<CommandSlot<NameBytes,..>>` and `SourceBindingPoint`. That formatting comes from the test closure's memo key:

```rust
format!("{}\u{0}{:?}\u{0}{:?}\u{0}{:?}", req.qname, req.body, req.params, req.param_constants)
```

at `analyser/diagnostics/tests.rs:5514-5519` (`cache.get` at line 5519).

`ir::Script` Debug now includes every command's retained `SourceInvocationBinding` lookup snapshots, so the key string is enormous.

- Class: (c), test-side.
- Fix: narrow test correction. Key on `req.body_source`, `qname`, `params` and `param_constants`. Optionally make `Debug` for `CommandTokens::source_binding` elide snapshots.

### `depth_far_past_cap_still_emits_e207_exactly_once` and `tcltest_aliases_preserve_the_reported_setup_body_cleanup_reads`

Not sampled. Most likely the C7(ii) stamping cost: 2000 nested `if` bodies means O(n²) nested bindings.

---

## Notes for the merge decision

- C1 alone explains most of the regression and affects the shipping LSP path, not just tests. For `tcl-lsp-core::diagnostic_report` on any tcl8.x/9.x document I expect zero CFG/SSA diagnostics; this was not exercised end-to-end.
- C1 needs one product contract decision (fresh-authoring entry versus unknown history for whole-document inputs). The fail-closed guards stay in place for explicitly unknown entries.
- After fixing C1, re-run the compiler lib inventory before working on C2, C3 and C5 individually.
- C4 and C8(memo) are the only clusters where a test correction is the right first move. C6 needs an owner decision; tclsh semantics favour publishing `::f`.
- The inventory profile prints only failing tests. Pass counts per module were not available.
