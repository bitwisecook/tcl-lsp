# Rust LSP server — deep review (2026-06-22)

> Independent, point-in-time deep review of the **native Rust LSP server**
> and its supporting crates, focused on the LSP surface: `tcl-lsp-server`,
> `tcl-lsp-core`, `tcl-lsp-db`, and the `tcl-lsp-py` bindings. Reviewed on
> branch `claude/exciting-planck-q7rj94` against the working tree at the
> date above.
>
> This complements the earlier workspace-wide
> [`review-findings.md`](review-findings.md) (commit `1abc0d35`, #542) and the
> maintainers' own **SRV-LSP** / **SRV-ROPE** tracking in
> [`../../rust-rewrite.md`](../../rust-rewrite.md#remaining-work). Where the prior
> review's headline findings have since been fixed, that is stated explicitly so
> the two documents reconcile rather than duplicate.
>
> **Priority order** (the maintainers'): correctness and precision first,
> performance second (time-to-first-semantic-tokens as the headline latency
> metric), memory a distant third. Findings are ordered to match.
>
> **Method.** The 8,818-line `tcl-lsp-server/src/lib.rs` was read directly;
> the supporting crates (`tcl-lsp-core`, `tcl-lsp-db`, `tcl-lsp-py`) and the
> test/CI surface were each deep-reviewed in parallel. Every finding carries a
> `file:line` anchor verified at review time.

## Verdict

This is a **mature, disciplined, high-quality** server. The bar is high and
mostly met:

- `unsafe` is forbidden workspace-wide; clippy pedantic is on.
- **Production server code contains no `.unwrap()` / `.expect()` / `panic!`**
  outside its `#[cfg(test)]` module — every one of the ~40 anchors is in tests.
- The async layer is careful: a documented `documents → workspace_index` lock
  order, no lock held across a heavy `.await`, a monotonic per-document
  `revision` guard, a coalescing debounced diagnostics scheduler, and salsa
  cancellation on superseding edits.
- Nearly every CPU-bound handler runs on `spawn_blocking`, so a parser panic
  is contained as a JSON-RPC error rather than unwinding the event loop.
- ~666 inline tests in `tcl-lsp-core`, 72 in `tcl-lsp-server`, plus a real
  (if unprovisioned) differential-parity strategy against Python and `tclsh`.

**The prior review's three correctness headlines are resolved.** This is worth
recording up front:

| Prior finding | Status now | Evidence |
|---|---|---|
| **C1** — byte columns, not UTF-16 | **Resolved, both directions.** Output lifts go through `position_at_utf16` (`lib.rs:5444`); the `did_change` **input** path decodes ranges with `offset_at_utf16` (`apply_content_change`, `lib.rs:5424-5426`); `position_encoding` is negotiated and advertised (`lib.rs:2949`, `:6167`). Core providers have **zero** byte `position_at` call sites (57 UTF-16 sites); the old `.chars().count()` token-length bug is gone (`utf16_len`). | verified |
| **C2** — no document-version guard | **Resolved.** `DocumentState` carries `revision`/`version` (`lib.rs:124-131`); workers re-check `doc.revision == revision` before insert+publish (`lib.rs:506`); `publish_diagnostics` forwards the version. | verified |
| **C3** — inline handlers don't contain panics | **Resolved.** All eight flagged handlers (folding, document_symbol, semantic tokens full/delta/range, document_link, formatting, range_formatting) now run on `spawn_blocking` with explicit "review-findings C3" comments. | verified |

So the findings below are **not** the prior ones rehashed. They are a fresh set,
weighted toward the parts of an async LSP server that are hardest to get right:
failure handling, realised (vs theoretical) incrementality, residual encoding
edges, and the safety net.

---

## Correctness and robustness

### F1 — Diagnostics worker treats a deterministic query **panic** as a cancellation, and retries it forever (High)

This is the headline server-level defect. The diagnostics worker cannot tell a
salsa *cancellation* (a superseding edit — retry is correct) from a *panic* in
the analysis pipeline (deterministic for that document — retry is a livelock).

`run_diagnostics_core` wraps the salsa read in `salsa::Cancelled::catch`, which
catches **only** the cancellation sentinel and re-panics everything else; the
re-panic unwinds inside `spawn_blocking`, whose `JoinError` is then folded into
the *same* arm as a clean cancellation:

```rust
// rust/tcl-lsp-server/src/lib.rs:404-417  (and identically :438-446)
match tokio::task::spawn_blocking(move || {
    salsa::Cancelled::catch(|| {
        tcl_lsp_db::file_analysis_incremental(&snapshot, file, config)
    })
    .ok()
})
.await
{
    Ok(Some(analysis)) => analysis,
    // Cancelled mid-read (a concurrent `set_text`) **or the worker panicked**
    // — don't publish; signal a retry of the document's latest state.
    Ok(None) | Err(_) => return false,
}
```

`return false` propagates to the scheduler, which re-marks the slot dirty and
loops:

```rust
// rust/tcl-lsp-server/src/lib.rs:2816-2823
let settled = run_diagnostics_core(inputs.clone(), &uri, job).await;
if !settled {
    // Cancelled mid-flight — re-mark dirty so we retry the latest state.
    if let Some(slot) = slots.lock().await.get_mut(&uri) { slot.dirty = true; }
}
```

**Impact.** If any document deterministically panics
`file_analysis_incremental` or `compiler_check_diagnostics` (the salsa review
notes these bodies call `analyse_per_item_with`, `build_cfg_function_with_upvars`,
`run_all_checks`, …, which *can* panic on adversarial input — the very reason C3
hardening exists elsewhere), the worker spins on a 50 ms debounce **forever**:
re-analyses, panics, swallows the `JoinError`, re-marks dirty, sleeps, repeats.
Consequences: (1) a CPU pinned at ~20 failed analyses/second until the document
changes or closes; (2) diagnostics for that document are **never** published;
(3) the panic payload is discarded (`Err(_)`), so there is no LSP-level log —
only raw `spawn_blocking` panic spam on stderr. Cancellation and panic need
different handling: on `JoinError::is_panic()`, log once and treat the run as
**settled** (publish empty or fall back to the uncached direct `analyse`), never
retry.

### F2 — Two reachable panics in `minify.rs`, one of them on the inline command path (Medium)

Both are reachable from input a user can plausibly hold, and both are a
one-line guard fix.

**F2a — UTF-8 char-boundary panic on an unbalanced `[` before a multibyte char.**

```rust
// rust/tcl-lsp-core/src/minify.rs:2820
let inner = &text[start + 1..i.saturating_sub(1).max(start + 1)];
```

When the `[` scan runs to EOF (`i == n`, unbalanced bracket) and the source ends
in a multibyte character, the slice end lands **mid-codepoint**. For `expr {[é}`
the inner text `[é` reaches `tokenise_expr`, giving `&text[1..2]` — byte 2 is
inside `é` → panic in *both* debug and release (a char-boundary violation is a
hard panic, not an overflow). It is **contained**: `minify_document_command`
runs the call on `spawn_blocking` (`lib.rs:1791`), so the user sees a "minify
worker panicked" JSON-RPC error rather than a crash — but `minifyDocument` fails
on input it should simply minify. Fix: clamp the end with `is_char_boundary` (or
floor it to `start + 1` when `depth` never closed).

**F2b — `usize` underflow on a `line 0` error reference, on the inline path.**

```rust
// rust/tcl-lsp-core/src/minify.rs:348-356
let map_line = |line_no: usize| -> Option<usize> {
    if line_no > min_commands { return None; }      // guards high, not zero
    ...
    let idx = ((line_no - 1) * last / denom).min(last);   // line_no == 0 → underflow
    Some(orig_non_empty[idx])
};
```

`line_no == 0` passes the guard, then `line_no - 1` underflows. Reached from
`unminify_error` / `remap_line_references` on an error string such as
`(procedure "f" line 0)` — i.e. **untrusted Tcl/iRule runtime error text**. In
debug this panics; in release it wraps to `usize::MAX`, the multiply overflows
(wrapping), and `.min(last)` yields an in-bounds but **wrong** line mapping —
a silent garbage answer. Unlike the minify path, `unminify_error_command` is
dispatched **inline** in `execute_command` (`lib.rs:4666`,
`Ok(Self::unminify_error_command(...))`), with no `spawn_blocking` guard. Fix:
`if line_no == 0 || line_no > min_commands { return None; }`.

### F3 — `signature_help` adds a UTF-16 column to a byte offset (Medium, precision)

This is the **only** place in `tcl-lsp-core` that still mixes the two encodings,
and it is on the input side, so the UTF-16 work elsewhere doesn't cover it.

```rust
// rust/tcl-lsp-core/src/signature_help.rs:170-183
... line_start.saturating_add(character).min(line_end).min(source_len) ...
```

`character` is an LSP UTF-16 column; `line_start` is a **byte** offset. On any
line with non-ASCII text before the cursor the resulting `cursor_offset` is too
small, so the active command segment and active-parameter index can be wrong —
the signature popup highlights the wrong parameter. It cannot panic (the offset
is only compared against spans). Fix: resolve the cursor with `offset_at_utf16`,
exactly as the neighbouring `byte_offset_at` path already does.

### F4 — Cold/fallback analysis diverges from the warm salsa config (Medium)

When the salsa `file_analysis` query misses (input not yet set, or cancelled),
the server falls back to building an analyser from a **different** config source
than the salsa path, so the same document can yield a different diagnostic/symbol
set depending on which path served it:

- The warm path keys on the salsa `db_config` (per-folder override resolved in
  `capture_job`, `lib.rs:292-298`).
- The diagnostics fallback builds from `self.analyser_config()` (`lib.rs:419-428`)
  — a different lock, not guaranteed to carry the folder override.
- The references / call-hierarchy / reindex fallbacks use a bare
  `Analyser::new()` (e.g. `lib.rs:2908` in the workspace scan, and the
  cross-document index paths), which **ignores disabled-diagnostics entirely**.

Mostly benign for index population; observable as an inconsistent diagnostic set
immediately after open or under folder-scoped W-code suppression. Route the
fallbacks through the same `configured_analyser(disabled, mode)` the warm path
uses.

### F5 — Residual latent panics / unchecked indices (Low; all currently contained)

None are reachable today, but each is an unchecked invariant in production code
that a refactor could arm. Listed because the rest of the codebase is held to a
no-panic standard.

| Anchor | Issue |
|---|---|
| `refactor/mod.rs:447` | `&ln[min_indent..]` byte-slices a min-indent width off *every* body line; a line whose first `min_indent` bytes contain a multibyte char splits mid-codepoint. Reachable from `if→switch` / extract-to-datagroup refactors on adversarial bodies. |
| `formatting/mod.rs:73` | `position_at_utf16(u32::try_from(source.len()).unwrap_or(0), …)` — on >4 GiB overflow the fallback `0` ends the whole-document edit at `(0,0)`, deleting the document. The sibling at `mod.rs:174` correctly uses `unwrap_or(u32::MAX)`; the two disagree. |
| `tcl-lexer/.../line_index.rs:174` | `position_at_utf16` is the single column-conversion choke point and its contract is "panic on a mid-char offset" (it clamps to `len` but does not snap to a boundary, unlike `offset_at_utf16`). Every provider funnels through it; an `is_char_boundary` clamp would make the choke point panic-proof. |
| `code_actions.rs:1436` | UTF-16 diag columns used directly as `chars` indices (no `utf16_col_to_char_col`); `.min(chars.len())` prevents a panic but the extracted `data_event` title substring is wrong when astral chars precede it. Low blast radius (title string only). |
| `lib.rs:5172` | `wrapped[p]` indexes `selection_range` `parent_index` unchecked; safe only because the core guarantees `parent_index < len`. Contained by `spawn_blocking` (`lib.rs:4789`); `.get(p)` is free insurance. |

---

## Incrementality — realised vs theoretical

The salsa engine in `tcl-lsp-db` is genuinely well-built (offset-invariant
per-item keys, proven early-cutoff, a shared `compilation_unit` seam, a sound
concurrency model). The gap is how little of that the **server** actually
exercises on the hot path.

### F6 — The per-item firewall is bypassed by every feature except diagnostics (High value)

```rust
// rust/tcl-lsp-db/src/lib.rs:117-127
pub fn file_analysis(db, file, config) -> Arc<AnalysisResult> {
    let mut analyser = Analyser::with_disabled_diagnostics(disabled)...;
    Arc::new(analyser.analyse(file.text(db), file.dialect(db)))   // whole-file walk
}
```

`file_analysis` (whole-file) and `file_analysis_incremental` (per-item, memoised)
are two distinct tracked functions. Only the **diagnostics** path calls the
incremental one (`lib.rs:406`). Every other feature — hover (`:5088`), definition
(`:1330`), references, completion, call-hierarchy, and `document_symbols`
(which reads `file_analysis`, db `lib.rs:713`) — goes through `db_file_analysis`
→ `file_analysis`, whose only input is the file text, so a single-character body
edit invalidates it **wholesale**. The per-item memoisation never helps them.

Compounding it (**F6b**), because the server demands `file_analysis` (features)
and `file_analysis_incremental` (diagnostics) in the *same* revision after one
`did_change`, the document is walked by the analyser **twice** per edit. The
corpus gates already prove `*inc == *full` (`file_analysis_corpus.rs:80`), so the
safe consolidation is to route `document_symbols` and the feature path through
`file_analysis_incremental` and retire `file_analysis`.

### F7 — `cached_analysis` deep-clones the whole `AnalysisResult` per feature request (Medium)

```rust
// rust/tcl-lsp-server/src/lib.rs:1028-1030
async fn cached_analysis(&self, uri: &Url) -> Option<AnalysisResult> {
    self.db_file_analysis(uri).await.map(|a| (*a).clone())   // full deep clone
}
```

`file_analysis` returns `Arc<AnalysisResult>` precisely so reads bump a refcount
(db `lib.rs:113`). `cached_analysis` immediately `(*a).clone()`s the entire
structure (procs map, classes map, diagnostics, scopes) to hand an owned value to
`spawn_blocking`, so every hover/definition/reference/completion pays an O(file)
deep copy. Move an `Arc<AnalysisResult>` into the worker and deref inside; most
core providers already take `&AnalysisResult`.

### F8 — `did_change` still re-analyses the whole document (tracked: SRV-ROPE)

`did_change` applies the incremental content edits but then re-runs whole-document
analysis (`lib.rs:3031-3035`, comment: *"the re-analysis below is still
whole-document"*). `DocumentState.text` is a `String` re-cloned on every edit, and
the salsa input interns a fresh `String`. This is the maintainers' known-open
**SRV-ROPE** item (`../../rust-rewrite.md` marks the document store 🔴 XL) — noted
here only for completeness and to connect F6/F7 to its root.

---

## Concurrency and performance (TTFST first)

The cold-start path is much improved over the prior review: the workspace scan
now runs off-loop on `spawn_blocking` and snapshots its locks first
(`lib.rs:2874-2917`), the per-dialect registry is built once and shared as a
durable db value (`registry_for_dialect`, `lib.rs:2613`), and diagnostics are
debounced (50 ms) and coalesced. The residual costs:

### F9 — `workspace_index` is a `Mutex` deep-cloned per completion/code-lens/symbol request (Medium)

```rust
// rust/tcl-lsp-server/src/lib.rs:3388, :4395, :4467
let workspace = self.workspace_index.lock().await.clone();   // full deep clone into the worker
```

Completion, code-lens, and workspace-symbol each deep-clone the entire
cross-document index to move it into `spawn_blocking`. And `remove_document`
(run on **every** `did_change`) is O(total index size): four full `Vec::retain`
passes (`workspace_index.rs:179-184`), with `document_uris()` doing
collect+sort+dedup over all vectors (`:264-274`). An `Arc<WorkspaceIndex>` +
`RwLock`, keyed by URI (`HashMap<uri, …>` buckets → O(1)/doc removal), removes
both the per-request clone and the per-keystroke retain storm.

### F10 — Workspace scan is serial and rebuilds the registry per file (Medium, TTFST)

```rust
// rust/tcl-lsp-server/src/lib.rs:2896-2910
for path in files {
    ...
    let mut analyser = Analyser::new();          // rebuilds the ~560-spec registry per file
    let analysis = analyser.analyse(&text, &dialect).clone();
}
```

Up to `WORKSPACE_SCAN_FILE_CAP` (2,000) files are analysed in one serial loop on
a single blocking thread, each constructing a fresh `Analyser` (so the shared
`registry_for_dialect` cache is bypassed in exactly the path that does the most
analyses), with no `rayon`. The scan is `await`ed in `initialized`. Thread the
shared registry into the scan and `par_iter` the file loop.

### F11 — Semantic-tokens delta is nominal: always returns the full set (Low)

The capability advertises `delta: true` (`lib.rs:6318`), but
`semantic_tokens_full_delta` always returns the **full** token stream
(`lib.rs:4151-4156`), with a freshly minted `result_id` each call and no
`result_id → tokens` cache, so a client's `previousResultId` can never match.
Spec-legal (the comment notes `Tokens` is accepted in place of `TokensDelta`),
but it defeats delta's bandwidth purpose — every edit re-ships every token. Either
keep a one-entry per-URI snapshot to diff against, or stop advertising delta.

### F12 — Providers rebuild `LineIndex` / re-lex per request (Medium, sustained)

Across `tcl-lsp-core`, `byte_offset_at` rebuilds a fresh `LineIndex::new(source)`
on every call (`definition.rs:325`), and many providers build one locally *and
then* call `byte_offset_at` 1–3 more times (`references.rs:125,128,189,239`;
`hover.rs:126,207`; `declaration.rs`, `implementation.rs`, `type_definition.rs`,
`linked_editing_range.rs`, …). Worse, several providers re-lex/re-segment the
whole buffer per recursion level — `refactor/mod.rs:228` (`find_command_at_inner`
re-segments at every level), `formatting/engine.rs:1104` (`format_body` re-lexes
per brace depth), `code_actions.rs:956` (rebuilds the registry **and** loads the
iRules dialect on every code-action request). These are the per-keystroke /
per-cursor-move sustained costs; threading one `LineIndex` and one CST per request
is the structural fix (the prior review's "one tree, reused" convergence).

---

## PyO3 bindings (`tcl-lsp-py`)

The binding crate is a clean, panic-safe, pyo3-only boundary (errors translate to
`PyValueError`; no input-reachable panics; integer widths match the core exactly).
Two structural issues:

### F13 — Zero `Python::allow_threads`: the GIL is held across all Rust compute (High, for threaded callers)

There is **no** `allow_threads` call site in the entire crate. Every
`#[pyfunction]` that does real work — `analyser_analyse` (`analyser.rs:60`),
`compiler_checks_run_all` (`compiler_checks.rs:51`), the CU-building functions
(`compilation_unit.rs`, `interprocedural.rs`, `gvn.rs`, `optimiser.rs`) — runs the
full analyser/IR/CFG/SSA pipeline **GIL-held**. A multi-threaded Python LSP
analysing several documents in a pool gets zero Rust parallelism. Wrap the pure
compute in `py.allow_threads(|| …)` (copy `&str` args to owned first). Highest-value
lever in the crate.

### F14 — Hand-rolled `*_to_dict` conversion drifts; 2 of ~38 providers exposed (Medium)

`analyser.rs:70-486` alone is ~20 hand-written `*_to_dict` functions that
serialise the recursive `AnalysisResult` shape key-by-key, with no compiler check
that they stay in sync with the core types (the module docstring literally tracks
drift "by C-number"). `registry.rs:119-131` decodes role strings by hand and
**silently returns `Vec::new()` on an unknown role**; `analyser.rs:204-209`
expands a bitfield flag-by-flag (a 7th flag would be silently dropped). Meanwhile
`features/mod.rs:15-23` exposes only **folding + document symbols** of the ~38
core providers, so the bindings are a *second* serialisation of core that lags the
server's 16 `lift_*` functions. Deriving `serde::Serialize` on the core types +
`pythonize` would delete most of `analyser.rs`/`signature_scan.rs` and make field
additions automatic. (See also the prior review's PyO3 layering section.)

---

## Testing and CI — the safety net has a hole

### F15 — The `main`-branch CI runs **no** `cargo test`; differential oracles are unprovisioned (High)

- `.github/workflows/ci.yml` (the pipeline for `main`) runs `make ci-fast`
  (Python lint/typecheck + the LSP-e2e pytest subset) and, tag-gated, a
  `cargo build`. It never runs `cargo test`. `grep -r 'cargo test' .github/` →
  only `rust-gate.yml:48`, which triggers **only on the `rust` branch**.
- No workflow installs `tclsh9.0` or fetches the `tmp/tcl*` source trees
  (`grep -r tclsh .github/` → no matches). So every oracle-backed differential
  harness **silently skips** (`eprintln!` + `return;` → the test passes):
  `differential_fold.rs:245` (no `tclsh9.0`), the `differential_segment` corpus
  sweep (`:228`, no `tmp/tcl*`), and `differential_incremental.rs:113` is
  `#[ignore]` so it never runs without `--ignored`.

**Net:** on `main`, the entire Rust suite — the 7 server smoke tests, ~738 inline
tests, and all parity differentials — is neither compiled nor run in CI. On the
`rust` branch, `cargo test` runs but the `tclsh`- and corpus-backed differentials
still skip. The parity guarantee rests on a developer running `make test-slow`
locally. At minimum, run `cargo test --workspace` on `main` PRs and provision the
oracles so the differentials **fail-not-skip**.

### F16 — e2e coverage is 7 shallow happy paths; the high-risk async behaviours are untested e2e (Medium)

The 7 smoke tests (`tcl-lsp-server/tests/*_smoke.rs`) genuinely drive the real
`tower-lsp` Backend over JSON-RPC, but each is a single `.contains(...)` assertion
on minimal **ASCII** input for one read-only feature. None of these are exercised
over the wire: concurrent `did_change` ordering / version races (the guard is only
unit-tested *sequentially*, `lib.rs:7853`), position encoding on **non-ASCII**
input (every smoke fixture is ASCII — a server-side span→Position regression on
multibyte text would pass CI), panic-safety on malformed/truncated/huge payloads,
request cancellation (`$/cancelRequest`), and live config toggles via
`didChangeConfiguration`. These are precisely the surfaces F1–F4 live on. An
e2e test that opens a multibyte document and round-trips a hover/definition
position would have caught F3.

---

## Maintainability notes (Low)

- **F17 — Latent salsa soundness trap.** Tracked queries read a
  `Mutex<HashMap>` registry cache (`tcl-lsp-db/src/lib.rs:69-87`, read at `:298`,
  `:500`, `:719`). Sound *today* only because the registry is process-immutable;
  salsa cannot track the read, so if the registry ever depends on a salsa input
  (user-configurable specs, workspace-loaded dialect packs) and the map mutates,
  tracked queries would serve stale results with no revision bump. The invariant
  is load-bearing but enforced only by a comment — seal it behind a build-once API
  or a debug-assert.
- **F18 — Stale doc-comments contradict correct code.** `hover.rs:104-111` claims
  it "treats character as a char-count index … can drift," but the code does
  proper UTF-16 conversion; `document_symbols.rs:83-95` documents "byte-column
  positions" while `span_to_range` emits UTF-16. A private `span_to_range` is
  duplicated across ~6 providers (all correct, but drift-prone). These mislead the
  next reader on exactly the axis the team just fixed.
- **Interned-key growth.** salsa interned keys (`ItemBodyKey`, `FnLatticeKey`, …)
  are never GC'd (`tcl-lsp-db/src/lib.rs:175,253`); a long editing session over a
  large file accumulates one-shot interned keys monotonically. Slow creep, not a
  leak — worth a bounded-interning note for a long-lived server process.

---

## Prioritised recommendations

**Correctness — do first**
1. **F1**: split cancellation from panic in the diagnostics worker — on a panicked
   `JoinError`, log once and mark settled; never retry. (Highest-severity server bug.)
2. **F2a/F2b**: add the two one-line guards in `minify.rs` (char-boundary clamp;
   `line_no == 0` rejection) and add a non-ASCII + `line 0` regression fixture.
3. **F3**: resolve the `signature_help` cursor via `offset_at_utf16`.
4. **F4**: route cold/fallback analyses through the same configured analyser as the
   salsa path.

**Performance — TTFST then sustained**
5. **F6/F7**: consolidate on `file_analysis_incremental` (retire the whole-file
   `file_analysis`); stop deep-cloning `AnalysisResult` — move an `Arc` into the worker.
6. **F9/F10**: `Arc<WorkspaceIndex>` + `RwLock` keyed by URI; thread the shared
   registry into the scan and `par_iter` it.
7. **F12**: thread one `LineIndex`/CST per request instead of rebuilding per call.

**Safety net**
8. **F15**: run `cargo test --workspace` on `main` and provision the differential
   oracles (fail-not-skip).
9. **F16**: add e2e tests for concurrent `did_change`, non-ASCII positions, and
   malformed input.

**Lower priority**
10. **F13/F14** (PyO3): `allow_threads` the heavy bindings; migrate the
    `*_to_dict` cascade to `serde` + `pythonize`.
11. **F5, F11, F17, F18**: residual panics, nominal delta, the salsa soundness
    seal, and the stale comments.

## Related

- [`review-findings.md`](review-findings.md) — the earlier workspace-wide review
  (C1/C2/C3 there are resolved; see the table above).
- [`../../rust-rewrite.md`](../../rust-rewrite.md#remaining-work) — SRV-LSP
  (landed) / SRV-ROPE (open) tracking.
- [`lsp-performance.md`](lsp-performance.md) — the Python-vs-Rust benchmark record.
- [`current-architecture.md`](current-architecture.md) — crate graph and runtime model.
