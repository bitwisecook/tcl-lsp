# Outstanding work: compiler-names agent

Area: command_binding.rs (except the C6 hunks), source name lookup, analyser scope/commands, nested lookups.

Last updated: 2026-10-22T10:50Z

Maintained by the compiler-names agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress

- **R1 guardrail on "undeclared descriptors count as closed"** (`command_binding.rs`, `command_world_closed_on_normal_return`). An invocation that has no closed receipt leaves the completed world available only if all of these hold:
  - it resolves to an unperturbed registry `Builtin` with no prefix;
  - its argument roles are complete and its subcommand was selected;
  - it has no command-table mutation;
  - if it declares transitions: the list is empty and there is no re-entrant callback (`requires_world_barrier`);
  - if it declares no transitions (`puts`): its traits claim no code evaluation, dynamic barrier or trace. That rules out evaluators and extension-default commands.

  New negative cases in `completed_world_follows…`:
  - `proc bad {} {missing_command}; bad`
  - `lsort -command greet {b a}`
  - `string $op x`
  - `eval $script`

  Positive cases: `proc say {} {puts hi}; say`, plus the existing `puts hi`, `[list ..]` and `[string length x]`.

  The build is running; the first attempt was OOM-killed. Ruling needed (see Blocked): `puts` has the registry's conservative wildcard footprint (UNKNOWN callback, AnyInterpreter).
- **(a) LSP-level R1 check.** Run the tcl-lsp-core `references_rename` tests one at a time after the compiler build: `rename_proc_updates_definition_and_all_call_sites` and `references_proc_from_definition_includes_decl_and_both_calls`.
- **Import alias inside `namespace eval`.** Root cause located. `namespace import` compiled inside an eval body (mode BytecodeObject) gets `actual_ensemble_plan` → Unknown, so the selection is `unknown=true` and the state goes opaque. `namespace export` in the same position gets NamedInvocation. Temporary `ZZ_PROBE_ENSEMBLE` probes in `ensemble_compilation.rs` will show which branch. Note: `original_namespace_import_completion_requires_closed_prelude_and_destinations` deliberately withholds the completed world while `auto_import` is present. A fix must keep that, and give the import site a resolved alias without claiming completion.

## Outstanding (in priority order)

1. Finish R1 guardrail verification: rerun the `command_binding::` filter, the 10 R1 regression tests and `completed_world_follows…`.
2. Import alias fix. Coordinate through the coordinator if it touches C6 hunks.
3. W210 in pkgIndex (`w210_pkgindex_reports_other_reads_inside_command_substitutions`). Run with `ZZ_PROBE_LOWER=1` (cfg_builder barrier flags), then fix or hand back.
4. (b) `namespace eval ::a { oo::class create Widget {} }; namespace eval ::a { Widget new }` rename → incomplete coverage. Find the LSP coverage refusal.
5. Remaining work-only `signature_scan` failures. These are real bugs, not wrong expectations:
   - `formal_parameters::…lambda…`: fixed (the whole-specifier field keeps its escaped source extent).
   - `formal_parameters::…worker…`: the extent part is fixed. Its counterfactual tcl9.1 policy-decline assertion still fails for one dialect, now labelled with the dialect name.
   - `signature_publication_keys_match_six_native_namespace_controls`: Jim only. jimsh 0.84 gives `namespace eval : {proc p}` → `:::::p` (current namespace `:::`), but the scan publishes `::p`, so the `:` namespace is lost. A Jim colon-rule bug, overlapping review N02.
   - `factory::tests` ×2: `::generated` is not published from `factory generated {} {}`.
   - `original_import_publication_tests`: no import pattern or declaration, from `namespace import` inside `namespace eval`. Likely the same root cause as the import alias item.
   - `ctx::…nested_bom_purpose`: hosted F5 `namespace eval nλ` procedures are missing.
6. (c) Class `variable n` rename (R3). Low priority.
7. Before hand-back:
   - remove the ZZ probes (`command_binding.rs` ZZ_COMMAND/ZZ_UNCLOSED, `cfg_builder` ZZ_EMBEDDED/ZZ_CALL_BARRIER, `ensemble_compilation.rs` ZZ_ENSEMBLE, test `zz_probe_import_and_crlf`);
   - run rustfmt and clippy on changed lines;
   - rerun the filters (command_binding / registry_invocation / signature_scan / source_graph);
   - update compiler-names-NOTES.md and the patch.

### Pending triage routing (external review naming-migration-review-20261022.md)

- N01: procedure and class identity keyed by a non-injective display string. `namespace eval a: {proc p}` and `namespace eval a {proc :p}` both become `::a:::p`, and the empty-tail `n::` is normalised away in workspace links.
- N02: there is no Jim command-name policy. Jim keeps `a::p` and `a:::p` distinct. The Jim `:` namespace scan failure above may belong here.
- N03, parser-debt paragraph: `is_braced_indirect_array_ref`, `normalise_var_name_for_style`.
- Section 3.3: `insert_rebound_candidates` treats any `::` as absolute.

Constraint until routed: build nothing that re-parses display strings into identities or assumes C Tcl colon rules for Jim.

## Blocked (on whom, and what is needed)

- **Coordinator ruling on `puts`.** The registry gives `puts` no declared transitions and no declared world effects. Its footprint is the conservative wildcard (`CallbackKinds::UNKNOWN`, `Reentrancy::AnyInterpreter`), which the registry docs call "missing metadata never means a command is pure". A strict reading of "anything the registry marks as having unmodelled effects" would withhold the world after `puts hi`, and so after `puts [greet]` (item (a)).

  Current cut: accept the undeclared wildcard only for a complete registry Builtin whose traits claim no evaluation, barrier or trace. Declared callbacks (`lsort -command`, `eval`) and extension-default commands are rejected.

  The alternative is to declare `puts`'s footprint in the registry. A reflected channel or `chan push` transform is a genuine re-entrant callback in C Tcl, so that would still need a ruling.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Earlier items, recorded in `reviews/merge-inventory-20261019/compiler-names-NOTES.md`:
  - B (self-dispatch tape), C (math in `[..]`);
  - nested `[..]` lookups and consumed-name lookups;
  - R1 first cut and the `unclosed_command_world` refinement (all 10 command_binding pass→fail fixed);
  - R2 (`return` lookup in entered bodies);
  - CRLF method bodies (`crlf_class_method_bodies_analyse_without_lexer_panics` passes);
  - `walker::multiple_handlers` fixture;
  - café expectation.
- `signature_scan::formal_parameters::tests::original_readonly_lambda_formal_value_preserves_ancestry_and_source_fields` now passes (build of 10:07, before the guardrail edit).
