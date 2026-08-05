# Incremental analysis — per-item walk with cascade invalidation

> Companions: the full experiment lab notebook (corpus, every experiment,
> discoveries, and the reasoning that produced this plan) is in
> [`incremental-analysis-experiments.md`](incremental-analysis-experiments.md);
> the shipped perf work and measurements are in
> [`lsp-performance.md`](lsp-performance.md); the runtime model in
> [`current-architecture.md`](current-architecture.md).

> **Phase-0 experiment findings** (`rust/tcl-compiler/examples/incr_experiments.rs`,
> over tcl8.6 + tcllib corpus):
> - **E3 — GO.** On practcl.tcl the per-proc *walk* is ~82% of `analyse`
>   (~785 ms) and the cross-item *tail* (W123/arity/interproc) is ~18%
>   (~167 ms). Per-item recompute attacks the dominant 82%; the unavoidable
>   per-edit aggregate floor is ~167 ms (itself incrementalisable later).
> - **E1 — 98.9% item-local.** Inserting a benign comment in the last proc
>   leaves every *preceding* item byte-identical in 345/349 files. The **4
>   exceptions are all global-variable diagnostics** (W210 read-before-set /
>   W211 set-but-unused) that span procs — a real, narrow cross-item class.
> - **E2 — GO (100%).** Prepending K blank lines and un-shifting every fact by
>   K reproduces the original `AnalysisResult` for 598/598 corpus files — the
>   analyser is fully offset-shift-invariant, so per-item facts produced at
>   offset 0 rebase exactly. (Validates the slice-2 relative-offset model.)
> - **E6 — GO.** On practcl: CompilationUnit build ~118 ms / 115 procs ≈ 1 ms
>   per proc; the interproc `pure`/effects fixpoint is only ~10.6 ms; per-proc
>   lattice recompute + interproc cascade is cheap.
> - **E7 — GO.** `optimise_with_dialect` rebuilds its own CU+interproc (~129 ms
>   redundant); sharing one CompilationUnit with `run_all_checks` recovers it.
> - **E4/E8 — GO.** A salsa event-counter prototype (`tcl-lsp-db/tests/
>   early_cutoff.rs`) confirms the cascade mechanism: a *body* edit re-executes
>   exactly **one** `item_analysis` (the unchanged sibling is reused); a
>   *signature* edit re-executes the cross-item aggregate but **zero** item
>   bodies (field-level deps + the caller→callee-sig edge); and a body change
>   that leaves the item's *output* unchanged cuts off the aggregate
>   (tracked-output early cutoff). **Finding:** salsa input setters always bump
>   the revision (no value-equality cutoff on inputs), so the per-item build
>   must set an item's body input **only when the item-tree diff says it
>   changed** — otherwise every keystroke re-runs direct dependents.
> - **E5 — FOUND A GAP (the fuzzer works).** The differential fuzzer
>   (`tests/differential_incremental.rs`, `incremental == fresh` under random
>   edits) shows `analyse_incremental` diverges from `analyse`. Root cause: even
>   *unedited*, `analyse(src) != analyse_commands(src, segment(src), true)` —
>   **`all_procs` match, but the diagnostic sets differ**. So the existing
>   pre-segmented entry point is not diagnostic-equivalent to `analyse`. The
>   structural analysis is already incremental-sound; the per-item build's job is
>   to make **diagnostic emission byte-identical to `analyse`**, with the fuzzer
>   as the gate and the full-rebuild fallback for anything it can't match.
> - **Implication / decision.** The per-item firewall is sound for ~99% of
>   edits, but **global-variable usage is a cross-item fact** (W210/W211 on
>   globals span procs). **Decision: detect cross-proc global-var flow and fall
>   back to a full rebuild for those files** (~1%) rather than model a
>   `global_var_usage` aggregate — simpler, and correctness is unconditional via
>   the fallback. Item-locality is a *perf* heuristic; the correctness contract
>   is the **fuzzer + full-rebuild fallback** below.

> Design for making the Tcl analyser incremental at *item* granularity, so a
> keystroke inside one proc recomputes that proc, not the whole file. Grounded
> in two data-flow audits (analyser walk; IR/lattice pipeline). Companion to
> [`target-architecture.md`](target-architecture.md) (Layer 4, the cascade).

## Why

`Analyser::analyse` is linear (~0.14 ms/line) but whole-file: an 8.5k-line file
costs ~1 s per edit. Measured split per edit on practcl.tcl:

| stage | cost | granularity today |
|---|--:|---|
| lex + green tree + segment | ~3 ms | whole-file (cheap) |
| **analyser walk (scope trees + diagnostics)** | **~1015 ms** | **whole-file** |
| CompilationUnit + `run_all_checks` | ~146 ms | per-`FunctionUnit`, built whole-file |
| `optimise_with_dialect` (2nd CompilationUnit) | ~141 ms | per-`FunctionUnit`, built whole-file |

The walk dominates and `analyse_incremental` does **not** help — it reuses
segmentation (3 ms) but re-walks every command. Parsing is already negligible.
The lever is making the walk and the lattices recompute **only the edited item**.

## The firewall: signatures vs bodies

The audits show the split that makes this tractable:

- A **proc/method body** is **item-local**: its `param_traits`, local command
  invocations, local diagnostics, and scope subtree are a pure function of
  *(body text, params, enclosing namespace, registry, dialect, stub overlay)* —
  nothing from sibling bodies (analyser audit §3B, §4).
- The **cross-item facts are signatures**: name resolution and the W123
  unresolved-command / arity passes read the file's *set* of
  `all_procs ∪ all_classes ∪ command_aliases ∪ ensemble_namespaces` and the
  namespace tree — i.e. item **headers**, not bodies (analyser audit §3A, §3C).
- The **lattices** are already per-`FunctionUnit` (SSA→def-use→SCCP→type→
  rendered→taint, GVN); the only genuine cross-item cascade is the
  interprocedural `pure`/effect fixpoint and the taint re-run that consumes it
  (lattice audit §4).

So the query graph keys **bodies** (expensive, item-local) separately from
**signatures** (cheap, cross-item). A whitespace or in-body edit changes one
body and leaves every signature — and therefore every other item's analysis,
the resolution table, and the interproc summary — untouched.

## Query graph

```
input    source(file), dialect(file), config(global), registry(static)
derived  tokens(file)        ← source                      # ~3ms, whole-file
derived  item_tree(file)     ← tokens                      # FIREWALL
derived  item_sig(item)      ← item_tree(file)             # header: name, params,
                                                           #   namespace, span, kind
derived  file_decls(file)    ← item_sig(item)*             # all_procs/classes/
                                                           #   aliases/ensembles + ns tree
derived  item_body(item)     ← item_tree(file)             # body token slice (rel offsets)
derived  item_analysis(item) ← item_body(item), file_decls(file), registry, config
                                                           # the expensive per-line walk,
                                                           #   bounded to one item; emits
                                                           #   RELATIVE-offset facts
derived  file_analysis(file) ← item_analysis(item)* , file_decls(file)
                                                           # rebase rel→abs, aggregate,
                                                           #   run W123 / arity over the union
# lattices (per FunctionUnit), cascade layer:
derived  cfg(item)           ← item_body(item)
derived  ssa/sccp/type/rendered/taint_intra(item) ← cfg(item), registry
derived  interproc(file)     ← item_sig(item)*, call_graph(file)   # pure/effects fixpoint
derived  taint(item)         ← taint_intra(item), interproc(file)  # cross-item re-run
derived  diagnostics(file)   ← file_analysis(file), checks(item)*, optimiser(item)*
```

**Early cutoff is the point.** `item_tree`/`item_sig` are keyed on *structure*,
so a body-only edit produces equal sigs → `file_decls`, `interproc`, and every
other `item_analysis` are reused; only the edited `item_analysis` + its lattices
recompute. A signature edit (rename/param change) changes `item_sig` → cascades
to `file_decls` (re-resolve call sites, re-run W123) and `interproc` (re-run
taint on callers) — exactly the dependents, nothing more.

## The load-bearing refactor: relative-offset item analysis

`item_analysis(item)` must be memoisable by *body text*, so an unmoved-but-
shifted proc (lines inserted above it) is a cache hit. Therefore the per-item
walk must emit **offsets relative to the item's start**, and `file_analysis`
**rebases** them to absolute positions using `item_sig.span.start`. This mirrors
rust-analyzer's `ItemTree` (position-independent) + per-body lowering.

Concretely (analyser audit §4), `analyse_commands` (`state.rs:773`) and the
`process_command` dispatch must stop mutating `self.result`/`self.const_strings`
directly and instead take the cross-item context as **inputs** and return the
item's facts as **outputs**:

