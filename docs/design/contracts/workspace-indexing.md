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
| `namespace_refs` | every word naming a namespace, declaring or not | qualified name (rooted at record time) |
| `sources` / `package_requires` / `command_links` / `glob_imports` / `namespace_exports` | the cross-file graph edges | — |

`glob_imports` and `namespace_exports` each carry their document **and their
byte offset within it**, because a `namespace import` binds the names its
source namespace exported *at the import site*, not in the workspace's final
export state (issue #1027). Offsets order events only *within* one document:
which of two files loads first is not a static fact, so an export in a
different file from the import is treated as unordered — its pattern still
counts, and a `-clear` it carries revokes nothing. Both this tier and the
same-document resolver in `definition.rs` decide through the one shared
function `tcl_lsp_core::namespace_import::exported_at_import_site`, so they
cannot disagree about what an import site sees.

Ordering within a document is **not** a plain offset comparison. A
`glob_imports` row also stores the innermost proc/class body containing the
import (`enclosing_body`), so the tier applies
`tcl_compiler::analyser::indirection::in_effect_within` — the same rule
`in_effect` applies same-file: an import inside a body observes every
*top-level* statement of its own file, wherever written, because the whole
file loads before any body runs, while a statement of that same body stays
ordered by offset.

The gate is not limited to glob imports.  An **exact** `namespace import
::src::p` installs nothing at all when `p` is not exported at that point, so
the `command_links` row it produces carries an `import_gate` and is only
*live* while the snapshot admits it.  Liveness is a function of the whole
index — the export usually lives in another document — so it is a lazily-built
whole-index derived view (rule 5 below), rebuilt per `generation()` and read
through `WorkspaceIndex::live_command_links`.  Deciding it when the link is
created would freeze a cross-document answer that goes stale as soon as the
exporting file is edited.  `interp alias` / `rename` links, and imports merely
*conjectured* from a tcllib `<NS>::import <ALIAS>` wrapper, carry no gate.

The edge also has a **lifecycle** after it is installed (issue #1103), so the
tier indexes its removals beside the exports: `namespace_forgets` (one row per
`namespace forget` pattern, carrying the qualified pattern's source namespace
or `None` for the simple form) and `command_deletions` (a *destroying* `rename
OLD {}` / `interp alias {} NAME {}` — a plain rename is not one, because the
alias holds the command object and survives it).  `glob_imports` and the exact
form's `import_gate` both carry `forced`, so a non-`-force` import onto a name
the target namespace already **defines** installs nothing, while a `-force` one
replaces it.  `WorkspaceIndex::resolve_wildcard_import` therefore takes a
`CallSite` (the calling document plus the call's own offset): removals are a
question about the *call*, not the import, and the same shared decision
function the same-document resolver uses
(`tcl_lsp_core::namespace_import::alias_live_at`) answers it for both tiers.

Ordering is per document on **both** sides of the comparison: an install
whose document differs from the call's is passed unordered too, because a byte
offset in the importing file and one in the calling file are unrelated numbers
— comparing them let a `namespace forget` in the caller revoke a cross-file
import purely because its local offset happened to be larger (issue #1116
finding 1).  Unordered, the shared function keeps the alias.

Three further points are deliberate.  A removal in a **different document**
from the call revokes nothing, the same unordered-event rule the `-clear`
tombstones follow.  Within one document the removal is ordered by a plain
offset comparison rather than `in_effect_within`, because the index stores an
enclosing-body span per *import* row, not per invocation, and building one per
call site would be O(procs × invocations) at index time; the missing fact can
only make a removal look not-yet-run, i.e. keep answering, never invent one.
And **destroying** the source command is not treated as a slot event on a
timeline at all — the command object is gone workspace-wide — so it revokes
wherever it is written.

The **exact**-import link tier runs the same decision function with no call
site (`WildcardImportIndex::link_alias_live`, issue #1116 finding 2): the
question a link answers is "does this alias exist for navigation", so every
recorded removal counts as having run and the ordering that remains is the
removal's position relative to the *import* — a forget or a redefinition of
the imported name in the import's own document revokes the link when written
after it, one before it is undone by the import, and one in another document
revokes nothing.  A non-`-force` exact import also conflicts with an earlier
exact import of the same name from a **different** source in the same document
(`earlier_conflicting_link`), matching the glob tier's
`conflicting_alias_at`.

Resolution follows **import chains**: when the hop's source namespace does not
itself define the name, the walk continues from there, bounded by
`tcl_compiler::analyser::indirection::MAX_COMMAND_NAME_HOPS`, so `::A`
importing `::B::*` where `::B` imported `::C::*` resolves to `::C::p` instead
of abstaining.

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

### The namespace tier

`namespace_refs` (issue #1088) holds one row per word the registry marks
`ArgRole::NamespaceName`: the `namespace eval` target that **declares** a
namespace (`declares: true`, from `Traits::DECLARES_NAMESPACE`) and every
other spelling of it — `namespace children`, `exists`, `delete`, `parent`,
`upvar`, `inscope`.  One table rather than two, because a namespace's
declaring site *is* one of its spellings: it is both what go-to-definition
answers with and a word find-references reports.

This tier needs **no** qualified-only bound, unlike the variable one.  A
namespace word roots against the command-resolution namespace in force where
it is written, and that is a lexical fact the recording document already
knows, so `tcl_compiler::analyser::NamespaceRef` stores the rooted name and
every indexed row names one namespace absolutely.  Two consequences the
providers rely on: a relative `inner` inside `namespace eval ::outer` is
indexed as `::outer::inner` and matches a sibling document's
`::outer::inner` exactly; and the index never has to re-derive the rooting
rule, so it cannot disagree with the same-document resolver
(`tcl_lsp_core::namespace_symbol::namespace_cell_at`, the single entry point
definition, hover, and references all answer through).

Two shapes are deliberately absent.  A **computed** target (`namespace eval
$ns { … }`) names no static namespace and is recorded nowhere.  A namespace
that exists only as an implicit **parent** — `namespace eval ::p::q::r {}`
really does create `::p` and `::p::q` on tclsh 9.0.4 and 8.6.16 alike — has
no declaring row, because its name is written nowhere; definition abstains
rather than pointing into the middle of another namespace's name word.

`observable_namespaces` (the discriminator between "this namespace does not
export the name" and "this namespace is not in the workspace at all", which
gates `live_command_links`) reads the declaring rows as a fourth source,
alongside proc/class owners and `namespace_exports`.  A `namespace eval ::ns
{ namespace import ::other::* }` block declaring no proc, class, or export
of its own *is* a namespace the workspace can see.

Both halves of a namespace query are a **union**, not a fallback: the
request's own analysis supplies the local sites and the index supplies every
other document's, deduplicated by `(uri, span)`.  Returning as soon as the
in-document provider answered — and excluding the request's own URI from the
index lookup, which the first cut did — reported only the local half of a
namespace reopened in two files, contradicting the contract that every
declaring block is a target (issue #1088 review, finding 2).  Reading the
local half from the analysis rather than the index is deliberate: a document
that is unindexed, or has been edited since it was indexed, still answers.
Hover counts the same merged set and states how many documents it counted, so
it cannot say "1 block" while go-to-definition offers three.

A namespace-name position is also **definitive** in the server, not just in
the providers: the query is answered by one branch taken before every other
tier.  An empty answer must not reach the proc/class tiers — an empty local
reference set (asking without declarations from a namespace's only declaring
block) used to route the query to `workspace_resolved_references`, and an
empty rename edit set falls through to the workspace-resolved rename branch
by design, which renamed a same-spelled *proc* instead (issue #1088 review,
finding 1).

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
- `rust/tcl-lsp-server/tests/e2e/issue1088_namespace_symbols.rs` (the
  namespace tier, single-file and cross-file)
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
