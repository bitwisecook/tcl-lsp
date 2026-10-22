# Outstanding work: compiler-names agent

Area: command_binding.rs (except the C6 hunks), command_binding/compiled_invocation.rs, source name lookup, analyser scope/commands, nested lookups, W210 pkgIndex entry plumbing.

Last updated: 2026-10-22T15:07Z (agent-reported work only)

Maintained by the compiler-names agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress (resumed in /workspace/naming-migration-active/tcl-lsp; g4 first)

Early g4 restarted from main at 15:06:56Z after coordinator Git CLEAR on fabfda18b. The first tool-sandbox attempt failed resolving sha2 before compilation. The network-enabled clean build then stopped after nine missing tracked include-file errors (five unique paths), zero tests, no product verdict; only its verified cargo/rustc subtree was terminated with coordinator authorisation. The coordinator materialised all 166 missing literal inputs from exact HEAD. The old paused orchestration was retired/reaped and failed logs preserved as `/tmp/compiler-names-g4/missing-includes-g4-main.*`, with `g4-main-interrupted.json`. Active logs/exits remain `/tmp/compiler-names-g4/`; only completed receipts are copied into evidence. Runner now aborts on build/harness failures while preserving assertion-failure receipts for attribution. No cleanup, audit admission or validated row changes.

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

N13 is additionally assigned after R1/W210: issue an authenticated source-only parent point/namespace/path seed from the existing private SourceLookupSnapshot/ModuleCommandBindings owner. Retain exact complete parent source/input/config/point/frame/provenance; refuse stale, wrong-frame, ambiguous or incomplete state. NativeCompilationEntry remains physical runtime capture and is never fabricated from namespace strings. LSP owns server/index transport and warm/fresh convergence; coordinate compiler compilation_unit hunks before editing. Planning only during early g4 and N17 build windows. Planned issuer `OriginalProjectSourceLoad::source_point_entry(&AnalysisResult)` returns an opaque Clone+Eq entry; LSP accepts this shape and separately transports current-interpreter project loads versus child-interpreter visibility advice. Hash/serialisation are not required by the host.

1. Verify g4: the main set, zz probes, W210 probe, the 8 regressions singly, `signature_scan::` and `command_binding::`. Report the numbers to the coordinator.
2. Run the LSP-level R1 check after the compiler is green: `cargo test --locked -p tcl-lsp-core --test references_rename rename_proc_updates_definition_and_all_call_sites -- --test-threads=1` and `references_proc_from_definition_includes_decl_and_both_calls`, then delete the executable.
3. Relative object name in `create` inside `namespace eval ::a` → `::a::rex`. C Tcl 8.6/9.0/9.1 independently confirm ::a::rex exists and ::rex does not (receipt `/tmp/compiler-names-g4/relative-object-oracle.json`). Static cause: OriginalNamedManufacture uses root-oriented C API publication instead of current-namespace object publication; TclNewObjectInstanceCommon calls TclGetNamespaceForQualName with current namespace. No fix written yet. The g3 zz probe shows `rex` resolving to root slot `[] rex`. Find where the constructed command slot is qualified (command_binding/object_instance.rs `constructed_command_name`, named_manufacture.rs).
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

- N01 assigned after R1: preserve typed declaration slots/allocation joins; the UI map is explicitly lossy and original_procedure_metadata already retains typed declarations. Prove current colon-collision behaviour before changing types. External review-triage owns actual provider replay; empty-tail link is reported fixed there.
- N02: no Jim command-name policy. The Jim `:` namespace signature_scan failure likely belongs here.
- N03 parser debt: `is_braced_indirect_array_ref`, `normalise_var_name_for_style`.
- Section 3.3: `insert_rebound_candidates` treats any `::` as absolute.

Until routed, build nothing that re-parses display strings into identities or assumes C colon rules for Jim.

## Blocked (on whom, and what is needed)

- External strict Clippy inventory (14:43, aeecf397, Rust 1.99.1) reports 789 Compiler and 379 VM errors; dependent consumers were not reached. This is external agent-reported evidence, not a local run or admission. Exact old-snapshot inventory `/tmp/naming-clippy-compiler-names.json` has 170 diagnostics/51 paths; names retains 156 after original_literal_argv_rewrite.rs 2 routed to LSP and body_effects.rs 8 plus four C6 root command_binding diagnostics routed to compiler. Two metadata tri-states will use compiler's InvocationMetadataSelection; class relation tri-states keep their separate meanings. Functional R1 attribution still precedes lint application. Plan off-tree during source holds; preserve guards and semantics, add no allows. Canonical incoming inventory: bigip-lsp/clippy-dependency-blockers-20261022.json.

- R1 and import fixes remain unverified; early g4 main/zz and eight regression attribution run first after coordinator GO. User decision 20261022T1317Z permits narrow after-load Reached corrections, jointly classified with compiler.
- The 8 `command_binding` pass→fail in g1 (`compiler-names/g1-command_binding-pass2fail.txt`) have been forwarded to the compiler agent to classify against C6. They were all ok in sol49 z2 (00:00). Their assertions (uncalled-body commands now carry proved execution targets; Reached versus Conditional) point at C6's `uncalled_body_after_load` and conditional in-frame hunks.

## Done since the last update (with evidence)

- Ruling on `puts`: keep the current cut, with the global hook guard added (on disk).
- `signature_scan::formal_parameters::tests::original_readonly_lambda_formal_value_preserves_ancestry_and_source_fields` passes (10:07 build). In `formal_parameters.rs`, a whole-specifier field keeps its escaped element source extent.
- Root causes found for the import alias and W210 (above).
- g1 full `command_binding::`: 864 ok / 316 FAILED. 8 pass→fail against my 00:12 run; attribution is with the compiler agent.
- g3: `crlf_class…` and `substituted_heads…` ok. `completed_world…` failed on positive cases, caused by the strict `::unknown` kind check, which is now relaxed.

## Resume

- **Next command** (separately locked complete cargo-test build/run filters; draining output capped at 20 MB; executables younger than 60 minutes retained):
  `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-names/buildrun.sh g4`
  - Active logs: `/tmp/compiler-names-g4/g4-*.log` (copy completed receipts into evidence); source `/tmp/tcl-lsp-naming-env.sh`; first run main/zz and eight regressions, broad batches later. Runner audited for stale paths, global lock, disk and bounded draining output before use.
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
