# `unknown`, `namespace unknown` and autoloading

How a call that finds no command is handled — the global `unknown` procedure, `namespace unknown` handlers, ensemble `-unknown`, TclOO's `unknown` method — and the library machinery behind the stock handler: `auto_index`, `tclIndex`, `auto_load`, `auto_mkindex`, `auto_reset`, `auto_qualify`, `auto_import`, `auto_execok`, `tclPkgSetup` lazy packages and `tcl_interactive`. Then how the analyser and the editor treat such calls (W123, navigation to autoloaded procedures, the workspace `tclIndex`). Method: 16 fixtures in [`oo-dispatch-probe/unknown-autoload/`](handover/oo-dispatch-probe/unknown-autoload/) run under tclsh 8.4.20–9.1.0, Jim, the VM and the runtime (with and without `run_script --init`, which loads the standard library) on `afe2f2d8` (`engines.tsv`); the analyser, definition, hover, references and rename through the MCP server (`analyser-mcp.out`); and a read-only census.

**Verdict:** dispatch through `unknown` is **right in both engines** for the global procedure, `namespace unknown` (handler words, reset, failing and missing handlers), ensemble `-unknown` and the default error and errorCode; the stock library works in the VM and, once loaded with `--init`, in the runtime. Two wrong answers: the **VM ignores a TclOO `unknown` method** (an error instead of dispatch), and the **runtime's `namespace import` never calls `auto_import`**, so a command named in `auto_index` is not imported. Analysis answers W123 in **four layers that each decide part of "is this command absent"** — command-binding slot presence, a separate scan of a user `unknown` procedure, a workspace-autoload check, and a server-side filter that deletes W123 after the fact using a third `tclIndex` reader — and on the cached MCP build they give false W123 for commands defined through `auto_index`, false W123 when a user `unknown` accepts everything, no W123 after `rename unknown {}`, and no W123 anywhere in a file once any namespace has a handler. HEAD's tests cover the `proc unknown` and `rename unknown` cases; the others are open.

## Measured

Execution (8.4 lacks `namespace unknown` and ensembles; 8.4/8.5 lack TclOO):

| Case | tclsh | VM | runtime (bare) | runtime `--init` |
| --- | --- | --- | --- | --- |
| user `proc unknown` gets the words; with `unknown` deleted, `invalid command name` (`u01`) | as Tcl | same | fails: no `unknown` to rename | same as tclsh |
| `namespace unknown` handler, extra words, reset, failing and missing handler (`u02`, `u03`, `u16`) | as Tcl | same | same | same |
| `auto_index` entry evaluated on first call; `auto_load`; namespaced key (`u04`) | as Tcl | same | `invalid command name` | same as tclsh |
| `auto_mkindex` then `auto_path` autoload (`u05`) | as Tcl, 20 ms | same, but **17–20 s** in the debug build | no `auto_mkindex` | same as tclsh |
| `auto_execok`, `auto_qualify`, `auto_reset`, recursion guard (`u06`, `u11`, `u14`, `u15`) | as Tcl | same | missing | same as tclsh |
| `tclPkgSetup` package: `package require` defines nothing; first call sources (`u08`) | as Tcl | same | `can't find package` | same as tclsh |
| ensemble `-unknown` returning a prefix, and one that adds the subcommand (`u09`) | as Tcl | same | same | same |
| **TclOO `unknown` method** (`u10`) | dispatches: `unknown method missing: 1 2` | **error** `unknown method "missing": must be destroy, known or unknown` | correct | correct |
| **`namespace import` of an `auto_index` name** (`u12`) | `auto_import` loads it; `util` runs | same | not imported | **not imported** |
| default unknown errorCode `TCL LOOKUP COMMAND x` (`u13`) | 8.6+ | same | same | same |
| `tcl_interactive` in file mode (`u07`) | defined, `0` | **not defined** | not defined | not defined |

No engine runs `exec` or abbreviation fallback from a script file, as tclsh does not (`info script` is set). Jim has no `namespace unknown`, `auto_*` or TclOO.

Analysis (MCP build of 19 Nov; dialect 9.0):

| Case | Report | HEAD |
| --- | --- | --- |
| `set auto_index(autofoo) {proc autofoo …}; autofoo 1` (`u04`, `u14`) | **W123** (false) | `auto_index` writes are now modelled as future entries (`command_binding/normal_result.rs:822`, `analyser/diagnostics/fp/obj.rs:556`); W123 not re-checked |
| `proc unknown args {…}`; `zzz` | **W123** (false: every call succeeds) | test asserts none (`analyser/diagnostics/unresolved.rs:1109`) |
| `rename unknown {}`; `zzz` | nothing (missed: certain error) | test asserts W123 (`unresolved.rs:1112`) |
| one `namespace unknown` in namespace `h`; `zzz` in `other` and at global | nothing (missed) | not checked |
| `lappend auto_path /some/lib`; `zzz` | nothing | correct abstention: any command may autoload, but it is file-wide |
| `[catch {echo hi} m]` inside a quoted word (`u07`, `u15`) | nothing | nested commands skip W123 ([fault 8](cross-cutting-review.md)) |
| ensemble `-unknown` handler's first parameter (`u09`) | **W214 unused** (false: fixed signature) | — |
| `$o missing` on a class with an `unknown` method / without (`u10`) | nothing / W308 | correct |
| `ls` | W123 "did you mean `le`?" (an `expr` operator) | — |

