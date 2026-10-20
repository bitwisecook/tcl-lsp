# Sol-42: C1 fresh-interpreter entry, follow-up to Sol-37

Implementer: Sol-42. Branch `work` at `efc58507c1` (merge of origin/rust `a3c6e6856b`), uncommitted.

- Patch: `sol-42-c1.patch`, which is `git diff` of `rust/tcl-compiler`, `rust/tcl-lsp-core`, `rust/tcl-lsp-db`, `rust/tcl-lsp-server` and `docs/design`.
- It includes Sol-37's `sol-37-fresh-entry-full.patch`. That patch applied cleanly on the merged tree. See `sol-37-fresh-entry-NOTES.md` for that part of the change.
- `rust/tcl-lsp-core` and `rust/tcl-lsp-server` have no changes in this patch.

User decision applied: `resume-20261017-diagnosis/user-decisions-20261019T1420Z.json`.

- A document with no explicit execution entry runs in a fresh interpreter of its dialect.
- Unknown runtime history applies only when a caller supplies it explicitly.

## Product changes (beyond Sol-37)

### 1. Literal-name synthetic statements on the policy route

Files: `place_bridge.rs`, `ssa.rs`, `ir.rs`.

- The fresh entry carries an authored native name policy. Because of that, `def_places_at` takes the policy route instead of the Logical (policy-less) route.
- On that route, synthetic statements were losing the names their producer had already proved. These are:
  - `<word-effects>`, the `[…]` substitution writes;
  - `<cond>`, the condition kill-set;
  - iteration bindings;
  - upvar invalidations;
  - opaque arm writes.
- The loss happened in two places:
  - `source_tokens_at` is None for these statements, so `def_places_at` returned `[]`;
  - `invocation_def_places` returned `unknown_top`, which `canonical_binding_value_key` then dropped.
