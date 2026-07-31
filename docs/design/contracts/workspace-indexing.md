# KCS: Workspace/indexing contracts

## Symptom

Cross-file navigation is stale or incomplete, symbol search misses expected items, or background scans/resolution lag behind edits.

## Operational context

Workspace services track per-document state, global proc indexes, scanning, and package resolution. LSP navigation and workspace symbols depend on this layer being fresh and deterministic.

## What the index holds

`WorkspaceIndex` (`rust/tcl-lsp-core/src/workspace_index.rs`) lifts a
*bounded* selection out of each document's `AnalysisResult`, chosen so every
entry has an identity another document can spell:

| Table | Holds | Bound |
|---|---|---|
| `procs` / `classes` | every `proc` / class definition | qualified name |
| `invocations` | every call site, with its ordered resolution candidates | — |
| `variables` | namespace- and global-scope variable declarations | **qualified name only** |
| `variable_refs` | occurrences written with a `::` qualifier | **qualified name only** |
| `sources` / `package_requires` / `command_links` / `glob_imports` / `namespace_exports` | the cross-file graph edges | — |

The variable tables are deliberately narrower than the proc/class ones.  A
namespace variable has one cell in one namespace, so `$::ns::v` in any
document names the same thing as `variable v` inside `namespace eval ns`,
wherever the two are written.  An **unqualified** `$v` does not: it names
whatever the local scope chain supplies, which is a per-document question
with no statically-sound cross-file answer.  Proc locals are therefore not
indexed at all, and an unqualified occurrence never consults the index —
widening it would be a guess, not a resolution.

The `variable_refs` rows come from `qualified_var_refs`, which the analyser
(`tcl_compiler::analyser::QualifiedVarRef`) records whether or not the named
cell resolves in the recording document: the cross-file case is precisely
the one where it does not.

### The variable tier's rename half

Go-to-definition, hover, and find-references answer a namespace variable
*from the index alone* — an exact qualified-name match, no other document
read.  **Rename cannot**, and the difference is contractual, not an
optimisation gap: the index deliberately holds only namespace-scoped
declarations and qualified occurrences, while a rename must also rewrite the
`variable v` / `global v` / `namespace upvar` **aliases** inside proc and
method bodies and every unqualified `$v` they enable.  Those are proc-scope
bindings — a per-document identity, so by rule 4 above they do not belong in
the index — but they name the very same cell, and leaving them behind breaks
the program.

So the rename tier uses the index only to pick *which documents to visit*,
then re-analyses each and computes its share with
`tcl_lsp_core::rename::namespace_variable_rename_edits`, which reads
`VarDef::link_target` for the alias union.  Four candidate sources, and all
four are load-bearing — no one of them subsumes another:

| source | catches |
| --- | --- |
| `variable_definitions_qualified` | the documents declaring the cell |
| `variable_refs_of` | the documents naming it qualified (`$::ns::v`) |
| `documents_in_namespace` | an unqualified alias written *inside* `::ns` (`namespace eval ns { proc p {} { variable v; … } }`) — proc-scope, so not indexed itself, but its enclosing proc is |
| `documents_aliasing_variable` | an alias written from *any other* namespace (a global `proc p {} { namespace upvar ::ns v local; … }`) — which declares nothing in `::ns` and writes no qualified occurrence, so all three of the above miss it |

That last table is the one exception to rule 4's "proc locals are not
indexed", and it is not really an exception: what is recorded is the
**qualified cell** the alias names — as spellable from another document as a
declaration is — never the local spelling, which stays a per-document
question.  It exists because coverage has to be *provable*: without it a
rename moves the declaration and the alias is left bound to a variable that
no longer exists (tclsh 9.0.4 / 8.6.16 alike: `can't read "local": no such
variable`).

Two shapes refuse the rename outright rather than emit an edit set that
silently misses something:

- a candidate document that **computes a variable name** (`set $n 1`,
  `variable $n`) — `tcl_lsp_core::rename_safety::namespace_variable_rename_hazard`;
- any document that **aliases a computed cell** (`namespace upvar $ns v
  local`) which could be this one — `documents_with_ambiguous_alias_of`.
  A computed cell names no fixed variable, so it can be neither found by a
  candidate scan nor rewritten.  The match stays narrow: whichever half of
  the cell is still written literally must agree with the cell being renamed.

