# The salsa interned garbage collector — the invariant the edit path rests on

> Contract for `rust/tcl-lsp-db`. Companion to
> [`incremental-analysis.md`](incremental-analysis.md) (the per-item cascade
> this crate implements) and [`lsp-performance.md`](lsp-performance.md) (how
> the edit path is measured).

## Summary

Six interned structs in `tcl-lsp-db` key on procedure-body content or its
interprocedural projections. Body edits can mint new interned ids, each retaining
analysis payloads in its memo table. Salsa's interned garbage collector bounds
this working set only when the keys are interned inside tracked queries at
`Durability::LOW`. The code and tests enforcing that contract are listed under
[Where it is enforced](#where-it-is-enforced).

## The six per-revision interned keys

All are defined in `rust/tcl-lsp-db/src/lib.rs`.

| Interned struct | Field that changes per revision | What the stale memo retains |
|---|---|---|
| `ItemBodyKey` | `body_text: Arc<str>` | the body's isolated `AnalysisResult` |
| `FnLatticeKey` | `body: Script` (the whole procedure IR) | the offset-0 `FunctionUnit` (CFG, SSA, types, taints) |
| `ProcBodyKey` | `body_text: String` | the lowered `Arc<Script>` |
| `OptDepsKey` | `body_source`, `proc_body_source` | the procedure's optimisation set |
| `TaintSummaryKey` | `reachable` (shifts as projections move) | the taint cascade result |
| `SummaryDepsKey` | `interproc_reachable`, `callee_summaries` | the interprocedural summary |

The following interned keys have different churn boundaries:

- `LexerCfgKey` (two booleans) and `CommandTail` (one command name) have a
  bounded key space, so even immortal slots cost a handful of tiny entries.
  This matters because `compilation_unit`'s own public signature takes a
  `LexerCfgKey`, which callers must therefore mint from untracked host code.
- `CfgContext` is content-keyed but *signature*-level: a body edit leaves it
  equal, so it churns only when a signature moves. It is reclaimed by the same
  mechanism when that happens.

## How the collector works (salsa 0.28.5)

From `salsa-0.28.5/src/interned.rs`:

1. **Slot reuse on a cold intern.** Interning a value that is not already in
   the table walks the shard's LRU list from the tail. Any slot whose
   `last_interned_at` is older than `DEFAULT_REVISIONS` (3) revisions is
   *reused*: its fields are overwritten with the new value, its id generation
   is bumped, and `clear_memos` is called on its memo table. That `clear_memos`
   is what actually drops the retained `Arc`s. Only if no stale slot is found
   does salsa allocate a new one.
2. **`Durability::LOW` only.** `is_reusable::<C>` returns `true` **only** for
   `Durability::LOW` (and only when the ingredient is not configured immortal —
   see rule 4). Collecting a more durable slot would require invalidating that
   durability's revision (`Database::synthetic_write`, which needs `&mut` on the
   database), so salsa refuses rather than risk an unsound
   `maybe_changed_after` short-circuit.
3. **Where a slot's durability comes from.** It is the *minimum* durability of
   the inputs the creating query had read at the moment it interned. With **no
   active query at all**, salsa stamps `Durability::MAX` and
   `Revision::MAX` — an immortal slot that is never even added to the LRU list.
4. **Immortal ingredients are never collected.** An interned
   struct declared without a `'db` lifetime must opt into
   `unsafe(no_lifetime)` + `revisions = usize::MAX`, which sets
   `Configuration::REVISIONS` to `IMMORTAL` and makes `is_reusable` return
   `false` for the whole ingredient. Every interned struct in this crate carries
   `<'db>`, so all of them stay collectable; dropping the lifetime from one of
   the six per-revision keys would disable the collector for it as surely as a
   durability bump.

The consequence worth stating plainly: **`lru = N` is not what keeps a typing
session bounded.** The `lru` caps documented in the crate's "Deep-memo
eviction" section bound *memo payloads* for a given key; they do nothing about
the interned tables themselves. The garbage collector is what bounds those, and
it is silent — there is no error, no warning, and no slow-down when it stops
working, only monotonically climbing memory.

## The two hazards

### 1. A durability bump on an input

Marking `SourceFile`, `AnalyserConfig`, `Project`, or `EvaluatorEpoch` as
`Durability::HIGH` lets revalidation skip dependency-tracing for queries that
read only durable inputs. It also prevents collection of slots whose creating
queries have no remaining `LOW` input read. A single raised input does not
necessarily disable collection: query durability is the minimum of its reads.
The control raises all three inputs its deep query path reads, including
`EvaluatorEpoch`, so that a remaining `LOW` read cannot mask the leak.

Salsa emits no diagnostic for ineligible slots; semantic results can stay
correct while retained memory grows.

### 2. Interning outside a tracked query

`memoised_compilation_unit` interns `FnLatticeKey`, `ProcBodyKey`,
`TaintSummaryKey`, `SummaryDepsKey`, and `OptDepsKey` directly. Called from a
tracked query it inherits that query's `LOW` durability; called from ordinary
code it mints **immortal** slots, each pinning a full procedure IR plus its
lattice memos.