- A new predicate, `CommandTokens::carries_proved_names`, identifies these statements. Their `defs` now resolve literally (the same way origin/rust's `def_places` did) through the continuation projection.
- `ssa::uses_in_call` now keeps their `reads` as well. Before, `selection.normal(tokens)` needs a source binding, so a marker statement had none and its reads were dropped.
- Effect: `[info exists a]` in a condition is a use of `a`. tclsh agrees, and W214 no longer fires.

### 2. Procedure publication key from an authored source context

File: `command_binding.rs`, `qualify_execution_name`.

- An authored `SourceContext(Authored(ns))` now qualifies like `Exact(ns)`.
- Before, every top-level `proc` under the fresh entry was an imprecise `Define`. That ran `mark_opaque_binding_mutation`, which withdrew the original command world.
- Every procedure body's binding timeline then started opaque. Condition footprints went opaque too, giving "condition runs an unreadable script" and registry barriers with no `<cond>` defs.
- Native and allocated identities still refuse.

### 3. W210 condition-writer guard

Files: `analyser/diagnostics/helpers.rs` (`ConditionWrites`, `collect_condition_writes`) and `dataflow.rs` (`definite_missing_read_spans`).

- The binding-context presence path (new on `work`) does not refine a read by its branch condition. The two rules below give it the condition guard the chain path already applies.
- **Bare-substitution conditions.** A read is guarded when it is dominated by the true arm of a condition that is one bare `[cmd …]` substitution and that cmd's kill-set writes the name. Examples: `if {[regexp … m]}`, `[catch … msg]`, `[gets $fp line]`.
- **Other condition writes.** A read after any condition write is guarded only when its presence alternatives may be defined (`SsaReadPresenceAlternatives::may_be_defined`, a new method).
  - This mirrors the chain path's `written_by_substitution_before`.
  - A proved absence still reports. For example, the no-match arm of `![regexp {x} y -> v]` still fires; tclsh errors there.

### 4. Superseded `proc` bodies

Files: `lowering/mod.rs` (`lower_declared_proc_body`) and `lowering/installed_bodies.rs`.

- A declaration that is replaced before any call never enters its body. Its body therefore lowered through the legacy path with no namespace context.
- The installed-body pass then also retained a duplicate `::original-declaration-body#N` unit at the same site. This disabled `ObjectReportSource`, so per-item and full analysis compared raw `ProcedureModuleId`s.
- **Fix.** The body is now retained from the unique `procedure_implementation_bodies` entry at that declaration site. Per C Tcl, `proc` created it, and it runs in the declaration's namespace.
- **Second fix.** The installed-body dedup now compares canonical formal lists, so `{"as" v}` equals the installed `as v`.
- Origin/rust expectations met:
  - `per_item` `duplicate_proc_*` (incremental equals full, last definition wins);
  - `fp_sty_08` (the quoted-marker exemption sees the parameter geometry).

### 5. Dispatch-table literal divergence

Files: `var_command.rs` (`harvest_table_command_value_spans` now also returns divergent spans), `analyser/state.rs` (`divergent_table_literals`), `const_dispatch.rs`.

- Take a literal whose consuming sites reach different implementations, as in `proc handler…; set ops(key) handler; $ops(key); proc handler…; $ops(key)`. tclsh calls both definitions there.
- Such a literal no longer keeps rename authority through the const-dispatch settlement row.

## Test-only changes

- **Bounded comparison output (coordinator request).**
  - `AnalysisResult::bounded_mismatch_summary` and `assert_analysis_results_eq` are new, in `analyser/types.rs` under `cfg(test)`.
  - On mismatch they print only the differing fields, each capped at 1500 bytes by a writer that aborts formatting early.
  - Used by `per_item` `eq`, `fast_path` and `falls_back`, and by `analyser::input` `full != per_item`.
  - Equality and assertions are unchanged.
  - Measured: a failing `duplicate_proc_with_params` printed 4.2 KB, against gigabytes before.
- **Memoized-unit test keys.**
  - `memoized_compilation_unit_diagnostics_match_whole_file` and `memoized_compilation_unit_shift_correctness` now key the body by its structural `Hash` (`memo_body_key`) instead of `format!("{:?}", req.body)`.
  - That is C8. The tests now finish in under 2 minutes instead of hanging; they still fail on their assertions.
- **Narrow, documented corrections against tclsh.**
  - **`issue_703_no_false_w210_on_fallthrough_group_var`.** `try {set v ok} on ok x - on error y { return $y }` inside a proc now expects 1 W210. tclsh 8.6.18, 9.0.4 and 9.1.0 all fail with `can't read "y": no such variable`: a literal `try` in a proc is byte-compiled and binds the matching handler's var. The `$x` case still expects 0.
  - **`mixin_method_inventory_withdraws_after_original_base_mutation`.**
    - Rename case: after `rename ::Observable ::Retired; oo::class create ::Observable {}`, `$o nosuch` now expects W308. tclsh 8.6.18, 9.0.4 and 9.1.0 report `unknown method "nosuch": must be destroy or subscribe`, because a mixin names the class object.
    - Unknown-method case: unchanged (silent).
  - The test comments record the tclsh evidence.

## Measurements

- Build: `tcl-compiler` lib tests, `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0`, `TCL_LSP_REQUIRE_TCLSH=9.0,8.6`, `TCL_LSP_JIMSH=tmp/jimtcl-oracle/jimsh`.
- Baseline: the HEAD binary built before any change.
- Skipped in every run: `memoized_*`, `tcltest_aliases_preserve_…`, `depth_far_past_cap…`, `ordinary_large_body…` and `many_small_bodies…`.
- Scratch results are in `scratchpad/sol42/` (`base/`, `fin/`, `base_other_all.txt`, `cur_other.txt`, `regex_*.txt`).

### C1-bucket modules (pass/fail)

| Module | Baseline (HEAD) | After |
|---|---|---|
| analyser::bounds_checks | 55 / 13 | 55 / 13 |
| analyser::irules_event_checks | 65 / 23 | 65 / 23 |
| analyser::diagnostics::fp:: | 365 / 216 | 442 / 139 |
| analyser::per_item | 54 / 14 | 54 / 14 |
| analyser::diagnostics::tests | 564 / 266 | 578 / 252 |

- Tests that passed at baseline and fail after: **none** (0). Newly passing: 91 (1103 → 1194 ok).
- All 12 Sol-37 regressions are fixed:
  - 9 false positives: W210 ×5, W214 ×2, W308 and dispatch-table;
  - 3 `per_item` `duplicate_proc_*` tests.

### Other modules

- Coverage: all other `tcl-compiler` lib modules, 157 filters.
- `regex_source` was run per test with a 120 s cap. `literal_flows_to_regexp_in_a_large_multi_proc_file` uses more than 5 GB and times out at both baseline and after. The other `regex_source` results are identical before and after (13 ok).
- Result: OTHER_RESULT.

### `tcl-lsp-core` and `tcl-lsp-db`

- `cargo check --lib --tests` is clean for both.
- `tcl-lsp-core --lib diagnostic`: 109 pass / 28 fail.
  - Sol-37's HEAD baseline was 107 / 30.
  - No test fails here that is absent from that baseline fail list.
- The `tcl-lsp-db` tests were not run because the shared disk was below 1 GB free.

### Clippy

- `cargo clippy -p tcl-compiler --lib`: no lint falls on a changed line.
- The crate already had about 775 pre-existing pedantic warnings.

## Remaining work

- **Hosted/iRules fresh entry: not done.** It still uses unknown history, so all 23 `irules_event_checks` failures remain.
  - The F5 authored Tcl 8.4 policy route (`authored_logical_name_simulation`) loses the `when` lookup in the binding analysis: `rule_loader head=when targets=0 unknown absent`.
  - On the policy route `source_keys_checked` → `original_registry_command_paths` resolves through the original command world. The `when` lookup there comes back unknown and absent. The likely causes, not proven, are:
    - `select_policy` / `source_root_namespace_key` for the vendor input;
    - the baseline metadata slot index built from `InterpreterRuntime` semantics rather than the `RuleLoader` realm.
  - The no-policy route resolves `when` (targets=1).
  - Without a policy, hosted inputs have no source purpose (`retained_original_source_purpose` → None). Event bodies then get "set upvar-aliases a dynamic caller variable" barriers.
  - Design needed: either a realm-aware authored command world for the rule loader, or a hosted source purpose that the consumers accept.
  - Event frames are already selected per body (`connection_scope::event_resolve_context`, TmmIRule).
- **`bounds_checks` (13): unchanged.** These depend on the C2 dynamic-name barrier and the SCCP branch facts.
- **`fp::` and `diagnostics::tests`: 139 and 252 failures remain.** They are mostly:
  - C2: `codes_for`/`w210_codes` standalone ingress and expression-substitution sites;
  - C3: const-`$cmd`/object dispatch, including most `dispatch_table_*` navigation tests, which fail at baseline too;
  - C6: standalone builders publish no named procedure; the `run_all_checks` half of `fp::codes` uses `build_for_profile`.
- **Integration binaries: not sampled.** These are `optimiser_coverage`, `checks` and the others from `ci-5201` shards 1 and 5. A "before" build of each needed more disk than was free.
- **Proof manifest digests.** The proof-check script already failed before this change. `check-name-resolution-proofs.py` will need the evidence digests refreshed for the edited files.
