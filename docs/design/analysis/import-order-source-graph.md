# A load-order partial order from the `source` / `package require` graph

The load order shared by the whole wildcard-import gating family (#1104 item
3, #1116 items 3 and 6, and the cross-file half of every rule in
[`contracts/command-resolution.md`](../contracts/command-resolution.md)'s import
section).

`tcl_lsp_core::source_graph::RunOrder` is the relation §6 describes.
`WorkspaceIndex::run_order` builds it once per `generation()` from the
host's resolver (`WorkspaceIndex::set_source_resolver`) **and** from
`WorkspaceIndex::package_run_edges`, and both wildcard-import tiers rank
their events with it through the two shared decision functions. Sections
1–5 record the facts and reasoning the design rests on, §6 is the relation
itself, and §7 is the `package require` half that reuses it.

## 1. What the order settles

Within one document the tiers order import-lifecycle events by **byte offset**,
via `tcl_compiler::analyser::indirection::in_effect_within`. Nothing in a Tcl
source tree says which *file* runs first in general, so across documents the
order comes from the `source` / `package require` graph:

| Fact in another file | With the load order |
|---|---|
| `namespace export p` | counts when the graph proves it ran first; **does not** when it proves it ran later |
| `namespace export -clear` | revokes when it provably ran between the export and the import |
| `namespace import` (an install) | counts |
| `namespace forget` | revokes when it provably ran before the call |
| a second import of the same name | conflicts when its site provably ran first (`cmp_run`) |

Where the graph proves no order — different trees, a re-sourced file, a
`source` cycle, a computed `source $dir/x.tcl`, or a host that installs no
resolver — `RunOrder` answers `None`, and the shared decision functions
(`namespace_import::exported_at_import_site`, `namespace_import::alias_live_at`)
abstain toward *answering*: a foreign export counts, a foreign revocation
revokes nothing.

## 2. What is already recorded

### 2.1 `source`

`tcl_compiler::signature_scan::types::SignatureSource` — one row per `source`
statement, surfaced in the index as `WorkspaceSource`:

| Field | Meaning | Enough for load order? |
|---|---|---|
| `raw_path` | the path word verbatim, `${var}` / `[cmd]` markers preserved | needs resolving |
| `range` | byte span of the path argument **in the sourcing file** | **yes** — this is the sequencing fact |
| `is_literal` | no `$` / `[` in the word | the decidability flag |
| `site_namespace` | command-resolution namespace at the `source` call | not needed here (used by M9 re-homing) |

The offset is the important one and it is already there: a `source` statement
is an ordinary statement of the sourcing file, so it is ordered against that
file's imports and exports by exactly the rule already in use.

`WorkspaceIndex` exposes the graph twice:

- `source_ancestor_package_requires(target_uri, resolve)` — reverse
  reachability, used by the workspace W120 refinement.
- `source_seed_map(resolve)` — the M9 namespace re-homing seeds.

Both take a `resolve` **closure supplied by the server**, because the index
holds no URI↔path mapping of its own. `crate::source_graph` owns the pure
half (`resolve_source_target`, `resolve_under`, `ancestor_requires`,
lexical `.`/`..` folding, no filesystem access).

### 2.2 `package require` / `package provide`

- `WorkspacePackageRequire { uri, name, at, enclosing_body, conditional }` —
  the same three sequencing facts `WorkspaceSource` carries.
- `WorkspacePackageProvide` (from the per-document
  `SignaturePackageProvide { name, version, range }`) — the other end of the
  edge, with an offset. `WorkspacePackageIfneeded` records the registrations
  that make a package's loading non-static.
- `SignatureAutoPathEntry { raw, range }` — `auto_path` mutations, one row per
  element, offsets included.
- `tcl_lsp_core::package_resolver` + the autoload tier resolve a required
  package to its `pkgIndex.tcl` and merge the defining library file into the
  index on demand.

## 3. What a partial order needs