- inputs: `body_text`, `params`, `namespace`, `&file_decls`, `registry`, `config`
- outputs: `param_traits`, `local_invocations` (rel spans), `local_diagnostics`
  (rel spans), `scope_subtree`, `const_strings`, `var_reads`

The order-sensitive file-global facts the audit flagged — `command_aliases`,
`ensemble_namespaces`, the namespace tree — are **top-level** concerns, so they
live in `file_decls` (computed from `item_sig`s in source order), not inside
body analysis. Bodies read them read-only.

## Cancellation

For diagnostics to rejoin the shared salsa graph (instead of the current direct
`Analyser::analyse` the server runs detached to dodge write contention), the
per-item walk must check `db.unwind_if_cancelled()` at command boundaries so a
new edit cancels an in-flight analysis quickly — the property
`Analyser::analyse` lacks today, which forced the decoupling in
`run_diagnostics_core`.

## Correctness guard

Every step must produce **byte-identical** `AnalysisResult` / diagnostics to the
current whole-file walk. Guards, in order:
1. the differential corpus (`rust/tcl-compiler/tests/differential_*`) and the
   analyser unit tests — pin exact diagnostics/spans;
2. `make test-lsp-e2e-rust` — observable parity;
3. a new property test: for random edits, `incremental == fresh` (the existing
   `test_edit_tracking_stress_e2e` already asserts this for tokens/symbols;
   extend to diagnostics).
When incremental can't prove equivalence (error recovery, stub overlays — the
existing `analyse_incremental` guard), fall back to a full walk.

### Fallback telemetry: which guard fired

A fallback is correct but not free — it re-walks the whole document on **every
keystroke**, so the per-body memoisation above it is dead weight for that file.
`Analyser::took_fast_path` answers "did we pay for a whole-file walk?", which is
the latency question; `Analyser::per_item_fallback` answers "why?", which is the
only actionable one. It is `None` on the fast path, otherwise one of
[`PerItemFallback`](../../../rust/tcl-compiler/src/analyser/per_item.rs)'s
variants, ordered by where the guard sits in the pass:

