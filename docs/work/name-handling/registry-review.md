# The registry: what is legitimately large and what is sprawl

**Question:** `tcl-registry` is about 606,000 lines. It is meant to define every Tcl command (Tcl 8.4–9.1, Jim, Tk, tcllib, itcl, iRules and BIG-IP, the EDA dialects, Expect, SpecTcl packs), *including the documentation*. Is it well organised, does anything else restate what it owns, and do its own facts agree?

**Method:**
- An independent structural review of the crate and of every place that duplicates its content.
- An in-process probe over every spec (2,300 command specs, 1,178 subcommands) comparing the facts each spec states more than once.
- Checks against tclsh 9.0 and Tk 9.0.3 for the disagreements found.

## Verdict

**The command data and its documentation are the legitimate bulk, and they are in good shape.**
- `commands/` is 185,000 lines of many small files.
- 1,899 hover blocks; only 3 specs have none.
- 1,017 have examples and 758 have return values.
- Sources are cited (Tcl and Tk man pages, tcllib packages, F5 clouddocs).
- The LSP hover, MCP `command_info` and the generated iRules event JSON derive from them.

**The sprawl is in four other places:**
- the registry also holds about **58,000 lines of interpreter algorithms** and about **176,000 lines of BIG-IP configuration schema**, neither of which is command data;
- about **52,700 lines are marked "@generated … Regenerate to refresh" with no generator in the repository**;
- a spec states the **same fact in several fields** that nothing cross-checks, and they already disagree;
- the engines state **usage and arity a third time**.

## What it holds

| Part | Lines | What it is |
| --- | --- | --- |
| `commands/` | 185k | Tcl command specs and documentation (iRules 63k in 1,018 files, Tcl 48k, tcllib 27k, Tk 22k, stdlib 11k); about 11k of it tests |
| `bigip/`, `base_objects/`, `profile_defaults/`, `f5/` | about 176k | BIG-IP configuration schema and a generated snapshot of the 17.1 base configuration; used only by `tcl-bigip`, `tcl-bigip-query` and `bigip-report-gen` |
| `events/`, `event_facts/`, `profiles/`, event descriptions | about 8k | iRules event and profile data |
| 110 `native_*` modules and their directories | 58k | algorithms modelling C Tcl and Jim internals: result protocols, eval objects, instruction plans, TclOO method-cache epochs, per-release compile recipes, Jim reference code embedded as Tcl |
| `value_transfer/`, `registry/`, `resolved_invocation/`, `model/`, `definer/`, `state_transition/` and other top-level files | about 130k | descriptor types plus resolution and evaluation engines (`value_transfer` alone has 763 functions) |

That is roughly 30% Tcl command data, 31% BIG-IP configuration data and 35% logic.

## Findings

1. **Interpreter algorithms in the data crate.**
   - The design contract says "The registry is data … they do not reach into compiler internals" (`docs/design/contracts/project-layout.md`, rule 3). Yet the `native_*` modules are algorithms, and `tcl-vm` imports them in 157 files.
   - The shared-utility manifest names them as owners of C runtime protocols, so the two documents contradict each other.
   - Every VM change touches the registry, and per-release behaviour is spread over about 40 modules (about 400 `V9_1` sites in 144 files).
2. **Generated files that cannot be regenerated.** These carry a "do not edit, regenerate" header with no generator in the repository: `base_objects/generated_{objects,names}.rs`, `bigip/references.rs`, `profile_defaults/generated.rs`, `events/event_props_table.rs`, `profiles/profile_table.rs`, `event_facts/generated.rs`, and `tcl-bigip/src/model/gen/`. By contrast, `closed_evidence` has `scripts/dev/gen_closed_evidence.py`. A new BIG-IP release means hand-editing "generated" code.
3. **BIG-IP configuration schema inside the command registry.**
   - It is restated by `tcl-bigip`'s generated model (17,000 lines) with no gate between them.
   - `conf_tokens.rs` partly restates the reference edges.
   - The `registry-axes` ratchet counts 219 places in `tcl-bigip` that compare against registry words.
4. **The same fact stated in several fields, unchecked.**
   - **Lowering identity** is stated in five to eight fields per spec (`successful_handler`, `native_compilation.operation`, `lowering_hook`, `native_lowering`, `codegen_hook`, `inline_codegen_hook`, `semantic_operation`, `const_fold`). Only 9 commands are cross-checked.
   - **Variable effects** are stated in traits (`FIRST_ARG_VARNAME`, `READS_BEFORE_WRITE`, `UNCONDITIONAL_VARIABLE_WRITE`), `arg_roles`, `assigns_variable_at`, `side_effects` and per-form roles.
   - **Mutation** is stated as `mutator: true` without side effects on iRules mutators, plus separate taint subcommand lists.
   - **The synopsis** is stated in both `forms` and `hover.synopsis`: 1,260 files repeat it identically and 209 differ.
5. **Arity disagrees with the synopsis, with a measured false error.**
   - Top-level arity is not checked against the synopsis; only flat subcommand lists are.
   - The probe found 510 of 2,198 commands whose declared arity differs from their synopsis, and 157 where the arity is *stricter*. Many come from the probe's rough synopsis parser (operators, option lists), but some are real:
     - **Tk `destroy`** is declared "at least 1". Tk accepts no arguments, and the synopsis says `destroy ?window window ...?`. The analyser reports a false E002 that quotes that contradicting synopsis.
     - **Tk `bell`** is declared unlimited, but Tk refuses more than three arguments ("wrong # args: should be "bell ?-displayof window? ?-nice?""), so errors are missed.
   - C Tcl and Tk print their own usage line in "wrong # args" errors, which makes them an oracle for both fields.
