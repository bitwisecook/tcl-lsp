# Outstanding work: compiler-names agent

Area: command_binding.rs (except the C6 hunks), command_binding/compiled_invocation.rs, source name lookup, analyser scope/commands, nested lookups, W210 pkgIndex entry plumbing.

Last updated: 2026-10-22T13:16Z (PAUSED by user instruction)

Maintained by the compiler-names agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress (paused; edits are on disk, verification build g4 not run)

1. **R1 guardrail plus global hook guard** (`rust/tcl-compiler/src/command_binding.rs`). Coordinator rulings of 11:20Z and 12:50Z.
   - `command_world_closed_on_normal_return(target, facts)`. A handler without a closed receipt keeps the completed world only when it:
     - resolves to an unperturbed registry `Builtin` with no prefix;
     - has complete argument roles and a selected subcommand;
     - has no command-table mutation;
     - if it declares transitions: the list is empty and there is no re-entrant callback;
     - if it declares none (`puts`): its traits carry no `extension_default::{EVALUATION, TRACES}`.
   - `installs_reentry_hook(facts)`: a COMMAND_PREFIX, TRACE or HOST callback, or the variable-trace trait, sets `unclosed_command_world` for the whole document (`lsort -command`, `chan push`, `chan create`, `trace add`).
   - `ModuleCommandBindings::unknown_handler_is_original()`: every `::unknown` target must be registry-backed and not source-allocated. The rule was relaxed after g3 and is **not yet verified**. g3's stricter rule (kind == Builtin) withheld the world for all positive cases, so the stock `::unknown` is not kind Builtin. The probe `ZZ_UNKNOWN_HANDLER` prints its targets.
   - Test `analyser::types::original_declaration_inventory_tests::completed_world_follows_the_normal_completion_path_and_entered_bodies_keep_lookups` now collects all mismatches. Its negative cases are unknown command, missing-command body, `lsort -command`, `string $op`, `eval`, `chan push`, `chan create`, `trace add variable`, `trace add execution`, `namespace unknown` and `proc unknown`.
   - Evidence: g1 (guard only) withheld everything except `lsort -command`, `chan push` and `proc unknown` (`compiler-names/g3-main.log` shows the g3 state).
2. **Import alias inside `namespace eval`** (`command_binding/compiled_invocation.rs`, now mine per coordinator).
   - Root cause: `original_named_selection_is_closed` (c0437decfb, 10-18) took the original-facts selection only when the transitions were empty or `literal_pattern_transfer_is_closed` held. That never holds for import while `auto_import` exists. Named compilers then fall back to `select_for_facts`, which returns Unknown, so the selection is unknown, the state goes opaque, and `p` gets no lookup. At top level (Direct mode) `p` resolves (see `compiler-names/g3-zz-probe.txt`: `ZZ_SELECT_TARGET … import … registered=Some(Unknown)` versus `export` → NamedInvocation).
   - Fix on disk: the gate requires only `facts.state_transitions.is_declared()`. C Tcl compiles `import`, `export` and `forget` alike with `TclCompileBasicMin0ArgCmd` (tclNamesp.c 8.6.18:151, 9.0.4:169). Closure stays the handler's obligation, so the auto_import contract test still withholds the world.
   - Removed the unused `original_namespace_binding::literal_pattern_transfer_is_closed`; `closed_pattern_transfer` is still used by the seal.
   - Not yet verified.
3. **W210 pkgIndex**, now mine per coordinator.
   - Root cause: the source walk raises at the undefined `$dir`, so `$missing` in `[file join $dir $missing]` is never walked and has no access record. The pkgIndex `dir` is only a report-level suppression (`pkgindex_implicit_vars`, analyser/diagnostics.rs ~597).
   - Plan:
     1. Message the compiler agent (a5be33b83bdcb93bf) before editing compilation_unit.rs.
     2. Add a defaulted `UnitBuildOptions` field carrying the file's pkgIndex fact or entry `incoming_formals`.
     3. For basename `pkgIndex.tcl`, start the walk with `incoming_formals = ["dir"]` (→ `bind_unknown_incoming`).
     4. Drop `pkgindex_implicit_vars` only if it becomes redundant.
   - Test row `other.tcl` corrected to `["dir"]` only. tclsh 9.0.4: `set f [file join $dir $missing]` → `can't read "dir": no such variable`, so `$missing` is never read. The comment is in the test; this still needs recording in NOTES.

## Outstanding (in priority order, after the above)

1. Verify g4: the main set, zz probes, W210 probe, the 8 regressions singly, `signature_scan::` and `command_binding::`. Report the numbers to the coordinator.
2. Run the LSP-level R1 check after the compiler is green: `cargo test --locked -p tcl-lsp-core --test references_rename rename_proc_updates_definition_and_all_call_sites -- --test-threads=1` and `references_proc_from_definition_includes_decl_and_both_calls`, then delete the executable.
3. Relative object name in `create` inside `namespace eval ::a` → `::a::rex`. The g3 zz probe shows `rex` resolving to root slot `[] rex`. Find where the constructed command slot is qualified (command_binding/object_instance.rs `constructed_command_name`, named_manufacture.rs).
4. (b) `namespace eval ::a {oo::class create Widget {}}; namespace eval ::a {Widget new}` rename → incomplete coverage. Compiler facts are good (world true; `Widget` has a lookup and reference to `::a::Widget` in g3), so this is likely on the LSP side; confirm and hand over.
5. Remaining `signature_scan` work-only failures:
   - `formal_parameters::…worker…`: the counterfactual tcl9.1 decline fails for one dialect (the message now names it);
   - Jim `namespace eval :` publication (`signature_publication_keys…`): jimsh gives `:::::p`, we give `::p`; overlaps review N02, so hold for triage routing;
   - `factory::tests` ×2: `::generated` is not published;
   - `original_import_publication_tests`: may be fixed by the import gate;
   - hosted `nested_bom_purpose`.