1. **Resolved edges in the index.** A `(parent_uri, child_uri, at)` triple per
   literal `source`, with `at` the sourcing statement's own offset. Today the
   URI half only exists inside a server-supplied closure; the graph consumers
   rebuild it per call. A load-order gate runs *inside* the per-call
   import walk, so the edges have to be resolved once and cached — naturally
   alongside `WildcardImportIndex`, which is already built once per query and
   invalidated per `WorkspaceIndex::generation()`.

2. **A "has already run" relation over (document, offset) pairs**, replacing
   the current `uri == other.uri` test. The relation `A@a ≺ B@b` should hold
   when there is a path `A → … → B` in the source graph whose first edge sits
   at an offset `≺` `a` under the existing `in_effect_within` rule, and `B`'s
   whole body precedes `b` only if `B` is the same document. Concretely the
   three cases the gate needs:

   - same document → today's `in_effect_within` (unchanged);
   - `A` sources `B`, and the `source` statement is in effect at the query
     point in `A` → every statement of `B` has run;
   - `A` sources `B` *after* the query point in `A` → **nothing** in `B` has
     run — the first ordering that can currently only abstain.

3. **Soundness gates on the edge set.** An edge is usable only when the whole
   path is unconditional and statically known:
   - `is_literal` (a `source $dir/x.tcl` sequences nothing);
   - the `source` statement is not inside a proc/class body whose execution is
     conditional — the same `nested` judgement `WorkspaceProc::nested` and
     `WorkspaceCommandLink::nested` already apply;
   - the graph is acyclic on the path used (Tcl tolerates re-sourcing; a cycle
     means the order is not a partial order and the pair must abstain);
   - a file reachable by **two** paths with different orders relative to the
     query point abstains — the order must be unique to be a fact.

4. **`package require` sequencing (§7).** A
   `package require` is an ordered statement too, and its package's files load
   at that point. It needs the `at` / `enclosing_body` pair
   `WorkspaceGlobImport` carries, plus the package→URI mapping in the index;
   `package provide` gives the other end. This is strictly more speculative
   than the `source` half: `auto_path` is mutable at runtime and a package may
   already be loaded, so "the require ran here" does not imply "the package's
   files ran here and not earlier". §7 records what that costs and what
   survives it.

## 4. Which abstentions it lifts

| Abstention | Resolved by |
|---|---|
| foreign `namespace export` patterns always count; foreign `-clear`s never revoke | `RunOrder::has_run`, all three §3.2 cases |
| two imports of one name in different files never conflict | `RunOrder::cmp_run` in `WildcardImportIndex::conflicting_alias_at` |
| `source lib.tcl ; namespace forget ::lib::p` — install counts, forget revokes nothing | §3.2 cases 2 and 3 together |
| in-document `-force` shadow with the export in another file | **Not ordering.** This is an *observability* question, answered by the workspace tier's whole-program export view, which has to be threaded through `resolve_called_proc`. |
| import-error control flow (a failed import aborts the rest of its script) | **Not ordering.** A different model — intra-script abort — though it shares the "what has run" vocabulary. |

Note the two negatives. A load order says *when* a statement ran; it does not
say whether a statement exists in a file nobody indexed, and it does not model
a script aborting part-way. Those stay separate.

**Measured reach.** Over the repository's Tcl corpora — Tcl 9.0.4's own
`library/`, `samples/`, and the multi-file editor/CLI fixtures: 247 documents,
19 `source` statements of which 3 resolve to an indexed document, 13 752 bare
call sites — the order changes **0** resolutions. Real Tcl code overwhelmingly
writes `source [file join $tcl_library init.tcl]`, whose `$tcl_library` no
static fold can place, so no edge is built and the pre-existing abstention
stands. The order is therefore a strict refinement that fires only where a
load order is genuinely provable; the cases where it does fire are pinned by
unit and end-to-end tests rather than by the corpus.

## 5. Cost, and why this is not a one-liner

The gate runs in the per-call import walk, which the workspace tier already
had to hoist an index out of to keep off the profiler
(`WorkspaceIndex::resolve_wildcard_import_indexed`'s doc records the
regression that forced it). Adding a graph reachability query per event would
put it straight back. The shape that fits is a **precomputed order** built
once per `generation()`:

- resolve every `source` edge the host can place once → `O(sources)` with the
  host's resolver;
- walk each document to its root once → `O(V · depth)`, storing its root-ward
  path of `source`-statement positions;
- compare two events by walking those two paths — `O(depth)`, and depth is the
  `source` nesting of a real project.

The one thing that cannot be precomputed cheaply is the *server-supplied
resolver*: `WorkspaceIndex` deliberately holds no filesystem knowledge, so
either the resolved edges get pushed in at `add_document` time (changing the
index's public shape) or the order is built lazily behind the same
`generation()` cache the live-link mask uses, taking the resolver as an
argument. The second is the smaller change and matches `live_command_links`'s
existing pattern.

The resolver is held *on the index* as a plain `fn` pointer
(`WorkspaceIndex::set_source_resolver`, installed by the server's
`new_workspace_index`), and the order is a `Derived<RunOrder>` view like the
rest. A per-call argument would have to be threaded through every caller of
`resolve_wildcard_import` and every derived view that builds a
`WildcardImportIndex`, because the order is consulted *inside* the per-call
import walk, not at its entry point. A `fn` pointer rather than a boxed
closure keeps the index `Debug + Clone + Default` with no manual impls, and
the resolver is a pure function of `(parent uri, raw path, is_literal)` in
every host — `WorkspaceIndex::SourceResolver`, the signature `source_seed_map`
takes, so one host resolver serves both and the order inherits the M9 tier's
statically-foldable computed-path half.

An index with **no** resolver installed derives an empty order and every tier
abstains, which is what
`without_a_resolver_the_same_workspace_keeps_abstaining` pins.

No smaller subset is provable without the resolver. Even "same document
`source a.tcl` written before an import of a namespace `a.tcl` declares" needs
the raw path resolved to a URI, which only the server can do, and that
resolution reaching into `exports_name_at` / `alias_live_at` — the whole
plumbing above.

## 6. The relation

Keep the discipline the family already has: **one** decision function per
question, both tiers calling it. The order lives behind a single relation with
the shape

```rust
fn has_run(&self, event: RunPoint<'_>, query: RunPoint<'_>) -> Option<bool>
```

— `None` meaning "no static order". `RunPoint` is the
`(uri, at, enclosing_body)` triple every event and query point carries.

### 6.1 `has_run` alone is not enough — the fold needs *comparability*

That predicate answers the gate, but it is only half of what the shared
decision functions do, and the other half is why `ExportEvent::at` /
`AliasEvent::at` cannot simply "become `Some(_)` in more cases".

Both functions are **latest-wins folds**, not filters:

- `namespace_import::exported_at_import_site` keeps two running maxima — the
  latest visible `-clear` and the latest visible matching pattern — and
  answers `latest_match > cleared_through`;
- `namespace_import::alias_live_at` does the same with the latest install and
  the latest removal.

Each maximum is over a *single* `u32`, which orders events within one
document. Two events in different documents that `has_run` both admits still
have to be ranked against **each other**, and a partial order gives no `max`.
Ranking them by their own file's offsets is unsound — a forget would revoke an
import merely because its local offset is larger — so the event key carries
more than an `Option`: the order needs

```rust
fn cmp_run(&self, a: (uri, at, enclosing_body), b: (uri, at, enclosing_body)) -> Option<Ordering>
```

with `None` — incomparable — folding to "abstain toward answering", the same
direction `at: None` took. `has_run` is then `cmp_run(event, query)`
against the existing body-leniency rule, so one relation serves both.

Both are thin readings of one private projection,
`RunOrder::common_frame(a, b) -> Option<(Placed, Placed)>`, which reduces two
points to positions in their deepest common document. `has_run` then applies
`indirection::in_effect_within` there and `cmp_run` applies
`namespace_import::load_order` — the same two single-document rules, over a
wider domain. The folds do not use `cmp_run` at
all: the leniency a removal needs is `has_run`'s, not `load_order`'s (a
maybe-never-run body statement must not be counted as having definitely run
*after* an install), so `ran_after(a, b)` is defined as
`has_run(a, b) == Some(false)` and the strict `cmp_run` is reserved for the
import-conflict rule, which inverts the sign — see `namespace_import::load_order`.

### 6.2 The shape that makes `cmp_run` total where it matters

A `source` graph is not merely a partial order over documents — it is a
**flattened execution sequence**, which is what makes the comparison
tractable. Sourcing a file inlines its whole body at the `source` statement's
position, so the DFS of the source forest *is* the run order, and two events
are comparable whenever they sit in one tree:

- lift each event to its root-ward path `[(root, at₀), (child₁, at₁), …]`,
  where `atᵢ` is the offset of the `source` statement that entered the next
  document;
- take the deepest common document, and compare the two paths' next entries
  there with the rule already in use for one document
  (`indirection::in_effect_within` for gating, `namespace_import::load_order`
  for conflicts);
- abstain (`None`) when the two events have different roots, when any edge on
  either path fails the §3.3 soundness gates, or when a document is reachable
  by two paths — a re-sourced file has no unique position and must not get
  one.

This keeps the whole order behind one relation and needs no numeric flattening
(no rank arithmetic to invalidate per generation): the paths are `O(depth)`,
and depth is the `source` nesting of a real project.

Two implementation notes on the abstentions:

- **Ambiguity is inherited.** A document below a re-sourced one has no unique
  position either, so `RunOrder::build` propagates the mark down the tree
  rather than only marking the doubly-entered file.
- **Two events in one *foreign* file are ranked against each other even
  though neither is ranked against the query.** A file's statements run
  consecutively, so a `namespace export p` followed by a `namespace export
  -clear` in one other file revokes — precision that needs no `source` edge at
  all, and that a single absent offset per event could not express, because it
  collapses "which file" and "where in it" into one value.

## 7. The `package require` half (issue #1279)

§4's measured reach is why the `package` half exists: over Tcl 9.0.4's
`library/`, `samples/`, and the multi-file fixtures the `source` order changes
**0** resolutions, because real Tcl writes
`source [file join $tcl_library init.tcl]` and no static fold can place
`$tcl_library`. Package-structured code writes its `source` statements the same
way — inside a generated `pkgIndex.tcl`, against a `$dir` the package loader
binds at run time — so the order it *does* have comes from `package`, not from
`source`.

### 7.1 The fact a `package require` establishes, and the one it does not

A `package require NAME` that **returns** has left `NAME` loaded, so the
providing file's statements have all run. That much is solid. What it does not
establish is that they ran *at the require*: the require is a load event only
for whichever require runs first, and every later one finds the package already
provided, returns its version, and evaluates nothing.

Oracle (byte-identical on tclsh 8.6.14 and 9.0.4). `a.tcl` and `b.tcl` hold the
same two lines — an "is it loaded?" probe, then `package require mylib` — and a
driver sources `a.tcl` and then `b.tcl`:

```text
  a.tcl: before its own require, is lib loaded? NO
  lib.tcl body running
  b.tcl: before its own require, is lib loaded? YES
```

Identical source text, opposite answers, decided by something neither file
says.

So the edge is **one-sided**: it places the provider at *at most* the require's
offset. `RunEdgeKind::PackageRequire` carries that, `Placed::exact` records it
through the projection, and `RunOrder::trusted` reads off the half of each
comparison the bound survives — "`a` ran first" needs `b` pinned, "`a` did not
run first" needs `a` pinned. Nothing else in the relation changes:
`common_frame` is the same projection, `in_effect_within` still answers
`has_run` and `namespace_import::load_order` still answers `cmp_run`, so the
gate and the conflict rule cannot drift apart.

Two consequences fall out rather than being coded:

- **A provider required from twenty files is still ordered against all twenty.**
  A `source` child entered twice is ambiguous (§6.2) because two `source`
  statements each claim it ran *there*; twenty requires each only bound it, and
  bounds compose. `RunOrder` keeps package edges out of the tree as
  `package_entries` and substitutes a point by each require site that bounds its
  tree.
- **Two documents that a `package require` each brought in are ranked against
  each other exactly as they were before: not at all.** Both sides are bounds,
  and two bounds settle nothing.

### 7.2 The three abstentions

Each is stated the way §6's own are, and each is pinned by a test that fails
when its gate is removed. The gates live on
`WorkspaceIndex::package_run_edges`, except abstention 2, which is structural.

| # | Uncertainty | What we do | Test |
|---|---|---|---|
| 1 | **`auto_path` is mutable.** A document's own `lappend auto_path` mutations are folded in for *resolution*, but they do not establish an order, and the path decides which copy of a package wins. | A package **two** indexed documents provide yields no edge at all. | `abstention_1_two_documents_providing_one_package_order_nothing` |
| 2 | **A package may already be loaded**, in which case the require is not a load event. | The edge is a *bound*, not a position: "the provider has already run" is a fact from the require onwards; "the provider has not run yet" is never a fact. | `abstention_2_a_statement_above_the_require_stays_unordered` |
| 3 | **`package ifneeded` bodies are arbitrary scripts**, so the mapping from require-site to the statements that run is not static. | A package this workspace registers a `package ifneeded` script for yields no edge. | `abstention_3_an_indexed_ifneeded_script_orders_nothing` |

Oracles for 1 and 3, both byte-identical on 8.6.14 and 9.0.4:

- *auto_path* — two directories each holding a `mylib2 1.0`, only one of which
  exports `p`. `lappend auto_path pkgA pkgB` gives `::lib::p` → `from A` with
  `p` exported; `pkgB pkgA` gives `from B` with nothing exported. Nothing in
  either file says which.
- *ifneeded* — one `pkgIndex.tcl` whose registered body branches on the
  environment answers `::c::p` → `real` (with the provider's `namespace
  export p` run) or → `stub` (with nothing exported), for the same
  `package require`.

Plus the gates the rest of the family already applies: a **conditional**
require (`if {[catch {package require Tk}]}`) may never run, a **conditional
provide** likewise — tcllib's `doctools2idx/import_json.tcl` writes `package
provide dict 1` inside a nested `if`/`catch`, a shim supplied only on an old
interpreter, and reading that as "this file provides `dict`" would name the
wrong provider everywhere else — a **non-literal** name (`package require
$pkg`) names nothing statically, and a document that requires the package it
itself provides is a self-edge, which `RunOrder::build` discards. A document that is both `source`d **and** `package require`d is
marked ambiguous — it would otherwise carry a tree position saying it ran
exactly there beside a bound saying it had already run, and those can
contradict.

### 7.3 Measured reach

Same method as §4: two indexes over the same documents, one with the
package-derived edges and one without, diffing `resolve_wildcard_import` at
every bare call site. Corpus: tcllib 2.0 `modules/` + `apps/` + `examples/`,
Tcl 9.0.4 `library/`, and the repository's `samples/` + `tests/`.

| | documents | `package require` sites | provable package edges | bare call sites | resolutions removed | added |
|---|---|---|---|---|---|---|
| every `.tcl`, `pkgIndex.tcl` included | 948 | 1 424 | 47 | 239 663 | 0 | 0 |
| `pkgIndex.tcl` excluded | 805 | 1 419 | **704** | 237 136 | **122** | 0 |
| tcllib `modules/` alone, `pkgIndex.tcl` excluded | 660 | 1 319 | 612 | 220 105 | 0 | 0 |

Three things follow.

**It moves numbers, where the `source` half moves none.** 704 provable edges
against the `source` half's 3 resolvable statements, and 122 changed
resolutions against 0.

**Every change is a removal, and every removal is correct.** All 122 are the
same shape: a bare call inside a *provider* file that would otherwise resolve
through a wildcard import written in some *consumer* file. The consumer requires the
package and only then imports, so the provider's whole body — the call
included — ran before the import existed. Oracle: inside the provider's own
body, `info commands ::helper` is empty; after the importer's `namespace import
::prov::*` it is not. 116 of them are calls in tcllib's
`modules/math/bigfloat2.tcl` reached through `examples/math/bigfloat.demo.tcl`'s
`namespace import ::math::bigfloat::*`, and 6 are calls in Tcl 9.0.4's
`msgcat.tcl` reached through `modules/doctools2idx/msgcat_*.tcl`'s
`namespace import ::msgcat::*`. Those call sites still resolve — through
ordinary namespace resolution, which is what real Tcl uses there — they just no
longer resolve through an import the program had not yet run.

**Abstention 3 costs almost all of the reach on a checkout that indexes its
package index files.** tcllib ships a `pkgIndex.tcl` per module, and each one
registers a `package ifneeded` for the packages that module provides, so the
gate fires on nearly every package: 704 edges become 47, and the measurable
effect becomes 0. That is the abstention working as specified — the registered
script is what actually runs, and `[list source [file join $dir base64.tcl]]`
does not tell us statically that it runs `base64.tcl`.

The way to lift it, deliberately **not** taken, is to fold `$dir` inside a
`pkgIndex.tcl`: the package loader binds it to the directory holding that file,
which is a documented Tcl contract rather than a guess. That would give the
*`source`* half those edges, not this one — the prize is the 657-edge gap
between the two rows above.

The package half needs no filesystem knowledge: a `package require NAME` names
its provider through the index's own `package provide` records, so an index
with no resolver installed still derives package edges while the `source` half
abstains.

§4's two negatives stay negative. A load order says *when* a statement ran;
`package require` widens the set of pairs it can say it for, and says nothing
about a statement in a file nobody indexed (#1116 item 1) or about a script
aborting part-way (#1116 item 4).


## 8. Conditional original source navigation purpose

The coordinate construction above is a source-order hypothesis, not an independently observed interpreter timeline. The actual WorkspaceIndex producer now uses `RunOrder::for_source_navigation` with retained `SourceOrderContext` values captured at the actual analysis ingress. That owner reuses the existing readonly original command/body inventory, original selected Registry words and original definition-body metadata. It keeps the whole Document image, exact immutable input and original Realm. Source graph edges require the actual original current-interpreter SourceLoad receipt, while package bounds require a genuine selected package requirement. Labels, raw paths, supplied body spans and unowned nominal commands cannot supply those receipts. Missing schema/body/input remains unknown, and the existing shared graph builder propagates unavailable source seeds through its original ambiguity rules.

`RunOrder::{navigation_precedence,navigation_comparison}` returns sealed `SourceNavigationOrder` values retaining Conditional purpose, except between two straight-line positions of one document (§8.1). Relative source coordinates do not establish handler/body entry, namespace event completion, successful loading, current package state, Native chronology or Normal completion. The legacy `has_run` and `cmp_run` methods always return None for that workspace purpose. `SourceNavigationOrder::modeled_source_exclusion` therefore withholds every conditional result from the namespace pattern/clear/alias/conflict exclusion gates; actual positive original evidence and unknown barriers remain available. A conditional clear — one in another document, a deferred body, a conditional branch or after a possible early exit — cannot revoke evidence merely because its source word follows it. Existing completed modeled command-publication worlds are not repurposed as namespace export/import or loader completion receipts.

`RunOrder::build` and Default retain their explicit constructed-input compatibility purpose and original coordinate algorithm. Its legacy fixed tests and assertions remain unchanged. Such constructed inputs supply no original-source or Native authority. Original source contexts canonicalize actual point/body geometry without relying on reporting Scope maps. Multiple accumulated source-site views retain an owner only when their original complete contexts agree; mismatch is sticky unavailable until the document is cleared. Every index revision invalidates the source-bearing order and export snapshot, while the existing full-versus-document-local command-settlement boundary is preserved.

Five new fixed source controls cover actual WorkspaceIndex export snapshot adoption, source/image/input/configuration and replacement currency, original body/report independence, Conditional versus constructed purpose, retained source/load operations, injected report-body refusal and the existing re-source/cycle ambiguity rules. These are unexecuted source definitions. The constructed-model source_graph and namespace_import regression controls are unchanged. The same-document controls changed with §8.1, and the WorkspaceIndex and Definition chronology tests whose events are straight-line positions of one document now receive that document's order; no event-completion premise is attached to any other shape.

The actual in-document Definition alias-lifetime and original-export timeline ingresses also use `in_document_source_order` with the same original analysis owner and source-purpose constructor. Missing input does not fall back to the constructed model. The unknown-export gate consumes typed navigation precedence. Pre-existing independent call-resolution/slot folds and StrongNative admission guards remain unchanged; source order supplies them no runtime fact. A sixth fixed source control binds the actual local original-export adopter and missing-input refusal, with no execution outcome attached. One lazily captured order is shared by every import site and the alias fold of a single resolution.

Off the same-document sequence, the original source ownership of both comparison points is checked before any package-require alternative is considered. A document-level bound cannot replace a dynamic, unrepresented deferred-body or missing-input point. The same check is an identity for explicitly constructed input models. A seventh fixed source control retains a genuine require-bound route and positive owned point while refusing both orientations/rankings for actual unowned points. It grants no successful package loading, namespace-event completion or Native/Normal result.

### 8.1 Same-document straight-line sequence

Within one document the source sequence is known, so its removals are ordered there: `namespace export -clear`, `namespace forget` and a redefinition of the imported name revoke what they follow in that document. Cross-document order stays Conditional, including along a resolved `source` or package edge.

The relation is reach-conditioned: when the later position is reached, every earlier straight-line position of the document has run. That is a fact for every query that is itself such a position — a statement or a gap between statements — and it is all the folds ask. The document end means *after the document completes normally*; an error path never reaches it, and no consumer may read it as "after a failed load". No production consumer queries it today: call and import sites are command positions. `SourceOrderContext` retains a `DocumentSequence` built from the same readonly command/body inventory as its sites in one start-ordered pass. A point is a *sequence position* when all of the following hold:

- it is not inside a definition body, whether or not that body is a retained region;
- its innermost retained script region is the document root, or a body entered once, immediately and in the current frame — `ScriptBodyFlow::Sequence` or `ScriptBodyFlow::ConcatenatedScript` with a single complete script word and no frame level word — by a straight-line command that lies outside every caller-context-unavailable body, has `Selected` original Registry words and whose command word no document procedure or class declaration shares;
- inside that region it is a gap between commands, the command word of a command, or an argument of a command with `Selected` words and complete roles, outside every operand whose role may carry a script (`Body`, `Expr`, `LambdaLiteral`, `OpaqueScript`, `CommandPrefix`); and
- it precedes every possible early exit.

`namespace eval` and `eval` at the document root therefore enter their bodies; `uplevel`, a nested `namespace eval` or `eval` inside one, and one whose name the document declares (`proc eval`, `proc namespace`, at any namespace) do not. Procedure, method and class bodies, callbacks, `if`/`switch` branches, loop, `catch` and `try` bodies and command substitutions are never sequence positions.

A possible early exit is a command outside definition bodies whose Registry traits terminate the block or the process (`return`, `error`, `throw`, `exit`), a straight-line command containing one in any nested region (`if {..} return`), and a straight-line command that may run a script operand now (any flow but `Deferred` and `None`) which the inventory does not retain as a region. Every position after the end of that straight-line command, and the document end, is withheld.

The document end has one more, end-only cutoff. It is withheld when any command outside definition bodies and stored callbacks — at the top level or nested in the immediately run bodies of a top-level command — may end the document's normal completion early: its traits transfer control (`break`, `continue`, `tailcall`), it selects `BodyExecutionSpec::SourceFile` (`source`, whose file may `return -code return`), it does not defer its scripts and either has a script-carrying operand (`Body`, `Expr`, `LambdaLiteral`, `OpaqueScript`, `CommandPrefix`) that is not retained as a region — `apply $l`, `expr $e`, `eval $s` — or may invoke a command prefix by the Registry's own prefix positions (`lsort -command cmp`, Tcl 9 `regsub -command`), or its words are unresolved, unvalidated or not `Selected` (a user procedure or a dynamic head may `return -code return`). An empty literal body runs nothing. Every `Expr` operand, retained or not, is parsed by the shared expression owner (`tcl_syntax::expr`) under the input's dialect: math functions are commands, so any function call — a user `::tcl::mathfunc` procedure, an overridden built-in, an undefined name reaching `unknown` — or a parse failure withholds the end, as does an operand that is not a literal word; command substitutions inside it are judged by their retained regions. Variable reads need no exclusion: a trace callback's non-OK completion becomes an error. Such a command does not cut later positions: a later position is reached only when control did pass. Errors raised by ordinary commands — an import conflict, a qualified `namespace export` pattern — need no cutoff under this reading, since an error path never completes normally; the Registry proves no command cannot raise, so an "after the load attempt" reading could never admit the end at all.

Known limits: a command hijacked from another document (a `proc eval` in a file sourced earlier, or a built-in math function overridden under `::tcl::mathfunc` by another document; every function call already withholds the document end, so no built-in allowlist depends on it) or through `interp alias` is outside this document's declarations; only the Realm barrier, which refuses `Selected` words for a command its source transition tape sees shadowed, deleted or renamed, guards it.

When both points of `navigation_precedence`/`navigation_comparison` are sequence positions of the same indexed document, the answer carries `SourceNavigationOrderPurpose::DocumentSequence` with the load-level `in_effect_within`/`load_order` relation, and `modeled_source_exclusion` admits it like a constructed model. The original-site check is not consulted for that pair: the retained sequence inventory is its owner. Every other pair keeps the existing Conditional or unavailable answer.

Only query-positioned consumers use the order. The shared folds apply a removal only when its own position against the query is known (`NamespaceStatementPurpose::removal_in_effect`), so a body-local or cross-document query still sees every candidate. With no query point — `WildcardImportIndex::link_alias_live` for exact links, and any other unpositioned caller, for exact and glob imports alike — no removal applies, which is the conditional answer. The import-conflict rule, which cancels a later install, needs the earlier alias to be certainly live at the later import (`alias_certainly_live`): every removal counts as possibly in effect unless it is known not to have run at the later import or known to have run before the earlier one. A forget in a sourced file or an `if` branch between two imports therefore never cancels the second. Patterns and installs keep the abstain-toward-in-effect gate, constructed models keep their original behaviour and source horizon advice is unchanged. The legacy `has_run`/`cmp_run` methods remain None for workspace navigation.

Controls (unexecuted source definitions):

- `original_workspace_order_retains_conditional_purpose_without_tombstone_authority` (source_graph), `workspace_order_keeps_original_export_evidence_without_conditional_clear_completion` (WorkspaceIndex) and `document_export_adopter_keeps_conditional_clear_and_missing_input_unknown` (Definition) change for the same-document shape only: purpose `DocumentSequence`, and the unconditional same-document `-clear` revokes at the document end. The Definition control also keeps a `-clear` in an `if` branch that leaves the export in place.
- Cross-document intent: `workspace_order_keeps_cross_document_clear_conditional`, `a_clear_in_the_sourcing_document_stays_conditional_for_the_sourced_export` (the clearing document sources the library) and `document_end_follows_every_straight_removal_but_no_foreign_one`, which also pins the unpositioned answer.
- Conflicts: `a_sourced_forget_between_two_imports_does_not_make_the_later_one_conflict` and `a_branch_forget_between_two_imports_does_not_make_the_later_one_conflict`.
- Admission: `document_sequence_orders_straight_line_bodies_and_withholds_branches`, `document_sequence_ends_at_a_possible_early_exit`, `document_end_is_after_normal_completion_only` (a sourced file or user procedure doing `return -code return`, a top-level `break`, `apply $l`, `lsort -command cmp`, `expr $e`, `expr {f(1)}`, `if {f(1)}`, an undefined function reaching a user `unknown`) and `document_sequence_refuses_shadowed_or_namespace_local_entering_commands` (including `rename myeval eval`).