## Derived views and their invalidation

`WorkspaceIndex::generation()` is bumped by `add_document` /
`remove_document`, and every derived cache is dropped with it.  A whole-index
derived view belongs *on the index*, built lazily and invalidated by that same
mutation hook, rather than rebuilt per request by its consumer — the workspace
command-name set the cross-file unknown-command pass consults
(`WorkspaceIndex::command_names`) is the reference example: ~20 000 names on a
400-file / 10 000-proc workspace, ~7 ms to build, ~120 ns to serve from the
cache.

## Decision rules / contracts

1. Document-state updates must preserve cache correctness across edits/errors.
2. Workspace index queries should tolerate partial/stale files conservatively.
3. Scanner and package resolver changes require cross-file navigation regression checks.
4. A new index table must carry a *cross-document identity*, not a
   per-document one; if a symbol kind only has meaning inside its own file,
   it does not belong here.
5. Any whole-index derived view lives on the index, built lazily and dropped
   by the same mutation hook that bumps `generation()`.
6. The extensions the background scan indexes, the
   `workspace/didChangeWatchedFiles` registration, and the `willRename` /
   `didRename` filter all come from the one `TCL_SOURCE_EXTENSIONS` list —
   a file the scan indexes but the watcher ignores goes stale on the next
   external edit.
7. Search-path and package facts follow real Tcl arity and version rules, not
   convenient approximations. `set auto_path` assigns a **list** (each element
   one directory, a braced element with spaces still one) while `lappend`
   appends one directory per argument word; path arithmetic runs in Tcl's
   slash form so a native Windows `[info script]` resolves against its own
   directory; and a `package require NAME VERSION` selects the highest release
   satisfying `package vsatisfies`, not whichever was discovered first.

## File-path anchors

- `rust/tcl-lsp-core/src/workspace_index.rs`
- `rust/tcl-lsp-core/src/package_resolver.rs`
- `rust/tcl-lsp-server/src/lib.rs` (`scan_workspace_folders`,
  `is_tcl_source` / `TCL_SOURCE_EXTENSIONS`, `build_package_resolver`,
  `extend_resolver_with_document_auto_paths`, `ensure_library_indexed`)
- `rust/tcl-compiler/src/analyser/scope.rs` (`namespace_variables`,
  `attach_qualified_var_references`)
- `rust/tcl-lsp-core/src/rename.rs` (`namespace_variable_rename_edits`)
- `rust/tcl-lsp-core/src/rename_safety.rs`
  (`namespace_variable_rename_hazard`)

## Failure modes

- Proc index stale after rename/move causing wrong definition targets.
- Background scanner missing updates under burst edits.
- Package resolution false negatives masking available APIs.
- A derived whole-index view rebuilt per request, turning a cheap
  cross-file check into a per-keystroke walk of every indexed symbol.

## Test anchors

- `rust/tcl-lsp-core/src/workspace_index.rs` (`mod tests`)
- `rust/tcl-lsp-server/tests/e2e/issue923_crossdoc.rs`
- `rust/tcl-lsp-server/tests/e2e/rename_safety.rs` (the variable tier's
  rename half: TP cross-document, TP out-of-namespace alias, FN-guard
  proc-local alias, TN sibling namespace, FP-guard computed variable name,
  FP-guard computed alias cell, FN-guard computed alias of another cell)
- `rust/tcl-lsp-server/tests/e2e/issue923_class_refs.rs`
- `rust/tcl-lsp-server/src/lib.rs` unit tests
  (`watcher_and_rename_globs_cover_every_indexed_extension`,
  `document_auto_path_mutation_feeds_the_package_database`,
  `set_auto_path_puts_every_list_element_on_the_search_path`,
  `a_versioned_require_indexes_the_release_it_asks_for`)
- `rust/tcl-lsp-core/src/workspace_index.rs`
  (`command_names_are_cached_until_the_index_changes`)
- `rust/tcl-compiler/src/auto_path_eval.rs` (`mod tests`)

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
- [package loading](../../../docs/design/contracts/package-loading.md)
- [stale cache troubleshooting](../../../docs/kcs/kcs-issue-stale-compiler-cache.md)
