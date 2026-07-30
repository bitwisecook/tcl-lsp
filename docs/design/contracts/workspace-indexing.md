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

## Derived views and their invalidation

`WorkspaceIndex::generation()` is bumped by `add_document` /
`remove_document`.  A consumer that derives a whole-index view must memoise
it on that counter rather than rebuilding per request — the workspace
command-name set the cross-file unknown-command pass consults
(`Backend::workspace_known_command_names`) is the reference example: ~20 000
names on a 400-file / 10 000-proc workspace, ~7 ms to build, ~120 ns to
serve from the memo.

## Decision rules / contracts

1. Document-state updates must preserve cache correctness across edits/errors.
2. Workspace index queries should tolerate partial/stale files conservatively.
3. Scanner and package resolver changes require cross-file navigation regression checks.
4. A new index table must carry a *cross-document identity*, not a
   per-document one; if a symbol kind only has meaning inside its own file,
   it does not belong here.
5. Any whole-index derived view is memoised on `generation()`.
6. The extensions the background scan indexes, the
   `workspace/didChangeWatchedFiles` registration, and the `willRename` /
   `didRename` filter all come from the one `TCL_SOURCE_EXTENSIONS` list —
   a file the scan indexes but the watcher ignores goes stale on the next
   external edit.

## File-path anchors

- `rust/tcl-lsp-core/src/workspace_index.rs`
- `rust/tcl-lsp-core/src/package_resolver.rs`
- `rust/tcl-lsp-server/src/lib.rs` (`scan_workspace_folders`,
  `is_tcl_source` / `TCL_SOURCE_EXTENSIONS`, `build_package_resolver`,
  `extend_resolver_with_document_auto_paths`, `ensure_library_indexed`)
- `rust/tcl-compiler/src/analyser/scope.rs` (`namespace_variables`,
  `attach_qualified_var_references`)

## Failure modes

- Proc index stale after rename/move causing wrong definition targets.
- Background scanner missing updates under burst edits.
- Package resolution false negatives masking available APIs.
- A derived whole-index view rebuilt per request, turning a cheap
  cross-file check into a per-keystroke walk of every indexed symbol.

## Test anchors

- `rust/tcl-lsp-core/src/workspace_index.rs` (`mod tests`)
- `rust/tcl-lsp-server/tests/e2e/issue923_crossdoc.rs`
- `rust/tcl-lsp-server/tests/e2e/issue923_class_refs.rs`
- `rust/tcl-lsp-server/src/lib.rs` unit tests
  (`workspace_known_names_are_memoised_on_the_index_generation`,
  `watcher_and_rename_globs_cover_every_indexed_extension`,
  `document_auto_path_mutation_feeds_the_package_database`)

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
- [package loading](../../../docs/design/contracts/package-loading.md)
- [stale cache troubleshooting](../../../docs/kcs/kcs-issue-stale-compiler-cache.md)