6. (c) Class `variable n` rename (R3). Low priority.
7. Before hand-back:
   - remove the probes (see Resume);
   - rustfmt my files;
   - clippy on changed lines;
   - final filters;
   - update NOTES and the patch.

### Pending triage routing (review naming-migration-review-20261022.md)

- N01: non-injective display-string identity (`a:` + `p` versus `a` + `:p` → `::a:::p`), and the empty tail `n::`.
- N02: no Jim command-name policy. The Jim `:` namespace signature_scan failure likely belongs here.
- N03 parser debt: `is_braced_indirect_array_ref`, `normalise_var_name_for_style`.
- Section 3.3: `insert_rebound_candidates` treats any `::` as absolute.

Until routed, build nothing that re-parses display strings into identities or assumes C colon rules for Jim.

## Blocked (on whom, and what is needed)

- Paused by user instruction (13:10Z). Waiting for the coordinator's resume message.
- The 8 `command_binding` pass→fail in g1 (`compiler-names/g1-command_binding-pass2fail.txt`) have been forwarded to the compiler agent to classify against C6. They were all ok in sol49 z2 (00:00). Their assertions (uncalled-body commands now carry proved execution targets; Reached versus Conditional) point at C6's `uncalled_body_after_load` and conditional in-frame hunks.

## Done since the last update (with evidence)

- Ruling on `puts`: keep the current cut, with the global hook guard added (on disk).
- `signature_scan::formal_parameters::tests::original_readonly_lambda_formal_value_preserves_ancestry_and_source_fields` passes (10:07 build). In `formal_parameters.rs`, a whole-specifier field keeps its escaped element source extent.
- Root causes found for the import alias and W210 (above).
- g1 full `command_binding::`: 864 ok / 316 FAILED. 8 pass→fail against my 00:12 run; attribution is with the compiler agent.
- g3: `crlf_class…` and `substituted_heads…` ok. `completed_world…` failed on positive cases, caused by the strict `::unknown` kind check, which is now relaxed.

## Resume

- **Next command** (one build, run, delete; filtered output capped at 20 MB):
  `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-names/buildrun.sh g4`
  - Logs: `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-names/g4-*.log`. To redirect them, edit `D=`.
  - It needs more than 2.3 GB free disk.
  - g4 has never run. Two attempts were stopped before the build finished: one when a pkill hit my own shell, and one by the pause.
- **Then:**
  1. Check `g4-main.log`: `completed_world…` should have no mismatches. If `puts hi` is still withheld, read `ZZ_UNKNOWN_HANDLER` in `g4-zz.log`.
  2. Check `g4-zz.log`: the eval-body import `p` should have a lookup and reference.
  3. Diff `g4-command_binding.log` FAILED against `compiler-names/baseline-command_binding-ok.txt`.
  4. Check `g4-signature_scan.log`.
  5. Then do the W210 plumbing (message the compiler agent first).
- **Files I touched this session** (beyond the earlier NOTES list):
  - `command_binding.rs`: guard, hook guard, `unknown_handler_is_original`, probes;
  - `command_binding/compiled_invocation.rs`: gate, probes;
  - `command_binding/original_namespace_binding.rs`: removed the helper, doc edit;
  - `command_binding/ensemble_compilation.rs`: probes only;
  - `analyser/diagnostics/dataflow.rs`: probe only;
  - `analyser/diagnostics/tests.rs`: `other.tcl` row;
  - `analyser/types.rs`: tests and zz probe;
  - `signature_scan/formal_parameters.rs`: fix and assertion message.
- **Temporary probes to remove before hand-back** (all `#[cfg(test)]` and env-gated):
  - `command_binding.rs`: ZZ_COMMAND, ZZ_UNCLOSED, ZZ_UNCLOSED_EARLY, ZZ_UNKNOWN_HANDLER;
  - `command_binding/ensemble_compilation.rs`: ZZ_ENSEMBLE ×4;
  - `command_binding/compiled_invocation.rs`: ZZ_REGISTERED, ZZ_SELECT_TARGET;
  - `cfg_builder/mod.rs`: ZZ_EMBEDDED, ZZ_CALL_BARRIER;
  - `analyser/diagnostics/dataflow.rs`: ZZ_W210;
  - test `zz_probe_import_and_crlf` in `analyser/types.rs` (copy: `compiler-names/zz_probe_import_and_crlf.rs.txt`).
- **Evidence in the repo:** `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-names/`:
  - `buildrun.sh`, `runfilters.sh`;
  - `baseline-command_binding-ok.txt` (00:12 run);
  - `g1-command_binding-pass2fail.txt`;
  - `g3-main.log`, `g3-regress.log`;
  - `g3-zz-probe.txt`: import selection, N05, rex, Widget rows;
  - `g3-w210-probe.txt`: presence probe showing only `$dir` accessed;
  - `zz_probe_import_and_crlf.rs.txt`.