## The rules

1. `SourceFile`, `AnalyserConfig`, `Project`, and `EvaluatorEpoch` stay at
   salsa's default `Durability::LOW`. No `Setter::with_durability`, no input-builder
   `durability` / `<field>_durability` call.
2. Anything that interns one of the six per-revision keys runs inside a tracked
   query. `memoised_compilation_unit` is `pub(crate)` for this reason; the
   sanctioned entry point is the tracked `compilation_unit` query.
3. A new interned struct whose key contains per-revision content joins the
   table above, the crate documentation, and `PER_REVISION_INTERNED` in
   `rust/tcl-lsp-db/tests/interned_gc.rs`.

## Where it is enforced

All three tests live in `rust/tcl-lsp-db/tests/interned_gc.rs`.

| Guardrail | Kind | What it catches |
|---|---|---|
| `memoised_compilation_unit` is `pub(crate)` | compile-time | any caller outside `tcl-lsp-db` interning body keys from untracked code |
| `interned_slots_stay_bounded_across_an_edit_session` | test-time | the live interned working set tracking the edit count instead of the program, from any cause — a durability bump inside the crate's own graph, an untracked intern on the edit path, or a salsa upgrade that changes the policy |
| `raising_input_durability_disables_the_collector` | test-time | a vacuous primary test: the raised-input control must exceed its slot-growth bound for all four body-keyed ingredients and report zero reuse for all six ingredients |
| `no_input_durability_is_raised` | test-time | a `with_durability` / builder-`durability` call site in `tcl-lsp-db` or `tcl-lsp-server`, i.e. the hazard-1 shape written where the behavioural tests cannot see it |
| `edit_session_memory_growth_plateaus` (`rust/tcl-lsp-db/tests/memory_growth.rs`) | test-time | the aggregate retained-bytes plateau (the #1035 regression class), of which this invariant is one contributor |
| Doc comments at the six struct definitions and the four inputs | doc-only | a reviewer reaching the definition site with no context |

### What the behavioural tests measure

Both drive the same generated corpus through `file_analysis_incremental` +
`compiler_check_diagnostics` for 24 and 96 edits. The primary test requires a
nonempty live slot set for each of the six ingredients after the short session,
and at most twice as many slots after the fourfold longer session. Comparing
sessions on the same machine avoids an absolute budget tied to salsa's shard
count, `(available_parallelism() * 4).next_power_of_two()`.

Reuse events are diagnostic counts, exposed by
`TclDatabase::with_interned_reuse_logger`. `EventKind::DidReuseInternedValue`
fires when the collector recycles a stale slot, but a bounded working set spread
across many shards can produce no reuse. In particular, this corpus's
`TaintSummaryKey` and `SummaryDepsKey` projections stay bounded under both
input-durability conditions; the four body-keyed ingredients discriminate the
leak. The raised-durability control sets `SourceFile`, `AnalyserConfig`, and
`EvaluatorEpoch` to `HIGH`, requires those four ingredients to exceed the same
slot-growth bound, and requires zero reuse for all six.

Measured on a four-core box (24 edits, four worker procedures re-keyed per
edit): 81 / 77 / 80 / 77 reuses for `ItemBodyKey` / `FnLatticeKey` /
`ProcBodyKey` / `OptDepsKey`, and 22 / 34 for `TaintSummaryKey` /
`SummaryDepsKey`. The raised-durability control records **zero** of any kind,
and its body-keyed slot counts grow with `WORKERS × EDITS`. These are scoped
reuse observations, not the primary test's acceptance criterion. The control
also requires at least one slot per short-session edit for each body-keyed
ingredient, so a corpus that stops re-keying those ingredients fails.

A scoped untracked-call observation recorded reuse changes of `FnLatticeKey`
77 → 0, `ProcBodyKey` 80 → 0, and `TaintSummaryKey` 22 → 0 when
`memoised_compilation_unit` was called once per revision outside a tracked
query. This is not a current test path: the helper is crate-private. The
mechanism is salsa taking the *maximum* durability across every query that
interns a value and dropping a slot from the LRU list when that maximum leaves
`LOW`.

### Why the source-level test exists as well

Durability is chosen at the *setter call site*, so a `Durability::HIGH` written
in `tcl-lsp-server` would never appear in a database `tcl-lsp-db`'s own tests
build. `tcl-lsp-server` is the only crate in the workspace that depends on
`tcl-lsp-db`, so those two crates are the whole search space.

## Related issues

- **#1035** — the original KB-per-keystroke leak (leaked `&'static` command
  specs), the class this invariant protects against re-entering.
- **#1181** — the corpus the edit path's memory behaviour is measured against
  (~0 bytes/edit steady state, ~66.5 MB across 60–500 edits).
- **#1144 / #1179** — deep-memo eviction (`lru = N`), a separate mechanism
  documented under the crate's "Deep-memo eviction" section.
