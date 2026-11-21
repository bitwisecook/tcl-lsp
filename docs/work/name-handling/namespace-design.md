# Namespaces: shared pieces, separate models

How Tcl namespaces are represented and handled across execution (VM, runtime, codegen) and analysis (compiler, analyser, LSP, registry). The companion of [call frames](frame-design.md); both feed step 1 of [tidying the architecture](architecture-tidy-plan.md).

**Verdict:** the low-level pieces are shared and good — name splitting (`tcl_syntax::naming`), the glob matcher, the `NsId` token, `ByteNamespacePath`, the import-pattern parse and ensemble subcommand resolution. On top of them the engines run two independent namespace models, and analysis runs at least four, most keyed by readable strings, so they cannot tell a deleted namespace from a recreated one of the same name. Behaviourally the engines are right on everything measured; analysis navigation is partly right and inconsistent between features.

## Measured behaviour

The 20 fixtures in [`oo-dispatch-probe/namespaces/`](handover/oo-dispatch-probe/namespaces/) cover qualifier and tail edge cases (`a::::b`, `:::a`, trailing `::`, empty), `exists`/`children`/`parent`, export and import patterns, `import -force`, delete and recreate (import links dropped, recreation not restored), `namespace path` (including a deleted path entry), `namespace unknown`, `namespace which`, the current namespace in procedures, `apply`, TclOO methods and nested `namespace eval`, implicit parent creation, `rename` across namespaces, `inscope`/`code`, `forget`, `info procs`/`vars` patterns, deleting the current namespace, the global forms, `origin` through an import chain, ensembles, `variable` and `namespace upvar`. All run identically under tclsh 8.6.18, 9.0.4 and 9.1.0 (and 8.4.20 where the commands exist).

**Execution:** all 40 VM and runtime runs (8.6 and 9.0) match tclsh. One fact worth pinning: `rename ::b::q ::nons::q` succeeds when `::nons` does not exist and creates it, as `oo::class create ::nope::X` does (the class-facts handover found the same), whereas `proc ::x::y` refuses.

**Analysis:** no false diagnostics on the 20 correct programs. Navigation at a marked call (12 fixtures in `namespaces/nav/`, `tcl-lsp-core` definition and hover, `tcl9.0`):

| Call | Go to definition | Hover |
| --- | --- | --- |
| through `namespace import` | correct | "Possible procedure" |
| through `namespace import -force` over a local procedure | correct | "Possible procedure" |
| through an import of an import (`origin` chain) | correct | "Possible procedure" |
| relative nested `b::p` inside `::a` | correct | none |
| through `namespace path` | correct | correct |
| after `namespace delete` and recreation | the new definition | correct |
| implicit parent `a::b::p` | correct | correct |
| `$::cfg::limit` | correct | correct |
| after `rename ::a::p ::b::q` | none | none |
| ensemble subcommand `geo area` | none | none |
| `::d::p` after `namespace delete ::d` | none, and no diagnostic for the call that will fail | none |
| after `namespace forget` | none, W123 (correct) | "Possible procedure" |

Hover and definition use different resolvers (the [command lookup census](command-lookup-centralisation.md)); imports are where it shows.

## Representations

- **Shared:** `NsId` and `ByteNamespacePath` (`tcl-core-types`), `NativeNamespaceContext` (`tcl-runtime-api`, embedded by analysis as `SourceNamespaceKey::Native`), the `Namespaces` trait, and `tcl-cmd-core`'s namespace backend traits.
- **VM:** a path-interned arena (`native_name_world.rs`: `ns_arena`, `ns_intern`, parents, children, dying and dead sets, `ns_paths`), per-namespace variable, export and unknown tables keyed by `NsId`, but **one flat command table keyed by readable strings** (`interp.rs`: `commands: HashMap<String, Command>`, *read*); deferred deletion through `RetainedNamespace`.
- **Runtime:** a real tree (`runtime/rust/src/namespace.rs`) with per-namespace command tables, path, exports and unknown handler; arena slots are never reused.
- **Analysis:** `SourceNamespaceKey` (`Authored(String)`, `Native`, `Allocated { site, incarnation, path }`), `ExecutionNamespace`, `SignatureNamespaceScope`, the analyser `Scope` (a `String` name beside an optional scope), `AnalysisResult.namespace_paths: HashMap<String, Vec<String>>`, the item tree's `namespaces` and `ensembles` string sets, `ResolveContext` with both a string and an identity, `CellOwner` with both `Namespace(String)` and `NamespaceIdentity`.
- **LSP:** `exports_by_ns: HashMap<String, …>`, `namespace_rename` on rooted strings, and `innermost_namespace_at`/`namespace_context_at` returning `String`.

## Identity over time

Only the engines and command binding keep "deleted and recreated" apart: the VM with monotonic tokens (though its command table and command generations are string-keyed), the runtime with never-reused slots and a `retained_fqn` for frames still inside a deleted namespace, command binding with `AllocationIncarnation` and its namespace-retirement modules. The analyser scope tree has no `namespace delete` handler at all (*read*); the item tree, `AnalysisResult.namespace_paths`, the LSP export snapshot, namespace rename and the context helpers key by name.

