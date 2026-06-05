# Rust workspace review — correctness, performance, memory

> Point-in-time review of the Rust workspace at commit `1abc0d35`
> (#542), reconciled against `origin/rust` #543. Every finding below
> was verified against the source at review time; file:line anchors
> are given so each can be re-checked or retired as the code moves.
>
> **Priority order (set by the maintainer):** correctness and
> precision first, performance second — with **time to first
> semantic tokens** called out as the headline latency metric — and
> memory a distant third. The sections are ordered to match.

## Verdict

The workspace is disciplined: `unsafe` is forbidden workspace-wide,
clippy pedantic is on, there are ~2,400 inline tests, the async layer
holds no lock across `.await`, and the crate graph is a clean acyclic
layering. The differential-parity strategy against Python and `tclsh`
is the right way to protect a port.

The issues are not rot. They are a small number of structural choices
plus two concrete precision defects:

1. **LSP positions are emitted in byte columns, not UTF-16** — a
   confirmed spec violation that produces wrong ranges (and can
   corrupt files via rename) on any non-ASCII line.
2. **No document-version guard** — stale analyses and diagnostics can
   overwrite fresh ones out of order.
3. **The differential safety net is not enforced in CI** — the very
   net that would catch a regression silently skips on the hosted
   runner.
4. **Nothing is shared or cached across the pipeline** — the full
   analysis pipeline and the command registry are rebuilt from
   scratch on every keystroke; this is the dominant performance cost.

## Correctness and precision

### C1 — Positions use byte columns, not UTF-16 (confirmed bug)

The LSP specification defines `Position.character` as a UTF-16
code-unit offset unless the server negotiates another encoding.

- `build_server_capabilities()` (`rust/tcl-lsp-server/src/lib.rs:2825`)
  never sets `position_encoding`, so the UTF-16 default applies.
- Every span→position conversion calls `LineIndex::position_at`,
  which returns a **byte** column
  (`rust/tcl-lexer/src/line_index.rs:92`). There are ~50 such call
  sites across `references`, `rename`, `folding`, `code_lens`,
  `inlay_hints`, `semantic_tokens`, `linked_editing_range`,
  `formatting`, and the server's cross-document range lifting.
- `LineIndex::position_at_utf16` — the LSP-correct variant, fully
  implemented and unit-tested (`line_index.rs:134`) — has **zero**
  callers.
- Semantic tokens compounds it: token length is `text.chars().count()`
  (`rust/tcl-lsp-core/src/semantic_tokens.rs:379`), a Unicode scalar
  count, which is neither bytes nor UTF-16 units (astral characters
  are two UTF-16 units).

**Impact.** On any line containing a multi-byte character (UTF-8
string literals, accented or CJK comments, iRules log strings, Tcl 9
Unicode identifiers), every column and length to its right is wrong:
highlight drift, mis-placed hovers and diagnostics, wrong
go-to-definition and reference targets, and rename edits applied at
the wrong columns. Latent today only because most Tcl is ASCII.

**Fix.** Route all span→`Position` conversions through a UTF-16
column computation (the primitive already exists) and measure token
length in UTF-16 units. Add a non-ASCII regression fixture. No
differential test covers this today.

### C2 — No document-version guard (confirmed latent race)

`DocumentState` is `{ text, dialect }` only —  no version
(`rust/tcl-lsp-server/src/lib.rs:98`). `did_change` evicts the cached
analysis (`:1278`) then awaits a fresh full analysis (`:1279`);
`publish_diagnostics(uri, diags, None)` passes `None` for the version
(`:1112`). If two `did_change`s for one URI are serviced
concurrently, the **older** analysis can finish last and re-insert its
stale `AnalysisResult` (`:1111`) and publish stale diagnostics after
the newer one, with no version check to reject it. The eviction at
`:1278` prevents serving stale data to interim requests but does not
sequence the workers.

**Fix.** Stamp each document with its LSP version, capture a per-URI
generation counter before `spawn_blocking`, re-check it before the
insert and publish, and pass the version to `publish_diagnostics`.

### C3 — Inline handlers do not contain a parser panic (robustness)

Most handlers offload to `spawn_blocking`, which catches a panic and
returns a JSON-RPC error. Eight handlers run CPU work **inline** with
no catch: `folding_range` (`:1331`), `document_symbol` (`:1342`),
`semantic_tokens_full` / `_delta` / `_range` (`:1851` / `:1876` /
`:1941`), `document_link` (`:2021`), and `formatting` /
`range_formatting` (`:2240` / `:2269`).

The input-facing hot paths are otherwise genuinely hardened: the
lexer clamps span ends so unterminated `{`, `[`, `"`, `${`, `$arr(`
never push `end` past `source.len()`
(`rust/tcl-lexer/src/lexer.rs:787`, `:967`), uses `saturating_sub` for
bracket underflow, and builds no reversed spans; recovery slices are
bounds-checked. So the real residual is narrow:

- the 8 inline handlers should move to `spawn_blocking` for
  defence-in-depth;
- two CST-builder `pop().unwrap()`s on the non-test path are the
  right fuzz target (`rust/tcl-compiler/src/parsing/syntax/build.rs:174`
  and `:178`);
- `LineIndex::new` asserts on sources over 4 GiB
  (`line_index.rs:60`).

### C4 — Comment classification is line-leading-only (precision limit)

`push_comment_tokens` (`semantic_tokens.rs:310`) treats `#` as a
comment only when it is the first non-whitespace character on a line.
Real Tcl also has `;# …` inline comments and `#` at any command
position; those are not highlighted. It mirrors Python's
`_collect_comments`, so it is correct by the project's parity
definition, but it diverges from true Tcl semantics.

### C5 — One deliberate semantic divergence (accepted, tracked)

A quoted word whose entire content is a line continuation
(`"\<newline>"`) is dropped by the Rust segmenter, where Tcl and
`build.py` keep it as an empty-ish word. Labelled `DELIBERATE
DIVERGENCE … tracked as SYNC-JUN08-1`
(`rust/tcl-compiler/src/parsing/syntax/build.rs:240`) and pinned by a
test. Worth a tracking issue so it is not forgotten when the rebase
settles.

### The safety net, and its enforcement gap

The differential design is excellent: three independent oracles — live
Python `core.compiler.codegen`, real `tclsh9.0` folds, and a *frozen*
pre-port segmenter snapshot — with tiered exact / semantic / divergent
classification and accepted divergences pinned as tests.

**But every oracle-backed harness degrades to a silent skip when its
oracle is absent, and the hosted CI `rust` job installs none of them**
(`.github/workflows/ci.yml:68` runs only `cargo test --workspace`; no
project pip-install, no `tclsh`). In CI:

- `differential_codegen` → `import core.compiler.codegen` fails →
  skips (`rust/tcl-compiler/tests/differential_codegen.rs:138`);
- `differential_fold` → no `tclsh9.0` → skips
  (`rust/tcl-registry/tests/differential_fold.rs:245`);
- `differential_segment` corpus → no `tmp/tcl*` trees → skips; only
  its 38 hand-written edge cases run.

So the parity guarantee rests on a developer running `make prep-pr`
in a fully-provisioned environment — green CI proves little about
Python/Tcl parity. And even when run, these harnesses compare
disassembly, fold values, and segmentation; **none exercises LSP
position encoding (C1) or server concurrency (C2)**, which is exactly
why the two highest-impact correctness bugs sit undetected.

The acknowledged gaps elsewhere are overwhelmingly false-negative
biased (`conservative` / decline-to-act), which is the right bias for
a correctness-first posture: the port errs toward silence, not wrong
answers. CRLF and lone-CR handling is correct and regression-tested
(`line_index.rs:34`), having previously produced a backwards range
that was found and fixed (#537).

### Correctness priority

| # | Severity | Finding | Primary evidence |
|---|---|---|---|
| Net | Posture | Differential oracles not provisioned in CI → skip, not fail | `ci.yml:68`; skip paths in the three `tests/differential_*.rs` |
| C1 | Confirmed bug | Byte columns / scalar lengths, not UTF-16 | `line_index.rs:92` vs `:134`; `lib.rs:2825`; `semantic_tokens.rs:379` |
| C2 | Confirmed latent race | No document-version guard | `lib.rs:98`, `:1111`, `:1112` |
| C3 | Robustness | Inline handlers do not contain panics; 2 fuzz targets | `lib.rs:1331…2269`; `build.rs:174`, `:178` |
| C5 | Accepted divergence | Quoted `"\<newline>"` dropped | `build.rs:240` |
| C4 | Precision limit | Line-leading-only comment scan | `semantic_tokens.rs:310` |

## Performance — time to first semantic tokens first

Good news up front: semantic tokens do **not** depend on the analyser
— `core_semantic_tokens::full(text, dialect, registry)` only segments
and classifies (`semantic_tokens.rs:162`), so the heavy
lower→CFG→SSA→passes pipeline is off the TTFST path. The cost is
everything around it.

### Cold-open critical path

1. `initialized` awaits a workspace scan that fully analyses up to
   2,000 files (`WORKSPACE_SCAN_FILE_CAP = 2000`,
   `rust/tcl-lsp-server/src/lib.rs:2734`; awaited at `:1214`) — a
   cold-start CPU storm.
2. `did_open` runs a full analysis for the opened file (`:1234`) —
   needed for diagnostics, not for tokens.
3. The first `semanticTokens` request runs **inline** on an executor
   thread (`:1851`), **cold-builds the ~560-spec registry** on first
   `registry_for_dialect`, then segments the whole document.

### TTFST levers (highest value first)

- **P1 — Pre-warm the token cache at `did_open`.** Tokens are computed
  lazily on first request and nothing pre-computes them
  (`semantic_tokens_cache` is only filled by the token handlers;
  `did_change` / `did_close` only evict). Compute tokens on the
  blocking pool during `did_open` (and on a debounced `did_change`)
  and cache them, so the first request is a cache hit. Biggest direct
  win.
- **P2 — Make the `range` provider actually range-bounded.** `range`
  calls `collect_entries` over the whole document and then `retain`s
  to the viewport (`semantic_tokens.rs:179`), doing no less work than
  `full` — defeating the editor's viewport fast-path. Narrowing it is
  subtler than it looks: `segment_commands_with_offset_and_config`
  only relocates offsets for the text it is handed, so lexing from the
  viewport's start would drop the enclosing-delimiter context and
  misclassify tokens inside a multi-line construct — the second line
  of `set x {line1\nline2}` would be read as code, not string content.
  A correct narrowing must lex from a safe enclosing synchronisation
  point (the start of the enclosing top-level command), not the raw
  viewport offset; until that context-aware slicing exists, the
  full-tokenise-then-filter path is the correct fallback, so P1
  (pre-warming) is the safer TTFST win.
- **P3 — Share / lazy-static the registry.** `Analyser::analyse`
  rebuilds the entire registry inside every call
  (`rust/tcl-compiler/src/analyser/state.rs:359`), ignoring the
  server's per-dialect cache (`lib.rs:1065`). Build once into a
  process-wide `OnceLock` and thread `&CommandRegistry` into
  `analyse(...)`. Removes registry construction from every hot path.
- **P4 — `spawn_blocking` the token handler and de-prioritise the
  startup scan.** Running tokenisation inline blocks an executor
  thread under the startup CPU storm; the 2,000-file scan is pure
  contention for TTFST (tokens need no cross-file index). Make the
  scan lazy or chunked.

### Sustained performance

- **Full recompute every keystroke; no incremental layer.** Each
  `did_change` re-runs the whole pipeline via `Analyser::new()
  .analyse(...)`; the `AnalyserSnapshot` chunked-reanalysis machinery
  is compiler-internal and unused by the server. A query / incremental
  engine (`salsa`, or reuse of unchanged green subtrees from the
  existing CST) is the highest-leverage sustained change.
- **The pipeline is rebuilt 2–3× per document.** The analyser builds
  one `CompilationUnit`
  (`rust/tcl-compiler/src/analyser/diagnostics.rs:1813`), the
  optimiser another (`rust/tcl-compiler/src/optimiser/manager.rs:51`),
  iRules a third (`rust/tcl-compiler/src/irules_checks.rs:464`); taint
  runs 2–3× (`rust/tcl-compiler/src/compilation_unit.rs:110` then
  `:317`). Build it once and share `&CompilationUnit`.
- **No parallelism.** `rayon` is absent; the per-proc
  `FunctionUnit::build` loop (`compilation_unit.rs:225`) and the
  optimiser pass loops are serial despite being independent. `par_iter`
  on the proc-build loop is a large win on big files.
- **No debounce.** Fast typing queues N full analyses; add a debounce
  with cancel-previous (ties to C2).
- **Coarse locking.** Read-mostly maps (`analyses`, `workspace_index`,
  `dialect_registries`) are `Mutex` (reads serialise), and
  `workspace_index` is deep-cloned into the worker on every completion
  (`lib.rs:1371`); `WorkspaceIndex::remove_document` does three full
  `Vec::retain` per keystroke
  (`rust/tcl-lsp-core/src/workspace_index.rs:153`). Use `RwLock` +
  `Arc<WorkspaceIndex>` and key the index by URI.

## Memory (distant third)

Recorded for completeness; pursue only where it reduces allocation
latency on the hot path.

- The green CST stores two owned `String`s per token and shares
  nothing (`#[derive(Clone)]`, no `Rc` / hash-consing) —
  `rust/tcl-compiler/src/parsing/syntax/green.rs:139`. `rowan` exists
  precisely for this.
- No string interner; CFG / SSA key on owned `String` + SipHash
  (`rust/tcl-compiler/src/cfg.rs:104`, `:179`;
  `rust/tcl-compiler/src/ssa.rs:103`). Swapping to `FxHashMap` (and
  eventually `Symbol(u32)`) is a near-zero-risk latency win, which is
  why it earns a mention here rather than purely as footprint.
- `Analyser::snapshot()` deep-clones the whole `AnalysisResult` per
  top-level command (`rust/tcl-compiler/src/analyser/snapshot.rs:97`).

Already good and worth preserving: the codegen `LiteralTable` /
`LocalVarTable` interners, the `Arc<ExprNode>` parse cache, the cached
green `full_width`, and the cheap `Copy` red-layer views.

## Crate split and external-crate leverage

The crate graph (lexer → registry → compiler → core → {server, py} →
alias) is a clean acyclic layering, and the "pure crates plus one
binding crate" rule is genuinely upheld. Two observations:

- **`tcl-compiler` is a 100k-line monolith** spanning parsing, IR,
  CFG, SSA, the dataflow engines, the analyser, the optimiser, and
  codegen. As the port stabilises it wants splitting along its
  existing seams (`tcl-syntax`, `tcl-ir`, `tcl-analysis`,
  `tcl-codegen`), which also unlocks compile parallelism and
  per-layer PyO3 wrapping.
- **Mature ecosystem crates are absent where the workspace hand-rolls
  their domain** (verified: zero in every manifest): `rowan`
  (red-green tree), `salsa` (incremental recompute), `petgraph` (CFG
  algorithms), a string interner, `rustc-hash` / `ahash` (the 555
  `HashMap`s use SipHash), and `ropey` (document text is `String`
  with full re-clone per change). `line-index` / `text-size` are also
  hand-rolled, but those are small and low priority. The CST is a
  faithful red-green split but a heavier realisation than `rowan`
  because it omits `rowan`'s structural sharing — adopting `rowan`,
  or adding `Rc` + hash-consing and source-range token text, is the
  right convergence.

## Separation of concerns and PyO3 layering

The shape is right for per-layer PyO3 — pure crates stay pyo3-free,
and `tcl-lsp-core` uses its own protocol-independent types so the
algorithm has one home. The friction is the conversion strategy:

- **Two parallel hand-written conversion layers** are maintained
  against the same core types: the server's 16 `lift_*` functions
  (core → `lsp_types`) and the bindings' cascade of `*_to_dict`
  functions building a nested `PyDict`
  (`rust/tcl-lsp-py/src/analyser.rs:70`). The latter is hand-rolled
  serialisation; deriving `serde::Serialize` on the core types and
  converting via `pythonize` would delete most of it.
- **The PyO3 surface lags the server.** The bindings expose only
  folding and document symbols among the feature providers
  (`rust/tcl-lsp-py/src/features/mod.rs:15`); the native server
  implements ~43 methods.
- **The GIL is held across all Rust compute** — `allow_threads` has
  zero uses, so multi-threaded Python callers cannot run Rust
  concurrently. Wrap the pure compute in `py.allow_threads(...)`.

The native server runs the Rust analyser ungated (the
`TCL_LSP_RUST_ANALYSER` env-var framing was retired in #543), so
CI-enforced parity matters more, not less. Bytecode codegen is more
complete than the tracking docs implied (no `todo!` stubs); the
genuinely unported block is the WASM emitter, which is out of scope
for the LSP path.

## Prioritised roadmap

**Correctness — do first**

1. Provision the differential oracles in CI and make the harnesses
   fail-not-skip; add LSP-range and server-concurrency differential
   cases (the meta-fix).
2. Fix the UTF-16 position encoding (C1).
3. Add the document-version guard (C2).
4. Contain panics in the 8 inline handlers and fuzz the two
   CST-builder `pop().unwrap()`s (C3).

**Performance — TTFST first, then sustained**

5. Pre-warm and cache tokens at `did_open` (P1); make `range`
   range-bounded (P2); share / lazy-static the registry (P3);
   `spawn_blocking` tokens plus a lazy workspace scan (P4).
6. Build one `CompilationUnit` per document; debounce with
   cancellation; `FxHashMap` in CFG / SSA.
7. Incremental layer (`salsa` or CST subtree reuse) and `rayon`
   per-proc — the large sustained wins.

**Memory — opportunistic**

8. Interner, `Arc` / copy-on-write snapshots, and rowan-style CST
   sharing, pursued where they cut allocation latency.

## Related

- [`current-architecture.md`](current-architecture.md) — crate graph,
  ownership rules, and authoritative paths.
- [`docs/rust-rewrite.md`](../../rust-rewrite.md) — chunking strategy
  and chunk log.
- [`docs/kcs/kcs-qa-rust-shim-env-vars.md`](../../kcs/kcs-qa-rust-shim-env-vars.md)
  — Rust shim env-var reference.
