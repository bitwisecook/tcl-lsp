# Incremental analysis — the per-item walk with cascade invalidation

How the Tcl analyser is made incremental at *item* granularity, so a keystroke
inside one procedure recomputes that procedure rather than the whole file: the
firewall that makes it sound, the query graph it runs on, the offset invariants
that make the memo hit, the fallbacks that keep it byte-identical to the
whole-file walk, and where the remaining per-edit cost sits.

Companions: [`incremental-analysis-experiments.md`](incremental-analysis-experiments.md)
(the corpus and the measurements this design rests on),
[`lsp-performance.md`](lsp-performance.md) (how the results are delivered to the
editor), [`current-architecture.md`](current-architecture.md) (the runtime
model), and [`target-architecture.md`](target-architecture.md) (Layer 4, the
general cascade this is one instance of).

## Why

`Analyser::analyse` is linear (~0.14 ms/line) but whole-file: an 8.5k-line file
costs about a second per edit. The measured per-edit split on `practcl.tcl`:

| stage | cost | natural granularity |
|---|--:|---|
| lex + tree + segment | ~3 ms | whole-file (cheap) |
| **analyser walk (scope trees + diagnostics)** | **~1015 ms** | **whole-file** |
| `CompilationUnit` + `run_all_checks` | ~146 ms | per-`FunctionUnit`, built whole-file |
| optimiser passes | ~141 ms | per-`FunctionUnit`, built whole-file |

The walk dominates, and reusing segmentation alone does not help — parsing is
already negligible. The lever is making the walk and the lattices recompute only
the edited item.

## The firewall: signatures vs bodies

The split that makes this tractable:

- A **proc or method body** is **item-local**: its parameter traits, local
  command invocations, local diagnostics, and scope subtree are a pure function
  of *(body text, params, enclosing namespace, registry, dialect, stub
  overlay)* — nothing from sibling bodies.
- The **cross-item facts are signatures**: name resolution and the W123
  unresolved-command / arity passes read the file's *set* of
  `all_procs ∪ all_classes ∪ command_aliases ∪ ensemble_namespaces` plus the
  namespace tree — item **headers**, not bodies.
- The **lattices** are already per-`FunctionUnit` (SSA → def-use → SCCP → type →
  rendered → taint, plus GVN); the only genuine cross-item cascade is the
  interprocedural `pure`/effect fixpoint and the taint re-run that consumes it.

So the query graph keys **bodies** (expensive, item-local) separately from
**signatures** (cheap, cross-item). A whitespace or in-body edit changes one
body and leaves every signature — and therefore every other item's analysis, the
resolution table, and the interprocedural summary — untouched.

## Query graph

The shape, with the queries that implement it in `tcl-lsp-db`:

```
input    source(file), config(global), project(files); registry is a durable field
derived  item_tree(file)            ← source                # FIREWALL: structure, not bytes
derived  item_sigs(file)            ← item_tree             # headers: name, params, ns, span, kind
derived  file_decls(file)           ← item_sigs             # procs/classes/aliases/ensembles + ns tree
derived  item_body_analysis(body)   ← ItemBodyKey           # the per-line walk, one item, offset 0
derived  file_analysis_incremental  ← item_body_analysis*, file_decls
                                                            # graft, rebase, then the cross-item tail
derived  lower_proc_body(proc)      ← ProcBodyKey           # per-item IR lowering
derived  compilation_unit(file,cfg) ← lower_proc_body*      # shared by both diagnostic consumers
derived  function_lattice(fn)       ← FnLatticeKey          # CFG/SSA/SCCP/type/rendered/taint, offset 0
derived  taint_cascade(fn)          ← TaintSummaryKey       # interprocedural taint, reachable-callee keyed
derived  function_optimisations(fn) ← FnLatticeKey + deps   # per-procedure optimiser memo
derived  compiler_check_diagnostics ← compilation_unit, function_optimisations*
derived  project_*                  ← item_sigs*            # cross-file resolution, arity, class index
```

**Early cutoff is the point.** `item_tree` and `item_sigs` are keyed on
*structure*, so a body-only edit produces equal signatures — `file_decls`, the
interprocedural summary, and every other item's analysis are reused, and only
the edited body plus its lattices recompute. A signature edit (rename, parameter
change) changes `item_sigs`, which cascades to `file_decls` (re-resolve call
sites, re-run W123) and the interprocedural layer (re-run taint on callers) —
exactly the dependents, nothing more.

Salsa input setters always bump the revision (there is no value-equality cutoff
on *inputs*), so an item's body input is set **only when the item-tree diff says
it changed**; otherwise every keystroke would re-run direct dependents
regardless.

