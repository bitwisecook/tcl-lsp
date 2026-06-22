# SRV-INCREMENTAL — making the per-edit pipeline incremental: measurement & track design

> **Status:** Design + measurement. This document scopes the **SRV-INCREMENTAL**
> track: finishing end-to-end per-edit incrementality so a keystroke recomputes
> only what the edit actually changed — *within a file and across the project*.
> It **supersedes the SRV-ROPE track** (a rope-backed `DocumentState`): a measured
> experiment showed a rope addresses ~**0.02%** of per-edit latency, so the rope
> survives here only as an optional, late micro-optimisation. The per-item
> analyser firewall this track builds on is designed and largely shipped in
> [`../rust/incremental-analysis.md`](../rust/incremental-analysis.md); this doc
> owns what remains, and adds the cross-file dimension that doc does not cover.

## TL;DR — the decision

The per-edit critical path is dominated by **whole-file `run_all_checks`**, not by
buffer edits, re-lex, or the per-item analyser walk. Measured on `linalg.tcl`
(2 299 lines), warm salsa db, one single-character edit inside a proc body:

| per-edit work | cost | incremental today? |
|---|--:|:--|
| buffer apply + `LineIndex` rebuild (the rope's slice) | ~85 µs (**0.02%**) | n/a — trivial |
| analyser walk (`file_analysis_incremental`, warm) | ~85 ms | ✅ per-item memoised |
| **compiler checks (`run_all_checks` + `optimise_unit`), whole unit** | **~405 ms** | ❌ **whole-file every edit** |
| **warm per-edit total** (the two queries; checks dominate) | **~411 ms** | |

(The stages share one built `CompilationUnit`, so the warm total is roughly the
cost of the checks query — not the sum of the rows; full `tail_profile` dump below.)

So:

1. **The prize is per-procedure check incrementality.** The shipped salsa firewall
   already makes the analyser *walk* and the per-proc *lattices* (CFG/SSA/SCCP/
   type/taint) incremental — but the *checks over those lattices*
   (`run_all_checks`, `optimise_unit`) re-run over the **whole unit** every edit,
   and that is ~99% of warm per-edit latency. Making them per-proc (keyed on the
   same offset-invariant lattice key the lattices already use) is the single
   highest-leverage change.
2. **Cross-file analysis is not on the incremental graph at all.** The
   `WorkspaceIndex` is a plain server struct, `resolve_proc_call` is per-file,
   arity never crosses files, and editing file A recomputes **nothing** in file B.
   The cross-file cascade has to be *designed* (as salsa edges), not merely tuned.
3. **The rope was the wrong lever.** It speeds up buffer apply (0.02% of per-edit
   time) and costs 1.4–1.9× memory on many small files. It is demoted to an
   optional final task, gated on the analysis floor actually being gone first.

The track below sequences the cheap wins first, makes the dominant slice
incremental, then extends incrementality across files, and only then revisits the
rope.

## What the server does per edit today (the baseline measured)

Per `textDocument/didChange` (`rust/tcl-lsp-server/src/lib.rs`, `did_change`):

1. `apply_content_change` per content change → splice the buffer; bump the doc
   revision. (~µs; the rope's slice.)
2. `db_set_source` → salsa `SourceFile::set_text` with the **whole-file `String`**
   (one flat input per file; no chunk/per-proc granularity), bumping the input
   revision and marking every dependent query dirty.
3. `workspace_index.remove_document(uri)` — drops file A's symbols from the
   cross-document aggregate. **This is the only cross-file action, and it triggers
   no re-analysis of any other file.**
4. debounced `schedule_diagnostics` → two salsa queries:
   `file_analysis_incremental(file, config)` (the per-item analyser walk) and
   `compiler_check_diagnostics(file)` (`run_all_checks` + `optimise_unit` over the
   built `CompilationUnit`). For every dialect but `tcl8.4`/`f5-irules` the two
   share one `compilation_unit` (a same-revision cache hit).

The server's own comment marks the key gap: *"The re-analysis below is still
whole-document; bounding it to `reparse_window` is a documented follow-up — the
primitives exist in `tcl-lexer`."*

## Where the per-edit time actually goes (measurement)

Three harnesses pin the numbers down. Two are reproducible, workspace-excluded
experiments in this directory; the third is a committed production example.

```
# (a) apply-side numerator — what a rope speeds up, in isolation:
cargo run --release --manifest-path docs/design/srv-incremental/experiment/Cargo.toml
# (b) per-edit denominator — what that apply is a fraction of (real analyser + salsa db):
cargo run --release --manifest-path docs/design/srv-incremental/experiment-pipeline/Cargo.toml
# (c) production per-edit profile, warm db, single-char edit (the real server shape):
FILE=tmp/tcllib-2.0/modules/math/linalg.tcl \
  cargo run --release -p tcl-lsp-db --example tail_profile
```

Harness (c) on `linalg.tcl` (2 299 lines, 81 functions), one space inserted in a
proc body, alternating warm→edited:

```
== full per-edit path (salsa, memoised) ==
  file_analysis_incremental (per edit)     85.0 ms   ← analyser walk, per-item memoised
  compiler_check_diagnostics (per edit)   444.5 ms   ← WARM, still ~whole-file
  BOTH queries per edit (production)      411.2 ms   ← real warm per-edit latency
== compiler-check tail breakdown (whole-file, no memo) ==
  CompilationUnit::build_for + interproc   59.0 ms
  run_all_checks                          405.1 ms   ← dominates
  optimise_unit                            15.4 ms
```

The decisive comparison: **warm `compiler_check_diagnostics` (444 ms) ≈ no-memo
`run_all_checks` (405 ms)**. The per-proc lattice memo (see below) successfully
makes the analyser walk cheap (85 ms) and reuses every unedited proc's SSA
lattice, but `run_all_checks`/`optimise_unit` consume the whole `CompilationUnit`
and are **not** behind a per-proc memo, so they re-run in full on every edit. On
`practcl.tcl` (~8.5k lines) the same warm per-edit is ~1.6 s, scaling with file
size exactly as a whole-file pass would.

A 5–12× speedup on the ~85 µs apply slice (harness (a), below) is invisible
against this. That is why the track targets the analysis floor, not the buffer.

## What is already incremental (the foundation)

The per-item analyser firewall is designed and largely shipped — full detail in
[`../rust/incremental-analysis.md`](../rust/incremental-analysis.md); the shape:

- **Signature firewall.** `item_tree(file)` extracts the proc/class/alias
  declarations (`structure_only().analyse`, no diagnostics); `item_sigs` strips
  bodies to headers; `file_decls` aggregates them. A body-only edit leaves
  `item_sigs` *equal*, so `file_decls` and every cross-item pass **backdate**
  (salsa early-cutoff) without re-running.
- **Per-body / per-proc memo, offset-invariant.** `item_body_analysis` (keyed on
  `ItemBodyKey`), `function_lattice` (keyed on `FnLatticeKey`: the offset-0 IR
  body + module context + params + dialect), and `taint_cascade` (keyed on
  `FnLatticeKey` + `TaintSummaryKey`) are all `#[salsa::tracked]`. The keys are
  *offset-invariant*, so a proc merely **shifted** by edits above it is a cache
  hit. Tests assert "exactly one body/lattice/cascade re-runs" on an unrelated
  edit, and "zero" on a pure blank-line prepend.
- **Coverage.** The per-item fast path covers the large majority of the corpus
  (92.2% at the last `incremental-analysis.md` baseline); the rest falls back to a
  full walk on incomplete or `syntax_error` input.

Two principles this track inherits and must not break:

- **Correctness rests on the `incremental == fresh` differential fuzzer plus the
  full-rebuild fallback — never on the assumption that an edit is local.**
  Item-locality is a *performance* heuristic; the fuzzer is the permanent gate.
- **Offset-0 + rebase-at-aggregation** is the established pattern (Approach B):
  each unit is built at offset 0 and consumers add `base_offset` at span-emit
  time. New per-item work follows this model.
- **Salsa input setters always bump the revision.** A per-item input must be set
  *only when the item-tree diff says it actually changed*, or every keystroke
  wakes all direct dependents (the E4/E8 finding). This rule is what makes the
  cross-file design below safe.

## Cascading changes — how incremental re-analysis stays correct and bounded

The heart of the track. "Recompute only what changed" is easy to assert and
subtle to get right, because one edit can legitimately invalidate work far away.
Two regimes: *intra-file* (mostly solved, one gap) and *inter-file* (greenfield).

### Intra-file cascade

Classify each edit by what the **structural diff** says changed:

**1. Body-only edit** — text changes inside one proc body; all signatures
unchanged.
- *What recomputes (correct, shipped):* `item_sigs`/`file_decls` backdate;
  exactly one `item_body_analysis` + one `function_lattice` + one `taint_cascade`
  re-run (the edited proc). Shifted siblings are cache hits via offset-invariant
  keys.
- *The gap:* `compiler_check_diagnostics` still re-runs `run_all_checks` +
  `optimise_unit` over the **whole** unit, because it depends on the file input
  and is not split per-proc. → **Task 2.**

**2. Signature change** — a proc's params/arity/name/namespace change.
- `item_sigs` changes → `file_decls` changes → the cross-item interproc summary
  and the arity/W123 passes recompute. This is *correct*: callers of the changed
  proc must re-check arity (E002/E003).
- *How the cascade stays bounded:* sibling **bodies** did not change, so their
  `item_body_analysis`/`function_lattice` stay cached — only the *cross-item*
  layer recomputes. The refinement (Task 2/6) is to model the
  call-site→callee-signature dependency as a salsa edge — an arity check keyed on
  the resolved callee's `signature` — so only **callers of the changed proc**
  re-check, not every proc in the file. Today the whole file's arity pass re-runs;
  that is correct but coarser than necessary.

**3. Structural edit** — add/remove a proc, an unbalanced brace, anything that
changes the item set.
- `item_tree` changes; the regions keyed on the changed span recompute. The
  structural-state index (`reparse_window`, `script_is_complete`, the
  bracket/brace/paren indexes already built in `tcl-lexer`) bounds re-lex /
  re-segment to the dirty span once wired in (**Task 5**). New/removed procs
  change `file_decls`, cascading to the cross-item layer as in case 2.
- On incomplete or `syntax_error` input, fall back to a conservative full rebuild
  (already the pattern) — correctness over locality.

In every case the backstop is the single-file `incremental == fresh` fuzzer.

### Inter-file cascade — changing a proc signature in one file, affecting others

**Today this cascade does not exist** — confirmed in both the Rust and (legacy)
Python servers:

- The `WorkspaceIndex` (`tcl-lsp-core`) is a plain server-owned struct, **not a
  salsa input**. It aggregates proc/class *definitions* and call sites by
  qualified name for *editor features* (completion, cross-document go-to-def,
  references, rename, call-hierarchy).
- The analyser's `resolve_proc_call` resolves **only against the current file's**
  `all_procs`; cross-file arity (E002/E003) is never checked.
- On `didChange`, file A is removed from the index and only A is re-analysed.
  **File B is untouched** until B is next edited. The one cross-file signal — W123
  (unresolved command) suppression against the set of workspace proc names — is a
  one-directional *filter applied at B's next analysis*, not a reverse-dependency
  cascade.

So inter-file incrementality is greenfield: there is nothing to make *faster* —
there is a correctness/coverage feature (cross-file diagnostics) to build
*incremental from the start*, the right way, rather than bolting a hand-maintained
reverse-dependency map onto an off-graph index.

**The design — cross-file dependencies as salsa edges (the rust-analyzer model),
bounded by Tcl's dynamic dispatch:**

1. **Lift the project signature table into salsa.** Add a project-level
   `project_signatures()` query (a def-index keyed by qualified name) that depends
   **only on each file's `item_sigs`/`file_decls`** — the signature firewall — and
   never on bodies. Because it reads only firewall outputs, a body edit in *any*
   file leaves it unchanged → it backdates → **zero cross-file work**. Only a
   signature/decl change recomputes it; exposing each symbol as its own
   `signature(qname)` query (point 2) then gives **per-entry** early-cutoff, so a
   change to one symbol does not recompute consumers of the others (early-cutoff is
   per-query-output, so the per-symbol query — not the whole table — is what bounds
   the fan-out).

2. **Make cross-file resolution and arity tracked queries.**
   `resolve_cross_file(call_site) → Option<def_id>` reads `project_signatures`; the
   per-call arity check depends on `signature(def_id)`, which may live in another
   file. When file A's proc signature changes, salsa invalidates **exactly the call
   sites whose resolution points at that proc — in any file** — and recomputes only
   their arity check, not those files' whole analysis.

3. **Reverse-dependency invalidation falls out for free.** This is the entire
   reason to put the edges on the salsa graph: you never hand-maintain a reverse-dep
   map (the index has none today, which is *why* there is no cascade). Salsa already
   records "B's query read A's signature"; bumping A's signature input invalidates
   B's dependent query precisely, and nothing else.

4. **The E4/E8 input discipline scales to the project.** A keystroke in A bumps
   only A's input; but if A's *signature-table entry* is unchanged (a body edit),
   the project table backdates and no file B wakes. This is the firewall extended
   across files — and it is what stops every keystroke in a heavily-depended-on
   utility file from waking the whole workspace.

5. **Tcl's dynamic dispatch bounds precision — stated honestly.** Tcl resolves
   command names at runtime: `eval`, computed names, `uplevel`/`upvar`,
   `namespace import`, `interp alias`, `[$obj method]`. Cross-file resolution is
   therefore **best-effort and name-based** (the same heuristic the index already
   uses for go-to-definition). The cascade is *precise* for the statically
   resolvable subset (qualified or unambiguously-named procs); for the dynamic
   remainder it falls back to the conservative filter (a name that disappears from
   the project surfaces W123 on the dependent's next analysis, as today). The track
   promises **incremental, no-worse-than-today** cross-file diagnostics with precise
   invalidation for the resolvable subset — not sound cross-file arity where Tcl
   semantics forbid it.

6. **Fan-out is correctness, not a bug — but must be cheap per dependent.**
   Changing a widely-called utility's signature legitimately re-checks every caller
   across the project. The firewall keeps each dependent's recompute to its
   *arity/resolution* layer (cheap), not its whole analysis, and only when the
   *signature* (not the body) changed. Debounce + salsa cancellation (already in the
   server) absorb edit bursts; a keystroke that does not alter a signature wakes
   nobody.

7. **Correctness gate: a multi-file differential fuzzer.** Extend
   `incremental == fresh` to *project* scope — a corpus of edit sequences that
   include cross-file signature changes, proc add/remove, and `source` /
   `package require` graph edits — asserting the incrementally-maintained project
   diagnostics equal a from-scratch project rebuild. This is the permanent backstop,
   mirroring the single-file fuzzer.

## The work to do — SRV-INCREMENTAL tasks

Ordered so each ships independently green and the cheap, high-leverage wins land
first. Every incremental path is gated by its differential fuzzer
(`incremental == fresh`); item-locality is never a correctness assumption.

1. **Persisted incremental `LineIndex` on the `String` store** *(S, no rope, do
   first).* Hold the `LineIndex` beside `DocumentState.text`; patch it in place on
   edit (shift line-starts past the splice; add/remove entries for the `\n` delta)
   instead of rebuilding per change; reuse it for `lift_span` / position lookups.
   ~0 memory cost. *Gate:* patched index byte-identical to a rebuilt one over an
   edit-fuzz corpus.

2. **Per-procedure `run_all_checks` / `optimise_unit` memo** *(L — the prize).*
   Wrap the per-function check + optimiser output in a salsa query keyed on the
   offset-invariant `FnLatticeKey` (mirroring `function_lattice` / `taint_cascade`),
   so an unedited proc's checks are a cache hit and only the edited proc's checks
   recompute. Split the interprocedural checks (interproc taint, iRules flow) behind
   per-proc summary firewalls — the `taint_cascade` pattern, already proven. Attacks
   the ~405 ms `run_all_checks` slice (~99% of the ~411 ms warm per-edit on
   `linalg.tcl`). Consumers already take
   `FunctionUnit` with `base_offset` (Approach B), so emit at offset 0 and rebase at
   aggregation. *Gate:* incremental check/optimiser output identical to the
   whole-unit pass over the corpus + edit-fuzzer.

3. **Approach A — incremental per-item IR lowering / CFG** *(L).* Per
   [`../rust/incremental-analysis.md`](../rust/incremental-analysis.md): lower per
   item-body keyed on offset-0 body text. Blocked today because whole-module passes
   (`specialise_factories`, `inline_uplevel_passthrough`, `extract_oo_methods_pass`,
   `populate_trace_facts`) make one body's IR depend on the others; resolve with the
   same "cross-item facts as inputs" split the analyser walk used. Attacks the
   ~59 ms lowering floor.

4. **Approach B follow-ups** *(M).* Per the same doc: remove the per-proc
   deep-clone, and add the per-function `optimise_unit` memo (needs
   `PassContext.interproc` to become `Arc` first — the naive memo was reverted for a
   perf regression). Folds into Task 2's optimiser half.

5. **Wire the structural-state index into the live re-lex path** *(M).* Bound
   `did_change`'s re-lex / re-segment (and `item_tree`'s structure extraction) to
   the dirty span via the already-built `reparse_window` / `script_is_complete` /
   bracket-brace-paren indexes in `tcl-lexer` (test-only today; the server's own
   comment flags this a documented follow-up). Modest absolute ms — re-lex is tens
   of µs–ms — but it removes the whole-file segmentation floor and is the substrate
   the rope (Task 7) needs to pay off.

6. **Cross-file cascade** *(XL).* Lift the project signature table into salsa
   (`project_signatures` over per-file `file_decls`); make cross-file resolution +
   arity tracked queries; get reverse-dependency invalidation for free; apply the
   E4/E8 input-setting discipline project-wide. Brings cross-file arity / W123 onto
   the incremental graph with precise invalidation for the resolvable subset and a
   conservative fallback for dynamic dispatch. *Gate:* the multi-file
   `incremental == fresh` differential fuzzer.

7. **(Optional, late) rope-backed store + chunk-addressable salsa input** *(XL,
   gated).* The demoted SRV-ROPE work — full sub-task breakdown and measurements in
   the experiment below. Justified **only after** Tasks 2–6, and **only if** the
   apply-side 0.02% slice has grown into a non-trivial share once the analysis floor
   is gone *and* the many-small-docs memory regression (1.4–1.9×) can be held under
   ~1.2×. Sub-tasks: feature-flagged rope `DocumentState` with burst-coalescing;
   `LineIndex::from_rope_slice` + `Lexer::with_source_map` rope-slice re-lex in
   `tcl-lexer`; chunk-addressable `SourceFile` input so `set_text` interns only
   changed chunks; MVCC write-window minimisation. The experiment is the gate.

**Benches & gates** *(S, throughout).* Fold `tail_profile` into a committed
per-edit bench: assert **no time-to-first-tokens regression** (the paramount
metric is a full-buffer `didOpen`, which none of this touches) and track warm
per-edit latency on the corpus task-by-task as each lever lands.

**Ordering rationale / exit criteria.** Tasks 1–2 deliver the bulk of the realistic
per-edit win (cheap apply win + the dominant `run_all_checks` slice) with no rope
and no cross-file work. 3–4 close the lowering floor. 5 unlocks bounded re-lex (and
the rope). 6 is the cross-file feature, built incremental-first. 7 (the rope) lands
only if its slice has grown measurable and the memory regression is contained —
otherwise the `String` store is retained. The experiment is the gate, not an
assumption.

## Experiment (evidence)

The two harnesses in this directory measured the SRV-ROPE decision and remain the
evidence for *why this track is incremental analysis, not a rope*. Harness (a)
depends on the **production** `tcl-lexer::LineIndex` and `ropey` 1.6; all inputs
are ASCII (byte == char == UTF-16 unit) so both arms do the same logical work,
isolating the structural difference. Numbers are indicative ratios from one dev-box
run, not absolutes.

### Edit application — ns per `didChange` carrying B edits (harness a)

Rope persists across edits; `flatten` is the `Rope::to_string()` the salsa input
forces; `rope_full = rope_edit + flatten`.

| size  | B  | string (ns) | rope_edit | flatten | rope_full | full ÷ string |
|------:|---:|------------:|----------:|--------:|----------:|--------------:|
| 1KiB  | 1  |         627 |       421 |     157 |       578 | 0.92× |
| 16KiB | 1  |       8 664 |       972 |     824 |     1 796 | 0.21× |
| 16KiB | 64 |     575 298 |    16 727 |     841 |    17 568 | 0.03× |
|256KiB | 1  |     274 556 |     1 225 |  10 097 |    11 322 | 0.04× |
| 1MiB  | 1  |   1 313 611 |     1 355 |  72 101 |    73 456 | 0.06× |
| 4MiB  | 1  |   7 375 485 |     1 527 | 353 275 |   354 802 | 0.05× |

The rope wins on apply ≥16KiB (the win is avoiding the `LineIndex` rebuild +
double-alloc, not the splice), and 20–500× on bursts (B=64; editors rarely send
burst `contentChanges`). But this is **apply machinery**, and apply is ~0.02% of
per-edit latency (above).

### High edit rate — 500 sequential single-edit `didChange`s (total ms, harness a)

| size  | string (ms) | rope_full (ms) | speedup |
|------:|------------:|---------------:|--------:|
| 16KiB |         4.5 |            0.9 |   5.1× |
|256KiB |        70.0 |            5.7 |  12.4× |
| 1MiB  |       301.6 |           37.1 |   8.1× |

5–12× on apply+flatten sustained — invisible end-to-end while `run_all_checks`
dominates per-edit latency.

### Memory — many small open documents (heap bytes, harness a)

| N    | file  | strings | ropes | rope ÷ string |
|-----:|------:|--------:|------:|--------------:|
| 1000 | 2KiB  |    1MiB |  2MiB | 1.43× |
| 5000 | 1KiB  |    4MiB |  9MiB | 1.90× |
|  200 | 16KiB |    3MiB |  3MiB | 1.02× |

The rope's B-tree leaf chunks cost **1.4–1.9× memory for small documents** — a real
downside for a workspace of many small iRules / config snippets, and one a `String`
store does not pay. This is the regression Task 7's gate must hold under ~1.2×.

### Why the rope cannot make salsa incremental

A rope **cannot** change the analysis floor. `set_text` interns a `String`, bumps
the input revision, and invalidates dependents regardless of how the buffer is
stored; the rope must *flatten* (O(n)) before every `set_text`. Real
incrementality requires the **input itself** to be chunk-addressable (Task 7) so
salsa interns unchanged chunks and the lexer re-lexes only the dirty span — and
even then it only attacks re-lex (tens of µs–ms), not `run_all_checks` (hundreds of
ms). The rope is the last and smallest lever; this track spends the first and
largest ones first.
