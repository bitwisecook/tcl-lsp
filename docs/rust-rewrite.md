# Python → Rust rewrite

tcl-lsp is ~360K lines of Python, organised since the May-2026
reorganisation into seven concern packages — `shared/`, `compiler/`,
`dialects/`, `analyser/`, `server/`, `tooling/`, and `ai/` — plus
`scripts/` and `tests/`.  (It previously lived under `core/`, `lsp/`,
`vm/`, `debugger/`, `fuzzing/`, `explorer/`, and `tclpkg/`; older path
references in this document map through the table in
[Python source layout](#python-source-layout-seven-concern-reorganisation)
below.)  We're rewriting **all of it** in Rust. The end goal is a repo whose
runtime, LSP server, bytecode VM, formatter, minifier, debugger,
refactoring engine, code-action surface, compiler explorer, iRule test
framework, BigIP / APL config parsers, and even the build/release
scripts run as Rust code, with **zero** Python in the shipping product.

A small, deliberately-scoped PyO3 surface survives the transition — but
not for the repo's own internals. Once everything ports across, the
PyO3 bindings exist purely as a **public API for downstream users**:
plugin authors who want to write custom analyses, embed the analyser
in their own pipeline, build alternative VMs, or extend the diagnostic
catalogue. That public surface is a separate, designed product — not a
catch-all for whatever the in-tree Python layer happened to call.

This is a multi-year project. Every step is a PR-sized change that
leaves `make prep-pr` green and every editor extension working. There
is no "big bang" branch, no pauses for rewrites, and no points at
which the Python build is intentionally broken.

This document explains what we're doing, how we're doing it, and —
most importantly — what a good port looks like. Read it before
touching anything under `rust/`, the PyO3 bindings, or the
native-extension bits of the zipapp builder.

## Python source layout (seven-concern reorganisation)

The Python source was reorganised (May 2026) from the old
`core/` + `lsp/` + `vm/` + `explorer/` + `fuzzing/` + `tclpkg/` +
`debugger/` shape into **seven concern packages** with a fixed
dependency direction enforced by `import-linter` (`.importlinter` at
the repo root, gated in `make ci-fast`).  **This matters for the Rust
rewrite because the enforced Python DAG is exactly the crate-boundary
DAG the Rust workspace already targets** — porting in dependency order
now means porting concern-by-concern, and a Python module's concern
tells you which crate its Rust port belongs in.

| Concern | Role | Old location(s) | Target Rust crate |
|---|---|---|---|
| `shared/` | Leaf utilities: Range/Token/SourcePosition, document buffer, source-map, ranges, codes, naming, `docstrings`, dialect-agnostic text | `core/common/` | `tcl-lexer` (span/tokens/line_index) + small shared mods |
| `compiler/` | Lexer, parser, IR, lowering, passes, optimiser, codegen (`codegen/bytecode/`, `codegen/wasm/`), WASM emitter, compiler-internal analyses (taint, var_escape, interprocedural, proc_arg_traits, var_scoping), command-registry **runtime**, position lookup, `Dialect` | `core/parsing/`, `core/compiler/` | `tcl-lexer`, `tcl-compiler`, `tcl-registry` (runtime) |
| `dialects/` | Per-dialect command **spec packs** + dialect data: `tcl/`, `tcllib/`, `expect/`, `eda/<vendor>`, `f5/{bigip,irules,iapps,query,xc}/`, `tk/` | `core/commands/registry/<dialect>/`, `core/bigip/` | `tcl-registry` (`commands/<dialect>/*.rs`) + F5/BigIP crates |
| `analyser/` | IDE-facing semantic model + checks: `semantic_model`, `proc_lookup`, `signature_scan`, `class_hierarchy`, MRO, `checks/`, `_analyser/`, `compiler_checks` | `core/analysis/` | `tcl-compiler` analyses + `tcl-lsp-core` |
| `server/` | LSP protocol surface: pygls wiring, `features/`, `workspace/`, diagnostics pipeline, `_lsp_conv` | `lsp/` | `tcl-lsp-core` + `tcl-lsp-server` |
| `tooling/` | Developer tools over the compiler stack: `tcl`/`f5`/`wasm` CLIs, `vm/`, `explorer/`, `debugger/`, `fuzzing/`, `tclpkg/`, `formatter/`, `minifier/`, `refactoring/`, `diagram/`, `irule_test/` | `vm/`, `explorer/`, `fuzzing/`, `tclpkg/`, `debugger/`, scattered | per-subsystem crates (`tcl-vm`, formatter, …) |
| `ai/` | AI integrations: Claude skills, MCP server, iRule context | `ai/` | binding-layer / out of scope for core crates |

**Registry mechanics vs. spec data is now a hard split** (the most
load-bearing change for the rewrite): the registry *engine* and runtime
data model live in `compiler/registry/` (`models.py`, `runtime.py`,
`command_registry.py`, `signatures.py`, `namespace_registry.py`), while
the *dialect command spec packs* live in `dialects/<dialect>/`.  This
mirrors the intended `tcl-registry` crate split exactly: registry types
are the crate's structs; dialect packs are `commands/<dialect>/*.rs`
data modules a utility can inspect without pulling compiler or LSP code.

### Dependency contracts (the crate-boundary DAG)

`shared → compiler → dialects → analyser → server/tooling → ai`, with
seven `import-linter` contracts.  As of this branch there are **zero
upward carve-outs in the analyser and dialects contracts** — both were
removed during PyO3-readiness work (see below).  The remaining
documented carve-outs are narrow and intentional:

- `dialects/` may import `compiler.registry` / `compiler.parsing` /
  pure-data compiler modules only (two carve-outs: the F5 XC translator
  consumes IR/lowering because it *is* an iRules→XC compiler; the
  vanilla const-fold spec uses `compiler.tcl_expr_eval`).
- `tooling/` ↛ `server`/`ai` (two carve-outs: the `f5-query irule
  context` verb lazy-imports `ai.shared.irule_context`; the incremental
  reparse fuzzer drives `server.workspace.DocumentState` as its test
  subject).

Read `docs/design/contracts/project-layout.md` (Python tree) for the
authoritative contract text — it is the spec the Rust crate graph
should not violate either.

### PyO3-readiness changes already made on the Python side

These are *Python* changes that pre-shape the eventual binding surface;
the Rust ports should preserve their shape:

- **Ambient-free F5 query session.** `dialects.f5.query` now exposes
  `QueryOptions` (frozen) + `QuerySession` + `prepare_query_session()` +
  `run_query_in_session()`.  The runner still uses `ContextVar`s
  internally, but the *public* surface is an explicit session a caller
  builds once and reuses — this is the shape `query_bigip` /
  `QuerySession` should own in Rust (own a parsed-config session, run
  many queries without reparsing).  `run_query()` and the fluent `q()`
  remain thin wrappers.
- **Upward dependencies inverted.** The proc-doc fallback extractor
  moved to the leaf `shared.docstrings` (was `tooling.formatter`); the
  iRule-simulation bridge moved to `tooling.f5.irule_simulation` and
  `dialects.f5.bigip.explain_flow` now takes an injected
  `IruleSimulator` instead of importing the test framework.  Net: the
  analyser and dialects crates will have **no edge into tooling**.
- **Module splits that map to crate modules.**
  `tooling.cli.pipeline` → `tooling.explorer.pipeline` (argparse-free,
  source-in/result-out); `compiler.codegen.wasm.__init__` (763 lines) →
  `api.py` + `proc_scan.py` with a re-export-only `__init__`;
  `dialects.f5.bigip.explain_flow` (2572 lines) → a `flow/` subpackage
  (`_model`, `packets`, `sessions`, `tshark`) for the config-agnostic
  half, leaving config-aware matching/policy/report in the parent.

### Recommended PyO3 facade surface (not yet built in Python)

The terminal public API (see *PyO3 public-API surface* below) should be
a small set of narrow facades — source/bytes/options in, structured
result out — over the layered crates, **not** a re-export of the whole
graph.  Suggested signatures, all returning the existing structured
result types rather than new `Any`-shaped dicts:

```text
parse_tcl(source, options)        -> tokens / parse tree
compile_tcl(source, options)      -> CompilationUnit
analyse_tcl(source, options)      -> AnalysisResult        (analyser.analyse today)
format_tcl(source, options)       -> String                (tooling.formatter.format_tcl today)
parse_bigip_config(source, opts)  -> BigipConfig            (dialects.f5.bigip.parser.parse_bigip_conf)
query_bigip(sources, query, opts) -> QueryResult            (run_query_in_session)
```

Pair with a typed public error hierarchy (`TclLspError` base →
`TclParseError` / `TclCompileError` / `TclAnalysisError` /
`BigipParseError` / `BigipQueryError` / `UnsupportedFeatureError`), each
carrying a stable code + message + optional URI/range, translated at the
facade boundary.  These facades + the error hierarchy are deliberately
**not** built in the Python tree yet (no consumer until the Rust binding
lands) — they are the design the binding crate should implement.

## What we're doing

The eventual end state is:

- **All** runtime logic lives in the Rust workspace under `rust/`. No
  Python is shipped or executed by the LSP server, the editor
  extensions, the zipapp, the compiler explorer, the MCP server, the
  debugger CLI, or any other entry point in this repository.
- The LSP server is a standalone Rust binary.
- The bytecode VM is a Rust crate. The Zig WASM runtime stays as the
  out-of-process runtime for compiled scripts; the VM is the in-process
  interpreter the analyser, debugger, and iRule test framework drive.
- The compiler explorer ships as a Rust → WASM web app (no Pyodide,
  no Python at runtime).
- The formatter, minifier, refactoring engine, code-action surface,
  iRule test framework, and BigIP / APL parsers are all Rust crates.
- Build / release scripts under `scripts/` are rewritten as
  `cargo xtask` subcommands or shell scripts, eliminating the Python
  toolchain dependency entirely.
- The **only** Python that lives on after the transition is the
  `tcl-lsp-py` crate's surface: a documented, semver-stable
  binding intended for downstream users to embed the analyser /
  compiler / VM in their own Python tooling. This API is **not** a
  shim for in-tree code; the in-tree code is Rust.
- All Python test suites get ported to Rust as cargo unit + integration
  tests. The legacy `tests/` directory shrinks to zero by the final
  retirement task.

We get there by porting the codebase bottom-up, in dependency order. The
foundation layers have **landed**: the lexer / segmenter / expr sub-lexer
(`tcl-lexer`, `tcl-syntax`), the compiler (IR, CFG, SSA, lowering, the
optimiser, and bytecode codegen in `tcl-compiler`), and the LSP server
(`tcl-lsp-server`) — which is **now the default backend** (the Python server is
an explicit opt-out). What remains is the bytecode VM, the WASM emitter, the
CLI / tooling layer, and finally the PyO3 public surface plus Python
retirement. That front of work, organised into parallel tracks in dependency
order, is the [Remaining work](#remaining-work) section below; the landed
foundation history is in the [archive](rust-rewrite-history.md).

`editors/zed/` is already a standalone Rust crate targeting WASM and is
unrelated to this rewrite. It's intentionally excluded from the main
Cargo workspace and should be left alone.

## Complete porting inventory

The terminal state has exactly **one** Python-importable artifact — the
`tcl-lsp-py` PyO3 wheel — and **zero** Python executed by any in-repo entry
point. Every Python package either (a) becomes a Rust crate, (b) folds into an
existing crate, or (c) is deleted once its consumers move to Rust. The PyO3
surface is then re-derived from the Rust crates as a *designed public API*, not
a transcription of whatever the in-tree Python used to call.

### The boundary rule

> Internal callers never import `tcl_lsp_py`. If in-tree Python imports the
> binding crate, that import is a porting TODO, not an architecture. When the
> last internal importer is gone, the remaining `#[pyfunction]` exports are
> reviewed against the public-API design and the soft-dependency shims are
> deleted.

The live per-subsystem status and the crate → remaining-work mapping are the
[Subsystem status](#subsystem-status-current-reality) and
[Track map](#track-map-dependency-order) tables under **Remaining work** below.
They supersede the historical coverage matrix and per-spec tracking tables, now
in the [archive](rust-rewrite-history.md).

## Non-negotiable principles

Two architectural constraints that every task is measured against.
They override local simplicity when they conflict; if you find
yourself working around them, stop and raise the design question.

### 0. C Tcl 9.0.3 is the reference standard

The Rust lexer, compiler, and eventual LSP server must produce
behaviour identical to **C Tcl 9.0.3** (the current stable release
of the upstream Tcl reference implementation). Every escape
sequence, quoting rule, brace-nesting edge case, and
backslash-continuation behavior is measured against what
`tclsh9.0` produces. The Python lexer was already built to match
C Tcl; the differential test harness (`tests/test_upstream_parse.py`
+ the dynamic corpus harvester) verifies the Rust lexer matches the
Python lexer, which transitively validates against C Tcl. The
`tests/test_upstream_parse.py` suite is ported from Tcl's official
`tests/parse.test`; any Tcl-version-specific behaviours (e.g. `{*}`
expansion was added in 8.5, not present in 8.4) are gated on
`LexerConfig` dialect flags.

### 1. Performance to first semantic tokens is paramount

The single user-visible latency metric that matters for the LSP
experience is **time from `textDocument/didOpen` to the first
`textDocument/semanticTokens/full` response**. That's what the user
sees as "how long until syntax highlighting shows up" when they open
a file. Every other performance consideration — throughput,
incremental update latency, memory footprint — is subordinate to
this one until time-to-first-tokens is in the single-digit
milliseconds for a typical file.

Consequences for every task:

- **Benchmark against `perf_track.py` before and after.** The
  task commit message cites the numbers. Regressions (beyond run
  noise) are a blocker; absence of improvement is OK as long as no
  regression lands.
- **The open → first-tokens path is hot.** Anything on that path
  gets optimised first — lexer, AST build, semantic-token encoding,
  JSON-RPC write. Anything off that path (incremental edits,
  diagnostics beyond the first batch, hover, completion) is
  secondary.
- **No lazy initialisation on the hot path.** Anything the first
  tokens response needs is eagerly computed on open, in parallel
  with the response construction where possible.
- **No blocking I/O, no DB roundtrips, no cross-process calls on
  the hot path.** If a feature needs them, it runs in the
  background and feeds results in when ready — it does not gate the
  first tokens response.
- **Measure before optimising.** The L3 benchmark showed that at
  the LSP pipeline level, L1-L3 haven't moved the needle because
  the lexer isn't wired in yet. That's expected; the point is that
  we _have_ the measurement and can see when each task starts
  showing up.

### 2. Async through and through

The Rust LSP server is async-first from the protocol handler down
to the analysis pipeline. Every layer above the raw lexer is
`async fn`, runs on Tokio, yields cooperatively, and composes with
cancellation. This is how we get responsiveness: a fresh
`textDocument/semanticTokens/full` request while an older one is
still computing should cancel the older one cleanly, not wait for
it to finish.

Consequences for every task:

- **`tower-lsp` is the LSP framework.** Ratified in the "Chosen
  libraries" section below — it gives us `async fn` handlers for
  every LSP method out of the box, built on Tokio.
- **`ropey` is the document buffer.** Also ratified below. Ropes
  are the standard async-friendly document storage: cheap clone
  (`Arc<Rope>`), O(log n) slicing, built-in line indexing,
  incremental-edit-friendly.
- **The lexer itself is synchronous** (fast, CPU-bound, a few μs
  per typical file), **but it is `Send`**, holds no thread-local or
  `static mut` state, and is trivially safe to call from any async
  task. For files large enough that lexing would block the
  executor for more than a frame (rare for Tcl, possible for
  generated iRules bundles), the caller moves the call into
  `tokio::task::spawn_blocking` or splits the work across tasks —
  a decision taken by the caller, not baked into the lexer.
- **The analysis pipeline (above the lexer) is async.** Each pass
  is `async fn` even when its body is CPU-bound, so it composes
  with cancellation tokens and can yield between phases. Long
  CPU-bound phases call `tokio::task::yield_now().await` at
  coarse-grained checkpoints so the executor stays responsive.
- **Document store updates use `tokio::sync` primitives.** No
  `std::sync::Mutex` on paths that can be held across `.await`,
  no blocking read locks. `RwLock<Arc<DocumentState>>` is the
  common pattern; read-heavy operations clone the `Arc` and drop
  the lock immediately.
- **No globals, no thread-locals, no singletons.** Anything that
  looks like state lives on an owned struct that the task holds or
  is passed in as a parameter. The Python lexer's `_thread_local`
  for `strict_quoting` is an explicit anti-pattern here; the Rust
  equivalent is a field on `LexerConfig`.
- **Diagnostics, semantic tokens, hover, completion, definition,
  references, formatting, code actions, inlay hints — every
  `async fn` handler.** Cancellation propagates via
  `tokio::select!` with the LSP cancellation token. No handler
  body blocks for more than a few μs without an `.await`.

These are non-negotiable because the whole point of the rewrite is
to be faster and more responsive than the Python server. Ignoring
either principle wastes the rewrite.

## Chosen libraries and data structures

The library choices below are selected **specifically** to serve
the two principles above: time-to-first-tokens and async-to-the-core.
Each entry notes why it wins for those criteria.

- **Buffer storage (LSP layer): [`ropey`](https://crates.io/crates/ropey).**
  Rope for the document store. Used by Helix, chosen for the Rust
  rewrite because: (a) O(log n) slicing means the time-to-first-
  tokens path can flatten a rope range into a `&str` and hand it
  to the lexer without an O(n) full-source copy — critical for
  large iRules bundles; (b) rope handles are cheaply shareable via
  `Arc<Rope>`, so async tasks that need to read the document
  concurrently don't contend on a lock; (c) built-in line indexing
  via `Rope::byte_to_line` / `Rope::line_to_byte` means we don't
  rebuild `LineIndex` on every edit in steady state. Adopted when
  the LSP server tasks land (R*). The lexer itself does **not**
  take a `Rope` — see the "Position infrastructure" section below.
- **LSP framework: [`tower-lsp`](https://crates.io/crates/tower-lsp).**
  Chosen for async-to-the-core: every LSP method is an `async fn`
  on the `Backend` trait, dispatched on Tokio, so cancellation
  composes trivially with `tokio::select!` and the LSP
  cancellation token. The alternatives were considered and
  rejected: `lsp-server` (from rust-analyzer) is synchronous and
  would force us to build our own async layer on top; `async-lsp`
  is also async but has a smaller ecosystem and less momentum.
  `tower-lsp` is the default modern choice and the one with the
  most production deployments (taplo, wat, nixd, …).
- **Error types: [`thiserror`](https://crates.io/crates/thiserror) in
  library crates, [`anyhow`](https://crates.io/crates/anyhow) in
  binaries.** Already in use in `tcl-lexer` for `LexError`.
- **Python bindings: [`PyO3`](https://pyo3.rs) + [`maturin`](https://maturin.rs).**
  Already in place for the soft-dependency build.
- **CLI argument parsing (when CLI tools arrive):
  [`clap`](https://crates.io/crates/clap) with `derive`.**
- **Logging: [`tracing`](https://crates.io/crates/tracing) + `tracing-subscriber`.**

### Spans threaded through everything

The single most important architectural invariant of the Rust side:
**every positional entity carries a [`Span`], not inline position
data**. Tokens today; IR nodes, CFG nodes, diagnostics, refactoring
ranges, semantic-token outputs, and everything else tomorrow. A
[`Span`] is just two `u32`s — an inclusive start and an exclusive
end, byte offsets into the source. It's 8 bytes, `Copy`, no
lifetime, trivially storable in containers.

To go from a span back to anything human-readable — text, line
number, LSP `Position`, etc. — callers thread a [`SourceMap`] (a
`&str` source buffer bundled with a [`LineIndex`]) and ask it:

```rust
let source_map = SourceMap::new(source);
let tokens = Lexer::new(source).tokenise_all()?;
for tok in &tokens {
    let text = source_map.text(tok.span);
    let (start, end) = source_map.range_positions(tok.span);
    println!("{:?} {:?} {}-{}", tok.kind, text, start.line, end.line);
}
```

This matches the design used by rust-analyzer (`TextRange`), swc
(`Span`), and tree-sitter (`Range`). It keeps entities tiny, keeps
position lookups in one place, and means a future incremental story
can invalidate spans without touching every downstream struct.

Consequences of the rule:

- **Tokens have no lifetime.** `pub struct Token { kind, span,
  in_quote }` is 16 bytes and `Copy`. A `Vec<Token>` is a plain
  buffer; it can be serialised, sent across threads, cached, and
  diffed without lifetime bookkeeping.
- **IR / CFG nodes (future tasks) carry `Span`s, not
  `SourcePosition`s.** Passes rewrite nodes freely; positions stay
  deferred to the `SourceMap` and are computed only at the point of
  diagnostic emission or LSP response formatting.
- **Diagnostics carry `Span`s.** LSP `Range` values are derived on
  publish, not stored on the diagnostic.
- **Sub-lexing** (L5 command substitution, future expression
  re-lexing) inherits the parent `SourceMap`. It never builds its
  own line index; the spans it produces are offsets into the same
  top-level buffer, so downstream consumers see one coherent
  coordinate system.
- **Positional parity with Python is a [`SourceMap`] concern.** If
  we ever need UTF-16 column parity with the LSP specification, we
  fix it once inside [`LineIndex::position_at`] and every
  downstream entity gets correct positions for free.

### Position infrastructure — lexer vs. document layer

The Python side has a single `DocumentBuffer` type that holds the
source text plus a `line_starts` tuple. In Rust we split the concern
along the same span-threading principle above:

- **At the lexer layer** (`rust/tcl-lexer/`), the lexer consumes a
  `&'src str` slice and produces `Token` values that carry only a
  `Span`. It owns a [`SourceMap`] — a zero-allocation wrapper over
  `(source, LineIndex)` — that any consumer can borrow via
  `Lexer::source_map()` or take over via `Lexer::into_source_map()`.
  [`LineIndex`] itself is a pure-Rust `Box<[u32]>` with O(log n)
  `partition_point` lookups — not a port of `DocumentBuffer`, just
  the minimum needed to resolve spans to positions.
- **At the LSP server layer** (future `rust/tcl-lsp-server/`), the
  Document store is a [`ropey::Rope`]. The rope has its own internal
  line index; when the document layer needs to lex a task it
  flattens the affected rope range into a `&str` (cheap: most ranges
  are a single contiguous task), wraps it in a [`SourceMap`], and
  hands the `SourceMap` to the lexer via `Lexer::with_source_map`.
  No `LineIndex` gets built twice, and no rope reference leaks into
  `tcl-lexer`.
- The seam between the two layers is one cheap flatten plus a
  [`SourceMap`]. When we need to avoid even that, a future
  `LineIndex::from_rope_slice` adapter will skip the linear scan and
  pull line offsets straight out of the rope's B-tree.

The key point: the **lexer's public API is `&str`-based**, the
**document store's public API is rope-based**, and the seam between
them is a cheap rope → `&str` flatten plus a shared [`SourceMap`].
No rope references leak into `tcl-lexer`, no `LineIndex` leaks into
the document store (beyond the rope adapter).

[`Span`]: ../rust/tcl-lexer/src/span.rs
[`LineIndex`]: ../rust/tcl-lexer/src/line_index.rs
[`LineIndex::position_at`]: ../rust/tcl-lexer/src/line_index.rs
[`SourceMap`]: ../rust/tcl-lexer/src/source_map.rs
[`ropey::Rope`]: https://docs.rs/ropey

### Deferred work (lexer)

The L0–L13 lexer migration is complete — every token kind, the expression
sub-lexer, the spans / line-index / source-map, and the red-green CST are in
Rust, and the segmenter is a view over the CST. The lone-CR line-index rule
(`\n`-only) and the UTF-16 column primitive (`LineIndex::position_at_utf16`)
have landed. The residual lexer-layer gaps — the `${name}` brace-depth scan,
the quoted `\<newline>` build divergence, and the nested-body E202/E203
detectors — were the **FE-LEX** track and have now **landed** (archived in
[`rust-rewrite-history.md`](rust-rewrite-history.md), 2026-06-19); routing the
last byte-column call sites through the UTF-16 primitive is part of the
**SRV-LSP** track.

## How we're doing it

### Layered crates, not shim-owned features

The Rust workspace has three kinds of crates:

- **Pure library crates** own product behaviour. They do not depend on
  `pyo3`, do not mimic Python object shapes, and are the crates the
  eventual Rust LSP server and CLI binaries link against directly.
- **Binding crates** expose Rust behaviour to Python. They contain
  `#[pyclass]`, `#[pyfunction]`, `PyErr` conversion, tuple/dict
  materialisation, env-var compatibility, and back-compat shims only.
- **Binary crates** provide entry points such as the native LSP server,
  debugger, compiler explorer helpers, and release tooling. They depend
  on pure crates, not on PyO3 bindings.

The current transitional binding crate is `rust/tcl-lsp-rust/` because
the Python shims already import the `tcl_lsp_rust` extension module. Treat
that crate as a compatibility wrapper. It must not own compiler, analyser,
registry, or LSP feature logic. When the public Python API is designed,
the stable binding crate becomes `rust/tcl-lsp-py/` and
`tcl-lsp-rust` either disappears or stays as a one-release import alias.

Target dependency direction:

1. `tcl-lexer` owns source text, spans, line indexes, and tokenisation.
2. `tcl-registry` owns command, dialect, argument, taint, effect,
   documentation, and hook metadata.
3. `tcl-compiler` owns parsing above the lexer, IR, CFG, SSA, analysis,
   lowering, optimisation, and codegen algorithms.
4. `tcl-lsp-core` owns pure LSP feature providers: folding, document
   symbols, hover, completion, references, rename, semantic tokens,
   diagnostics projection, and code actions.
5. `tcl-lsp-server` owns the `tower-lsp` binary, async document store,
   request routing, cancellation, progress, and editor-facing protocol
   plumbing.
6. `tcl-lsp-py` owns the public PyO3 API for downstream users.

No LSP feature provider lands directly in a PyO3 crate. If Python needs
to call a new Rust feature during the migration, put the implementation
in `tcl-lsp-core` first and expose it through a small binding wrapper.

### Python compatibility lives only in the binding layer

If the current Python API demands something awkward — thread-local flags,
class-level mutable state, stringly-typed kwargs, magic singleton modules
— the binding crate implements the awkwardness and hides it from the
pure crate. The pure crate gets clean `&Config` parameters or equivalent,
returns `Result<T, Error>`, and never has to apologise for Python.

This rule is non-negotiable. A pure crate that imports `pyo3` "just for
this one function" is a sign that the binding crate needs another wrapper
type, not that the rule should bend.

### Command facts live in the registry

The command registry is the source of truth for command facts. The
compiler, analyser, LSP feature providers, and diagnostics may own
algorithms, but they must not own independent command tables.

Registry-owned facts include:

- command names, aliases, dialects, arity, subcommands, option forms,
  and command forms;
- argument roles, including dynamic role resolvers for count-dependent
  and subcommand-dependent calls;
- lowering and codegen hook identifiers;
- taint sources, taint sinks, sanitiser roles, setter constraints, and
  protocol-specific sink shapes;
- side-effect summaries, variable read/write summaries, and storage
  effects;
- help snippets, hover text, examples, KCS links, and editor setting
  catalogue facts.

Compiler and analyser code should ask the registry a precise question:
"what command form is this call?", "which argument indices are bodies?",
"does this call write a variable?", "which lowering hook applies?",
"which taint sink shape applies?". A hardcoded `HashSet` of command names
or a `match cmd.name` dispatcher outside registry-owned routing is a
design debt item, not a pattern to copy.

Hook identifiers are part of the registry contract. They should be typed
enums or strongly-typed generated constants, not public bare `u16`
values. The compiler maps hook identifiers to algorithms; the registry
decides which identifier applies to a command form.

### Restructure before new surface area

Before adding more migrated feature surface, pay down the architectural
debt exposed by the review. Land the restructure in small, shippable
tasks:

1. Create `tcl-lsp-core` and move pure LSP provider logic out of
   `tcl-lsp-rust`. Start with folding because it is already isolated.
2. Rename the PyO3 boundary in documentation and code ownership terms:
   `tcl-lsp-rust` is transitional compatibility; `tcl-lsp-py` is the
   intended public binding crate.
3. Expand `tcl-registry` so command forms, hook IDs, taint metadata,
   side effects, and option/form knowledge are registry data.
4. Replace compiler/analyser name dispatch with registry-driven
   resolution. The first consumer should be lowering/codegen hooks, then
   structured command lowering, then taint/iRules diagnostics, then
   side-effect classification.
5. Add a current-state architecture doc once the split starts. It should
   say which Rust paths are authoritative, which Python supplements still
   exist, and which fallbacks are planned for removal.

### Always shippable, small tasks

Every PR leaves the extension fully working. The CI gate is the same gate
as every other PR in the repo: `make prep-pr` must pass, all editor
packages must still build, and no existing test is allowed to regress.

One task = one logical surface. Good examples:

- "Port `backslash_subst`." (~100 LOC Python → Rust + PyO3 wrapper + shim.)
- "Port `TokenType` / `SourcePosition` / `Token`."
- "Port brace-string lexing in the Rust lexer."
- "Port the branch-folding optimiser pass."

Bad examples:

- "Port the whole lexer." (Too big, parity can't be reviewed.)
- "Rewrite `core/parsing/`." (Even worse.)
- "Port the lexer and the compiler IR together because they share a
  data structure." (Split the data structure out first.)

Each task that replaces real logic needs a differential test: run the
Python and Rust implementations in parallel on every fixture and assert
identical output. The lexer task (L3 onward) introduces
`tests/test_rust_lexer_differential.py` for this. Use the same pattern
for the compiler.

### Soft dependency during rollout

Until a task explicitly flips the default, the Python code imports the
Rust wheel via `try: from tcl_lsp_rust import … except ImportError`.
A missing wheel is a performance no-op, not a regression. This lets a
developer work on fresh clones without `make rust-build`, and it lets
releases ship even if a platform wheel fails to build.

Once a task flips the default (the Rust implementation becomes the
preferred path and the Python fallback is just there as a safety valve),
the pure-Python fallback is kept for exactly one release cycle and then
removed outright in a follow-up task. Do not let fallbacks accumulate.

### Packaging and CI

- The main `pyproject.toml` stays on `hatchling`. The Rust wheel is built
  by `maturin` from its own binding-crate `pyproject.toml` and is a
  **separate** distribution. During the transition that crate is
  `rust/tcl-lsp-rust/`; after the public API split it is
  `rust/tcl-lsp-py/`. No mixed hatchling/maturin hybrid.
- Rust wheels ship as GitHub release artifacts on tagged releases, not
  PyPI. `scripts/build_zipapp.py` fetches them at packaging time and
  bundles them alongside the zipapp.
- `scripts/build_zipapp.py::_pip_install_pure` preserves native
  extensions whose package name is in `_RUST_NATIVE_PACKAGES` and strips
  everything else. Extend that set rather than widening the strip.
- PR CI builds a single linux x86_64 wheel and runs the Python test
  suite against it. Tagged releases build the full matrix: linux
  x86_64, linux aarch64, macOS x86_64, macOS arm64, windows x86_64.
- The Rust toolchain tracks `stable` floating via `rust-toolchain.toml`.

## What a good port looks like

The point of moving to Rust is to benefit from enums, lifetimes,
iterators, `Result`, zero-copy slices, and an ownership model that
catches bugs at compile time. A port that preserves every Python data
shape and pattern has missed the point.

Reshape the design. Rename things. Split or merge modules. Use Rust
idioms even when they diverge sharply from the Python layout. The
binding layer absorbs any Python-facing API drift.

Some concrete rules of thumb:

### Data structures

- Use **enums** for sum types. Python `Enum` with `auto()` becomes a
  Rust enum with explicit variants. `str` sentinels and `isinstance`
  checks become `match`.
- Use **structs with named fields** for product types. Python dataclasses
  become Rust structs. Frozen dataclasses become `#[derive(Clone, Copy,
  Debug, Eq, PartialEq, Hash)]` as appropriate.
- **Positional entities carry a `Span`, not inline positions.**
  Tokens, IR nodes, CFG nodes, diagnostics — anything that refers to
  a region of source — stores a byte range, not a start/end
  `SourcePosition` pair. Text and `(line, character)` positions are
  resolved on demand via a `SourceMap`. This keeps entities tiny,
  centralises position lookups, and lets the whole rewrite inherit
  the same coordinate system. See the "Spans threaded through
  everything" section above.
- Use **`&str` and `Cow<'_, str>`** instead of `String` wherever you can
  borrow. The caller usually owns the buffer; a new allocation per
  token is a waste. The PyO3 wrapper clones on the way out if needed.
- Use **`Option<T>`** for "may be absent", **`Result<T, E>`** for "may
  fail". Do not invent sentinel values.
- Prefer **`SmallVec`**, **`Cow`**, **`Arc`** where they genuinely help.
  Don't reach for them by default.

### Control flow

- Prefer **iterators** over stateful classes. A lexer becomes an
  `Iterator<Item = Result<Token<'src>, LexError>>`, not an object with
  a `get_token()` method. The PyO3 wrapper presents the stateful API
  Python expects.
- Prefer **`match`** over `if let` chains, and prefer exhaustive matches
  over wildcard arms that silently swallow future variants.
- Keep function bodies flat. Early returns are fine. Deeply nested
  conditionals almost always want to be split into helpers.

### Errors

- All errors go through **`thiserror::Error`** in the pure crate. No
  panics for recoverable conditions, no `Option` where `Result` is
  meaningful.
- The binding crate converts the pure-crate error type into a matching
  Python exception. Preserve message text and position information.
- Warnings (non-fatal diagnostics) are collected into a `Vec` on the
  result value, not mutated onto a global. The Python-facing wrapper
  exposes them as an attribute on the returned object if the Python API
  previously did so.

### Configuration

- Global, thread-local, and class-level flags from Python become **fields
  on a `Config` struct** passed to constructors. No `lazy_static`, no
  `thread_local!`, no module-level mutable state in the pure crate.
- The binding crate may keep a thread-local or class-attribute façade if
  that's what Python callers depended on, and translate it into a fresh
  `Config` on each call. The pure crate stays pure.

### Modules and naming

- Split by responsibility, not by a line-count budget. A module with one
  300-line function is fine; a module with eight unrelated 50-line
  functions usually wants to split.
- **Do not reproduce Python's mixin composition in Rust.** Python
  codebases like `core/compiler/codegen/_emitter.py` compose a single
  class from 7 mixins via multiple inheritance. The Rust port does not
  need to preserve that shape — use multiple `impl CodegenCtx` blocks
  across separate files, one file per responsibility (parsing,
  emission, control flow, block ordering, …). The mixin boundary is a
  Python artefact, not an architectural principle.
- **Break monster functions into per-case handlers.** Python's
  `generate()` is a ~450-line block-type dispatch loop. In Rust, lift
  each `if bname.startswith("foreach_header_")` branch into its own
  named handler function. The top-level loop should read as a
  dispatcher, not a state-machine implementation. Keep deferred-label
  state in an explicit struct (`GenerateState`) rather than
  ten loose local maps.
- Use UK spelling (`normalise`, `optimiser`, `analyse`) in identifiers
  and comments, matching the rest of the repo.
- Doc comments describe invariants and non-obvious decisions. Don't
  paraphrase the code. Don't add banner-style dividers.
- Every public item gets a doc comment. `#![deny(missing_docs)]` is on.

### Tests

- Unit tests live next to the code they cover (`#[cfg(test)] mod tests`).
  Integration tests go under `tests/` inside the crate when they need
  multiple modules.
- Every task that replaces real Python logic ships with a differential
  test harness: feed the same inputs through both implementations and
  assert identical outputs. Do not flip any default until the
  differential harness is green across the whole corpus.
- Avoid golden-file tests for things that are cheap to compute. Prefer
  assertions that state the actual invariant.
- **Test audit.** Every task classifies the pytest tests it touches as
  **ported** (Rust has equivalent coverage), **bridge-only** (Python-
  specific behaviour — kept in pytest, not ported), **remove at end**
  (low-value, flagged inline with an `AUDIT:` comment and tracked for
  deletion when the Python layer is retired), or **deferred** (covered
  by a later task). The living audit lives in
  [`rust-rewrite-test-audit.md`](rust-rewrite-test-audit.md); update
  the relevant section in the same commit that lands the task. No
  pytest test is deleted during the rewrite — the Python suite is the
  behavioural oracle for every task, and only comes out when the
  Python layer itself comes out.

### What a bad port looks like

If your port has any of these, reshape it before asking for review:

- A `#[pyclass]` in the pure crate.
- An IR node, CFG node, or diagnostic that stores `start:
  SourcePosition, end: SourcePosition` instead of a `Span`. Positions
  belong on the `SourceMap`, not on every entity.
- A second line-index implementation. There is one `LineIndex`, owned
  by the `SourceMap`. Everything else borrows it.
- A `String` field where `&'src str` would borrow from the caller's
  buffer.
- A translation of Python's class-level `strict_quoting = False` into a
  `static mut` or `lazy_static`.
- A match arm that reproduces a three-arm `if/elif/else` ladder verbatim
  when two of the arms have the same body.
- A function signature that takes `Option<Option<T>>` because Python used
  `None` as both "absent" and "error".
- An `unwrap()` anywhere in a hot path.
- A panic in a pure parser crate for malformed input. Malformed input is
  a `Result`, not a crash.
- Pure LSP provider logic in a PyO3 crate. Bindings wrap providers; they
  do not implement them.
- A command-name table in the compiler, analyser, LSP layer, or
  diagnostics layer when the same fact belongs in `tcl-registry`.
- A comment that says "TODO: make this idiomatic later". Do it now.

## Reference file layout

```
Cargo.toml                               workspace manifest
rust-toolchain.toml                      channel = "stable"
rust/
  tcl-lexer/                             pure Rust lexer crate
    Cargo.toml
    src/
      lib.rs
      substitution.rs                    backslash_subst (L1)
      tokens.rs                          Token, TokenType, SourcePosition (L2)
      span.rs                            Span — byte range (L3)
      line_index.rs                      LineIndex — byte offset → line/col (L3)
      source_map.rs                      SourceMap — source + LineIndex (L3)
      lexer.rs                           Lexer skeleton (L3)
  tcl-compiler/                          pure Rust compiler crate
    Cargo.toml
    src/
      lib.rs
      analyses.rs                        LatticeValue, FunctionAnalysis, ModuleAnalysis (C5)
      cfg.rs                             Block, Function, CfgModule, Terminator (C2)
      codegen/                           bytecode emitter directory module
        mod.rs                           Op, Instruction, LiteralTable, CodegenCtx (C4)
        helpers.rs                       list/dict/format folding helpers (C11)
        values.rs                        push_lit, load/store_var, emit_incr (C11)
        expressions.rs                   emit_expr for ExprNode variants (C11)
        statements.rs                    emit_stmt dispatch (C12)
        peephole.rs                      post-emission cleanups (C13)
        layout.rs                        jump shrinking + byte offsets (C14)
        format.rs                        disassembly rendering (C14)
        cmd_subst.rs                     [cmd ...] inline dispatch (C15)
        control_flow.rs                  catch/try inline emission (C16)
        emitter/                         main emitter loop (C17)
          mod.rs                         module glue + public API
          ordering.rs                    linearise, loop body, branch folding
          terminator.rs                  CFG terminator emission
          proc_defs.rs                   proc def interleaving
          loop_blocks.rs                 foreach/while/for block handlers
          try_blocks.rs                  try/finally CFG detection
          generate.rs                    main generate() dispatcher
          bytecoded.rs                   registry-backed codegen hooks
      expr_ast.rs                        ExprNode, BinOp, UnaryOp, render_expr (C0)
      expr_parser.rs                     Pratt parser: ExprToken → ExprNode (C1)
      ir.rs                              Statement, Script, Procedure, Module (C0)
      naming.rs                          normalise_var_name (C1)
      ssa.rs                             Phi, SsaBlock, SsaFunction, dominators (C3)
      types.rs                           TypeLattice, type_join (C5, re-exports TclType from registry)
  tcl-registry/                          command registry — single source of truth (R0)
    Cargo.toml
    src/
      lib.rs                             crate root, prelude
      arg_role.rs                        ArgRole enum (12 variants)
      arity.rs                           Arity { min, max }
      traits.rs                          Traits bitflags (u64, 38 flags)
      dialects.rs                        DialectSet bitflags
      types.rs                           TclType (canonical home)
      spec.rs                            CommandSpec, SubCommand
      registry.rs                        CommandRegistry facade
      hover.rs                           HoverSnippet, OptionSpec, FormSpec
      side_effects.rs                    SideEffect, StorageType
      hooks.rs                           typed LoweringHookId / CodegenHookId, ArgTypeHint
      forms.rs                           command / subcommand form descriptors
      taint.rs                           taint source/sink/sanitiser metadata
      commands/tcl/*.rs                  one file per Tcl command (114 ported)
      commands/irules/*.rs               one file per iRules command (1015 ported)
  tcl-lsp-core/                          pure Rust LSP feature providers
    Cargo.toml
    src/
      lib.rs                             feature module exports
      folding.rs                         folding range provider
      diagnostics.rs                     diagnostic projection helpers
      symbols.rs                         document/workspace symbol providers
  tcl-lsp-rust/                          transitional PyO3 binding crate
    Cargo.toml
    pyproject.toml                       maturin build backend
    src/
      lib.rs                             #[pymodule] tcl_lsp_rust
      folding_binding.rs                 wrapper around tcl-lsp-core::folding
  tcl-lsp-py/                            final public PyO3 API crate
    Cargo.toml
    pyproject.toml                       maturin build backend
    src/
      lib.rs                             #[pymodule] tcl_lsp_py
  tcl-lsp-server/                        tower-lsp binary
    Cargo.toml
    src/
      main.rs                            Tokio runtime + stdio transport
      server.rs                          Backend implementation
      document_store.rs                  ropey-backed async document state
scripts/
  build_zipapp.py                        _RUST_NATIVE_PACKAGES strip rule
.github/workflows/ci.yml                 rust job + release wheel matrix
Makefile                                 rust-build/test/lint/format
tests/test_rust_bindings_smoke.py        end-to-end bridge smoke test
```


## Tracking `main` — branch model

The rust workstream lives on the long-running `rust` branch: ahead of `main`
on rewrite work, behind on everything else. The load-bearing overlap —
analyser, registry, IR/CFG/SSA passes, the expression evaluator — must be kept
current with `main` or the Rust analyser silently drifts from the canonical
Python behaviour it mirrors. Per-task workflow: rebase the touched files off
`main`, port the delta, run the differential parity gates
(`differential_codegen` / `differential_segment` / `differential_incremental`
plus the `test_fp_*` ground-truth battery), and keep `make prep-pr` green. The
full historical drift log is in the [history archive](rust-rewrite-history.md).

## Testing strategy

The 448 pytest files / ~14 K test functions sort into four buckets; port each
file's coverage **alongside** the code it covers, following the crate DAG
(lexer → syntax → compiler → registry → analyser → lsp-core).

| Bucket | ~files | ~tests | Destination |
|---|---|---|---|
| **RUST-UNIT** | ~170 | ~6,500 | Rust crate unit tests (source-in → structured-out) |
| **E2E** | ~70 | ~1,800 | `tests/lsp_e2e` (JSON-RPC; validates *either* backend) |
| **PY-INTERNAL / VM-INTEGRATION** | ~190 | ~6,000 | Python-product/tooling, or runtime-port + WASM differential |
| **COVERED / meta** | ~18 | ~250 | already differential / e2e / catalogue meta-tests |

The **`test_fp_*` false-positive / ground-truth battery** (locked against
`tclsh9.0`) is the analyser's acceptance gate and must be carried verbatim into
Rust. The full file-by-file disposition (which test maps to which crate, the
E2E migration list, and the flagged coverage gaps) is preserved in the
[history archive](rust-rewrite-history.md#testing-strategy--porting-the-14k-test-pytest-suite-to-rust).

---

## Remaining work

This is the live plan. Everything below is **not yet done**; landed work lives
in the [history archive](rust-rewrite-history.md), and the deep per-item
evidence behind each front-end gap is in
[`design/rust/compiler-pipeline-parity.md`](design/rust/compiler-pipeline-parity.md).
The plan reflects current source as of 2026-06-19.

### Vocabulary

One set of terms, used consistently (the older *chunk / phase / slice / strip /
strand / family / wave / pillar / candidate* vocabulary survives only in the
archive):

- **Stage** — a dependency layer (1 Front-end → 2 Runtime → 3 Server →
  4 Tooling → 5 Public API). Stages are ordered; tracks within a stage are not.
- **Track** — a parallel workstream that owns a bounded set of crates/modules
  and can be progressed independently of the other tracks in its stage.
- **Task** — a discrete, PR-sized unit of work within a track.
- **Step** — an ordered sub-part of a task.

Task status is either **open** or **partial** (with a note on what remains).

### How to read this

- Tracks are grouped into **stages 1–5 in dependency order** (Front-end →
  Runtime → Server → Tooling → Public API). Within a stage, tracks own
  **disjoint** crates/modules, so they can run in parallel without colliding. A
  later-stage track may depend on an earlier one; the dependency is stated.
- Each track names its **owned crates/files** — that ownership boundary is what
  makes the tracks parallel-safe.
- **Stage 5 (PyO3 interfaces + Python retirement) is intentionally last**: it can
  only close once every consumer above has ported.
- Several items the old chunk-log still listed as open have since **landed** and
  are deliberately absent here — the ghost-token recovery engine (E201–E206),
  the security/injection check family (W102/W103/W300-series + T100–T106 +
  W313), the iRules taint sinks (IRULE3001–3004), the `fp_rch` break-edge CFG
  modelling, O109 / O116 / O120, the upvar transitive-merge, the
  document-version guard (review-findings C2), the CST descent, and the whole
  **FE-LEX** track (`${name}` brace-depth, quoted `\<newline>`, nested-body
  E202/E203 — archived 2026-06-19). Trust this plan and the source over the
  archive's dated rows.
- **This document is a forward-looking plan, not a changelog.** It lists only
  **open** / **partial** work. The narrative of *what landed and why* is
  history — record it in [`rust-rewrite-history.md`](rust-rewrite-history.md),
  not here. When a track finishes, delete its detailed `####` section and leave
  only its rows in the subsystem-status and track-map tables (mark them ✅ /
  🟢); add the landed detail to the history file in the same change. Do **not**
  accumulate `**done**` bullets in this plan.
- **Verify every port against real Tcl behaviour.** Check the produced result
  against **tclsh 8.4–9.0** (the four source trees live under `tmp/tcl<ver>/`;
  build a missing one with `.claude/skills/fetch-tcl-source` + `configure &&
  make` under `unix/`), and consult the **C Tcl source** for the reference
  algorithm — `tmp/tcl9.0.3/generic/` carries the `tclParse.c` / `tclUtil.c` /
  `tclExecute.c` files the ports mirror. Gate version-specific behaviour (e.g.
  `0o` / `0b` integer prefixes exist in 8.5+ but not 8.4; `{*}` expansion is
  8.5+) on the registry / `LexerConfig` dialect flags, never hardcode one
  version. C Tcl 9.0.3 is the reference standard — see
  [`rust-rewrite-history.md`](rust-rewrite-history.md) §"C Tcl 9.0.3 is the
  reference standard".
- **A discovery in one track that affects another must update the other
  track's entry here, in the same change.** When working a track surfaces a
  wrong assumption, a shared invariant, or a handoff (e.g. a residual that
  belongs to a different owner), edit that other track's row/section now —
  don't leave the finding buried only in a commit message or PR description.

### Subsystem status (current reality)

Replaces the historical coverage matrix above. Status: ✅ done · 🟢 done bar
listed residuals · 🟡 partial · 🔴 not started.

| Subsystem | Crate(s) | Status | Remaining (→ track) |
|---|---|---|---|
| Lexer / segmenter / expr-lexer / CST | `tcl-lexer`, `tcl-syntax`, `tcl-compiler::parsing` | ✅ | FE-LEX complete — `${name}` brace-depth, quoted `\<nl>`, nested-body E202/E203 landed (see [history](rust-rewrite-history.md), 2026-06-19) |
| IR / lowering / CFG / SSA | `tcl-compiler` | 🟢 | `IRUpFrame` clobber; dynamic-`uplevel` barrier; minor IR fields → **FE-DATAFLOW**, **FE-DIAG** |
| SCCP / intervals / memory-SSA | `tcl-compiler` | 🟡 | escaping-var widening; optimistic deferral; break-exit/static-loop folding; W233 interval path; `complexity_guard` → **FE-DATAFLOW** |
| Type inference / shimmer / shapes / rendered-props | `tcl-compiler` | ✅ | **FE-TYPESHIM** complete; precise TclOO `object_of` typing now tracked under **FE-DIAG** (its W307/W308 consumer) |
| var-escape | `tcl-compiler::var_escape` | 🟡 | unwired (no orchestrator); `pure_leaf` family → **FE-VARESCAPE** |
| Optimiser passes | `tcl-compiler::optimiser` | 🟡 | O114/O108 gates; O104/O119 applied; O128/O130; O106 category+gates; general inliner → **FE-OPT** |
| Bytecode codegen | `tcl-compiler::codegen` | 🟡 | statement-position specialisations; const-fold; `esc`/`{*}`/`set x [cmd]` → **FE-CODEGEN** |
| Analyser diagnostics | `tcl-compiler::analyser` | 🟢 | E001/W125/IRULE5005/IRULE1001; snit; OO body-walks; C44 path-sensitivity; source-style/W108 → **FE-DIAG** |
| F5 dialect diagnostics | new `tcl-xc`, `tcl-bigip`, tk slice | 🔴 | TK1001-3, BIGIP6001-11, IAPP7001-3, XC100-301 → **FE-DIAG-F5** |
| WASM codegen + runtime | `tcl-compiler::codegen::wasm`, `runtime/zig`, new `tcl-wasm` | 🔴 | emitter (IR/encoding only today); `IRInterpBoundary`; codegen DCE/GVN; `tcl-wasm` CLI → **RT-WASM** |
| Bytecode VM | `tcl-vm` | 🟡 | TclOO; clock/encoding/interp/IO/after; CLI/REPL binary → **RT-VM** |
| LSP server / core / db | `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db` | 🟢 | incremental reanalysis; UTF-16 residuals; registry/CU sharing + debounce; panic containment; token/reposition cache; rope state → **SRV-LSP** |
| `tcl` CLI | `tcl-cli` | 🟡 | `dis`/`compwasm`/`pkg`/`venv`/`docker` verbs → **TOOL-CLI** |
| `f5-query` CLI | `f5-cli`, `tcl-bigip*`, `tcl-irules` | 🟢 | `explain-flow --simulate/--tshark/--keylog`; a few parity files → **TOOL-F5** |
| Formatter / minifier / diagram | `tcl-lsp-core`, `tcl-cli` | ✅ | — |
| Refactoring transforms | `tcl-lsp-core::code_actions` | 🟡 | 5 of 7 transforms missing → **TOOL-REFACTOR** |
| Compiler explorer | `tcl-explorer`, `tcl-explorer-wasm` | 🟡 | `wasm` view (blocked on **RT-WASM**) → **TOOL-EXPLORER** |
| Package manager (`tclpkg`) | new crate | 🔴 | full port (manifest/resolver/lockfile/CAS/fetchers/venv/docker) → **TOOL-TCLPKG** |
| Differential fuzzer | new crate | 🟡 | campaign runner/generator/findings (corpus diff exists) → **TOOL-FUZZ** |
| Debugger (DAP) | new crate | 🔴 | full debugger over `tcl-vm` → **TOOL-DEBUGGER** |
| iRule test framework | new crate | 🔴 | TMM-sim harness (topology/profile-gen/orchestrator) → **TOOL-IRULE-TEST** |
| PyO3 public API + retirement | `tcl-lsp-py`, `xtask` | 🔴 | designed public surface; TEST-MIGRATE; PYTHON-RETIRE → **API-PYO3** |
| `ai/` (MCP + skills) | — | n/a | stays Python by design |

### Track map (dependency order)

| Stage | Track | Owns | Depends on | Size |
|---|---|---|---|---|
| FE | **FE-DATAFLOW** | `tcl-compiler::{sccp,intervals,interval_bounds,memory_ssa,ssa}` | — | M |
| FE | **FE-TYPESHIM** ✅ | `tcl-compiler::{type_infer,value_shapes,rendered_properties,shimmer}` | — | M |
| FE | **FE-VARESCAPE** | `tcl-compiler::var_escape` | (consumers in FE-OPT) | M |
| FE | **FE-OPT** | `tcl-compiler::optimiser`, `inlining` | — | L |
| FE | **FE-CODEGEN** | `tcl-compiler::codegen` (non-wasm) | — | M |
| FE | **FE-DIAG** | `tcl-compiler::analyser`, `irules_checks` | — | M |
| FE | **FE-DIAG-F5** | new `tcl-xc`, `tcl-bigip` analyser slices, tk | `tcl-bigip` | L |
| RT | **RT-WASM** | `tcl-compiler::codegen::wasm`, `runtime/zig`, `tcl-wasm` bin | FE-CODEGEN | L |
| RT | **RT-VM** | `tcl-vm` | `tcl-bytecode` | L |
| SRV | **SRV-LSP** | `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db` | FE-DIAG, FE-DATAFLOW | L |
| TOOL | **TOOL-TCLPKG** | new `tcl-pkg` crate | — | XL |
| TOOL | **TOOL-REFACTOR** | `tcl-lsp-core::code_actions` | SRV-LSP | M |
| TOOL | **TOOL-F5** | `f5-cli` | — | XS |
| TOOL | **TOOL-EXPLORER** | `tcl-explorer`, `tcl-explorer-wasm` | RT-WASM | S |
| TOOL | **TOOL-FUZZ** | new `tcl-fuzz` (xtask/bin) | RT-VM | M |
| TOOL | **TOOL-DEBUGGER** | new `tcl-debugger` | RT-VM | L |
| TOOL | **TOOL-IRULE-TEST** | new `tcl-irule-test` | RT-VM, `tcl-registry` | XL |
| TOOL | **TOOL-CLI** | `tcl-cli` | RT-WASM, RT-VM, TOOL-TCLPKG | S |
| API | **API-PYO3** | `tcl-lsp-py`, `scripts`→`xtask`, `tests` | everything above | L |

---

### Stage 1 — Front-end residuals (FE-*)

The front-end crates are largely ported; what remains is precision and
soundness. These tracks own **disjoint modules** within `tcl-compiler`, so they
parallelise cleanly.

#### FE-DATAFLOW — SCCP / intervals / memory-SSA precision ✅
Owns `tcl-compiler::{sccp,intervals,interval_bounds,memory_ssa,ssa}`. All
residuals have landed:
- **done** SCCP escaping-var widening — `sccp` consults
  `var_observability::escaping_var_names` and forces `::`-qualified / aliased /
  traced definitions to OVERDEFINED (mirrors `_is_externally_mutable`).
- **done** SCCP optimistic (Wegman–Zadeck) UNKNOWN deferral + monotone
  finalising pass (`branch_deferrable`; the driver runs at most twice).
- **done** static-loop → SCCP fold — `LoopNode` carries the `IRFor` and the
  branch decision wires `summarise_for_statement`, folding a post-loop branch
  on a loop variable. The break-exit case needs no SCCP precompute: the Rust
  CFG builder already lowers `break` to a direct loop-exit edge, so the
  post-loop block is reachable by construction.
- **done** memory-SSA `IRUpFrame` clobber + the shared-caller upvar may-alias
  edge (`upvar 1 x a; upvar 1 x b` merge into one alias set).
- **done** W233 interval div-by-zero path — the production emitter now delegates
  to the (previously dead) interval-based `find_divide_by_zero`, the single
  canonical implementation; the SCCP-only copy is gone.
- **done** `complexity_guard` — `ssa::{COMPLEXITY_GUARD_BLOCKS,
  DEEP_ANALYSIS_BODY_BYTES, is_complexity_guarded}`, a trivial-SSA short-circuit
  in `build_ssa`, a `FunctionUnit::complexity_guarded` flag set from the
  block-count or body-byte ceiling, and `CompilationUnit::analysable_functions`
  filtering guarded bodies out of every per-proc diagnostic / optimiser pass.
  (The interprocedural summary is IR-based, so it needs no guard — it never
  develops the over-optimistic empty-SSA summary the Python guard exists to
  prevent.)

#### FE-VARESCAPE — var-escape wiring
Owns `tcl-compiler::var_escape`.
- **open** top-level orchestrator + `CfgEscapeResult → ProcEscapeSummary` driver
  (transfer functions are ported and tested but reachable only from their own
  tests).
- **open** the `pure_leaf` family (`safe_to_inline` / `safe_to_dce` /
  `safe_for_frame_elision`) + its interprocedural fixpoint. *(gated on FE-OPT's
  inliner / DCE consumers existing)*

#### FE-OPT — optimiser passes
Owns `tcl-compiler::optimiser`, `tcl-compiler::inlining`.
- **open** O120 string-compare (**soundness**) — `streq_promote_node`
  (`helpers/expr_simplify.rs:1040`) promotes any `==`/`!=` with a `String`
  operand to `eq`/`ne` with no non-numeric proof, so `$x == "1"` → `$x eq "1"`
  flips the result when `$x` is numeric. Gate on provably-non-numeric operands,
  mirroring Python's `_is_provably_non_numeric_expr_node` (`_expr_simplify.py:484`).
- **open** O114 incr-idiom — add the variable-numericity (INT) gate
  (`pattern_recognition.rs:219` gates only the literal; unsound for float `$x`).
- **open** O108 ADCE — restore the substitution / execution-intent purity gate
  (`elimination.rs` treats all assignments as side-effect-free).
- **partial** O104 (string-build chain) and O119 (multi-set packing) emit
  hint-only; port the applied rewrites + dead-write deletions.
- **open** O128 (end-offset index) and O130 (lappend chain) — implement
  (profile entries only today).
- **open** O106 — add `("O106", CodeMotion)` to `profiles.rs::OPT_CATEGORIES`
  (unsuppressable otherwise), thread execution-trace facts into the GVN/LICM
  family, and add the latch-dominance "runs every iteration" gate.
- **open** residuals: O103 namespace-chain resolution + rename gating; O123
  accumulator over-fire guards; O125 cross-event-var + multi-branch sinking;
  O110 missing instcombine identities; O101/O115 branch-condition coverage.
- **open** general proc inliner — port `compiler/inlining/` (the v0–v3 splice
  inliner, ~1900 LOC); only the narrow uplevel-inline idiom exists. *(large)*

#### FE-CODEGEN — bytecode codegen
Owns `tcl-compiler::codegen` (non-wasm).
- **open** statement-position specialisations — `append` / `lappend` / `unset` /
  `upvar` / `global` / `tailcall` / `concat` / `string` / `regexp` / `lindex` /
  `lreplace` / non-proc `dict` currently fall to generic `invokeStk`
  (0 emit-sites for the named opcodes); Rust specialises mostly in value
  position only. Add statement-position hooks; add divergent-corpus fixtures.
- **open** codegen-level expression constant folding (`expr {1+2}`).
- **open** small items: `esc` astral / C0-control escaping; `{*}` cmd-subst
  expansion; `set x [cmd]` pure-cmd-subst assign; the `builtin_is_trusted`
  rename gate.

#### FE-DIAG — analyser diagnostics & dialect checks
Owns `tcl-compiler::analyser`, `tcl-compiler::irules_checks`.
- **open** missing emitters: **E001** (missing subcommand), **W125** (orphaned
  control-flow keyword), **IRULE5005** (direct proc call without `call`),
  **IRULE1001** (command invalid/ineffective in event — high-impact,
  registry-legality-matrix driven).
- **open** snit OO support (`snit::type`/`widget`/`widgetadaptor` as ClassDef)
  and the OO body-walks Rust skips (`oo::class new`/`createWithNamespace`,
  `initialise` body, `property -get/-set` accessor bodies).
- **open** **W307 / W308** method-dispatch type checks + the precise TclOO
  `object_of` typing they consume (handed off from **FE-TYPESHIM**, which
  widens constructor results to OVERDEFINED today). The type lattice already
  models `TypeLattice::object_of(class)` and the join widens mismatched
  classes; what is missing is the *known-classes* set fed to
  `type_infer::return_type_for_command` so `[Foo new]` / `[Foo create x]` /
  `[Foo %AUTO%]` / `[Widget .path]` type as `OBJECT(::ns::Foo)` (relative
  names resolved against the call-site namespace). Source the class set from
  the analyser-layer `signature_scan` (the Rust IR keeps `namespace eval`
  bodies as raw barriers, so the Python `class_names.extract_class_names`
  IR-walk misses namespace-scoped classes — do **not** port it as-is), then
  thread it through `FunctionUnit::build`→`propagate_types`/
  `infer_function_return_type` and key the per-procedure lattice memo
  (`LatticeRequest` + `tcl-lsp-db`) on it, mirroring Python's
  `known_classes_fp`. Port `_return_type_for_command`'s constructor
  recognition (`new`/`create`/`createWithNamespace`/`%AUTO%`/leading-`.`),
  keeping the D4-F6 guard (no `object_of` from the `new` spelling alone when
  the command is not a known class).
- **open** IRULE1201 / 1202 / 5002 / 5004 path-sensitivity — the emitters are
  linear-scan MVPs (single shared `responded` flag); add per-branch state and
  restore the dropped quick-fixes (the C44 follow-up).
- **open** `DynamicNameLocal` reconciliation — Rust marks `scan`/`lassign`/
  `regexp`/`regsub` out-vars `VarWrite` and omits the trait, which the archive
  argues is benign; confirm whether withholding `VarWrite` changes caller-side
  W211/W214 suppression, then either add the trait or close the row.
- **open** source-style pass + W108 `non_ascii_mode` (line-length /
  trailing-whitespace / line-endings / comment-continuation / missing
  `package require`) and the GAP-C1 feature-config toggles.

#### FE-DIAG-F5 — F5 dialect diagnostics
Owns new analyser slices on `tcl-bigip` / `tcl-xc` and the tk dialect.
- **open** the four Python-only families with zero Rust emitters: **TK1001-1003**
  (geometry/widget/option), **BIGIP6001-6011** (config validator), **IAPP7001-7003**
  (iApp template), **XC100-301** (BIG-IP→F5-XC translator). *(large; decide
  Python-only vs Rust port per family)*

### Stage 2 — Runtime & execution (RT-*)

#### RT-WASM — WASM codegen + runtime
Owns `tcl-compiler::codegen::wasm`, `runtime/zig`, new `tcl-wasm` bin.
- **open** finish the WASM emitter (`wasm_codegen_module`) — only the Phase-1 IR
  + encoding (~1 K LOC) is ported vs the ~13-module Python package. *(large)*
- **open** `IRInterpBoundary` IR node + insert pass; the IR-rewriting
  `passes/dce.py` / `passes/gvn.py`; `source_inliner` / `stdlib_prelude`
  (WASM-bundle self-containment).
- **open** `tcl-wasm` CLI + `--link` (Binaryen) bundling; wire `tcl compwasm`.

#### RT-VM — bytecode VM
Owns `tcl-vm`.
- **open** the missing command surface: **TclOO** (largest), `clock`,
  `encoding`, full `interp`, real I/O (`open`/`gets`/`seek`), `after`/`time`,
  residual `file`/`info`/`namespace` subcommands. The engine core is solid
  (loads the real Tcl 9 `tcltest.tcl` end-to-end).
- **open** a VM CLI/REPL binary (`tcl-vm` is lib-only today) — also unblocks the
  `tcl dis` verb.

### Stage 3 — LSP server (SRV-LSP)

#### SRV-LSP — server / core / db
Owns `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db`.
- **open** incremental server reanalysis — the server re-analyses the whole
  document per edit; wire the bounded `reparse_window` + `analyse_incremental`
  (`analyser/state.rs:984`, already differential-fuzzed).
- **open** C1 UTF-16 residuals — route the remaining byte `position_at` sites
  (`folding.rs`, `irules_context.rs`) through the UTF-16 variant and set
  `position_encoding`. *(small; the hot paths already converted)*
- **open** performance — build the ~560-spec registry once into a process-wide
  `OnceLock` (rebuilt inside every `analyse`); build one shared
  `CompilationUnit` (taint/CU built 2–3× per document); add a debounce with
  cancel-previous.
- **open** panic containment (review-findings C3) — move the ~8 inline handlers
  to `spawn_blocking`.
- **open** token memo / reposition cache (the archive's `SYNC-MAY31-6/-11`) and a
  rope-backed `DocumentState`.

### Stage 4 — Tooling (TOOL-*)

These own distinct crates and parallelise; the ones marked *depends* are gated
on a library track above. This is the layer that, per the `tcl`/`f5` pattern
already started, brings **every** Python tool across to Rust.

- **TOOL-TCLPKG** *(new `tcl-pkg` crate; independent — start anytime)* — **open**:
  full package-manager port (manifest, MVS resolver, lockfile, CAS, fetchers,
  venv, docker), then wire the existing `pkg`/`venv`/`docker` CLI stubs in
  `tcl-cli`. *(XL)*
- **TOOL-REFACTOR** *(owns `tcl-lsp-core::code_actions`)* — **open**: port the 5
  missing transforms (extract/inline variable, if↔switch, switch→dict,
  extract-datagroup); 2 of 7 (extract/inline proc) exist. *(M)*
- **TOOL-F5** *(owns `f5-cli`)* — **partial**: `explain-flow`
  `--simulate`/`--tshark`/`--keylog`; dedicated `completion`/`graph` parity
  files. The rest (27/27 verbs, 262 parity tests) is done — the template for the
  other tool ports. *(XS)*
- **TOOL-EXPLORER** *(owns `tcl-explorer`, `tcl-explorer-wasm`; depends on
  RT-WASM)* — **partial**: the `wasm` view (other views parity-done; the
  Pyodide web server stays Python). *(S)*
- **TOOL-FUZZ** *(new `tcl-fuzz`; depends on RT-VM)* — **open**: a campaign runner
  + random generator + findings registry on top of the existing `differential_*`
  harnesses. *(M)*
- **TOOL-DEBUGGER** *(new `tcl-debugger`; depends on RT-VM)* — **open**: DAP server
  over `tcl-vm` (breakpoints/stepping/backends). *(L)*
- **TOOL-IRULE-TEST** *(new `tcl-irule-test`; depends on RT-VM + `tcl-registry`)* —
  **open**: the TMM-simulating test framework (topology/SCF parsing, profile-gen,
  mock-stub codegen, orchestrator). Note `tcl-irules` is the BIG-IP
  reference-extractor, **not** this. *(XL)*
- **TOOL-CLI** *(owns `tcl-cli`; depends on RT-WASM, RT-VM, TOOL-TCLPKG)* —
  **partial**: finish `dis` (after RT-VM), `compwasm` (after RT-WASM),
  `pkg`/`venv`/`docker` (after TOOL-TCLPKG). 20/26 verbs done. *(S glue)*

### Stage 5 — PyO3 interfaces & Python retirement (API-PYO3 — last)

#### API-PYO3
Owns `tcl-lsp-py`, the `scripts`→`xtask` migration, and `tests`. **This is the
final track** — every consumer above must port first.
- **open** the designed public PyO3 surface — re-derive it as a semver-stable
  API for downstream embedders, not a transcription of in-tree calls; the
  bindings today expose only folding / document symbols while the native server
  has ~43 providers.
- **open** TEST-MIGRATE — port each remaining pytest file to per-crate Rust
  tests (delete the `test_*.py` in the same change); the `test_fp_*` battery is
  the analyser acceptance gate.
- **open** rewrite `scripts/` build/release as `cargo xtask` (eliminate the
  Python toolchain dependency).
- **open** PYTHON-RETIRE — delete `compiler/`, `analyser/`, `server/`, and the
  ported `tooling/` subtrees once their consumers are Rust. `ai/` (MCP server +
  Claude skills) stays Python by design.

### Cross-cutting (fold into the owning track)

- **clippy hygiene** — 13 `too_many_lines` fn-level allows remain (codegen
  emitters, optimiser passes, registry-list fns); retire alongside the owning
  track's edits.
- **double taint / CU rebuild** — perf, owned by **SRV-LSP** (build once, share
  `&CompilationUnit`).
- **stale code comment** — `analyses.rs` "deferred" header contradicts the
  implemented `sccp.rs` / `type_infer.rs`; one-line fix.

## History

The full dated chunk-log — every landed `SYNC-*`, `GAP-AUDIT`, `ARCH*`, `C*`,
and `S*` entry, the per-spec command-tracking tables, and the detailed
sub-plans of work that has shipped — is archived in
[`rust-rewrite-history.md`](rust-rewrite-history.md). It is provenance, not a
plan; this document and the source are authoritative for current status.