| Variant | Guard |
| --- | --- |
| `IncompleteScript` | unbalanced braces/quotes — the transient mid-typing state |
| `StubDirective` | an inline `tcl-lsp: stub` overlay |
| `TkActive` | Tk checks accumulate whole-file widget/geometry state — the `tk` dialect at entry, or a walk-recorded `package require Tk` (#1188) |
| `GhostRecovery` | ghost-token error recovery engaged |
| `PartialCommand` | an unterminated command survived segmentation |
| `ErrorDiagnostic` | an `E…` code — `analyse` ran recovery machinery |
| `OversizedBody` | a deferred body exceeds `OVERSIZED_BODY_BYTES` — the isolated-body path is a pessimisation there (#1188) |
| `DuplicateMethod` | one method qualified name defined twice |
| `EnclosingContext` | a body links a qualified sub-namespace variable, or `namespace import`/`export`s from inside a body |
| `DuplicateProcInBody` | a body defines an already-defined proc |
| `ClassFactsCollide` | a body extends a class whose facts already exist |
| `MethodInstanceReplay` | a method body's object-instance tracking cannot be replayed |

`rust/tcl-compiler/examples/per_item_fallbacks.rs` sweeps a corpus (`tmp/`, or
`ROOT=<dir>`) and reports the distribution weighted three ways — by document, by
source line, and by measured milliseconds. They rank the guards differently: a
guard that fires on a few very large documents is rare by count and dominant by
time, and time is what the user feels. `COMPARE=1` additionally times both paths
per document, and `TK_AUDIT=1` audits the `TkActive` guard specifically.

Measured over tcllib + Tk (1026 documents, 512k lines): **20.5% of documents
fall back, accounting for 46.8% of source lines and 52.3% of analysis time**;
mean 19.1 ms per document on the fast path against 81.4 ms on the fallback. The
three dominant guards by time are `TkActive` (25.4%), `ErrorDiagnostic` (13.4%)
and `EnclosingContext` (11.1%). Note that `IncompleteScript` never fires on
documents at rest but fires constantly *while typing*, so the live rate is worse
than this at-rest figure.

Two findings from that sweep drove the #1188 work below, and they had to be
resolved *together* — fixing either alone makes things worse:

- **`TkActive` was ~78% false positives.** The guard was three *independent*
  substring tests (`package`, `require`, `Tk`, anywhere, comments included). 64
  documents tripped it, 14 genuinely `package require Tk`, and the other 50
  emitted no Tk diagnostic at all — so tightening it to a real `package require
  Tk` is behaviour-preserving on this corpus.
- **…but tightening it exposes a scaling cliff.** On `fumagic/filetypes.tcl`
  (85k generated lines, one 71k-line body) the incremental path costs 139,010 ms
  against 5,312 ms for the plain whole-file walk — 26x slower, per keystroke.
  The decomposition is not the cause: the same body extracted and analysed as a
  standalone script takes 36,634 ms against 5,312 ms for the whole document
  containing it, so isolated analysis of a large body is ~7x more expensive than
  the identical content analysed in place.

### Resolving both: registry-driven Tk activation + an oversized-body guard (#1188)

**The gate is no longer a substring scan.** Tk activation has exactly two
inputs, and they are now both taken from where the truth actually lives:

- the `tk` dialect — decidable at entry, so a `wish` document short-circuits
  before any work; and
- a `package require Tk` — a *whole-file* fact, recorded during the walk by the
  registry's `AnalyserHookId::PackageRequire` hook, which is the very fact
  `flush_tk_geometry_diagnostics` gates the TK diagnostics on. The two therefore
  cannot disagree, and a `-exact` flag, a version constraint, line
  continuations, and `package require` inside a `namespace eval`, an `if`, or a
  proc body all fall out of the ordinary command walk rather than needing a
  bespoke scanner.

Because the second input is only known after the walk, `analyse_per_item_with`
checks it twice — once after the shell pass (which catches the top-level
`package require Tk` of essentially every real Tk script, before the body pass
is paid for) and once after the body pass (which makes it complete, since
`graft_proc_body` merges the requires a body contributed). A genuinely-Tk
document therefore pays one discarded per-item pass; that is the price of never
paying a whole-file re-analysis, on every keystroke, for a document that merely
*mentions* the word `Tk`.

What remains a substring test is `tk_checks_could_apply`, and only as a
performance precheck for the per-command accumulation: a sound *necessary*
condition (`dialect == "tk" || source.contains("Tk")`) that can over-approximate
freely, because everything the walk buffers is discarded unless the exact
activation fact holds. The per-item path pins it to `false` outright — it
accumulates no Tk state at all, since a Tk document's result is thrown away for
a full re-analysis anyway.

**Documented conservative limits.** A dynamic package name (`package require
$p`) is recorded verbatim and so never matches; a `package` reached after a
`rename`, or hidden in a safe interpreter, is not a resolvable `package
require`. Both leave the checks off — matching the whole-file walk exactly, so
per-item and full analysis still agree byte for byte. Separately,
`::package require Tk` does **not** activate, because `resolve_analyser_hook_call`
deliberately refuses a `::`-qualified spelling of a bareword global command
(pinned by issue #923). That is a pre-existing false negative of the whole-file
walk — `analyse` emits no TK diagnostic there either — not something this gate
introduced; lifting the `::`-bareword guard would fix it, but it widens hook
dispatch for *every* stamped command and belongs to its own change.

**The cliff is guarded by body size, not by Tk.** `fill_deferred_bodies` hands
off with `OversizedBody` when any deferred body exceeds `OVERSIZED_BODY_BYTES`
(256 KiB), checked before any body is analysed so an oversized document pays
only the shell pass that discovered it. 256 KiB is far above any hand-written
procedure (~6,000 lines of ordinary Tcl); it exists to catch *generated*
single-body files, which are the only place bodies that size occur and the only
place the isolated-body path is a dramatic pessimisation. The guard is **per
body, not per document**: a large file made of many ordinary procs is exactly
the case per-body memoisation pays off for and stays on the fast path.

Two of tcllib 2.0's 882 documents exceed it, both generated:
`fumagic/filetypes.tcl` (one ~1.2 MB body) and the `i.map` variant of
`textutil/wcswidth.tcl` (34,856 lines in three procedures, two of them ~2.2 MB).
`wcswidth.tcl` did previously take the fast path, and its cold cost is roughly
unchanged (999 ms for the whole-file walk, 1,153 ms through the guard) — but a
*warm* edit there would have re-analysed a 2.2 MB body in isolation, half the
document at the ~7x isolated-body penalty, so removing it from the incremental
path is the right call for the case the path exists to serve.

**Measured, tcllib 2.0 (882 documents, 460,103 lines, `tcl8.6`):**

| guard | before: files / lines / ms | after: files / lines / ms |
|---|---|---|
| `tk-active` | 33 / 127,247 / 4,908 | 9 / 4,160 / 343 |
| `oversized-body` | — | 2 / 119,897 / 5,135 |
| `error-diagnostic` | 35 / 27,116 / 2,614 | 41 / 43,424 / 4,621 |
| `enclosing-context` | 41 / 38,226 / 1,941 | 45 / 48,495 / 2,814 |

`tk-active` drops from 27.7% of corpus lines to 0.9%: 24 of the 33 documents it
claimed were false positives, `filetypes.tcl` and `practcl.tcl` among them.
Neither reports `tk-active` any more — `filetypes.tcl` is caught by the
oversized-body guard instead (`analyse` 3,427 ms vs 3,219 ms through the guard,
so the guard costs nothing over the plain walk), and `practcl.tcl` turns out to
trip `error-diagnostic`, which the Tk false positive had been masking.

That masking is the honest cost of the change: a document whose *real* gate sits
after the walk now pays a per-item pass before reaching it, where the entry-time
Tk guess used to short-circuit. On `practcl.tcl` that is 883 ms against 345 ms
for the whole-file walk. The corpus total moves 21,413 ms → 25,790 ms for the
same reason. This is not a regression the Tk fix introduced so much as one it
*revealed* — `error-diagnostic` and `enclosing-context` firing only after the
full decomposition is a pre-existing property of those guards, and hoisting them
is the natural follow-up.

More broadly, the decomposition alone is close to break-even — over 210 sampled
fast-path documents `analyse_per_item` is *slower* than `analyse` on 23% of
them, mean ratio 1.08. Its value is the memoisation layered on top (a warm
one-character body edit rebuilds 1 procedure of 40), so the cold-path overhead
is the price of that option, and a document where the memo cannot pay off should
not take the path at all.

## Staging (each slice behaviour-preserving, differential-guarded)

1. **`item_tree` + `item_sig` + `file_decls`** as salsa queries — additive,
   no behaviour change; assert `file_decls` matches the current decl scan.
2. **Relative-offset item body analysis** — refactor `analyse_commands` to the
   pure inputs→outputs shape above, returning rel-offset facts; `file_analysis`
   rebases + aggregates. Prove byte-identical to the whole-file walk on the
   corpus. (This is the bulk.)
3. **Memoise `item_analysis`** in salsa keyed on `item_body` — the perf win:
   body edit ⇒ one item recomputes. Measure heavy-edit drop.
4. **Per-item lattices + `interproc` cascade** — key SSA/SCCP/type/taint per
   item; `interproc` fixpoint over `item_sig`s; taint re-run only on affected.
5. **Cancellation-aware walk** — `unwind_if_cancelled` at command boundaries;
   move diagnostics back onto the shared query graph (retire the direct-analyse
   detour) and delete the residual `document_analysis_gate`.

## Verification

`cargo test --workspace` (incl. differential corpus), `make test-lsp-e2e-rust`,
and `scripts/dev/bench_lsp_backends.py` heavy-edit (target: single-proc edit on
practcl.tcl from ~1 s to low-ms).

## Implementation status

| Piece | State |
|---|---|
| salsa query DB (`tcl-lsp-db`): inputs + `file_analysis`/`document_symbols`/`semantic_tokens`/`folding_ranges` | **shipped** |
| Server caches → queries (`analyses`/`hover`/`semantic_tokens`/`dialect_registries` deleted) | **shipped** |
| Async + debounced diagnostics (heavy-edit fix) | **shipped** |
| Shared CompilationUnit (E7) — `optimiser::optimise_unit` | **shipped** |
| Phase-0 experiments E1–E8 + differential fuzzer + cascade prototype | **shipped** (this doc's findings) |
| Slice 1 — `item_tree`/`item_sig`/`file_decls` | **shipped** (corpus-gated) |
| Slice 2 — per-item walk via deferred bodies (`analyse_per_item`); `incremental == fresh` fuzzer to 0 | **shipped** (byte-identical over corpus) |
| Slice 3 — memoise per-body analysis (`item_body_analysis`); offset-invariant keys | **shipped** (firewall + offset-invariance tests) |
| Slice 4 — per-procedure lattice memoisation (`build_for_memoized` + `lattice_rebase`), offset-invariant | **shipped** (byte-identical; e2e parity) |
| Slice 5 — diagnostics on the cancellable salsa graph (`file_analysis_incremental`), coalescing per-URI worker (CPU-stress-robust), `document_analysis_gate` retired | **shipped** (e2e parity; edit-storm stress green) |
| Slice 6 — `semantic_tokens` / `project_class_index` / `project_proc_var_index` moved onto `file_analysis_incremental` (were still on the coarse, uncancellable `file_analysis`, so a token or index request could be starved behind a whole-file walk a diagnostics run had already been made cancellable for); server-side fast-path race (enriched vs. a 40 ms budget timer) + `workspace/semanticTokens/refresh` background delivery so a cold large file still serves promptly (#829, see [`lsp-performance.md`](lsp-performance.md) §7) | **shipped** (unit + db event-log tests proving zero duplicate `item_body_analysis`; native lsp_e2e; direct-infra + LSP-API stress suites in `scripts/stress/`) |
| Salsa-native per-procedure lattice graph (`function_lattice` query interning each proc's offset-0 body + `CfgContext`); both consumers (analyser tail + optimiser via `compiler_check_diagnostics`) on salsa; process-wide content cache retired | **shipped** (byte-identical; offset-invariant firewall + e2e parity) |
| Per-item walk Phase B — widen the fast path: qualified `$::g` reads captured + replayed on the shell's globals at graft; the enclosing-context fallback narrowed from "any namespace/global/`variable`/qualified-read body" to **duplicate definitions** (top-level + nested self-redefinition) and **class-defining / qualified-writer bodies** | **shipped** (≈42% of corpus files now take the incremental fast path, up from the qualified-read-trigger floor; byte-identical — corpus + fuzzer gated) |
| Per-item walk Phase C — widen the fast path to the enclosing-variable class: the three residual divergences made byte-identical (W120 anchored at the source-earliest invocation; W002 disabled-command shadow check deferred to the tail over the merged `all_procs`; W304 `$var` resolution deferred to the tail over the full source); qualified-read replay gated on source-order visibility; `body_needs_enclosing_context` narrowed to just `variable ns::x`; post-walk `E…` syntax-error backstop added (mirrors `analyse_incremental`) | **shipped** (≈42%→**86.6%** of corpus files take the fast path; byte-identical — `per_item_corpus` *and* the new `per_item_matches_analyse_under_edits` edit fuzzer + `differential_incremental` + e2e gated) |

> **Phase B — what the fast path now covers, and what still falls back.**
> The per-item walk's enclosing-context fallback (`body_needs_enclosing_context`)
> used to fire on *any* body touching namespace/global/`variable` state or a
> qualified `$::g` read — i.e. almost every non-trivial file.  Phase B:
>
> - **Captures + replays qualified reads.** A body's `$::g` read is recorded
>   (`Analyser::capture_global_reads`) when it misses the isolated body's empty
>   global scope, then replayed on the shell's real `::g` at graft — so a body
>   that only *reads* enclosing globals no longer needs the fallback.
> - **Narrows the fallback** to the two patterns the isolated-body decomposition
>   genuinely can't reproduce byte-for-byte: (1) **duplicate definitions** — the
>   same proc/method qualified name defined twice (platform-conditional `proc`s
>   *and* the lazy-init self-redefinition `proc p {a} { …; proc p {a} {real} }`,
>   detected by tracking every defined proc name across the shell + body
>   fragments); (2) **class-defining or qualified-writer bodies** (`oo::class` /
>   `oo::define`, or `set ::ns::x`) — whose `all_variables` / class-method facts
>   accumulate across bodies in a whole-file walk but only *merge* in the graft.
>
> Result: ~42% of corpus files take the incremental fast path (a body edit
> recomputes one body + shell + tail).  The remaining ~58% — those that
> **declare/write** enclosing variables (`variable` / `global` / `set ::x`) or
> define classes in a body — still fell back (addressed in **Phase C** below).

> **Phase C — the enclosing-variable / class class joins the fast path
> (shipped).**  A probe (`tcl-compiler/examples/per_item_divergence`,
> analyse-vs-per-item over the `tmp/` corpus with the enclosing-context fallback
> bypassed) showed the broad Phase-B fallback was ~98.6% unnecessary: bypassing
> it left only **three** diagnostic-divergence classes and two book-keeping nits,
> all narrow and reproducible.  Each was made byte-identical at the
> isolated-body / graft / tail seam — the rust-analyzer "cross-item facts live in
> the aggregate, not the body" split — so the fallback could be narrowed to
> almost nothing:
>
> - **W120** (missing `package require`) now anchors at each command's
>   *source-earliest* invocation instead of the first in walk order, making it
>   independent of whole-file-DFS vs per-item shell-order (it is emitted before
>   `canonicalize_result_order`).  Applies to both paths — no behaviour change for
>   `analyse` beyond determinism.
> - **W002** (disabled-in-dialect command) — its user-proc-shadowing suppression
>   reads the file's whole `all_procs`, which an isolated body lacks.  The body now
>   *captures* would-be-W002 sites (`pending_disabled_commands`) and the tail
>   re-applies the shadow check against the merged `all_procs`
>   (`flush_disabled_command_diagnostics`) — mirroring `pending_arity` /
>   `capture_global_reads`.
> - **W304** (missing `--`) — only its `$var` branch is source-dependent
>   (`last_literal_set_value_for_var` scans the *whole* source for the most-recent
>   literal `set`).  That branch is deferred (`pending_w304`) and classified in the
>   tail where `self.source` is the full file; every other W304 branch stays
>   inline.
> - **Qualified-read replay** is now gated on source-order visibility: a captured
>   `$::ns::v` is replayed only when its enclosing definition *precedes the body*
>   (a whole-file DFS walks a proc body before a later top-level `set ::ns::v`, so
>   it records no reference there).
> - **`body_needs_enclosing_context`** is narrowed from "any
>   `variable`/`global`/`set ::x`/class body" to just **`variable ns::x`** (a
>   `variable` linking to a *sub-namespace* variable — the lone residual whose
>   merged `warn_if_unused` / `definition_span` is walk-order-sensitive).  Plain
>   `variable x`, `global`, `upvar`, qualified writes, and class definitions all
>   take the fast path.
> - A post-walk **`E…` syntax-error backstop** (mirroring `analyse_incremental`)
>   re-analyses fully whenever the per-item result carries any error diagnostic —
>   locally-unbalanced braces that pass `script_is_complete` engage `analyse`'s
>   error-recovery machinery, which the per-item walk does not reproduce.
>
> Result: **86.6%** of corpus files take the fast path (up from ~42%), 100%
> byte-identical; large *non-duplicate* files (e.g. `pki.tcl`, 3.3 kLOC) drop from
> ~360 ms full-analyse to ~115 ms per warm edit.  Gated by the unedited
> `per_item_corpus` test **and** a new edit fuzzer
> (`per_item_matches_analyse_under_edits`, the `incremental == fresh` contract for
> the per-item walk under random edits — which the older `differential_incremental`
> does not cover, as it exercises `analyse_commands`-based `analyse_incremental`).
>
> **Where the remaining per-edit time actually goes (post-Phase-C profiling).**
> Phase-timed on the salsa graph (warm DB, single-char body edit), the picture
> that the original "memoise the tail" sub-tasks assumed turns out **not** to
> hold — the tail's *emission* is already cheap; the cost is the unit **build**:
>
> - **The analyser tail's `emit_cfg_ssa_diagnostics` emission is ~6 ms** once it
>   consumes the memoised `cu_override`; the 130 ms it costs on the *no-override*
>   path (`analyse_per_item`) is entirely the **`CompilationUnit` build** it does
>   for itself.  Every other tail emitter (W123 / arity / W120 / var-usage / W002
>   / W304) is ~0 ms.  So *making the emission incremental buys almost nothing.*
> - The dominant per-edit cost on a fast-path file is the **`CompilationUnit`
>   build** — `memoised_compilation_unit`, run once in *each* of the two tracked
>   queries (`file_analysis_incremental` + `compiler_check_diagnostics`).  On
>   `parse_lemon.tcl` (7.4 kLOC, 177 fns): `build_for_memoized` ≈ 80 ms +
>   `with_interprocedural` ≈ 19 ms, ×2 queries.  The per-function *lattices* are
>   memoised (cache hits across edits), so this 80 ms is the **non-memoised
>   whole-module work**: `lower_to_ir` + module `build_cfg` +
>   `collect_call_site_constants` + the 177 lattice clone/rebases — i.e. O(file)
>   lowering, the genuine floor.  `run_all_checks` (≈ 23 ms) + `optimise_unit`
>   (≈ 13 ms) are minor by comparison.
> - **Net:** fast-path large files already sit at the *target* — `parse_lemon`
>   ≈ 150 ms (`file_analysis_incremental`) + 141 ms (`compiler_check`); `pki.tcl`
>   (3.3 kLOC) ≈ 115 + 119 ms.  Driving them lower means attacking the
>   lowering/CFG floor (incremental lexing/lowering, or sharing one built unit
>   between the two queries when their `LexerConfig`s coincide — they differ only
>   for tcl8.4 / iRules) — a larger architecture step, *not* the diagnostic-tail
>   memoisation the original plan named.
>
> **Shared `CompilationUnit` build (shipped).** The unit was built twice per
> edit (once per diagnostics query).  A tracked `compilation_unit(file, cfg)`
> query keyed on an interned `LexerCfgKey` lets the analyser tail
> (`file_analysis_incremental`) and the optimiser/compiler-checks
> (`compiler_check_diagnostics`) **share one build per edit** whenever their
> lexer configs coincide — every dialect but `tcl8.4` / `f5-irules`.  Combined
> per-edit latency on `parse_lemon.tcl` (7.4 kLOC) drops ~287 ms → ~202 ms (one
> build eliminated).  `CompilationUnit` gained `PartialEq` so salsa returns
> `Arc<CompilationUnit>`; gated by `compilation_unit_shared_across_consumers`.

> **Duplicate procs + multi-method classes on the fast path (shipped).** The
> remaining fallback census (`duplicate` 86, `variable_ns` 44, `proc_collision`
> 12, `syntax_error` 11) was dominated by two avoidable causes:
>
> - **Multi-method classes were a false duplicate** — method deferred-bodies had
>   an empty `scope_name`, so the duplicate detector treated every method after
>   the first as a duplicate of `(is_method, "", "")`; *any* class with 2+
>   methods fell back.  Methods now carry their qualified name, so distinct
>   methods are distinct (only a genuinely twice-defined method falls back).
> - **Genuine proc duplicates** now graft byte-identically: a whole-file
>   `define_var` unions each definition's locals with last-definition-wins on
>   shared keys, reproduced by overwrite-on-collision for body-owned
>   `all_variables` keys while param keys keep the shell's real definition span
>   (taking only the last body's references).
>
> The cross-item facts an isolated/in-place body can't reproduce are handled
> precisely (not by a blanket fallback): **object instances** (`[Cls new]` /
> `Cls create v`) are captured in the isolated proc body and replayed against the
> shell's full `all_classes` at graft (no memo-key change), with a per-method
> snapshot fallback for in-place method bodies; **class extension** (`oo::define`)
> falls back only on an `all_classes` key collision.  Result: fast-path coverage
> **86.6% → 92.2%**, 100% byte-identical, 0 diagnostic divergence.
>
> **Still not won.**
> - **The `variable ns::x` (`variable_ns`) + `syntax_error` fallbacks** remain
>   (correct, narrow).
> - **The `CompilationUnit` per-edit floor is architecture-level.** Phase-timed
>   on `parse_lemon.tcl` (7.4 kLOC, 177 procs), the per-edit CU rebuild splits
>   into: `lower_to_ir` ~26 ms + module `build_cfg` ~10 ms (both O(file)) + the
>   **per-procedure loop ~52 ms** + `with_interprocedural` ~19 ms.  The
>   per-procedure 52 ms is *not* lattice compute — the lattices are memoised
>   (`function_lattice`) — it is the **offset-invariant plumbing run for every
>   procedure on every edit**: clone the proc's offset-0 IR body + `rebase_script`
>   it, then deep-clone the memoised `FunctionUnit` + `rebase_function_unit` it to
>   the proc's real offset.  Because the rebase target (the offset) changes
>   whenever an edit shifts a procedure, the rebased unit can't be cached across
>   edits, so this plumbing is irreducible without either **incremental
>   lowering/CFG** or making the diagnostic consumers **offset-aware** (consume
>   offset-0 units + a per-proc offset, no rebase) — both large refactors.
>   - **`function_lattice` barely engages for real files.** Instrumenting the
>     proc loop shows *every* procedure in init/pki/parse_lemon/textutil takes the
>     **fresh-build** branch — `params_constants_from_call_sites` returns `Some` for
>     nearly all of them, and the memo was keyed *without* `param_constants` so
>     those bypass it.  So the 52 ms is fresh SSA/SCCP/type/taint builds, and the
>     headline lattice memo only helps callee-free procs.
>   - **`param_constants` folded into `FnLatticeKey` (shipped).** The
>     caller-uniform-literal SCCP seeds (`param_constants` /
>     `params_constants_from_call_sites`) are now part of the memo key, so a
>     procedure with them engages the `function_lattice` memo instead of
>     bypassing it via the old `param_constants.is_none()` guard.  The seeds are
>     carried through `LatticeRequest` in a deterministic, hashable,
>     position-independent encoding (`(param, version, string)`, sorted —
>     `encode_param_constants` / `decode_param_constants`), with a defensive
>     fresh-build fallback if a seed is ever not a string const (the only shape
>     the producer emits).  `function_lattice` decodes the seeds and builds via
>     `FunctionUnit::build_with_param_constants`, so a procedure rebuilds its
>     lattice only when a caller's literal at that position changes (a new key)
>     and is an offset-invariant cache hit otherwise.  With the memo now engaging
>     for nearly all procs, clone+rebase is **~9× cheaper than fresh-build** (the
>     measured-in-isolation figures: pki 4 ms vs 38 ms; parse_lemon 6 ms vs
>     52 ms).  On the noisy CI box the aggregate BOTH-queries win is partly
>     masked by the O(file) lowering floor that now dominates, but the achievable
>     floor drops (pki BOTH min-of-N **~175 ms → ~153 ms**, matching the original
>     ~150 ms experiment).  Byte-identical — the new
>     `function_lattice_memoises_param_constant_procs` db test proves a
>     param-constant callee now executes a `function_lattice` query and is reused
>     across an unrelated edit, and the full corpus differential
>     (`compiler_check_memo_matches_uncached_over_corpus`) + `differential_incremental`
>     + `per_item_corpus` + e2e all stay green.  This was previously tried and
>     reverted only because it *extended* the memo byte-identity bug below to more
>     procedures; re-landed on top of that fix (#616, see "Memo byte-identity
>     (fixed)").
> - **`practcl.tcl`** still falls back on a genuine twice-defined method-style
>   definer and is OO-heavy.  **This is the per-item *analyser walk*'s fallback**
>   (`body_needs_enclosing_context` / duplicate-definition detection), *not* the
>   per-edit CU lowering cost — so backlog #3 Approach B (offset-aware CU
>   consumers) does **not** move it off the full-rebuild path.  Backlog #4 needs
>   either the duplicate-method-definer grafting (the analyser-walk analog of the
>   shipped duplicate-*proc* grafting: union the two definitions'
>   locals/method-facts with last-definition-wins, replayed at graft) or the
>   incremental-lowering work of Approach A — a separate effort from this slice.

> **Memo byte-identity (fixed).**  The salsa-native lattice graph (#604) promises
> that the memoised `build_for_memoized` (offset-0 per-procedure `function_lattice`
> + rebase) is byte-identical to a fresh whole-module `build_for_with_config`.  A
> corpus differential (`tcl-lsp-db/tests/compiler_check_corpus.rs`, comparing
> `compiler_check_diagnostics` vs `compiler_check_diagnostics_uncached`) found
> ~30% of files diverging.  The root cause was **nondeterminism, not an
> offset-0-vs-whole-module analysis difference** — both builds disagreed run-to-run
> with themselves.  Five `HashMap`-iteration-order dependencies, each fixed to a
> stable order:
>
> 1. **`shimmer::span::phi_span`** picked the *first* incoming def span in
>    `phi.incoming` (a `HashMap`) → nondeterministic `S101` span; now the earliest
>    (min) span.
> 2. **`compiler_checks::run_all_checks`** emitted diagnostics in producer order
>    (per-function `HashMap` walks); now sorted on a total
>    `(span, code, category, severity, message, replacement)` key
>    (`sort_diagnostics`).
> 3. **`optimiser::optimise_unit`** allocated monotonic group ids and emitted in
>    `cu.procedures` / def-use-chain `HashMap` order; now canonicalised before
>    overlap arbitration (stable sort) with group ids renumbered by first
>    appearance (`renumber_groups`).
> 4. **`taint::find_destructive_file_warnings`** (W313) named the *first offending
>    path variable* from `ssa_stmt.uses` (a `HashMap`), so `file rename $a $b`
>    reported `$a` or `$b` at random; now iterates path variables in argument
>    (source) order (`arg_var_names_ordered`).
> 5. **`type_infer`** folded the order-sensitive `type_join` over a phi's
>    predecessors in `HashSet` order — and `type_join` records only a `(from, to)`
>    pair for a 3+-way shimmer, so the `S101` message named different types
>    run-to-run; the predecessor list is now sorted before the fold.
>
> With these, the memo and whole-module builds agree byte-for-byte across the full
> `tmp/` corpus (893 files), stable across process-level `HashMap` seeds.  The
> corpus differential is the (now-passing) regression guard, `#[ignore]`d only for
> being slow (`--ignored`, ~100 s).  This **unblocks extending the lattice memo**
> (the `param_constants` win above can now land on top).

> **Memo *key* determinism (fixed) — the lattice cache was flaking.**  Distinct
> from the byte-identity fix above (which made the *output* seed-stable), the memo
> *key* itself was nondeterministic: `function_lattice`'s `FnLatticeKey` embeds the
> module `CfgContext` (`prepare_cfg_context`'s `upvar` + `proc_params`), and
> `proc_params` inserted **both** the short and qualified name of every procedure
> — so two procedures sharing a short name (`::a::x` and `::b::x`) raced on the
> `"x"` entry with **last-write-wins by `HashMap` iteration order**.  A
> determinism experiment (`exp_cfg_context_determinism`) showed the `proc_params`
> checksum varying run-to-run; the db-level probe showed a whole-file-shift edit
> re-executing **0 or all 74** pki lattices depending on seed.  Because the key
> flaked, an edit *anywhere above a procedure* could miss the cache for **every**
> procedure and rebuild the whole module — undermining the per-procedure memo that
> #1 (taint cascade) and #3-B both sit on.  Fix: iterate `module.procedures` in
> **sorted qualified-name order** in `prepare_cfg_context`, so a short-name
> collision resolves deterministically.  Output is unchanged (the collision value
> never affected diagnostics — pinned tests + e2e + `compiler_check_corpus` all
> green; this is why the byte-identity gate didn't catch it), but a whole-file
> shift now **reliably** reuses every procedure's lattice (`fn_lattice_reexec=0`
> across runs).  Guarded by `prepare_cfg_context_short_name_collision_is_deterministic`
> + `function_lattice_reused_on_whole_file_shift`.

> **OO method-body memoisation (shipped).** Method bodies were walked *in place*
> in pass 2 (not memoised), so a body edit re-walked every method.  They are now
> analysed as offset-0 isolated units and grafted like procs: `DeferredBody`
> carries the method's defining namespace + params + class instance variables,
> `analyse_proc_body_isolated` reconstructs a `ScopeKind::Method` scope (so
> `in_method` dispatch recording fires) with the instance variables pre-bound,
> and the proc/method pass-2 branches are unified through `body_fn` +
> `graft_proc_body`.  `ItemBodyKey` gains `is_method` + `class_variables`.  W308
> object tracking is the one method-specific subtlety — a method resolves the
> class against the whole file's classes, not analyse's DFS prefix, so a method
> that *actually* records an instance falls back (detected precisely: captured
> candidates are replayed at graft and the fallback fires only when
> `instance_classes` truly changed, so benign `dict create` stays fast).  Result:
> pt_rdengine_oo.tcl (2.2 kLOC, 113 methods) per-edit **165 ms → 44 ms**;
> cookiejar / disjointset / metaclass 8–16 ms.  Coverage holds at 92.2%,
> byte-identical; gated by `method_body_edit_recomputes_one_item` + the corpus +
> edit fuzzer.

> **Salsa-native lattice graph (shipped).** The per-procedure baseline lattices
> (CFG → SSA → def-use → SCCP → type → rendered → intra-procedural taint) are now
> memoised by the salsa-native `function_lattice` query, replacing the
> process-wide content cache (so salsa garbage-collects unreferenced entries).
> `build_for_memoized` normalises each procedure body to offset 0 and hands it
> (with the module's `CfgContext`, interned once per build) to a callback that
> interns the `FnLatticeKey` and demands `function_lattice`; the builder rebases
> the returned offset-0 unit to the procedure's real span (`lattice_rebase`).
> Reuses the same `build_cfg_function_with_upvars` call `build_cfg` makes per
> procedure, so the rebuilt unit equals the whole-module build's (modulo offset),
> under `db.registry` (byte-identical to both consumers' `build_default` +
> `load_dialect`).  The optimiser path was the architectural blocker — it ran
> *off* salsa with no db handle; it now runs through the `compiler_check_diagnostics`
> tracked query (server filters the master-switch / per-code disables + lifts), so
> both diagnostics consumers share the same memoised lattices.  Gated by the
> per-item corpus + `incremental == fresh` differential fuzzer + the db
> equivalence/offset-invariance tests (`function_lattice_reused_on_body_shift`,
> `compiler_check_diagnostics_matches_uncached`) + e2e parity.
>
> **Remaining (future).**
>
> - **Interprocedural taint cascade (shipped — backlog #1).**
>   `with_interprocedural` re-ran `propagate_taints` for every procedure every
>   edit.  A salsa-native `taint_cascade` query (`tcl-lsp-db`) now layers on
>   `function_lattice`'s offset-0 baseline so an unchanged procedure whose
>   reachable-callee summaries are unchanged reuses its cached taints.  The taint
>   lattice is `ValueKey`-keyed (span-free), so the cascade runs over the offset-0
>   baseline and installs the result directly into the rebased unit — **no rebase
>   needed**.  The key (`TaintSummaryKey`) captures exactly what `propagate_taints`
>   reads from the summary: the full procedure-name set (call-resolution domain)
>   plus the taint-relevant projection (`writes_global`, `calls`,
>   `return_passthrough_param`, `params`) of the cascade root + its transitive
>   callees — so a body edit that flips a reachable callee's passthrough /
>   global-write re-interns the key for exactly the callers that reach it.
>   `CompilationUnit::with_interprocedural_memoized` routes each procedure's taint
>   through a callback (top level stays fresh); the whole-module summary is still
>   built per edit (it is the memo's input), so the saving is only the
>   per-procedure re-run — **net-small, as predicted**.  Byte-identical: the
>   `compiler_check` corpus differential proved the minimal-summary reconstruction
>   cold over 893 files, and a new edit-staleness db test
>   (`taint_cascade_matches_uncached_under_edits`) proved no stale cache survives a
>   callee-behaviour edit.  Gated also by `taint_cascade_reused_on_unrelated_edit`
>   (one cascade recomputes per unrelated edit) + e2e.

## Scoping the per-edit lowering floor (backlog #3)

The `param_constants` fold (above) made `function_lattice` engage for nearly
every procedure, so the per-procedure *lattice compute* is now memoised and the
per-edit cost is dominated by the **non-memoised whole-module work** in
`build_for_inner` / `memoised_compilation_unit`.  Fresh phase-timing
(`cargo run --release -p tcl-lsp-db --example tail_profile FILE=…`, warm DB,
single-char body edit):

| file | LOC / fns | `file_analysis_incremental` | `compiler_check` | BOTH | CU build+interproc |
|---|---|--:|--:|--:|--:|
| `pki.tcl` | 3.3k / 75 | ~111 ms | ~133 ms | ~150–180 ms | ~66 ms |
| `parse_lemon.tcl` | 7.4k / 177 | ~161 ms | ~157 ms | ~223 ms | ~110 ms |

Component split of the per-edit CU build (`parse_lemon`, from the prior
proc-loop instrumentation, still representative): `lower_to_ir` **~26 ms** +
module `build_cfg` **~10 ms** + the per-procedure clone/rebase loop **~52 ms** +
`with_interprocedural` **~19 ms**.  The 52 ms is *not* lattice compute (memoised)
— it is the offset-invariant plumbing run for **every** procedure every edit:
clone the proc's offset-0 IR body + `rebase_script`, then deep-clone the
memoised `FunctionUnit` + `rebase_function_unit` to the proc's real span
(`lattice_rebase.rs`).  The rebase target (the offset) shifts whenever an edit
moves a procedure, so the rebased unit can't be cached across edits.

Two ways to kill it, both larger refactors.  Neither alone is a knockout — they
attack disjoint parts of the floor.

### Approach A — incremental per-item IR lowering / CFG (attacks the ~26+10 ms)

Lower **per item-body**, keyed on the offset-0 body text (mirroring
`function_lattice` for CFGs and the per-item analyser walk for diagnostics), so
a body edit re-lowers one body and reuses the rest.  `item_tree` already gives
the body spans; `build_cfg`'s per-proc loop already builds each CFG from one
body, so the CFG half is nearly free to make per-item.

- **Pro:** reuses the *proven* offset-0 + rebase machinery (byte-identical for
  lattices today); does **not** touch the ~50 span-emit sites in the consumers
  (see Approach B's surface).  Architecturally consistent with slices 2–4.
- **Con / blocker:** lowering is **not** body-local.  `lower_to_ir_with_config`
  runs whole-module passes whose output for one body depends on the others —
  `specialise_factories`, `inline_uplevel_passthrough`, `extract_oo_methods_pass`,
  and `populate_trace_facts` (scans every body for `trace add execution …`,
  setting module-wide `traced_commands` / `has_dynamic_trace` that GVN reads).
  So a per-body lowering memo needs the same "cross-item facts live in the
  aggregate, fed in as an input" split the analyser walk used (Phase B/C): the
  body memo keys on the module-wide trace/factory facts, which a signature-class
  edit invalidates.  Tractable but non-trivial; gated by the same corpus
  differential.
- **Ceiling:** ~36 ms (lower+cfg).  The 52 ms rebase/clone **remains** — A does
  not remove it.  `collect_call_site_constants` reads the module CFG, so the
  per-proc CFGs can't simply be skipped; rework it to walk IR call statements
  directly first (small, independent, unblocks skipping redundant CFG builds for
  memoised procs).

### Approach B — offset-aware diagnostic consumers (attacks the ~52 ms)

Leave each `function_lattice` unit at **offset 0** and hand every consumer the
proc's base byte-offset, added at span-emit time — eliminating both the
`rebase_function_unit` walk **and** the deep-clone (consumers borrow the cached
`Arc<FunctionUnit>` + offset).  This is the single biggest lever (~52 ms).

- **Surface (from the consumer span-read audit).**  Four consumers read
  *absolute* spans out of a unit's `cfg`/`ssa`/`sccp`:
  - **taint** (via `with_interprocedural`) — **2 sites** (`taint.rs:1680,2313`);
    trivially offset-at-emit.  *Best first target.*
  - **analyser CFG/SSA tail** (`emit_cfg_ssa_diagnostics`) — **~8 sites** across
    5 emitters (W220/W211/H300/W210/W307); moderate.
  - **`run_all_checks`** — **~15–20 sites** across `gvn.rs` / `shimmer/span.rs` /
    `taint.rs` / irules; one aggregator, many helpers.
  - **`optimise_unit`** — **~25+ sites** across **9 passes**; widest.
- **Leakage risks (a pure offset-0 unit + whole-file `source` don't mix).**
  - `optimiser/propagation.rs` validates `source[span.start()..span.end()]`
    against the body text (O102) — with an offset-0 span over a whole-file
    `source` this slices the wrong bytes.  Needs offset-aware slicing (add the
    base before indexing) — the principal correctness hazard.
  - the analyser's W307 / `$var` branches already read the **full** `self.source`
    (deferred to the tail in Phase C); those stay absolute and must not be
    offset-shifted.  So the analyser becomes a **hybrid** (offset-0 unit spans +
    absolute source scans) — workable but fiddly.
  - sort-by-`span.start()` determinism (gvn/shimmer/optimiser) is offset-shift-
    invariant, so *relative* ordering is safe — but a build that mixes offset-0
    and absolute spans in one sort would corrupt it.  All-or-nothing per unit.
- **All-consumers-or-no-win constraint.**  `file_analysis_incremental` and
  `compiler_check_diagnostics` **share one** built `compilation_unit`.  If even
  one consumer (the optimiser, the hard one) still needs absolute spans, the
  shared unit must still be rebased → **no win**.  So B only pays off once
  *every* consumer of the shared unit is offset-aware, or the shared-unit query
  is split so the offset-aware consumers take an un-rebased unit.  A phased
  "taint + analyser first" rollout validates the machinery but banks **zero**
  wall-clock until the optimiser is converted too.

### Approach B — shipped (offset-aware consumers, rebase walk removed)

The memoised build no longer rebases each procedure's unit to its real position.
`FunctionUnit` carries a `base_offset`; the memoised arm leaves the unit at
offset 0 and sets `base_offset = body_offset`, and every diagnostic consumer adds
it at emit time (`abs_span` / `abs_pos`).  `rebase_function_unit` (the O(unit)
per-procedure span walk) is **deleted**.  The conversion followed the
span-provenance rule above (only `fu.cfg`/`ssa`/`sccp`-sourced spans shift;
`cu.ir_module`-walking optimiser passes are untouched), and is gated by a new
`file_analysis_corpus` differential (`file_analysis_incremental` vs `analyse`
over 893 files, the analyser-tail analog of `compiler_check_corpus`) **plus**
`compiler_check_corpus` — both byte-identical — and e2e.  Measured: pki
`compiler_check` per edit ~133 → ~111 ms; the aggregate BOTH win is smaller (the
O(file) lowering floor + the still-present per-proc deep-clone dominate).

**Two follow-ups remain to fully bank the lever:**

- *Eliminate the per-proc deep-clone.*  `build_for_inner` still
  `(*function_lattice).clone()`s each unit before setting `base_offset`.  Storing
  `Arc<FunctionUnit>` in `cu.procedures` (offset-0, shared straight from
  `function_lattice`) + a per-proc offset would drop the clone too — but it
  ripples through every `cu.procedures` / `cu.function()` / `cu.functions()`
  accessor, so it is its own change.
- *Backlog #2 (per-function `optimise_unit` memo): correctness-de-risked, but a
  major refactor.*  The optimiser is now offset-aware (its prerequisite).  The
  feared cross-function `PassContext` coupling turns out to be **inert**: the only
  writes to `propagated_branch_uses` / `propagated_use_groups` /
  `propagated_expr_stmts` are in **unit tests** — in production those sets are
  always empty, so the `propagation.rs` reads have no effect, and
  `reset_function_state` is dead (test-only).  `next_group` is canonicalised by
  `renumber_groups`, and O127's `rewritten` snapshot only overlaps spans **within
  one function's source region** (cross-function opts are in disjoint regions).
  So a per-function optimise **can** be byte-identical to the whole-unit run.
  - **Design.**  A salsa query `function_optimisations(FnLatticeKey + ctx)` that
    optimises one procedure **in isolation at offset 0** and returns offset-0
    optimisations; `optimise_unit` then rebases each by the proc's `base_offset`,
    merges, and runs the existing whole-unit canonicalise + `select_non_overlapping`
    + `renumber_groups` arbitration (cheap).  Offset-invariance requires the run's
    `ctx.source` to be the **proc's body substring** (`source[proc.span]`, stable
    across edits when the body is unchanged) so offset-0 spans index it, plus the
    proc's offset-0 IR body — both already produced for the `function_lattice`
    key.  The cross-function *read-only* context the passes consume (`interproc`,
    `command_mutations`, `proc_cfgs`, `cross_event_vars`) must be interned into the
    key (coarse: a summary edit invalidates all per-proc optimise memos; a
    reachable-context projection like `taint_cascade`'s would be tighter).
  - **Cost.**  Building the single-function offset-0 view touches nearly every
    `CompilationUnit` field the passes read (`ir_module`, `cfg_module`,
    `connection_scope`, …), so it is a refactor on the order of Approach B for the
    ~12 ms `optimise_unit` spends — gate with `compiler_check_corpus` (fast, the
    O127-overlap inertness assumption is exactly what it verifies).  Sequence it
    after the deep-clone removal.
  - **Isolation seam shipped + validated.**  `optimise_unit_per_function`
    optimises each `::top`/procedure in an isolated single-function view and
    merges + arbitrates; a new `optimise_per_function_corpus` differential proves
    it byte-identical to `optimise_unit` over the corpus.  Finding: a fresh
    `PassContext` restarts `next_group` at 0, so two functions' rewrite groups
    both come out `0` and are **conflated** on merge — fixed by remapping each
    run's group ids into a globally-unique range before merging (the final
    `renumber_groups` then re-canonicalises identically, since the canonical id
    depends only on the partition + sorted order).  **`unused_procs` (O124) is
    iRules-only *and* whole-module** (reachability from `::when::*` handlers via
    `ctx.interproc`), so the memo must run it whole-module, not per-function (the
    tcl validation doesn't exercise it).
  - **Open question for the salsa step — does the memo net-win?  *Measured: yes.***
    The optimiser passes read the **whole** `interproc` summary, so the memo key
    must capture it; I worried interning it per build (`O(procs)`) plus the
    coarse invalidation (any *summary* edit invalidates every per-proc optimise
    memo) might approach the ~12 ms saving.  The
    `optimise_memo_experiments` harness disproves that:
    - **E1 (savings ceiling):** `optimise_unit` is **12.9 ms / 74 procs** (pki),
      **18.9 ms / 176 procs** (parse_lemon) — ~0.1–0.2 ms per proc.  A warm
      single-proc edit re-optimises one proc + arbitration, so the memo removes
      essentially all of the 13–19 ms.
    - **E3 (key cost):** serialising `interproc` into a hashable key is
      **0.005–0.02 ms** — ~600× cheaper than the optimise it gates.  The interning
      worry was unfounded.
    - **E2 (hit rate):** over 5 265 per-procedure body edits across 276 corpus
      files, a benign body edit leaves `interproc` **byte-identical 100.0 %** of
      the time (the summary is offset-independent + structural).  So even the
      *whole-`interproc`-key* memo hits ~100 % of non-signature edits; when a
      summary does move it is **~1** procedure, so a reachable-key memo (à la
      `taint_cascade`) reuses everything except that proc's callers.
    - **Verdict from E1–E3: build it.**  A whole-`interproc`-key memo wins on the
      common benign edit; the reachable-key projection is a cheap refinement.
  - **Built it — then reverted (the experiments were necessary but not
    sufficient).**  The full salsa memo was implemented (`function_optimisations`
    keyed on the offset-0 `Procedure` + `opt_context` with `PartialEq`
    early-cutoff) and is **byte-identical** to whole-unit `optimise_unit` over the
    893-file corpus (`compiler_check_corpus` green) — correctness is *proven*.
    But benching revealed a perf **regression** E1–E3 missed: each procedure's
    optimise runs in a single-function `CompilationUnit` view that must own a copy
    of the whole `interproc` summary (`PassContext` takes it by value), so when
    many procedures miss in one edit the per-proc loop is **O(file²)** in
    `interproc` clones.  The standard bench edit (`find("\n    ")`, early in the
    file) shifts *every* procedure and — because `function_lattice` itself
    cache-misses on a whole-file shift in this path — all procedures miss at once,
    pushing pki `compiler_check` from ~111 ms to ~200+ ms.  (For *localized* edits
    the memo hits and helps: an edit near EOF recomputes ~5 of 75 procs.)
    - **To ship it needs two more things, for a ~12 ms ceiling:** (1) make
      `PassContext.interproc` an `Arc` so the single-function view shares it
      instead of cloning per proc (kills the O(file²) cliff); and (2) the benefit
      is still **capped by `function_lattice`'s hit rate** — on whole-file-shift
      edits it misses everything, so the optimise memo can't help those either.
      Given the ~12 ms ceiling and these two dependencies, it was **not worth
      shipping a regression** now; reverted to whole-unit `optimise_unit`.
    - **Kept (the durable, byte-identical artifacts):** `optimise_unit_per_function`
      + the `optimise_per_function_corpus` isolation differential (proving the
      isolation property a future memo builds on), the `optimise_memo_experiments`
      harness, and `Procedure: Eq+Hash` (enables interning the offset-0 proc).

### Recommendation (for the lowering floor proper)

Sequence by risk-adjusted value:

1. **Micro-win — *not* as cheap as it looks.**  Reworking
   `collect_call_site_constants` to walk IR call statements (so memoised procs
   needn't have their real-offset CFG built in `build_cfg`) is **divergence-prone**
   and was deferred:
   - The module CFG omits calls inside **top-level deferred loops** when
     `defer_top_level = true` (the codegen path), while a naive recursive IR walk
     would include them.  A faithful IR walker must replicate that `defer_top_level`
     gating exactly.
   - `build_for_inner` (hence `collect_call_site_constants`) is shared by the
     **codegen** build too, and `param_constants` seed SCCP — so a *uniform*
     behaviour change isn't caught by the memoised-vs-uncached
     `compiler_check_corpus` differential (both sides change together); only the
     pinned analyser unit tests + e2e would catch it.  Do this only with those
     pins watched, and prove the `defer_top_level` parity first.
   - **Why it matters:** on a *warm* edit `function_lattice` is a cache hit, so the
     proc's offset-0 CFG is already built; `build_cfg`'s real-offset per-proc CFG
     is then **pure redundant work** (~10 ms on `parse_lemon`).  Skipping it needs
     call-site collection off the CFG first — hence this micro-win is the unblock.
2. **Approach A** (incremental per-item lowering): lower risk (no consumer
   surface), architecturally consistent, ~36 ms ceiling — but first prove the
   body-lowering memo is byte-identical under the whole-module trace/factory/
   uplevel passes (the cross-item-facts-as-input split).
3. **Approach B** last and only committed-to wholesale: stage the machinery on
   **taint** (2 sites) + the **analyser tail** (~8) behind a `SpanOffset` newtype
   to prove byte-identity on a narrow surface, then convert `run_all_checks` and
   finally the **optimiser** (resolving the `propagation.rs` source-slice
   leakage) — banking the ~52 ms only when the last consumer flips.  Highest
   value, highest risk; do not start partial expecting a wall-clock win.

> **Backlog #2 (memoise `optimise_unit` per-function) is the optimiser half of
> Approach B, not an independent item.**  `optimise_unit`'s passes read
> `ctx.source[span]` at real offsets (`propagation.rs` O102), so an
> *offset-invariant* per-function optimiser memo — the only kind that survives the
> shifts a real edit causes — needs exactly the offset-aware/source-slice refactor
> Approach B's optimiser step describes.  A non-offset-invariant memo (keyed on the
> proc's real-offset unit) would miss on every edit that shifts the proc, so it is
> not worth building.  **Do #2 as part of #3-B's optimiser conversion**, not before.

All steps keep the full-rebuild fallback and gate on
`compiler_check_memo_matches_uncached_over_corpus` + `per_item_corpus` +
`differential_incremental` + e2e (the existing byte-identity contract).

### Approach B — landed foundation + validated execution plan

**Foundation (shipped).**  `FunctionUnit` gained a `base_offset: i64` plus
`abs_span` / `abs_pos` helpers: `absolute = local + base_offset`.  A unit built
at its real position keeps `base_offset = 0` (`abs_span` is identity); the flip
will build memoised procedure units at **offset 0** with `base_offset = body
offset`.  No behaviour change yet (`base_offset` is 0 everywhere, unread).

**The span-provenance rule (the key to a *tractable* conversion).**  Only spans
read from a `FunctionUnit`'s `cfg` / `ssa` / `sccp` (incl. the `CommandTokens`
`argv_span`s those statements carry) become offset-0 after the flip and need
`abs_span` / `abs_pos`.  **`cu.ir_module` keeps absolute (whole-file) offsets** —
it is lowered once whole-file and only the per-proc `function_lattice` *key* is
normalised to 0 — so every optimiser pass that walks `cu.ir_module` is
**unaffected**:

- *No change (IR-absolute):* `pattern_recognition`, `code_sinking`,
  `structure_elimination`, `tail_call`, `unused_procs`, `expr_simplify` — they
  iterate `cu.ir_module`, not `cu.functions()`.
- *Needs conversion (reads `fu.cfg`/`ssa`/`sccp`, via `cu.functions()`):*
  - analyser tail (`emit_cfg_ssa_diagnostics`): ~8 sites (W220 `diagnostics.rs`
    ~5660, W211 ~5739, H300 ~5824, W210 ~6134/6408/7027, W307 ~8314, IRULE4005
    ~8642).  **Hybrid risk:** W307 (`emit_var_command_diagnostics`) also scans the
    full `self.source` — those reads stay absolute; only the `fu`-sourced span
    shifts.
  - `compiler_checks`/`run_all_checks`: `gvn.rs` 744/1494/1506, `shimmer/span.rs`
    29/57/66, `taint.rs` 1680/2313, `sccp.constant_branches` (compiler_checks:194).
  - `elimination.rs` 361/549/721 + their `full_rewrite_span(ctx.source, …)` (some
    in `emit_unreachable`/`emit_adce` helpers where `fu` must be **threaded in**).
  - `propagation.rs` O102/O127 (175/191/423/428/434/444): `full_word_span(ctx.source,
    argv_span)` and `source[…]` slices where `argv_span` is from `fu.cfg`
    `CommandTokens` → wrap with `fu.abs_span` / `fu.abs_pos`; several are in
    `forward_candidate`-style helpers needing `base_offset` threaded.
  - `branch_folding.rs` 171/252 (`Terminator::Branch` span from `fu.cfg`).

**Fast-gate strategy (why this is *not* the hours-long grind it first looks).**
The flip only changes the **memoised db path** — `analyse` / `analyse_per_item`
build their own real-offset units (`base_offset = 0`), so **`per_item_corpus` and
`differential_incremental` do not exercise it** and need not be re-run per
iteration.  The fast oracle is:
- `compiler_check_memo_matches_uncached_over_corpus` (~130 s) — covers the
  optimiser + `run_all_checks` + taint consumers, catching **both** a missed
  `fu`-span (stays offset-0 → wrong) and an over-wrapped IR-absolute span (shifted
  → wrong).
- a **new** file-analysis corpus differential (`file_analysis_incremental` vs
  `analyse` over `tmp/`, mirroring `compiler_check_corpus`) — add this to gate the
  analyser-tail conversion (no such corpus gate exists today; `file_analysis`'s
  taint-derived diagnostics make it worth having regardless).
- e2e.

**The flip.**  In `build_for_inner`'s memoised arm, set `fu.base_offset =
i64::from(body_offset)` and **drop** the `rebase_function_unit` call (it becomes
dead — keep `rebase_script`, still needed to form the offset-0 key).  Convert
**every** site above first (each is byte-identical at `base_offset = 0`, so the
conversion commits cleanly ahead of the flip); the flip is the single atomic
switch that banks the win.  Removing `rebase_function_unit` eliminates the
per-proc rebase walk (~half of the 52 ms); a follow-up storing `Arc<FunctionUnit>`
in `cu.procedures` (offset-0, shared from `function_lattice`) eliminates the
remaining per-proc deep-clone.

**Backlog #4 (`practcl.tcl`) is *not* unblocked by Approach B.**  practcl's
fallback is the per-item **analyser walk**'s genuinely-twice-defined method-style
definer (a correctness fallback in `body_needs_enclosing_context` /
duplicate-detection), not the per-edit CU lowering cost Approach B attacks.  It
needs the incremental-lowering / duplicate-grafting work (Approach A's territory +
the duplicate-definer handling), so it should follow A, not B.

## How to run the experiments

- **E1/E2/E3/E6/E7** (item-locality, offset-invariance, cost split, lattice
  costs, shared-CU saving): `cargo run --release -p tcl-compiler --example
  incr_experiments` (reads the `tmp/` corpus).
- **E4/E8** (salsa early-cutoff + cascade breadth): `cargo test -p tcl-lsp-db
  --test early_cutoff`.
- **E5** (the differential fuzzer, `incremental == fresh`): `cargo test -p
  tcl-compiler --test differential_incremental -- --ignored` (corpus-gated,
  slow). Currently surfaces the pre-existing `analyse` vs `analyse_commands`
  diagnostic inconsistency — slice 2 must close it; this test is the gate.

## Slice-1 execution notes (for whoever picks it up)

The non-trivial part of `item_tree` is reproducing the analyser's
**namespace-qualified** item detection without divergence. The detection lives
in the stateful scope walk:

- `handle_proc_command` (`analyser/handlers.rs:292`): `name = args[0]`,
  body span = `arg_tokens[2].span`, qualified via
  `command_resolution_namespace(scope_path)` + `qualify(ns, name)`
  (`analyser/scope.rs`, currently `pub(super)` — expose it).
- `handle_namespace_eval_command` (`handlers.rs:477`) creates the child scope
  whose path drives qualification; `oo::class create` /`oo::define`
  (`handlers.rs:1154,1229`) for classes/methods; `interp alias` /`namespace
  ensemble` for the alias/ensemble sets in `file_decls`.
- Bodies can define **nested/global procs** (E1's perf-file finding) — the
  extractor must recurse into bodies, and a body edit that adds/removes a nested
  def is a *signature* change (cascade), not a pure body edit.

The safest path is to derive `item_tree` from the **CST** the analyser already
builds (so detection can't diverge), gated by a corpus test asserting
`file_decls`'s proc/class/alias/ensemble set equals the current
`AnalysisResult.all_procs`/`all_classes`/`command_aliases`/ensembles. Do not
ship slice 1 until that test is green across the corpus.