Navigation: definition on an `auto_index`-defined call returns nothing; definition on `$o missing` does not offer the class's `unknown` method; hover on a call routed to a namespace handler returns nothing. References and rename of a `namespace unknown [list ::ns::handler]` target find and rewrite the handler word correctly. Workspace `tclIndex` navigation is not reachable through the single-document MCP tools and was not measured.

## Census

**Engines.** The VM sources the real `init.tcl` from `TCL_LIBRARY` (`tcl-vm/src/interp/bootstrap.rs`); the runtime embeds a vendored library (`runtime/rust/src/embedded_stdlib.rs:53`, `vendor/tcl_library`) but `run_script` loads it only with `--init`, and `set_startup_globals` (`interp/setup/bootstrap.rs:84`), which sets `tcl_interactive`, is not reached in file mode (*inference* from `u07`). Unresolved-command dispatch is written twice: VM `interp/namespaces/path_and_unknown.rs:88–154` and runtime `interp/dispatch/command_dispatch.rs:316` with `cmd_namespace.rs:331`. The import preload is a registry fact (`tcl-registry/src/state_transition/namespace_transition.rs:201` `import_preload_command`) used by the VM (`interp/namespaces/export_import.rs:410`) and by command binding (`original_namespace_binding.rs:733`) but **not by the runtime**. The TclOO `unknown` method: the runtime walks it (`cmd_oo/engine/method_invocation.rs:485`); the VM returns the "unknown method" error as soon as the call chain is empty (`cmd_oo/method_resolution.rs:291`). Ensemble unknown dispatch is copied per engine (VM `interp/commands/ensemble_dispatch.rs`, runtime `interp/dispatch/ensembles.rs`).

**Registry.** Specs for `unknown`, every `auto_*` command, `tclPkgSetup` and `tclPkgUnknown` (`commands/tcl/`), `auto_index` as a special variable (`special_vars.rs`), the `UNRESOLVED_COMMAND_HANDLER` trait (`traits.rs:1033`, read in three compiler files), the `NamespaceUnknown` world-state domain, the `allow_unknown_commands` SpecTcl setting (`scoped.rs:129`), and roster entries in `irules_policy.rs` and `f5/corpus.rs`.

**Analysis and editor — W123 is decided in four places:**

1. Command binding's selected-slot presence at the source point (`tcl-compiler/src/analyser/diagnostics/unresolved.rs`, 1474 lines), with `has_earlier_unknown_mutations` and the `allow_unknown_commands` escape.
2. A second scan of a user `unknown` procedure's dispatch shape (`analyser/oo/unknown_proc.rs`, 415 lines, `UnknownProcInfo`): whether it chains the original handler, spawns processes or loads units.
3. Workspace autoload (`unresolved.rs:193` `is_workspace_autoloaded`), from sibling files offered as autoload modules.
4. A post-filter in the server (`tcl-lsp-server/src/diagnostics/command_refinement.rs:94`, 571 lines) that deletes W123 when the package database's `tclIndex` or a required package's files define the name, treating a guarded (`Conditional`) package as present.

**`tclIndex` is read three ways:** `tcl-lsp-core/src/package_resolver.rs:452` `parse_tcl_index` (both index formats, for the package database), `package_resolver/original_names/index_loader.rs` (1283 lines, the "original source" survey of index filenames), and the compiler's own reading of `set auto_index(…)` rows in analysed source (`registry_invocation/source_structure/registration_abstention.rs`, `source_load.rs`). **`auto_qualify` has one owner** (`tcl-syntax/src/naming/autoload.rs:31` native, `:108` analytical), wrapped by `package_resolver.rs:717` and used by binding (`source_name_lookup.rs:1095`, `native_autoload_source.rs:292`). Hard-coded names: `command_binding/source_transition_advice/source_class.rs:529` and `:549` match the method name `"unknown"` directly rather than through an object-system profile. Navigation to `tclIndex` targets lives in the server (`navigation/definition.rs:191`, `navigation/hover.rs:285`), not in `tcl-lsp-core::definition`, so MCP and CLI definition cannot follow it.

**Disagreements.** The analyser's per-point world already records `namespace unknown` handlers per namespace (`ModuleCommandBindings`, [design integration](design-integration.md)), but the measured W123 behaviour treats one handler as covering the whole file. The server filter answers "present" for a `Conditional` package while binding answers per point; a W123 that binding proved absent at a source point (after `rename p {}`) can still be deleted by the filter because a `tclIndex` maps the name, which is right only if `auto_path` reaches that index at that point. The runtime and the VM disagree on `auto_import` and the TclOO `unknown` method.

## Design