6. **Name-keyed tables inside the registry** that restate spec fields: for example `value_transfer/reference_body.rs`'s `variable_words` restates the variable positions of `set`, `incr`, `lappend`, `lassign`, `foreach`, `scan`, `regsub`, `regexp`, `dict`, `binary` and `switch`. The `registry-axes` lint does not scan inside the registry.
7. **Engine usage and arity as a third statement.** About 300 "wrong # args: should be" literals across `tcl-vm`, `runtime/rust` and `tcl-cmd-core`, each with its own count check, are tied to the registry only by command existence (`command-backing`). C-exact messages may legitimately differ from man-page synopses, but nothing compares the two.
8. **Checked-in generated copies with weak gates.**
   - The SpecTcl equivalence packs (`equivalent_tcl8_6.tclspec`, 62,000 lines; `equivalent_f5_irules.tclspec`, 33,000) carry all the hover prose, but their check is advisory and outside `xtask-check`.
   - The 7.3 MB report-generator WASM embeds registry data and is rebuilt by hand.
9. **Documentation held outside the specs.**
   - `clock format` specifier documentation exists only in `tcl-lsp-core` (`hover/format_specifiers.rs`).
   - Event prose is a separate name-keyed table with no completeness check.
   - Spec field documentation is written three times (Rust doc comments, the Spec Studio schema, Spec Studio help).
   - MCP `command_info` answers only for iRules.
10. **Authoring cost.**
    - A new iRules command touches three files with four edits (plus runtime backing); a new Tcl command touches a file, two lines in `tcl/mod.rs` and both engines.
    - A new `CommandSpec` field (147 today) touches the struct, defaults, the SpecTcl loader and renderer, the Spec Studio schema and help, and the parity list.
    - A new Tcl release touches about 400 sites.

## Design

- **Split by kind, not by size:**
  - `tcl-registry` keeps command data, documentation and descriptor types.
  - The `native_*` interpreter models move to a native-model crate beside `tcl-runtime-api`, which the VM, the runtime and the compiler depend on.
  - `value_transfer` and `resolved_invocation` evaluation move to the analysis layer.
  - The BIG-IP configuration schema becomes its own data crate (`tcl-bigip-schema`) that `tcl-bigip` and the report generator depend on, with `tcl-bigip`'s model generated from it and gated.
- **Every generated file has a generator in the repository** and a `--check` gate. Files that cannot be regenerated lose their "generated" header and are treated as hand-maintained data with a schema check.
- **One statement per fact, the rest derived or checked:**
  - lowering identity becomes one `operation` field from which the hooks are derived;
  - variable and world effects come from one effect descriptor (plan item 80), with traits derived from it;
  - the synopsis is stated once in `forms`, and hover displays it;
  - arity is derived from the forms or checked against them.
- **Usage conformance:** a vector of every command and subcommand's "wrong # args" usage line from each tclsh, Tk and Jim release, checked against the spec's synopsis and arity and against the engines' messages. It replaces the three unconnected statements with one checked chain.
- **Documentation complete in one place:** clock and format specifier documentation, event prose and spec-field documentation in the registry, with completeness gates. MCP `command_info` answers for every dialect from the same specs.
- **Release cost:** per-release facts in tables keyed by release (plan item 79) rather than `V9_1` branches inside algorithms.

## Plan

Items 116–122 in [plan.md](plan.md). The false `destroy` E002 and the missed `bell` arity are correctness fixes for the current release scope.

## BIG-IP questions answered

The questions this review raised were answered with lab evidence (BIG-IP VE 21.1.0.1 build 0.0.26; [data provenance](evidence/bigip-data-provenance.md), [language and release](evidence/bigip-language-and-release.md)):

- **Generated files without a generator (Q2).** Git history recovers the original `tcl-bigip` model generator (`553b71b6dd:scripts/codegen/gen_bigip_model_rust.py`, removed in the Python retirement) and an earlier profile-default generator (`58633c7049:scripts/registry-audit/gen_profile_defaults.py`). The base-object extractor, the reference-edge emitter and the event-property and profile-table emitters were not found. Decision per file: rebuild the base-object extraction from a versioned manifest; derive reference edges from the property descriptors; port profile-default extraction to Rust over the SCF parser; keep event properties and profile relationships as authored data with provenance and consistency checks, generating only projections. Finding 2 above stands for the unrecovered emitters; plan item 119 now follows these decisions.
- **Versions (Q3).** 17.1 is the only base snapshot. The 21.1 appliance's shipped and effective SSL `options` (`dont-insert-empty-fragments no-ssl no-tlsv1.1 no-tlsv1`) differ from the registry's open-ended 17.1 override (`no-tlsv1.3 no-dtlsv1.2`, `profile_defaults/mod.rs:331`–`:339`), so overrides must carry release intervals. Snapshots are captured per exact build with deltas or computed intervals; 17.1.x, 17.5.x and 21.1.x are the live lanes, 15.1.x and 16.1.x historical.
- **Placement (Q4).** The BIG-IP configuration schema becomes its own data crate consumed by both `tcl-registry` and `tcl-bigip`; `tcl-bigip`'s generic model is generated from it plus explicit annotations; `tcl-bigip` owns SCF semantics, scoped names, typed values and the device graph; query, report and XC own projections; iRules event order, phases and profile requirements stay in the registry (finding 3 and plan item 118 now follow this).
