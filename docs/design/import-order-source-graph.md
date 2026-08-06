# A load-order partial order from the `source` / `package require` graph

The load order shared by the whole wildcard-import gating family (#1104 item
3, #1116 items 3 and 6, and the cross-file half of every rule in
[`contracts/command-resolution.md`](contracts/command-resolution.md)'s import
section).

**Status: implemented.** `tcl_lsp_core::source_graph::RunOrder` is the
relation §6 calls for; `WorkspaceIndex::run_order` builds it once per
`generation()` from the host's resolver
(`WorkspaceIndex::set_source_resolver`), and both wildcard-import tiers rank
their events with it through the two shared decision functions. Sections 1–5
below record the facts and the reasoning the design rests on; §6 is the shape
that was built.

## 1. The abstention, and what lifted it

Both LSP resolution tiers used to order import-lifecycle events by **byte
offset within one document**, via
`tcl_compiler::analyser::indirection::in_effect_within`. Across documents
they ordered nothing, because nothing in a Tcl source tree says which file
runs first in general. Every cross-file event was passed to the shared
decision functions (`namespace_import::exported_at_import_site`,
`namespace_import::alias_live_at`) with `at: None`, and those abstained
toward *answering*:

| Fact in another file | Before | With the `source` order |
|---|---|---|
| `namespace export p` | counts (the import may resolve) | counts when the graph proves it ran first; **does not** when it proves it ran later |
| `namespace export -clear` | revokes nothing | revokes when it provably ran between the export and the import |
| `namespace import` (an install) | counts | counts, now as a fact |
| `namespace forget` | revokes nothing | revokes when it provably ran before the call |
| a second import of the same name | never conflicts (#1116 item 6) | conflicts when its site provably ran first (`cmp_run`) |

Each old row was one-directional leniency. The genuinely-sequenced idiom
`source lib.tcl ; namespace forget ::lib::p` got *both* halves wrong: the
install from `lib.tcl` counted (right), and the forget beside it revoked
nothing (wrong). One relation fixed the whole table at once, which is why it
was worth doing as one piece of work rather than five.

Everywhere the graph does **not** prove an order — different trees, a
re-sourced file, a `source` cycle, a computed `source $dir/x.tcl`, or a host
that installs no resolver — the old column still applies, unchanged and by
construction: `RunOrder` answers `None`, and `None` folds to exactly the
abstention `at: None` expressed.

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

- `WorkspacePackageRequire { uri, name }` — **no offset, no version**. It
  says *that* a file requires a package, not *where*, so it cannot sequence
  anything as it stands.
- `SignaturePackageProvide { name, version, range }` per document — the other
  end of the edge, with an offset.
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

4. **`package require` sequencing (optional, second phase).** A
   `package require` is an ordered statement too, and its package's files load
   at that point. To use it, `WorkspacePackageRequire` needs the `at` /
   `enclosing_body` pair `WorkspaceGlobImport` already carries, plus the
   resolver's package→URI mapping recorded in the index rather than consulted
   ad hoc. `package provide` gives the other end. This is strictly more
   speculative than the `source` half: `auto_path` is mutable at runtime and
   a package may already be loaded, so "the require ran here" does not imply
   "the package's files ran here and not earlier".

## 4. Which abstentions it lifts

| Issue | Abstention | Status |
|---|---|---|
| #1104 item 3 | foreign `namespace export` patterns always count; foreign `-clear`s never revoke | **lifted** — `RunOrder::has_run`, all three §3.2 cases |
| #1116 item 6 | two imports of one name in different files never conflict | **lifted** — `RunOrder::cmp_run` in `WildcardImportIndex::conflicting_alias_at` |
| #1116 (finding 1 note) | `source lib.tcl ; namespace forget ::lib::p` — install counts, forget revokes nothing | **lifted** — §3.2 cases 2 and 3 together |
| #1116 item 1 | in-document `-force` shadow with the export in another file | *not* lifted by ordering — it is an *observability* question, answered by the workspace tier's whole-program export view, which has to be threaded through `resolve_called_proc` |
| #1116 item 4 | import-error control flow (a failed import aborts the rest of its script) | *not* lifted — a different model (intra-script abort), though it shares the "what has run" vocabulary |

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

**What was built:** the resolver is held *on the index* as a plain `fn`
pointer (`WorkspaceIndex::set_source_resolver`, installed by the server's
`new_workspace_index`), and the order is a `Derived<RunOrder>` view like the
rest. Taking the resolver as a per-call argument would have meant threading it
through every caller of `resolve_wildcard_import` and every derived view that
builds a `WildcardImportIndex` — the order is consulted *inside* the per-call
import walk, not at its entry point. A `fn` pointer rather than a boxed
closure keeps the index `Debug + Clone + Default` with no manual impls, and
the resolver is a pure function of `(parent uri, raw path, is_literal)` in
every host — `WorkspaceIndex::SourceResolver`, the signature `source_seed_map`
already took, so one host resolver serves both and the order inherits the M9
tier's statically-foldable computed-path half for free.

An index with **no** resolver installed derives an empty order, and every
tier then behaves byte-identically to the pre-#1104-item-3 code. That is the
property the unit suite pins (`without_a_resolver_the_same_workspace_keeps_abstaining`),
and it is why the change could land without re-baselining the existing
cross-document tests.

**A trivial subset was considered and rejected for this round.** "Same
document `source a.tcl` written before an import of a namespace `a.tcl`
declares" still needs (1) the raw path resolved to a URI, which only the
server can do, and (2) that resolution reaching into `exports_name_at` /
`alias_live_at`, i.e. the whole plumbing above. There is no subset that is
provable without the resolver, so the work does not decompose below §3.1.

## 6. The shape that was built

Keep the discipline the family already has: **one** decision function per
question, both tiers calling it. The order lives behind a single relation with
the shape

```rust
fn has_run(&self, event: RunPoint<'_>, query: RunPoint<'_>) -> Option<bool>
```

— `None` meaning "no static order", which is what the old `at: None` event
encoding expressed. `RunPoint` is the `(uri, at, enclosing_body)` triple every
event and query point already carried as loose fields.

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

Both maxima are over a *single* `u32` because every ordered event today lives
in one document. Two events in different documents that `has_run` both admits
still have to be ranked against **each other**, and a partial order gives no
`max`. Feeding cross-document events into the current fold with their own
file's offsets is exactly the bug PR #1115's finding 1 removed (a forget
revoking an import because its local offset happened to be larger). So the
event key has to change shape, not just its `Option`-ness: the order needs

```rust
fn cmp_run(&self, a: (uri, at, enclosing_body), b: (uri, at, enclosing_body)) -> Option<Ordering>
```

with `None` — incomparable — folding to "abstain toward answering", the same
direction `at: None` took. `has_run` is then `cmp_run(event, query)`
against the existing body-leniency rule, so one relation serves both.

**As implemented**, both are thin readings of one private projection,
`RunOrder::common_frame(a, b) -> Option<(Placed, Placed)>`, which reduces two
points to positions in their deepest common document. `has_run` then applies
`indirection::in_effect_within` there and `cmp_run` applies
`namespace_import::load_order` — the *same* two single-document rules the
tiers used before, now with a wider domain. The folds do not use `cmp_run` at
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

Every current caller keeps its shape, the shared decision functions gained
their comparator instead of assuming one, and the abstention table in §1
shrank without a second rule appearing anywhere.

Two implementation notes on the abstentions:

- **Ambiguity is inherited.** A document below a re-sourced one has no unique
  position either, so `RunOrder::build` propagates the mark down the tree
  rather than only marking the doubly-entered file.
- **Two events in one *foreign* file are ranked against each other even
  though neither is ranked against the query.** A file's statements run
  consecutively, so a `namespace export p` followed by a `namespace export
  -clear` in one other file revokes — a precision win that needs no `source`
  edge at all, and one the old one-`Option<u32>`-per-event encoding could not
  express because it collapsed "which file" and "where in it" into a single
  absent offset.
