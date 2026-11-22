# Rename and alias tracking

How Tcl's `rename`, `interp alias`, import links, command traces and TclOO `forward`/`renamemethod` are modelled across execution and analysis, and how the editor's rename-symbol feature decides what to edit. Part of [tidying the architecture](architecture-tidy-plan.md).

**Verdict:** the engines are sound apart from one alias detail. Analysis follows the common cases correctly (navigation through `rename` and `interp alias`, arity through both, the old name absent after `rename`), but models rename and alias in about five places with separate identity schemes, and the editor rename is safe where it acts but refuses most programs that use `rename`, `interp alias`, imports, callbacks or qualified calls, usually without saying why.

## Measured behaviour

Fixtures in [`oo-dispatch-probe/rename-alias/`](handover/oo-dispatch-probe/rename-alias/), checked under tclsh 8.4.20 (where the commands exist), 8.6.18, 9.0.4 and 9.1.0.

**Tcl facts worth pinning:** an alias's target is resolved from the global namespace at every call, so it works for a target defined later, breaks when the target is renamed away and works again when it is recreated; a renamed alias stays registered under its creation name (`interp aliases` lists the old name, and `interp alias {} new {}` fails with `alias "new" not found`); renaming an import's origin keeps the import working and `namespace origin` reports the new name; `rename` inside `namespace eval` resolves both names relative to that namespace; command traces fire for rename and delete; a TclOO `forward` target is late-bound the same way.

**Execution:** 26 of 28 runs (14 fixtures, 8.6 and 9.0) match tclsh in both the VM and the runtime. Both engines delete a renamed alias by its *new* name, where tclsh refuses (`r05`).

**Analysis and navigation** (`tcl-lsp-core` definition and hover, analyser diagnostics, on `770a8c60`):

| Case | Result |
| --- | --- |
| call through the new name after `rename p q` (top level and inside `namespace eval`) | definition and hover reach `p` |
| call through `interp alias {} al {} t 1` | definition and hover reach `t` |
| too few arguments through the alias, or through the renamed name | E002 (correct) |
| the old name after `rename` | W123 (correct) |
| an alias whose target was renamed away | no diagnostic (the call fails) |
| a TclOO `forward` method | no definition, no hover |

**Editor rename** (rename `helper` to `zz` at its definition, apply the edits, run both scripts under tclsh):

| Use of `helper` | Edits | Behaviour |
| --- | --- | --- |
| plain calls, a call inside a procedure | applied | unchanged |
| a namespaced procedure with qualified and relative calls | applied | unchanged |
| a string argument that only spells the name | definition only | unchanged |
| renaming onto an existing alias name | refused | — |
| a `rename helper h2` operand, an `interp alias` target, `namespace export`/`import`, `after`/`{*}$cb`/`trace` callbacks, `info commands helper`, `$n` with `n` set to the name, a TclOO `forward` target, a qualified `::helper` call, `uplevel 1 {helper}` | refused, with no reason given | — |

Every edit applied was behaviour-preserving. The refusals include cases with a well-defined answer (operand words of `rename`, `interp alias`, `namespace import`, literal callback lists, qualified calls, `uplevel` bodies).

## Structure

- **Identity across rename has five schemes:** the VM's command identity index and generations, the runtime's per-interpreter generations with generation-exact import links, the compiler's `CommandIdentity` with allocation sites and incarnations, the analyser's legacy `renamed_commands`/`rename_offsets` maps (one string record per destination, with an "as of" offset standing in for identity), and the signature scan's own map. No command token is shared between the engines or between engines and analysis.
- **Alias semantics:** both engines resolve targets from the global namespace per call (Jim from the caller's), check loops at definition and refuse a rename that would close one; the VM has the call path twice. In analysis, command binding installs aliases with the shared `AliasTargetLookup`; the analyser skips non-global aliases and **drops non-literal prefix arguments silently** (`analyser/handlers/command_aliases.rs`, `filter_map(TransitionSubject::literal)`, *read*), so `interp alias {} a {} t $x` is treated as a plain second name (inferred); the reporting walk follows aliases late-bound with an eight-hop cap. The registry's `interp alias` spec treats the target word as a list-shaped prefix where C takes one command name (inferred).
- **Rename is modelled about five times in analysis:** the analyser's rename handler, the signature scan's, the command-binding original table (which resolves in scope, refuses an occupied destination and treats an empty name as delete), captured-mutation receivers and source-transition advice, and module-binding invalidation.
- **Editor rename has two tiers.** The original tier (`original_command_rename.rs`) refuses the whole rename on any command-name or command-prefix argument role, any non-define binding transition, any namespace or trace transition, or any name-reflecting command in coverage, which is why almost every program above is refused. The lexical tier (`rename/commands.rs`) follows indirection to the target, checks collisions only against procedures (the measured refusal onto an alias came from the other checks), skips unsafe or indirect sites, and returns an empty edit list instead of a reason; the MCP tool turns every refusal into `null`. `rename_safety.rs` covers member, receiver and variable hazards, not command aliases.
- **TclOO:** `forward` targets are late-bound in the object's namespace in the VM and kept as prefixes in analysis; `renamemethod`/`deletemethod` are implemented separately in each engine and move the member in the analyser; renaming an object command is handled in the VM.
- **Contracts disagree:** `docs/design/contracts/command-binding-and-aliasing.md` says renaming or deleting an import's source "splices the redirect out"; `command-resolution.md` and every measured interpreter keep the import working after a rename.

## Conformance

The shared command-resolution vectors (`tcl_syntax::naming::conformance`) have no mutation column, so there are no rename or alias vectors; each side pins its own cases (VM end-to-end tests, runtime alias tests, analyser unit tests, LSP end-to-end tests). Only a few tests apply rename edits and execute the result, and they skip when no tclsh is installed; none covers a rename or alias scenario.

## Plan

1. **Fix the engines' alias registration:** keep aliases under their creation name after `rename`, so `interp alias {} new {}` refuses as tclsh does; add the fixture to both suites. Merge the VM's two alias call paths.
2. **One command identity** shared by command binding, the semantic model and (through `tcl-runtime-api`) the engines' tokens; the analyser's string maps and the signature scan's map become projections of command binding's model or are deleted.
3. **One analysis model of rename and alias** in command binding, consumed by navigation, references, editor rename and diagnostics: aliases with dynamic prefixes are unknown rather than plain names; the target word is one command name; report a call through an alias whose target has gone; navigate `forward` methods to their targets.
4. **Editor rename that edits what it can prove and explains what it cannot:** edit the operand words of `rename`, `interp alias`, `namespace export`/`import` and literal callback lists when the lookup owner proves they name the symbol; edit qualified and `uplevel`-body calls; refuse with a stated reason (returned to the editor and the MCP tool) where a dynamic name, a name-reflecting command or an unproven link makes the answer unknown; check collisions against every command kind.
5. **Execution-checked rename:** apply the edits for every rename fixture and run both versions under each tclsh, making it a required check rather than one that skips when tclsh is missing.
6. **Rename and alias vectors:** add a mutation column to the command-resolution vectors (`rename`, `interp alias`, import, forget, delete, recreate) and run it through both engines, command binding and the LSP; correct the contract that says a source rename removes an import.