## Namespace operations are implemented about seven times

Import, export, forget and origin semantics exist in: the VM (`interp/namespaces/export_import.rs`, `import_graph.rs`), the runtime (`cmd_namespace/import_export.rs`), command binding (`module_bindings/namespace_transitions.rs`, `original_namespace_binding.rs`), source-transition advice (`source_namespace.rs`, `c_namespace.rs`), the analyser's namespace hook handlers, the signature scan, and the LSP (`namespace_import.rs`, `workspace_index/namespace_imports/`). Only the glob matcher and the import-pattern parse are shared, and some files mix the plain and policy-aware matchers. Two analysis drivers read different registry descriptors for the same commands: the analyser runs on `Hook::Namespace*`, command binding on `NamespaceTransition`.

`namespace path` is stored seven ways (VM, runtime, API, command binding, source-transition advice, autoload source, the analyser's string map, which the LSP reads). `namespace unknown` keeps the handler in the engines, only its presence in command binding, and a "dynamic providers" flag in the analyser. Ensemble subcommand resolution is shared, but the native ensemble object code is copied between the VM and the runtime, and the `namespace` subcommand table is written three times (VM, runtime, registry).

## Current namespace

The VM keeps a readable namespace stack in lockstep with a token stack and gives frames both `ns` and a readable `ns_eval`; the runtime keeps a frame `current_ns` and an interpreter-level cell. Analysis chooses the current namespace in at least six places (`command_resolution_namespace`, `command_resolution_namespace_at`, `original_namespace_scope_at`, the signature scan's `current_namespace`, `ExecutionNamespace::for_head_context`, `ResolveContext`), and the LSP adds `namespace_overrides` for `apply` in its own helper. These are the same choosers as the frame study's.

## Leaks around the shared naming layer

- Descendant tests by string prefix, `starts_with(format!("{parent}::"))`: `command_binding/namespace_context.rs:153` (*read*), `var_resolve/cell_key.rs`, `variable_bindings/namespace_retirement.rs`, `captured_cell.rs`. Wrong for components containing colons (inferred).
- An ad hoc glob-literal test, `contains(['*','?','['])`, in `module_bindings/namespace_transitions.rs:672` (*read*) and `analyser/commands/command_references.rs`, instead of `tcl_syntax::glob::is_literal`, which also treats a backslash as special.
- `enclosing_namespace` and `join_ns` copied verbatim into `analyser/item_tree.rs` and `tcl-lsp-core/src/document_symbols.rs`.
- Codegen rebuilding a `ByteNamespacePath` from a display key (`tcl-codegen/src/codegen/mod.rs`).

## Conformance

`tcl-syntax/tests/data/namespace_op_vectors.txt` (172 rows: colon normalisation, import/export/forget, which/origin, path, unknown, the deletion cascade) is run only against tclsh by `tcl-syntax/tests/namespace_op_conformance.rs` (*read*). The command-resolution vectors are the only namespace-sensitive table that drives implementations (VM, runtime, compiler settlement).

## Plan

1. **Drive the existing vectors through everything.** Run the 172 namespace-operation vectors through the VM, the runtime, command binding and the analysis query layer, alongside the fixtures above. This is cheap and turns "matches tclsh" into a gate.
2. **One analysis namespace identity.** `SourceNamespaceKey` (with its incarnation) becomes the only namespace key in analysis; the analyser scope tree, `namespace_paths`, the item tree, the LSP index, namespace rename and the context helpers carry it, and strings are display only. Model `namespace delete` in the one binding walk ([architecture plan](architecture-tidy-plan.md) step 5) so a call into a deleted namespace is reported and navigation follows the live incarnation.
3. **One analysis model of namespace operations.** Import, export, forget, origin, path, unknown and ensembles live in command binding, driven by the registry's `NamespaceTransition` descriptors; retire the analyser hook handlers, the signature-scan copy and the LSP's own import and export logic in favour of queries. Follow `rename` across namespaces and ensemble subcommands for navigation.
4. **One engine contract.** Extend the `Namespaces` trait with the operations both engines implement separately (import/export/forget, path, unknown, ensemble objects), share the native ensemble object code and the `namespace` subcommand table from the registry, and key the VM's command table by namespace token and tail rather than readable string. Whether the VM adopts the runtime's tree is a later decision the trait makes possible.
5. **One current namespace per frame,** as part of the frame value ([call frames](frame-design.md)); drop the VM's parallel readable stack and `ns_eval` once frames carry the token and a retained name for display.
6. **Close the leaks:** a `tcl_syntax::naming` descendant predicate in place of the string-prefix tests, `tcl_syntax::glob::is_literal` in place of the ad hoc checks, one `enclosing_namespace`/`join_ns`, and codegen taking the path from the binding instead of a display key; add these spellings to the `resolution-drift` or `segmentation-drift` gate.
