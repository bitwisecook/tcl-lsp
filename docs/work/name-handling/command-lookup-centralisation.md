# Command lookup is not centralised

This branch exists to give each naming question one shared owner ([naming API guide](naming-api-guide.md); [command-resolution contract](../../design/contracts/command-resolution.md)). For "which command does this name mean here, and is it there" that has not happened. The candidate *order* is shared; the *context* a name is looked up in, and the *answer* consumers act on, are not. This page records the evidence and the plan. It is the common cause behind causes 1 and 4 of [TclOO and Tk naming: root causes](oo-tk-root-causes.md).

## What the contract requires

- A body resolves in the namespace that is current when it runs: a procedure in its defining namespace; `namespace eval` and `namespace inscope` in the named namespace; `apply` in its lambda's namespace; `uplevel #0` in the global namespace; `after`, `interp alias` targets and traces from the global namespace at call time.
- A TclOO method body runs in the object namespace with `::oo::Helpers` on its path, then the global namespace. "The class's defining namespace is **never** searched." Where the object namespace is unknown statically, "Original lookup keeps that namespace unknown rather than treating the approximation as a frame receipt."
- Each purpose has one owner; consumers ask it rather than re-deriving the answer.

## What consumers actually answer

Each fixture in [`oo-dispatch-probe/frames/`](handover/oo-dispatch-probe/frames/) was run under tclsh 8.6.18 and 9.0.4, then given to `tcl-lsp-core` definition and hover and the analyser's diagnostics at the marked call, on `63d8e443`:

| Call context | Tcl runs | Go to definition | Hover | Diagnostics |
| --- | --- | --- | --- | --- |
| procedure in `::a` | `::a::helper` | `::a::helper` | `::a::helper` | — |
| `namespace path ::lib` | `::lib::helper` | `::lib::helper` | `::lib::helper` | — |
| TclOO method, helper also in the class's namespace | `::helper` | `::helper` | `::helper` | — |
| `next` in a method | `B`'s `m` | `B`'s `m` | "built-in command" | **W123 unresolved** |
| `my b` in a method | `C`'s `b` | `C`'s `b` | method `::C::b` | **W123 on `my`** |
| `apply {… helper} ::lib` | `::lib::helper` | `::lib::helper` | "Possible procedure" | — |
| `uplevel #0 {helper}` in `::a` | `::helper` | `::helper` | "Possible procedure" | — |
| `namespace eval ::a {helper}` | `::a::helper` | `::a::helper` | `::a::helper` | — |
| `namespace import` | `::lib::helper` | `::lib::helper` | `::lib::helper` | — |
| `interp alias {} ::a::al {} helper` | `::helper` | none | none | — |
| `namespace inscope ::lib {helper}` | `::lib::helper` | none | none | — |
| `after 0 {helper}` from `::a` | `::helper` | none | "Possible procedure" | — |
| `namespace code {helper}` in `::lib` | `::lib::helper` | none | "Possible procedure" | — |
| TclOO `forward f helper` | `::helper` | none | none | — |
| `helper` in `::b`, defined only in `::a` | error | none | "Possible procedure" | W123 |

Three features give three answers for `next`; definition and hover disagree for `apply` and `uplevel`; five evaluation contexts have no answer at all.

## Why: the census

A read of every crate (on `63d8e443`; decisive lines spot-checked where marked *read*) found about **22 resolvers**. All candidate orders come from one primitive, `tcl_syntax::naming::command_lookup_bases`, through a String family (`command_resolution_candidates`, `resolve_command_with`) and a Native family (`NativeNameProtocol` projections, with the 8.4 and Jim recipes). Above that primitive nothing is shared.

**At least seven resolvers deviate from the order or the existence rule:**

| Resolver | Deviation |
| --- | --- |
| `Analyser::finalise_invocation_resolutions` (`tcl-compiler/src/analyser/scope/invocation_resolution.rs:552`) | recovers the namespace from the resolved name by string stripping; with an implicit helper path it drops the current namespace and puts the helper first (:712-724, *read*) |
| `UserResolutionFacts::resolves_to_user` (`analyser/diagnostics/validity.rs:167`), behind W002 and E002 | given the implicit path but not the declared `namespace path` by its callers; keeps the current namespace ahead of the helper (*read*) |
| `is_workspace_declared` (`analyser/diagnostics/unresolved.rs:176`), W123 suppression | tests only `name` and `::name` |
| `resolve_called_proc` (`tcl-lsp-core/src/definition/call_resolution.rs:141`), definition, hover, signature help, inline | a deliberate lenient tail match (`fallback_proc_by_simple_name`) after the real lookup, and a builtin check on the raw word (*read*) |
| `ensemble_subcommand_target` (`tcl-lsp-core/src/definition/command_heads.rs:118`) | re-resolves the target at `::` |
| `invocation_references_named` (`tcl-lsp-core/src/references/call_sites.rs:496`), references, rename, code lens | accepts a bare call when its namespace equals the target's or is on the implicit path; ignores declared `namespace path` and higher-priority candidates (:528-539, *read*) |
| `alias::resolve_alias`, `constructor_object_type` (`tcl-compiler/src/alias.rs:115`, `type_infer.rs:106`) | the existence test sees only aliases, or only classes, so a procedure of the same name does not stop the search as it does in Tcl |