- **Absence is a query of the potential-execution owner.** W123 is "no binding at this point, and no handler that could supply one": the per-point name world (decision 2) holds the global `unknown` command's identity, each namespace's `unknown` handler, `auto_index` entries as possibly-bound slots, and `auto_path` reachability as a provider. The answer is `Answer<CommandRef>` (decision 7): **absent** only when the slot is empty, the effective handler is the stock `unknown`, and no reachable index or `auto_index` entry offers the name; **unknown with reason** "a user handler may supply it" or "an index on `auto_path` may supply it" otherwise; **proven** when an index or `auto_index` script defines it, with a receipt naming the index line.
- **Handlers are registry contracts, not scans.** The stock `unknown`, `auto_load`, `auto_import` and `tclPkgSetup` get effect descriptors (`unknown`: may evaluate `auto_index($name)`, may `exec` only when interactive at level 0 with no script; `auto_import`: may evaluate `auto_index` entries matching the pattern). A user `unknown` procedure becomes a summary (decision 6) — "accepts every name", "chains the original", "unknown" — replacing `unknown_proc.rs`'s scan.
- **One index model.** A `tclIndex` is a provider behind the lookup owner's interface (the workspace index of phase 3), read by one parser that both the package database and the analyser use; `auto_index` rows in analysed source go through the same model. Navigation and hover follow the receipt to the defining file in `tcl-lsp-core`, so the server, MCP and CLI agree.
- **Object systems.** TclOO's `unknown` method, ensemble `-unknown` and namespace handlers are object-system and namespace profile facts (fixed handler signatures, so W214 abstains; method-dispatch fallback, so definition can offer the `unknown` method with a "fallback" receipt).
- **Engines.** Unresolved-command dispatch belongs behind the `Namespaces` engine trait ([fault 10](cross-cutting-review.md)), with the import preload and the TclOO fallback implemented once per engine against registry facts; the runtime's runner loads the library by default.
- **Conformance.** The fixtures become matrix rows: engines against tclsh, and W123/W214 and definition columns for the analyser.

## Plan

- **VM: dispatch to the TclOO `unknown` method:** when the call chain for a method is empty, build the chain for `unknown` with the method name prepended before reporting "unknown method" (`tcl-vm/src/cmd_oo/method_resolution.rs:291`), as the runtime does (`cmd_oo/engine/method_invocation.rs:485`). Done when `u10` matches tclsh 8.6/9.0 and a VM end-to-end test pins it.
- **Runtime: call `auto_import` from `namespace import`:** read `NamespaceTransition::import_preload_command` as the VM does (`export_import.rs:410`). Done when `u12` matches tclsh with `--init`.
- **Runtime runner loads the standard library by default:** make `--init` the default of `run_script` (or the conformance harness passes it) and set `tcl_interactive` in file mode in both engines. Done when every `u*` fixture matches tclsh under the plain runner and `u07` prints `1 0`.
- **Measure the VM's `auto_mkindex` cost:** profile `u05` (17–20 s debug against 20 ms in tclsh) and record whether release builds are acceptable; owner `tcl-vm` child-interpreter creation (about 1 s per `interp create` in the debug build).
- **Re-run the W123 rows at HEAD and make them tests:** `u04`, `u14`, the `namespace unknown` and `auto_path` variants and `u09`'s W214, in `analyser/diagnostics/unresolved.rs`'s test module beside the existing `proc unknown`/`rename unknown` cases. Done when each row is a test with tclsh's behaviour as the expectation.
- **Scope `namespace unknown` per namespace in W123:** take the handler of the call's namespace from the per-point world (`ModuleCommandBindings`), not a file-wide flag. Done when the `h`/`other` variant reports W123 for `other` and global calls only.
- **Fold the server's W123 filter into binding:** make the package database a provider that binding consults at the source point (`tcl-lsp-server/src/diagnostics/command_refinement.rs` → lookup owner), with `Conditional` giving `Answer::Unknown` with a reason rather than "present". Done when `command_refinement.rs` no longer edits diagnostics and the CLI, MCP and server give the same W123 set.
- **One `tclIndex` reader:** `package_resolver.rs:452` and `original_names/index_loader.rs` share one parser, also used for `set auto_index(…)` rows in analysed source. Done when one function reads both index formats and the others are deleted.
- **Replace `unknown_proc.rs` with a procedure summary:** "accepts every name", "chains original", "unknown", computed by the interprocedural summaries and read through the contract. Done when `analyser/oo/unknown_proc.rs` is gone and W123 behaviour is unchanged on the test corpus.
- **Navigation follows autoload receipts in `tcl-lsp-core`:** move `tclIndex`/`auto_index` definition and hover from `tcl-lsp-server/src/navigation/` into `tcl-lsp-core::definition` and `hover`. Done when MCP `goto_definition` on an `auto_index`-defined call returns the defining location.
- **Profile facts for handler signatures:** ensemble `-unknown` handlers, `namespace unknown` handlers and the TclOO `unknown` method get fixed-signature callback descriptors so W214 abstains (`u09`), and `source_class.rs:529`/`:549` read the method name from the TclOO profile instead of the literal `"unknown"`.
- **Stop suggesting operators:** W123's "did you mean" draws only from command names (`ls` → `le`).
