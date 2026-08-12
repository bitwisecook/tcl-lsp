# The salsa interned garbage collector — the invariant the edit path rests on

> Contract for `rust/tcl-lsp-db`. Companion to
> [`incremental-analysis.md`](incremental-analysis.md) (the per-item cascade
> this crate implements) and [`lsp-performance.md`](lsp-performance.md) (how
> the edit path is measured). Issue #1299.

## Summary

Six interned structs in `tcl-lsp-db` key on content that changes on **every
keystroke** inside a procedure body. Typing therefore mints a brand-new
interned id for each of them per edit, and each id holds a body's worth of
`Arc` payload in its memo table — the exact shape of an unbounded interning
leak.

It does not leak. The reason is **salsa's interned garbage collector**, an
implementation detail of salsa 0.27 that nothing in this repository used to
assert. Two plausible, review-friendly changes would silently disable it and
restore a KB-per-keystroke leak of the [#1035](#related-issues) class. This
document is the written form of that invariant; the code and tests that pin it
are listed under [Where it is enforced](#where-it-is-enforced).

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

Two other interned structs are deliberately **outside** this contract:

- `LexerCfgKey` (two booleans) and `CommandTail` (one command name) have a
  bounded key space, so even immortal slots cost a handful of tiny entries.
  This matters because `compilation_unit`'s own public signature takes a
  `LexerCfgKey`, which callers must therefore mint from untracked host code.
- `CfgContext` is content-keyed but *signature*-level: a body edit leaves it
  equal, so it churns only when a signature moves. It is reclaimed by the same
  mechanism when that happens.

## How the collector works (salsa 0.27.2)

From `salsa-0.27.2/src/interned.rs`:

1. **Slot reuse on a cold intern.** Interning a value that is not already in
   the table walks the shard's LRU list from the tail. Any slot whose
   `last_interned_at` is older than `DEFAULT_REVISIONS` (3) revisions is
   *reused*: its fields are overwritten with the new value, its id generation
   is bumped, and `clear_memos` is called on its memo table. That `clear_memos`
   is what actually drops the retained `Arc`s. Only if no stale slot is found
   does salsa allocate a new one.
2. **`Durability::LOW` only.** `ValueShared::is_reusable_with_durability`
   returns `true` **only** for `Durability::LOW`. Collecting a more durable
   slot would require invalidating that durability's revision
   (`Database::synthetic_write`, which needs `&mut` on the database), so salsa
   refuses rather than risk an unsound `maybe_changed_after` short-circuit.
3. **Where a slot's durability comes from.** It is the *minimum* durability of
   the inputs the creating query had read at the moment it interned. With **no
   active query at all**, salsa stamps `Durability::MAX` and
   `Revision::MAX` — an immortal slot that is never even added to the LRU list.

The consequence worth stating plainly: **`lru = N` is not what keeps a typing
session bounded.** The `lru` caps documented in the crate's "Deep-memo
eviction" section bound *memo payloads* for a given key; they do nothing about
the interned tables themselves. The garbage collector is what bounds those, and
it is silent — there is no error, no warning, and no slow-down when it stops
working, only monotonically climbing memory.

## The two hazards

### 1. A durability bump on an input

Marking `SourceFile`, `AnalyserConfig`, or `Project` as `Durability::HIGH`
("the config only changes when the user edits settings", "the file set changes
rarely") is a normal salsa optimisation — it lets revalidation skip
dependency-tracing for queries that read only durable inputs. Here it is a
memory-leak regression: every interned key above is minted by a query that has
read one of those inputs, so every slot would be stamped non-`LOW` and the
collector would never touch it again.

There is no diagnostic for this. The change looks beneficial, the tests keep
passing on behaviour, and memory grows by kilobytes per keystroke.

### 2. Interning outside a tracked query

`memoised_compilation_unit` interns `FnLatticeKey`, `ProcBodyKey`,
`TaintSummaryKey`, `SummaryDepsKey`, and `OptDepsKey` directly. Called from a
tracked query it inherits that query's `LOW` durability; called from ordinary
code it mints **immortal** slots, each pinning a full procedure IR plus its
lattice memos.

## The rules

1. `SourceFile`, `AnalyserConfig`, and `Project` stay at salsa's default
   `Durability::LOW`. No `Setter::with_durability`, no input-builder
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
| `raising_input_durability_disables_the_collector` | test-time | the first test going vacuous — it asserts the leaking session really does break that test's bound — and any change to salsa's "`LOW` only" collection policy that would make the durability rule stop mattering |
| `no_input_durability_is_raised` | test-time | a `with_durability` / builder-`durability` call site in `tcl-lsp-db` or `tcl-lsp-server`, i.e. the hazard-1 shape written where the behavioural tests cannot see it |
| `edit_session_memory_growth_plateaus` (`rust/tcl-lsp-db/tests/memory_growth.rs`) | test-time | the aggregate retained-bytes plateau (the #1035 regression class), of which this invariant is one contributor |
| Doc comments at the six struct definitions and the three inputs | doc-only | a reviewer reaching the definition site with no context |

### What the behavioural tests measure

Both drive 24 revisions of a small generated corpus through
`file_analysis_incremental` + `compiler_check_diagnostics` and count salsa's
`EventKind::DidReuseInternedValue` per ingredient, surfaced by
`TclDatabase::with_interned_reuse_logger`. That event fires **exactly** when the
collector recycles a stale slot, so the count is a direct read of the mechanism
rather than a proxy for it.

Live slot counts are the more obvious measure and were rejected: their steady
state depends on salsa's shard count,
`(available_parallelism() * 4).next_power_of_two()`, so any absolute budget
would be tuned to the machine that wrote it and would fail on a bigger one.
Reuse counts have no such dependence — the collector either runs for an
ingredient or it does not.

Measured on a four-core box (24 edits, four worker procedures re-keyed per
edit): 81 / 77 / 80 / 77 reuses for `ItemBodyKey` / `FnLatticeKey` /
`ProcBodyKey` / `OptDepsKey`, and 22 / 34 for `TaintSummaryKey` /
`SummaryDepsKey`. The raised-durability control records **zero** of any kind,
and its slot counts climb to `WORKERS × EDITS` instead. That control's
slot-count assertion is what keeps the corpus honest: if a future change stopped
re-keying the ingredients once per edit, the control would stop accumulating and
fail, rather than letting the first test pass on a session that interned
nothing.

Hazard 2 was checked the same way while the guardrail was being written, by
temporarily re-exposing `memoised_compilation_unit` and calling it once per
revision from ordinary, untracked test code. Reuse for the ingredients that call
reaches collapses to exactly zero — `FnLatticeKey` 77 → 0, `ProcBodyKey` 80 → 0,
`TaintSummaryKey` 22 → 0 — while the ingredients it does not reach are
unaffected. One untracked call per revision is enough to poison the shared
slots, because salsa takes the *maximum* durability across every query that
interns a value and drops a slot from the LRU list as soon as that maximum
leaves `LOW`.

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
- **#1144 / #1179** — the deep-memo eviction (`lru = N`) work, which is the
  *separate* mechanism the crate documentation describes under "Deep-memo
  eviction".
- **#1299** — pinning this invariant.