The project layer is on the same graph: a `Project` input carries the workspace
file set, so cross-file resolution and arity are tracked edges with
reverse-dependency invalidation, with a per-symbol `command_arity` early cutoff
so an unrelated procedure's signature edit does not wake a file.

## Offset invariance is the load-bearing property

A memo only pays off if an unmoved-but-shifted procedure (lines inserted above
it) is a cache hit. Two rules make that true:

- **Bodies are analysed at offset 0 and grafted.** The per-item walk emits facts
  with offsets relative to the item's start; the aggregate rebases them to
  absolute positions using the item's span. The analyser is fully
  offset-shift-invariant (verified across the corpus), so this is exact.
- **Lattices stay at offset 0; consumers add the base.** `FunctionUnit` carries
  a `base_offset`, and every diagnostic consumer converts at emit time via
  `abs_span` / `abs_pos`. There is no per-procedure span-rebase walk.

**The span-provenance rule** (what must be converted, and what must not): only
spans read from a `FunctionUnit`'s `cfg` / `ssa` / `sccp` — including the
`CommandTokens` argument spans those statements carry — are offset-0 and need
`abs_span`. `cu.ir_module` keeps absolute whole-file offsets, because it is
lowered once for the file and only the per-procedure lattice *key* is normalised
to 0. So optimiser passes that walk `cu.ir_module` are untouched, while passes
and emitters that reach through `cu.functions()` must convert. Mixing the two
conventions inside one sort or one source slice corrupts both: sorting by span
is offset-shift invariant only when every span in the sort shares a convention,
and a source slice taken with an offset-0 span over the whole-file text reads
the wrong bytes.

Synthetic identities follow the same rule: `@dynns@` / `@dynclass@` /
`@autoname@` names embed the offset at which they were minted, so they are
minted through a recorded helper and rewritten by the body's offset delta at
graft. That keeps the memoised fragment offset-invariant while still matching
the names a whole-file walk would produce.

## Cross-item facts live in the aggregate, not the body

An isolated body cannot see the file. Rather than falling back whenever a body
touches file-wide state, the body **captures** what it would need and the tail
resolves it once the file's facts are merged:

- **Qualified reads.** A `$::g` read that misses the isolated body's empty
  global scope is recorded and replayed against the shell's real global at
  graft, gated on source-order visibility — a whole-file walk visits a proc body
  before a later top-level `set ::ns::v`, so it records no reference there
  either.
- **W002** (command disabled in this dialect) — its user-proc-shadowing
  suppression reads the file's whole `all_procs`, so the body captures
  would-be-W002 sites and the tail re-applies the shadow check against the
  merged set.
- **W304** (missing `--`) — only its `$var` branch is source-dependent (it scans
  for the most recent literal `set`), so that branch is deferred to the tail
  where the full source is available; every other branch stays inline.
- **W120** (missing `package require`) anchors at each command's *source-earliest*
  invocation rather than the first in walk order, which makes it independent of
  whole-file-DFS versus per-item shell order.
- **W103/W300 `$var` classification**, widget-dispatch sites, constant-dispatch
  sites, instance-creation sites, scoped command regions, and the alias / rename
  / deletion tables are all carried on the body fragment and rebased at graft;
  instance creations are replayed in source order against only the classes whose
  definition precedes the site.

## Fallbacks: correct unconditionally, cheap usually

Where the decomposition cannot be proven equivalent, the analyser falls back to
the whole-file walk. That is always correct, but not free — it re-walks the
document on **every keystroke**, so the per-body memo above it is dead weight for
that file. `Analyser::took_fast_path` answers "did we pay for a whole-file
walk?"; `Analyser::per_item_fallback` answers "why?", which is the actionable
question. It is `None` on the fast path, otherwise one of
[`PerItemFallback`](../../../rust/tcl-compiler/src/analyser/per_item.rs)'s
variants, ordered by where the guard sits in the pass:

| Variant | Guard |
| --- | --- |
| `IncompleteScript` | unbalanced braces/quotes — the transient mid-typing state |
| `StubDirective` | an inline `tcl-lsp: stub` overlay |
| `SidecarStub` | a nearest `<dialect>.tcl.stubs` sidecar, whose signatures must reach every isolated body |
| `TkActive` | Tk checks accumulate whole-file widget/geometry state — the `tk` dialect at entry, or a walk-recorded `package require Tk` |
| `GhostRecovery` | ghost-token error recovery engaged |
| `PartialCommand` | an unterminated command survived segmentation |
| `ErrorDiagnostic` | an `E…` code — `analyse` ran recovery machinery |
| `OversizedBody` | a deferred body exceeds `OVERSIZED_BODY_BYTES` |
| `DuplicateMethod` | one method qualified name defined twice |
| `EnclosingContext` | a body links a qualified sub-namespace variable, or `namespace import`/`export`s from inside a body |
| `DuplicateProcInBody` | a body defines an already-defined proc |
| `ClassFactsCollide` | a body extends a class whose facts already exist |
| `MethodInstanceReplay` | a method body's object-instance tracking cannot be replayed |

`rust/tcl-compiler/examples/per_item_fallbacks.rs` sweeps a corpus (`tmp/`, or
`ROOT=<dir>`) and reports the distribution weighted three ways — by document, by
source line, and by measured milliseconds. They rank the guards differently: a
guard that fires on a few very large documents is rare by count and dominant by
time, and time is what the user feels. `COMPARE=1` times both paths per
document; `TK_AUDIT=1` audits the Tk guard specifically. Note that
`IncompleteScript` never fires on documents at rest but fires constantly *while
typing*, so a live session's fallback rate is worse than any at-rest figure.

### Tk activation is a registry fact, not a substring scan

Tk activation has exactly two inputs, and both are taken from where the truth
lives: the `tk` dialect, decidable at entry so a `wish` document short-circuits
before any work; and a `package require Tk`, a whole-file fact recorded during
the walk by the registry's `PackageRequire` analyser hook — the very fact the Tk
geometry diagnostics gate on, so the two cannot disagree. A `-exact` flag, a
version constraint, line continuations, and a `package require` inside a
`namespace eval`, an `if`, or a proc body all fall out of the ordinary command
walk rather than needing a bespoke scanner.

Because the second input is only known after the walk, the per-item entry point
checks it twice: after the shell pass (which catches the top-level `package
require Tk` of essentially every real Tk script, before the body pass is paid
for) and after the body pass (which makes it complete, since the graft merges
the requires a body contributed). A genuinely-Tk document therefore pays one
discarded per-item pass — the price of never paying a whole-file re-analysis, on
every keystroke, for a document that merely *mentions* the word `Tk`.

What remains a substring test is `tk_checks_could_apply`, and only as a
performance precheck for per-command accumulation: a sound *necessary* condition
that may over-approximate freely, because everything the walk buffers is
discarded unless the exact activation fact holds. The per-item path pins it to
`false` — it accumulates no Tk state at all.

Two conservative limits are deliberate and match the whole-file walk exactly, so
the two paths still agree byte for byte: a dynamic package name (`package
require $p`) is recorded verbatim and never matches, and a `::package require
Tk` does not activate because hook resolution refuses a `::`-qualified spelling
of a bareword global command.

### The oversized-body guard

`fill_deferred_bodies` hands off with `OversizedBody` when any deferred body
exceeds `OVERSIZED_BODY_BYTES` (256 KiB), checked before any body is analysed so
an oversized document pays only the shell pass that discovered it.

The reason is a scaling cliff, not a correctness issue: isolated analysis of a
very large body is roughly 7× more expensive than the identical content analysed
in place, so on a generated single-body file the incremental path can cost
tens of times the plain whole-file walk *per keystroke*. 256 KiB is far above
any hand-written procedure (~6,000 lines of ordinary Tcl); it exists to catch
generated files, which are the only place bodies that size occur. The guard is
**per body, not per document** — a large file made of many ordinary procedures
is exactly the case per-body memoisation pays off for, and stays on the fast
path.

More broadly, the decomposition alone is close to break-even on a cold analysis:
over a sampled corpus the per-item walk is *slower* than the whole-file walk on
roughly a quarter of documents (mean ratio 1.08). Its value is entirely the
memoisation layered on top — a warm one-character body edit rebuilds one
procedure of forty — so the cold-path overhead is the price of that option, and
a document where the memo cannot pay off should not take the path at all.

## Cancellation

The per-item walk checks `db.unwind_if_cancelled()` at command boundaries, so a
new edit cancels an in-flight analysis promptly. That is what lets diagnostics
run on the shared query graph at all: without a cancellation checkpoint, a read
handle held across a whole-file analysis blocks the next edit's write (salsa's
`set_text` takes global write exclusivity) and stalls every other reader. See
[`lsp-performance.md`](lsp-performance.md).

## The correctness contract

Every layer must produce **byte-identical** `AnalysisResult` and diagnostics to
the whole-file walk. Item-locality is a *performance* heuristic; correctness is
unconditional, and rests on the gates rather than on the heuristic:

- **Corpus differentials.** `per_item_corpus` (per-item walk vs `analyse`),
  `file_analysis_corpus` (`file_analysis_incremental` vs `analyse`), and
  `compiler_check_corpus` (memoised vs uncached compiler checks) run over the
  `tmp/` corpus; a non-`#[ignore]`d `samples/**/*.tcl` slice runs in every
  `cargo test` so the gate cannot rot silently.
- **Edit fuzzers.** `per_item_matches_analyse_under_edits` and
  `differential_incremental` assert `incremental == fresh` under random edits —
  the property a static corpus cannot check — plus a corpus-scale multi-file
  fuzzer for the cross-file cascade.
- **Db-level reuse tests.** Each memo has a test proving it re-executes exactly
  once for the edited item and zero times for the rest (for example
  `method_body_edit_recomputes_one_item`,
  `function_optimisations_reused_on_unrelated_edit`,
  `function_lattice_reused_on_whole_file_shift`).
- **e2e parity** via the native `lsp_e2e` suite.

**Determinism is part of byte-identity.** A memoised build and a fresh
whole-module build can only be compared if each is stable run to run, so every
place a diagnostic's content depended on `HashMap` iteration order is fixed to a
total order: shimmer phi spans take the earliest incoming def, `run_all_checks`
sorts on `(span, code, category, severity, message, replacement)`, the optimiser
canonicalises before overlap arbitration and renumbers groups by first
appearance, the destructive-file check names path variables in argument order,
and type-join folds a sorted predecessor list. Memo **keys** need the same care:
the module `CfgContext` inserts both short and qualified procedure names, so
`prepare_cfg_context` iterates procedures in sorted qualified-name order —
otherwise two procedures sharing a short name race, the key flakes, and an edit
anywhere above a procedure misses the cache for *every* procedure.

## Where the per-edit cost is now

On a fast-path file the per-procedure lattice compute is memoised, so the
remaining cost is the non-memoised whole-module work in the `CompilationUnit`
build: whole-file IR lowering, the module CFG build, call-site constant
collection, and the interprocedural summary. On a 7.4 kLOC, 177-function file
that is roughly `lower_to_ir` ~26 ms + module `build_cfg` ~10 ms +
`with_interprocedural` ~19 ms, against ~23 ms for `run_all_checks` and ~13 ms for
the optimiser passes. The analyser tail's *emission* is ~6 ms once it consumes
the memoised unit — making emission incremental buys almost nothing; the build
is the floor.

Two structural properties bound how much further this goes:

- **Lowering is not body-local.** `lower_to_ir_with_config` runs whole-module
  passes whose output for one body depends on the others (factory
  specialisation, `uplevel` passthrough inlining, OO method extraction, and the
  trace-fact scan that sets module-wide traced-command state GVN reads). The
  per-item lowering memo therefore keys on those module-wide facts, and
  `lowering::body_cache_eligible` conservatively disqualifies any body
  mentioning `namespace` / `interp` / `rename` / `method` / `when` / `apply` /
  `alias` / `proc` or an OO or `itcl` marker, with a file-level precondition
  (`source_may_alias_commands`) on top, because an alias declared outside any
  body would resolve differently under isolated lowering. Widening that gate
  recovers more warm-edit reuse and is the main remaining lever.
- **A per-procedure optimiser memo is bounded by the lattice memo's hit rate.**
  It is byte-identical to the whole-unit run (proven over the corpus) and it
  hits on the common benign edit, because a body edit leaves the interprocedural
  summary byte-identical essentially always. But an edit that shifts every
  procedure misses the lattice memo, and then the per-procedure loop misses too;
  the single-function view must not clone the whole interprocedural summary per
  procedure, or that case is quadratic in the file.

## Running the experiments

- **Cost split, item-locality, offset-invariance, lattice costs, shared-unit
  saving:** `cargo run --release -p tcl-compiler --example incr_experiments`
  (reads the `tmp/` corpus).
- **Salsa early cutoff and cascade breadth:** `cargo test -p tcl-lsp-db --test
  early_cutoff`.
- **`incremental == fresh` differential:** `cargo test -p tcl-compiler --test
  differential_incremental -- --ignored` (corpus-gated, slow).
- **Per-edit phase timing:** `cargo run --release -p tcl-lsp-db --example
  tail_profile FILE=…` (warm db, single-character body edit).
- **Fallback distribution:** `cargo run --release -p tcl-compiler --example
  per_item_fallbacks` (`ROOT=` picks the corpus).