The String family has no 8.4 or Jim recipe; only the Native family does.

**No type holds a frame's lookup context.** The pieces are spread over `NativeNameContext` (namespace only), `SignatureNamespaceScope` and `SignatureSourceLookup` (namespace and recipe), `SourceNamespaceKey` with `namespace_paths` and the receiver path held separately, `SourceInvocationBinding`, `ExecutionNamespace` (`Exact | SourceContext | RuntimeSelected`), the runtime's `NativeCompilationNamespace`, and `tcl-lsp-core`'s `CallResolution` (no namespace at all). The analyser passes a `String` namespace and a `&[&str]` path and asks for the helper path separately.

**The frame namespace is chosen in four places, by different rules:**

1. `advance_command_resolution_namespace` (`analyser/scope/command_namespaces.rs:49`): procedures and methods use their defining namespace, TclOO methods use `::` (the reporting approximation), `uplevel #0` uses `::`.
2. `innermost_namespace_at` in `tcl-lsp-core` (`definition.rs:932`): consults `namespace_overrides` (`apply`) before the analyser walk, which never reads them.
3. `executed_script_entry_namespace_context_at` (`command_binding/origin_inventory.rs:802`): the observed entry frame.
4. `ExecutionNamespace::selected_frame` (`ir/script.rs:113`): any non-zero `uplevel` is runtime-selected.

The W123 presence query takes a fifth route for entered TclOO bodies: the deferred body carries its defining namespace (`deferred_method/retained_entries.rs:646`), so `my`, `self` and `next` are sought in the class's namespace — the one place the contract says is never searched — and reported absent instead of unknown.

**Missing knowledge has no common meaning.** Resolvers return `Present/Absent/MayPresent/Unknown`, `Option`, `bool`, "overdefined", or keep a local-first guess. An `Absent` from one and an `Unknown` from another meet in the same feature.

**The conformance gate covers the part that is shared.** The 138 rows of `tcl-syntax/tests/data/command_resolution_vectors.txt` give a current namespace, a path and the defined commands; they pin the candidate order against every tclsh. They do not select the frame (procedure, method, `apply`, `uplevel`, alias target, `after`, `namespace code`, `inscope`, `forward`), and they drive the pure resolver, the analyser's settlement, the VM and the WASM runtime — not the presence query, the binding realm, or any LSP feature.

## Plan

1. **A lookup-frame value.** One type, built once where a script body starts to execute: current namespace (or an explicit unknown), declared path (with its own unknowns), implicit helper path, receiver namespace contents (known commands plus an open unknown), the selected lookup recipe (C Tcl release, Jim), and `namespace unknown`/`unknown` handlers. Source positions map to frames; consumers never rebuild one from a string.
2. **Evaluation contexts declared in the registry.** Every body- or command-prefix-taking argument declares how its frame is chosen: caller frame, named namespace (`namespace eval`, `inscope`, `code`, `apply`), global at call time (`after`, `interp alias` targets, traces, `bind`), level-selected (`uplevel`), object frame from the profile (TclOO methods, `forward`, other systems through the [object systems plan](object-systems-plan.md)). Today this is spread across hand-written cases in the analyser, the binding walk and the LSP layer.
3. **One lookup owner with one answer type.** `lookup(frame, name bytes, purpose) -> Present(target) | MayPresent(targets) | Absent | Unknown`, extending `SourceCommandBindings::invocation_at_source`. Every consumer listed above asks it: W123, W002, E002, W307, W308, go to definition, hover, signature help, references, rename, code lens, completion, call hierarchy, the workspace settlement, the type inference used by W307/W308, the optimiser and the method-body SSA guard. The lenient tail match, if kept, becomes an explicitly labelled "possible target" answer from the same owner, never a definition.
4. **Retire the parallel resolvers** in the table above and the four frame choosers, with an xtask gate that forbids calling the candidate-order primitives outside the owner (a ratchet like `registry-axes`).
5. **Consumer conformance.** Extend the vectors with a frame column (body kind and how it is entered) and run every row through every consumer: the presence answer, the definition target, the hover target and the reference set must agree with tclsh's winner. Adding a lookup behaviour stays "add a vector", and now covers all consumers.

The same shape — one value, one owner, one answer type, consumer conformance — applies to instance identity (cause 2 of the root-causes page): objects, widgets, images and fonts as one value from declared creation and destruction effects.
