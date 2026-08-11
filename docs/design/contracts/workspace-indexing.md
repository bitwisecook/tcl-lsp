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
| `invocations` | every call site, with its ordered resolution candidates and its enclosing body span | — |
| `variables` | namespace- and global-scope variable declarations | **qualified name only** |
| `variable_refs` | occurrences written with a `::` qualifier | **qualified name only** |
| `namespace_refs` | every word naming a namespace, declaring or not | qualified name (rooted at record time) |
| `sources` / `package_requires` / `command_links` / `glob_imports` / `namespace_exports` | the cross-file graph edges | — |
| `defined_symbols` | registry symbol-definer definitions — `tcltest` test cases / constraints / match modes, iRules `when EVENT` handlers | qualified name |

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

That decision function gates **installs as well as removals** (issue #1104
item 1): a bare call written before its own `namespace import` reaches nothing
(oracle: first call `invalid command name`, post-import call works), so an
install that has not run at the query point does not count.

Ordering is the **workspace run order**
(`tcl_lsp_core::source_graph::RunOrder`, issues #1104 item 3 and #1279).  A byte offset in
the importing file and one in the calling file are unrelated numbers —
comparing them let a `namespace forget` in the caller revoke a cross-file
import purely because its local offset happened to be larger (issue #1116
finding 1) — so two events in different documents are ordered only where a
`source` path proves it.  Sourcing a file inlines its whole body at the
`source` statement's position, so the DFS of the `source` forest *is* the run
order: each point is lifted to its root-ward path of `source`-statement
offsets, and the deepest document the two paths share is where the ordinary
single-document rule applies.  The index builds the order once per
`generation()` from the host's resolver (`WorkspaceIndex::set_source_resolver`;
the index holds no URI ↔ path mapping of its own) over **literal** `source`
targets only.

A **`package require`** contributes a second kind of edge
(`WorkspaceIndex::package_run_edges`, issue #1279), and needs no resolver: it
names its provider through the index's own `package provide` records.  Its
edge is *one-sided* — a require that returns has left the package loaded, but
a require of an already-loaded package evaluates nothing, so it bounds the
provider's position from above rather than pinning it.  `RunEdgeKind` carries
that through the same projection and `RunOrder::trusted` reads off the half of
each comparison the bound survives: "the provider has already run" is a fact
from the require onwards, "it has not run yet" never is.  No edge at all for a
package two indexed documents provide (`auto_path` order decides which wins),
for one this workspace registers a `package ifneeded` script for (the script
is what runs), or for a conditional or non-literal require.

The relation answers `None` — incomparable — for two documents in different
trees, for a file reachable from two different `source` sites or on a cycle
(Tcl tolerates re-sourcing, and a doubly-entered file has no unique position),
for a computed `source $dir/x.tcl` **whose `$dir` the shared path evaluator
cannot fold** (the index carries each document's raw path-constant write
facts — `WorkspaceIndex::path_constant_assignments`, top-level `set`s plus
`variable`/`set` writes inside literal `namespace eval` bodies — and hands
them to the host's resolver, so both the chained `set dir …; set sourceDir
[file join $dir src]; source [file join $sourceDir x.tcl]` idiom and the
namespace-variable `namespace eval ::snit:: { variable library [file dirname
[info script]] }; source [file join $::snit::library main1.tcl]` idiom *do*
resolve and rank), and for a host that installs no resolver.  Cross-file
constants extend the same fact one hop further (issue #1368):
`WorkspaceIndex::imported_constants` computes, per document, the values its
source-graph ancestors establish before it runs — position-gated at each
parent's `source` statement, agreed across every route that reaches the
reader, and iterated with the edge set in whole rounds so a shipped edge is
always consistent with the shipped import map (a non-converging workspace
ships no imports rather than a map its own edges contradict).  The host
installs a `ConstantFolder` alongside the `SourceResolver` so the index can
compute what one document provides to those it sources.
`None` folds to "abstain toward answering" in both decision functions: an
unrankable install counts, an unrankable removal revokes nothing.  A workspace
with no resolvable `source` edge therefore behaves exactly as it did before the
order existed.

Within one document the comparison is `in_effect_within` on **both** sides
too, which is why `CallSite` carries the call's own `enclosing_body` span and
`invocations` rows store one.  A plain offset test was wrong in both
directions: it left a body-local call resolving through a top-level
`namespace forget` written before it (issue #1116 item 3, the lenient
direction), and — once installs became order-gated — it would have dropped the
alias of every proc body that calls a name its own file imports further down,
which is the ordinary shape of a library module (tcllib's
`modules/uev/uevent.tcl` writes its procs first and its `namespace import`s
last).  The column is built with one stack sweep over each document's body
spans and call offsets rather than one `innermost_definition_body_span` per
row, so the cost is `O((P + I) log (P + I))` per document instead of the
`O(procs × invocations)` that kept the fact out of the index.

Two further points are deliberate.  A removal the order cannot rank against
the install revokes nothing, the same rule the `-clear` tombstones follow.  And
**destroying** the source command is not treated as a slot event on a timeline
at all — the command object is gone workspace-wide — so it revokes wherever it
is written.

Both decision functions are **latest-wins folds over a partial order**, not
over a `u32` maximum: two events the gate admits still have to be ranked
against each other, and a partial order has no `max`.  A pattern survives
unless some tombstone is known to have run after it; an install survives unless
some removal is.  "Known to have run after" is `has_run(a, b) == Some(false)` —
the *same* relation the gate uses, so the gate and the ranking cannot mean
different things.  One consequence needs no `source` edge at all: two events
written in one *foreign* file are ranked against each other (a file's
statements run consecutively), so a `namespace export p` followed by a
`namespace export -clear` there revokes, where the old encoding could only call
both "unordered" and kept the export.

The **exact**-import link tier runs the same decision function with no call
site (`WildcardImportIndex::link_alias_live`, issue #1116 finding 2): the
question a link answers is "does this alias exist for navigation", so every
recorded removal counts as having run and the ordering that remains is the
removal's position relative to the *import* — a forget or a redefinition of
the imported name in the import's own document revokes the link when written
after it, one before it is undone by the import, and one in another document
revokes nothing.

The import **conflict** rule is one function over both tables
(`WildcardImportIndex::conflicting_alias_at`).  Tcl installs one alias per
name; whether the import that installed it was spelled as a glob or as an
exact pattern is a fact about the source text, not about the command table, so
an earlier import of either spelling makes a later non-`-force` import of the
other install nothing (issue #1116 item 7 — asking each side only about its own
kind made the rule directional).  A same-source re-import is a silent no-op,
never a conflict, and two imports in different documents conflict only where
the `source` order ranks one strictly before the other (issue #1116 item 6).

"Earlier" is **load order**, not byte offset
(`tcl_lsp_core::namespace_import::load_order`, shared with the same-document
tier's slot-log fold): a load-level statement runs before every body of its
file however far below it is written, a body-local one never counts as having
run at load level, and within one tier the key degenerates to the offset
comparison it replaces.  Across documents it is the same key read at the two
sites' deepest common document (`RunOrder::cmp_run`).  A raw offset test let a body-local import install over
a top-level one written after it — oracle (8.6.14 / 9.0.4): with `proc p {}
{namespace import ::B::x}` in `::dst` followed by a top-level `namespace import
::A::*`, `namespace origin ::dst::x` is `::A::x` and `::dst::p` raises `can't
import command "x": already exists`.  This is deliberately *not*
`in_effect_within`, the primitive the lifecycle checks use: that one counts an
event in a body that may never run, which is the safe direction for a removal
and the unsafe one for a conflict — it would make both imports above cancel
each other and the name resolve nowhere.

A pattern rooted at the global namespace (`namespace import ::p`,
`namespace import ::*`) splits to an *empty* source namespace, which both tiers
once read as "no source" and skipped — the last import shape that bypassed the
gate.  It is `::`, the same spelling a global-level `namespace export` record
carries, and it is gated like any other (#1104's review note; oracle: an
unexported global command makes the import a silent no-op).

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

`workspace/symbol` is answered from here too (`symbols_matching`, issue
#1156), which is why a class record carries its methods' `name_span`s and its
`constructor_spans`, and why `defined_symbols` exists: the picker must be able
to locate a member in a file the editor never opened.  Cross-file *dispatch*
needs none of those spans; they are carried for the picker alone.

## How the tables are stored

The tables are held **per document** (`DocumentRecords`, one slot per URI)
and exposed as workspace-wide iterators that chain the slots in slot order —
`procs()`, `classes()`, `invocations()` and the rest return an
`impl Iterator`, not a slice.  That is what makes `remove_document` cost the
removed document's own rows rather than a pass over the whole workspace
(issue #1149): flat workspace-wide vectors made a removal fourteen
`Vec::retain` passes with a `String` compare per element, over tables that
hold one row per call site and per qualified variable occurrence — 10⁵–10⁶
rows on tcllib — and the server re-indexes a document on every diagnostics
publish.

A removal clears the document's slot and returns it to a LIFO free list, so
the remove-then-add of a publish hands the document straight back the slot it
just gave up: its position in the workspace-wide order is stable across a
re-index, and the slot vector stays bounded by the workspace's peak document
count.  Adding the **same** URI twice without an intervening removal
accumulates (it does not replace): the M9 source-rehoming pass indexes one
analysis per source-site namespace, and those views are several runtime
identities of one physical file.

An **edit** does not touch the index.  `did_change` commits the buffer splice
and the salsa source only; `publish_diagnostics_result` then uses
`replace_document` under the `documents` lock behind its `is_current` re-check,
so it can only install the analysis of the revision the buffer actually holds.
Replacement keeps the document's slot, its table allocations, and its
workspace order.  It is intentionally distinct from remove + add: a temporary
missing-definition state loses the information needed to recognise a
body-only edit as incremental.

## Derived views and their invalidation

`WorkspaceIndex::generation()` is bumped by `add_document`, `replace_document`,
and `remove_document`.  A whole-index derived view belongs *on the index*,
built lazily and invalidated by the mutation hook, rather than rebuilt per
request by its consumer — the workspace command-name set the cross-file
unknown-command pass consults
(`WorkspaceIndex::command_names`) is the reference example: ~20 000 names on a
400-file / 10 000-proc workspace, ~7 ms to build, ~120 ns to serve from the
cache.

The rule is a single type, `Derived<T>`, rather than one hand-rolled
`OnceLock` + reset per view (issue #1105).  Every view uses it:
`command_names`, the `command_links` liveness mask, the two
`defined_command_names` readings (with and without the names links
introduce — a consumer wants exactly one, since folding link names into the
direct set would let rename rewrite a call that merely spells an imported
name), the `command_link_map`, and the export snapshot.  `defined_command_names`
was the motivating regression:
`workspace_command_exists` rebuilt the whole set per call, and
`follow_import_chain` asks it once per candidate per hop.  Measured on the
same 400-file / 10 000-proc workspace, 10 000 existence checks take **870 µs**
cached against **7.87 s** rebuilding per call.

The settled-target reverse index is deliberately more selective (issue #1319).
It holds one contribution per document and a target-indexed `BTreeSet` of
`(document slot, invocation index)` pairs.  After an edit inside a proc body,
the server removes that document's old contribution and re-settles only its
new call sites; a lookup for a common command does not scan every caller or
regroup the workspace.  The insertion/removal structure keeps source order
stable without a linear pass over that command's callers.

This shortcut is valid only while the complete **settlement dependency
surface** is unchanged: dialect, proc/class definitions, namespace declarations,
`source` and package-order rows, command links, wildcard imports, export and
forget events, and command deletions.  `SettlementDependencies` compares those
records exactly, rather than by a lossy hash.  A change to any of them can
alter another document's resolution (for example, deleting `::helper` or
unexporting a wildcard-imported name), so it deliberately clears the reverse
index and re-settles all documents on the next query.  Invocation rows are not
part of that surface: changing them can affect only their own document.

The tests prove both sides: a changed call body replaces exactly one document's
contribution, while a cross-file definition or export change re-settles every
document and removes the stale answer.  This is the before/after performance
contract: the ordinary body-only edit changes the next references lookup from
one whole-workspace settlement to one document settlement; an edit that can
change another file's answer retains the necessary whole-workspace cost.

## Decision rules / contracts

1. Document-state updates must preserve cache correctness across edits/errors.
2. Workspace index queries should tolerate partial/stale files conservatively.
3. Scanner and package resolver changes require cross-file navigation regression checks.
4. A new index table must carry a *cross-document identity*, not a
   per-document one; if a symbol kind only has meaning inside its own file,
   it does not belong here.
5. Any whole-index derived view lives on the index, built lazily and dropped
   by the same mutation hook that bumps `generation()`. The settled-target
   index is the narrow exception: it is a per-document reverse index, and it
   retains only contributions proven independent of the edit.
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
