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
removed during PyO3-readiness work.  The remaining
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

We get there by porting the codebase bottom-up, in dependency order: each
layer's behaviour is reproduced and proven against the Python oracle before the
layer above it leans on it. The foundation layers (lexer, compiler, and the LSP
server) have already landed — that history is in the
[archive](rust-rewrite-history.md). What remains, organised into parallel tracks
in dependency order, is the [Remaining work](#remaining-work) section below.

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

The binding crate is now `rust/tcl-lsp-py/` (the `#[pymodule] tcl_lsp_py`
public PyO3 surface); `rust/tcl-lsp-rust/` has been reduced to a **transitional
alias** that re-exports `tcl-lsp-py` under the legacy `tcl_lsp_rust` module name
the Python shims still import, and retires in vNext. Treat both as compatibility
wrappers: neither owns compiler, analyser, registry, or LSP feature logic. The
public-API design work (the `tcl-lsp-py` surface proper) is **API-PYO3**, the
final track.

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
  **separate** distribution. That crate is now `rust/tcl-lsp-py/`
  (`rust/tcl-lsp-rust/` is a retiring re-export alias). No mixed
  hatchling/maturin hybrid.
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

The workspace (`Cargo.toml` members) as it stands today — crate granularity,
roughly in dependency order. New crates the [Remaining work](#remaining-work)
plan still calls for (`tcl-wasm`, `tcl-xc`) are **not** listed here because they
do not exist yet; `tcl-fuzz` (the differential fuzzer), `tcl-irule-test` (the
iRule-test glue scaffold), and `tcl-debugger` (the step-debugger scaffold) have
since landed.

```
Cargo.toml                workspace manifest        rust-toolchain.toml  channel = "stable"
rust/
  # --- shared vocabulary / host seam (leaf) ---
  tcl-core-types/         dependency-free shared vocabulary (Code, Completion, opaque handles)
  tcl-platform/           host-capability seam (Filesystem/Clock/Env/StdIo/Sockets/Process)
  tcl-host-native/        std-backed NativeHost (full-capability Host impl)
  tcl-cmd-core/           portable Tcl command logic (string/list/dict/…) generic over ValueOps
  tcl-runtime-api/        Family-B runtime-state contract (handles, role traits, CompileService)
  # --- lexer / syntax ---
  tcl-lexer/              position-aware lexer (Span, LineIndex, SourceMap, CST) for Tcl + dialects
  tcl-syntax/             shared parse-tree + byte-exact semantics (lists, subst, expr, format)
  # --- registry (single source of truth) ---
  tcl-registry/           command metadata: ArgRole, Arity, Traits, taint, hooks, BytePayloadSpec,
                          commands/{tcl,irules}/*.rs (one file per command)
  # --- compiler + runtime ---
  tcl-bytecode/           Tcl 9 bytecode artifact types (opcodes, FunctionAsm/ModuleAsm, layout, disasm)
  tcl-compiler/           IR, lowering, CFG, SSA, dataflow (sccp/intervals/memory_ssa), type_infer,
                          shimmer, var_escape, optimiser, inlining, analyser, irules_checks, codegen/{,wasm}
  tcl-vm/                 native Rust bytecode VM (TCLVM)
  # --- F5 dialect crates ---
  tcl-bigip/              BIG-IP object model + config parser     tcl-bigip-io/  UCS archive + path resolver
  tcl-bigip-query/        BIG-IP query DSL (front-end landed)     tcl-irules/    BIG-IP object-ref extractor
  # --- LSP ---
  tcl-lsp-core/           pure LSP feature providers (folding, symbols, diagnostics, inlay_hints, source_style)
  tcl-lsp-db/             salsa incremental DB (file_analysis_incremental, semantic_tokens, lattice memo)
  tcl-lsp-server/         tower-lsp binary (async document store, request routing, cancellation)
  tcl-lsp-py/             public PyO3 API crate (#[pymodule] tcl_lsp_py)
  tcl-lsp-rust/           transitional alias re-exporting tcl-lsp-py under the legacy tcl_lsp_rust name
  # --- tooling ---
  tcl-explorer/           compiler-explorer pipeline + serialiser (CLI/TUI/WASM consume this)
  tcl-explorer-wasm/      Rust → WASM compile() facade for the explorer GUI (excluded from the workspace)
  tcl-cli-support/        shared CLI plumbing for the native tcl / f5 CLIs
  tcl-cli/                native `tcl` toolchain CLI                f5-cli/  native `f5-query` CLI
  tcl-fuzz/               differential fuzzer (seeded generator + tclvm-vs-tclsh harness + findings)
  tcl-irule-test/         iRule TMM-sim glue: SCF→orchestrator topology + session bootstrap (driver gated on RT-VM)
  tcl-debugger/           working record-and-replay step debugger over tcl-vm + the `tcl-debug` CLI front-end
runtime/
  zig/                    Zig WASM runtime (out-of-process runtime for compiled scripts)
  rust/                   tree-walking reference runtime (RT-VM parity oracle)
scripts/build_zipapp.py   _RUST_NATIVE_PACKAGES strip rule
.github/workflows/ci.yml  rust job + rust-gate (cargo tests + lsp_e2e) + release wheel matrix
Makefile                  rust-build/test/lint/format; check-rust; test-rust
tests/test_rust_bindings_smoke.py   end-to-end bridge smoke test
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
The plan reflects current source as of 2026-06-20 (`rust` branch). The
**FE-DATAFLOW**, **FE-TYPESHIM**, **FE-VARESCAPE**, **FE-DIAG** front-end tracks
and **SRV-LSP** have landed since the last audit; their detail moved
to the [history archive](rust-rewrite-history.md) and they survive here only as
table rows (✅ / 🟢).

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
  document-version guard (review-findings C2), the CST descent, the whole
  **FE-LEX** track (`${name}` brace-depth, quoted `\<newline>`, nested-body
  E202/E203 — archived 2026-06-19), and the now-complete **FE-DATAFLOW** /
  **FE-TYPESHIM** / **FE-VARESCAPE** / **FE-DIAG** tracks (archived 2026-06-19).
  Trust this plan and the source over the archive's dated rows.
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
| IR / lowering / CFG / SSA | `tcl-compiler` | ✅ | `IRUpFrame` clobber + dynamic-`uplevel` barrier (`body_has_dynamic_barrier`) + minor IR fields landed under **FE-DATAFLOW** / **FE-DIAG** |
| SCCP / intervals / memory-SSA | `tcl-compiler` | ✅ | escaping-var widening, optimistic deferral, static-loop folding, W233 interval path, `complexity_guard` all landed — **FE-DATAFLOW** complete (see [history](rust-rewrite-history.md)) |
| Type inference / shimmer / shapes / rendered-props | `tcl-compiler` | ✅ | core landed; precise TclOO `object_of` typing landed under **FE-DIAG**; **S110** byte-array-corruption shimmer (Python #656) ported (`tcl-compiler::shimmer::byte_array` + `tcl-registry` `BytePayloadSpec`) — **FE-TYPESHIM** complete |
| var-escape | `tcl-compiler::var_escape` | ✅ | orchestrator (`analyse_var_escape` IR + CU paths) + `pure_leaf` family (`safe_to_inline`/`safe_to_dce`/`safe_for_frame_elision`) + transitive fixpoint landed (FE-VARESCAPE complete, see [history](rust-rewrite-history.md)) |
| Optimiser passes | `tcl-compiler::optimiser`, `tcl-compiler::inlining` | 🟢 | every O-code pass + the inliner v0/verbatim shapes landed (see [history](rust-rewrite-history.md)); sole remaining: inliner **v3** (α-rename, gated on the RT-WASM consumer for execution-differential verification) → **FE-OPT** |
| Bytecode codegen | `tcl-compiler::codegen` | 🟢 | state-mutating statement-position specialisations + `expr` const-fold + byte-wise `esc` landed (byte-true vs tclsh9.0; VM opcodes implemented to match); residual: `set x [cmd]` (reverted — needs VM value opcodes), bare-statement `string`/`regexp`/`lindex`/`lreplace`, non-proc `dict` (ensemble `invokeReplace`), `{*}` cmd-subst expansion → **FE-CODEGEN** |
| Analyser diagnostics | `tcl-compiler::analyser` | ✅ | every family ported + verified (E001/W125/IRULE5005, snit, OO body-walks, W307/W308, C44 path-sensitivity + IRULE5002/5004/2001 quick-fixes, `when`-body gating, source-style/W108, #662 lockstep fixes) — see [history](rust-rewrite-history.md). The two consumer-wiring residuals (per-check config toggles, flow-warning code actions) landed under **SRV-LSP** |
| F5 dialect diagnostics | `tcl-compiler::analyser::tk_checks`, `tcl-bigip::{validator,apl}`, new `tcl-xc` | 🟢 | TK1001-3 + BIGIP6001-11 + IAPP7001-3 ported (TK live in the analyser); residuals: XC100-301 (gated on a `tcl-xc` translator port of `lower_to_ir`-walking `translator.py`, Python-only meanwhile) + BIG-IP/iApp native-server consumer-wiring (SRV-LSP-style) → **FE-DIAG-F5** |
| WASM codegen + runtime | `tcl-compiler::codegen::wasm`, `runtime/zig`, new `tcl-wasm` | 🔴 | eval-fallback emitter + `tcl compwasm` wiring landed (binary/WAT, `wasmtime`-validated); residual: `IRInterpBoundary`; codegen DCE/GVN; `--link` (Binaryen) bundling → **RT-WASM** |
| Bytecode VM | `tcl-vm` | 🟡 | tcltest parity vs `runtime/rust` in progress (info/proc hangs; namespace/var/upvar depth; error `[try]`-coverage); TclOO; clock/encoding/interp/IO/after. `tclvm` CLI/REPL binary landed (`tcl-vm-cli`) → **RT-VM** |
| LSP server / core / db | `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db` | ✅ | #670 bulk + the two consumer-wiring residuals (GAP-C1 per-check config toggles; IRULE5002/5004 flow-warning code actions) landed — see [history](rust-rewrite-history.md). The rope-backed `DocumentState` is split out into its own **SRV-ROPE** track (need evaluated with measurements in [`design/rope/`](design/rope/README.md)) |
| Document store / incrementality | `tcl-lsp-server`, `tcl-lexer`, `tcl-lsp-db` | 🔴 | rope-backed store + chunk-addressable salsa input + rope-slice re-lex → **SRV-ROPE** (see [`design/rope/`](design/rope/README.md)) |
| `tcl` CLI | `tcl-cli` | ✅ | all 26 verbs ported & dispatched (`dis`/`compwasm` + `pkg`/`venv`/`docker` wired via TOOL-TCLPKG) → **TOOL-CLI** |
| `f5-query` CLI | `f5-cli`, `tcl-bigip*`, `tcl-irules` | 🟢 | `explain-flow --tshark/--keylog/--tshark-filter` landed; `--simulate` gated on the native iRule VM (**RT-VM**); `completion`/`graph` parity files → **TOOL-F5** |
| Formatter / minifier / diagram | `tcl-lsp-core`, `tcl-cli` | ✅ | — |
| Refactoring transforms | `tcl-lsp-core::code_actions` | ✅ | all 7 transforms ported (`tcl-lsp-core::refactor`), byte-parity vs the Python oracle → **TOOL-REFACTOR** |
| Compiler explorer | `tcl-explorer`, `tcl-explorer-wasm` | 🟢 | `wasm` view renders the eval-fallback emitter's WAT; rich per-instruction web-GUI shape (`to_explorer_json`) is a refinement → **TOOL-EXPLORER** |
| Package manager (`tclpkg`) | `tcl-pkg` | ✅ | full port (manifest/resolver/lockfile/CAS/fetchers/venv/docker) + wired `pkg`/`venv`/`docker` CLI → **TOOL-TCLPKG** |
| Differential fuzzer | `tcl-fuzz` | 🟢 | campaign runner + seeded generator + findings registry land (`tclvm` vs `tclsh`); broaden the generator grammar as the VM surface grows → **TOOL-FUZZ** |
| Debugger | `tcl-debugger` | ✅ | record-and-replay step debugger over `tcl-vm` (VM debug-hook seam) with a `tcl-debug` CLI **and** a DAP server for editors (`--dap`): breakpoints, step in/over/out, continue, stack/scopes/variables, evaluate → **TOOL-DEBUGGER** |
| iRule test framework | `tcl-irule-test` | 🟡 | crate scaffolded: SCF→orchestrator topology generator (parity-checked vs Python) + session-bootstrap assembly; the live event round-trip is gated on the VM iRule surface (**RT-VM**) → **TOOL-IRULE-TEST** |
| PyO3 public API + retirement | `tcl-lsp-py`, `xtask` | 🔴 | designed public surface; TEST-MIGRATE; PYTHON-RETIRE → **API-PYO3** |
| `ai/` (MCP + skills) | — | n/a | stays Python by design |

### Track map (dependency order)

| Stage | Track | Owns | Depends on | Size |
|---|---|---|---|---|
| FE | **FE-DATAFLOW** ✅ | `tcl-compiler::{sccp,intervals,interval_bounds,memory_ssa,ssa}` | — | M |
| FE | **FE-TYPESHIM** ✅ | `tcl-compiler::{type_infer,value_shapes,rendered_properties,shimmer}` | — | M |
| FE | **FE-VARESCAPE** ✅ | `tcl-compiler::var_escape` | — | M |
| FE | **FE-OPT** | `tcl-compiler::optimiser`, `inlining` | — | L |
| FE | **FE-CODEGEN** | `tcl-compiler::codegen` (non-wasm) | — | M |
| FE | **FE-DIAG** ✅ | `tcl-compiler::analyser`, `irules_checks` | — | M |
| FE | **FE-DIAG-F5** 🟢 | `tcl-compiler::analyser::tk_checks`, `tcl-bigip::{validator,apl}` slices, tk; XC residual → `tcl-xc` | `tcl-bigip` | L |
| RT | **RT-WASM** | `tcl-compiler::codegen::wasm`, `runtime/zig`, `tcl-wasm` bin | FE-CODEGEN | L |
| RT | **RT-VM** | `tcl-vm`, `tcl-vm-cli` (`tclvm` bin) | `tcl-bytecode` | L |
| SRV | **SRV-LSP** ✅ | `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db` | FE-DIAG, FE-DATAFLOW | L |
| SRV | **SRV-ROPE** | document store: `tcl-lsp-server` `DocumentState` + `tcl-lexer` rope-slice `SourceMap` + `tcl-lsp-db` chunk input | FE-LEX (CST/structural-state index), SRV-LSP | XL |
| TOOL | **TOOL-TCLPKG** ✅ | `tcl-pkg` crate | — | XL |
| TOOL | **TOOL-REFACTOR** ✅ | `tcl-lsp-core::code_actions` | SRV-LSP | M |
| TOOL | **TOOL-F5** | `f5-cli` | — | XS |
| TOOL | **TOOL-EXPLORER** 🟢 | `tcl-explorer`, `tcl-explorer-wasm` | RT-WASM | S |
| TOOL | **TOOL-FUZZ** 🟢 | `tcl-fuzz` (bin) | RT-VM | M |
| TOOL | **TOOL-DEBUGGER** ✅ | `tcl-debugger` | RT-VM | L |
| TOOL | **TOOL-IRULE-TEST** 🟡 | `tcl-irule-test` | RT-VM, `tcl-registry` | XL |
| TOOL | **TOOL-CLI** ✅ | `tcl-cli` | RT-WASM, RT-VM, TOOL-TCLPKG | S |
| API | **API-PYO3** | `tcl-lsp-py`, `scripts`→`xtask`, `tests` | everything above | L |

---

### Stage 1 — Front-end residuals (FE-*)

The front-end crates are largely ported; what remains is precision and
soundness. These tracks own **disjoint modules** within `tcl-compiler`, so they
parallelise cleanly.

#### FE-OPT — optimiser passes
Owns `tcl-compiler::optimiser`, `tcl-compiler::inlining`. **Every O-code
optimiser pass is complete**, as are the **inliner v0 + verbatim** shapes
(2026-06-19; archived in [history](rust-rewrite-history.md)). The sole remaining
task:
- **open** general proc inliner **v3 (parameterised)** — α-renaming via
  `_rename.py` over value strings / expr ASTs / defs-reads / foreach-catch
  bindings, variadic packing, parameter defaults, trailing-vs-non-trailing
  `return` for/break wrapping, plus dead-proc elimination. **Cross-track
  dependency (handoff):** the inliner's only consumer is the WASM codegen
  (`compiler/codegen/wasm/api.py`), so execution-differential verification of
  the capture-sensitive value-string rewriter is gated on **RT-WASM** (🔴
  unported) — v3 must land alongside that consumer rather than as
  IR-shape-only unit tests (the repo's differential-test standard cannot be
  met otherwise). *(large)*
- **open** *(non-correctness nuances, optional)* the O110 regex/glob →
  string-op rewrites (gated on the iRules `MatchesGlob`/`MatchesRegex` expr
  operators) and threading richer execution-trace facts deeper into the
  GVN/LICM family — precision niceties, never miscompiles.

#### FE-CODEGEN — bytecode codegen
Owns `tcl-compiler::codegen` (non-wasm). The state-mutating statement-position
specialisations, integer `expr` const-folding, and the byte-wise disassembly
escaping have landed, each verified byte-true against tclsh9.0 with golden
fixtures (`append`/`lappend`/`unset`/`upvar`/`global`/`tailcall`/`concat`;
`expr {1+2}`; `esc` astral/C0). Their bytecode VM counterparts (appendScalar/
Array, lappendScalar/Array/List, unsetScalar/Array, upvar, nsupvar, concatStk)
were implemented in `tcl-vm` so the codegen runs end-to-end. Remaining:
- **open** `set x [cmd]` pure-command-substitution assign — inlining was tried
  (route the single-`[cmd]` value through the inline-cmd-subst emitter) but
  reverted: the emitter produces value opcodes the partial bytecode VM does not
  implement (`regexp`, `strclass`, `numericType`, `dictGet`), which broke the
  VM execution tests. Re-land once **RT-VM** implements those value opcodes
  (the inlining itself was byte-true for the VM-supported commands).
- **open** statement-position specialisations for the *value-returning*
  commands used as bare statements — `string` / `regexp` / `lindex` /
  `lreplace` (result discarded; the value-position inline forms exist, so this
  is value-emit + `pop`, gated on threading the per-arg braced-flag through the
  hook). Low frequency; `regexp` with match-vars is the one with real
  statement-position semantics.
- **open** non-proc `dict` — top-level `dict set`/etc. compile to the **ensemble
  `invokeReplace`** form in C Tcl (`push …; push ::tcl::dict::set;
  invokeReplace`), not the proc-local `DICT_*` opcodes. This is the shared
  ensemble-rewrite mechanism (applies to all top-level ensembles), tracked as a
  small follow-on rather than a dict-only hook.
- **open** `{*}` expansion inside a *command substitution* in value position
  (`set x [cmd {*}$args]` → `expandStart … expandStkTop N; invokeExpanded`);
  statement-position `{*}` already lowers correctly. (The `builtin_is_trusted`
  rename gate originally filed here is a **WASM-emitter** concern —
  `compiler/codegen/wasm/_emitter/*` only — and belongs to **RT-WASM**.)

#### FE-DIAG-F5 — F5 dialect diagnostics
Owns new analyser slices on `tcl-bigip` / `tcl-xc` and the tk dialect. The
per-family port decision (the track's explicit "decide Python-only vs Rust
port per family") is made and three of the four families are landed; the
fourth is gated on a separate subsystem port and stays Python-only until it
lands.
- **landed** **TK1001-1003** (geometry/widget/option) — ported to
  `tcl-compiler::analyser::tk_checks`, gated on the `tk` dialect: TK1002
  non-existent-parent + TK1003 unknown-option run per command, TK1001
  pack/grid conflict is decided post-walk from accumulated per-parent
  geometry usage (flushed from `run_diagnostic_emitters`).
- **landed** **BIGIP6001-6011** (config validator) — ported to
  `tcl-bigip::validator` (`validate_bigip_config` / `validate_bigip_source`)
  producing ranged `ConfigDiagnostic`s over a parsed `BigipConfig`, reusing
  the lint engine's `ModelView` / `KindMap` / `resolve_name` (now
  `pub(crate)`). The `regex` crate has no look-around, so the two Python
  negative look-aheads (`pool !member`, `persist !none`) are filtered in
  code.
- **landed** **IAPP7001-7003** (iApp template) — ported to
  `tcl-bigip::apl::{iapp_vars,iapp_diagnostics}`:
  `extract_iapp_var_refs` pulls `$::section__field` implementation
  references (ReDoS-safe regex), and `validate_iapp_presentation` /
  `validate_iapp_implementation` emit IAPP7001/7002/7003 over the existing
  `AplModel`, gated on the `f5-iapps` / `f5-tmsh` / `f5-bigip` dialects.
- **open (deferred, Python-only)** **XC100-301** (BIG-IP→F5-XC translator)
  — the XC-series diagnostics are a thin wrapper over `translate_irule`
  (`dialects/f5/xc/diagnostics.py`), but that translator (`translator.py`
  ~1.2 K LOC + the 13-type `xc_model` + `mapping`) walks the **IR produced
  by `lower_to_ir`** and is a distinct, large subsystem in its own right.
  Porting it is a separate effort (akin to how **FE-OPT** v3 is gated on
  **RT-WASM**); until a `tcl-xc` translator port lands, XC100-301 stays the
  Python emitter. Handoff: a future `tcl-xc` crate owns this — re-home the
  XC diagnostic wrapper there once the translator exists.

Consumer wiring: TK1001-1003 run inside the analyser, so they already
surface through the native server's diagnostics path (the
`TCL_LSP_DIAG_BACKEND=rust` bridge). The BIG-IP and iApp checks are
model-level validators (`validate_bigip_source` /
`validate_iapp_{presentation,implementation}`) exposed as `tcl-bigip` crate
APIs; routing them into the native server for BIG-IP-config / APL documents
(the analogue of `server/features/diagnostics.py`’s file-type dispatch) is a
**SRV-LSP**-style consumer-wiring residual, mirroring the per-check toggle /
flow-warning code-action residuals **FE-DIAG** handed to **SRV-LSP**.

### Stage 2 — Runtime & execution (RT-*)

#### RT-WASM — WASM codegen + runtime
Owns `tcl-compiler::codegen::wasm`, `runtime/zig`, new `tcl-wasm` bin.
- **open** finish the WASM emitter (`wasm_codegen_module`) — only the Phase-1 IR
  + encoding (~1 K LOC) is ported vs the ~13-module Python package. *(large)*
- **open** `IRInterpBoundary` IR node + insert pass; the IR-rewriting
  `passes/dce.py` / `passes/gvn.py`; `source_inliner` / `stdlib_prelude`
  (WASM-bundle self-containment).
- **done** wire `tcl compwasm` (`tcl-cli`): drives the eval-fallback emitter,
  writes the binary module (`--output`) and optional WAT (`--wat-output`); the
  emitted bytes pass `wasmtime compile`. The `_write_binary_output` analogue
  landed as `tcl_cli_support::write_binary_output`.
- **open** `tcl-wasm` CLI + `--link` (Binaryen) bundling (standalone
  self-contained module via `source_inliner` / `stdlib_prelude`).

#### RT-VM — bytecode VM
Owns `tcl-vm` (+ the `tcl-vm-cli` / `tclvm` CLI driver).

The engine core is solid (loads the real Tcl 9 `tcltest.tcl` end-to-end). The
active workstream is **tcltest pass/fail/skip parity** with the more-complete
tree-walking `runtime/rust`, both pinned against C Tcl 9.0.3 (§0). Harness:
`tmp/parity.sh` runs `tcl-vm`'s `run_test` example and `runtime/rust`'s
`run_script --init` over each `tmp/tcl9.0.3/tests/*.test` and tabulates P/S/F; a
suite is at parity when the two columns match. The landing log is in the
[history](rust-rewrite-history.md) (2026-06-19); the per-suite snapshot + gaps
below are the live state.

The 2026-06-19 parity push (`namespace eval`/`inscope` real call frames,
`try`/`throw` as VM builtins, nested-`foreach`/`lmap` runtime routing, `namespace
delete`, and the supporting `return`/`error`/list-parse fixes — error.test
44 → 280, lmap 21 → 61, namespace 93 → 112) is archived in the
[history](rust-rewrite-history.md). The live open gaps:

- **open (P1) "suite-zeroing hangs" are mostly mis-attributed** (diagnosed
  2026-06-21). The `info.test` / `proc.test` "hangs" are **not** deadlocks: built
  `--release`, `info.test` runs to *info-8.3* in ~12 s and `proc.test` runs to
  `cleanupTests` in ~8 s. Two real effects masquerade as a hang under the
  harness, in priority order:
  - **(a) Debug-build slowness × the harness timeout.** A `--release` worker is
    ~10–30× faster; the `run_test`/`run_script` parity harness must build
    `--release` (a trivial tcltest case costs ≈1 s/test in `debug`, so a
    ~200-test suite blows any timeout in `debug` while finishing in seconds in
    `release`). **First action: switch `tmp/parity.sh` to `--release`** and
    re-baseline — several "hung" suites likely already score.
  - **(b) Uncaught-error abort.** A test-body error that escapes tcltest's
    `catch` propagates to the module top and aborts the *whole* `run_test`
    driver (e.g. `info.test` halts at info-8.3 with `can't read "text": no such
    variable`; `proc.test` ends on a bare `VM error:`). That single escape, not
    a loop, is what zeros the remainder of the suite — the real P1 to chase
    (an `uplevel`/`catch`/error-propagation gap), and far more tractable than a
    deadlock hunt.
- **fixed (2026-06-21) the runtime `while`/`for` frozen-loop infinite spin.** A
  loop whose **condition is a bare command substitution** can't inline to a CFG
  loop (only an *expression* condition — `[cmd] > 0`, `$x`, `!![cmd]` — does), so
  `cfg_builder` converts it to a "frozen" runtime `while`/`for` call. That
  conversion built the barrier with `tokens: None`, so the codegen lost each
  word's source kind and pushed the braced `{cond}` word **non-verbatim** —
  `subst_word` evaluated the condition's command substitution **once** at the
  call site, freezing the loop into an infinite spin (`while {[string length
  $x]} {set x ""}` never terminated; the `> 0` variants did). The `While`/`For`
  IR now carries the segmenter's `raw_tokens`, and the frozen barrier reuses
  them so each word is pushed exactly as written (braced → verbatim, `$body` →
  substituted). Verified byte-equal to `tclsh9.0` and covered by
  `tcl-vm/tests/language_e2e.rs::while_command_subst_condition_*`. **This was the
  most common real "hang" idiom** — tcltest's own
  `while {[string length $argList]}` word-splitter spun — so several suites that
  appeared to "hang" should now progress (re-baseline the parity harness in
  `--release`). The **same token-loss bug** was then fixed for the other
  runtime loop fallbacks — the `dict for`/`map` barrier and the runtime
  `foreach`/`lmap` call (qualified loop vars / non-inline) — by extending
  `raw_tokens` to `Foreach`. **Audit:** only the three loop types have a
  runtime fallback; `switch`/`catch`/`try`/`if` inline their bodies and need no
  token preservation.
- **fixed (2026-06-21) composite array-element index substitution.** A read or
  write of `$a(prefix$var)` / `$a(-$opt)` / `$a(${item}suf)` pushed the index as
  a raw literal, so the embedded variable never expanded (`can't read
  "a(x$item)"`). tcltest's `$testAttributes(-$item)` option processing hit this,
  aborting `info.test` after the loop fix. `push_array_key` now decodes a
  composite index into its substitution parts and `STR_CONCAT`s them (byte-equal
  to `tclsh9.0`).
- **open** `namespace` / `var` / `upvar` depth (≈ 290 failures combined) — the
  namespace-eval-frame fix corrected the *mechanism*; the remainder is
  feature/semantics depth (namespace-name canonicalisation of multiple/trailing
  `::` runs; deeper variable-scoping / introspection). The three share the model
  and likely move together.
- **open** `error.test` (29 failing) — 22 are the `[try]` coverage generated
  tests (`return -level $level -code $code`), which need the **`-level`
  countdown** the VM simplifies (only `-level 0` is immediate today); the rest
  are `info errorstack` / the `-errorstack` option (unimplemented) and errorInfo
  edge cases.
- **open** smaller per-suite gaps — `switch` (14), `for` (8), `foreach` (6),
  `incr` (3), `if`/`set` (2), `while` (1): individual bugs to chase after the
  structural items.
- **open** structural follow-up — give the bytecode backend real exception
  ranges (`beginCatch`) and a fixed nested-complex-foreach / lmap-collecting
  codegen, then drop the `for_bytecode` barriers (`try`/nested-`foreach`/`lmap`
  run via runtime builtins today — correct but not inline).
- **open** the missing command surface: **TclOO** (largest), `clock`,
  `encoding`, full `interp`, real I/O (`open`/`gets`/`seek`), `after`/`time`,
  residual `file`/`info`/`namespace` subcommands. Concretely, `info` is missing
  `cmdcount` / `frame` / `functions` / `hostname` (they error "unknown or
  ambiguous subcommand" today); note `info cmdcount` cannot reach exact-count
  parity with C Tcl without matching its per-bytecode command counting, so it
  is a "exists but approximate" subcommand rather than a parity win.
- **done** a VM CLI/REPL binary — the `tclvm` binary in the new `tcl-vm-cli`
  crate (a thin compiler-backed `CompileService` driver, keeping the `tcl-vm`
  lib compiler-optional). Runs script files (with `argv`/`argv0`/`argc`),
  evaluates inline scripts (`-c`), pipes stdin, and offers an `info
  complete`-aware REPL (`-i` forces it without a TTY). The `tcl dis` verb it
  was paired with is wired in `tcl-cli` (bytecode disassembly via
  `format_module_asm`, with `--optimise`).

### Stage 3 — LSP server (SRV-LSP) — complete

**SRV-LSP has landed** (`tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db`). The
#670 bulk (incremental salsa reanalysis, UTF-16 `position_encoding`, registry/CU
sharing + debounce, `spawn_blocking` panic containment, the `semantic_tokens`
token memo, `codeLens/resolve`, inlay type hints) and the two consumer-wiring
residuals handed over from **FE-DIAG** — GAP-C1 per-check config toggles and the
IRULE5002/5004 flow-warning code actions — are all shipped; the detail is in the
[history archive](rust-rewrite-history.md).

The rope-backed `DocumentState` that previously sat here as a deferred bullet is
now its own track, **SRV-ROPE** (below), with the need evaluated against
measurements rather than asserted.

#### SRV-ROPE — rope-backed document store + incremental pipeline
Owns the document-store seam across `tcl-lsp-server` (`DocumentState`),
`tcl-lexer` (rope-slice `SourceMap`), and `tcl-lsp-db` (chunk-addressable salsa
input). Depends on **FE-LEX** (the landed CST descent + structural-state index,
which bounds the dirty re-lex region) and **SRV-LSP**. Full motivation, the
reproducible experiment, and task breakdown live in
[`design/rope/README.md`](design/rope/README.md); the headline:

- **Measured, not asserted.** A workspace-excluded harness
  ([`design/rope/experiment/`](design/rope/experiment/)) compares the current
  `String` edit path against `ropey` across file sizes, edit-burst sizes, high
  edit rates, salsa-flatten cost, position lookups, and many-small-doc memory.
- **A standalone `DocumentState` swap is not worth it.** A rope cannot help the
  paramount metric (time-to-first-tokens is a full-buffer `didOpen`), cannot make
  salsa incremental (the input interns a `String`; the rope must flatten `O(n)`
  every edit), and costs **1.4–1.9× memory** for the many-small-files workload.
  The per-edit bottleneck is *analysis* (re-lex + salsa invalidation), O(n) and
  rope-invariant until the pipeline goes incremental.
- **Most of the apply-side win needs no rope.** The `String` path is slow because
  it rebuilds `LineIndex` and double-allocates a spliced `String` per edit; a
  *persisted, incrementally-patched `LineIndex`* captures the bulk at ~0 memory
  cost. **That is SRV-ROPE Task 1 and the recommended first step.**
- **open** Tasks (smallest-first, each independently shippable): (1) persisted
  incremental `LineIndex` on the `String` store — *do first, no rope*; (2) rope
  behind a feature flag in `DocumentState` with burst-coalescing + a
  many-small-doc memory guard; (3) `LineIndex::from_rope_slice` +
  `Lexer::with_source_map` rope-slice re-lex in `tcl-lexer`; (4) **the real
  prize** — chunk-addressable salsa `SourceFile` input so `set_text` interns only
  changed chunks and `file_analysis_incremental` / the segmenter re-lex only the
  dirty span (touches `tcl-lsp-db`, `tcl-compiler::parsing`, the recovery index);
  (5) MVCC write-window minimisation (folds into 4); (6) a committed
  `perf_track` bench gating **no time-to-first-tokens regression**.
- **Exit criterion:** keep the rope only if, with Task 4 landed, end-to-end
  per-edit latency on large files improves materially *and* many-small-doc memory
  stays under ~1.2×. If Task 1 alone captures the realistic win, Tasks 2–5 stay
  deferred and the `String` store is retained — the experiment is the gate.

### Stage 4 — Tooling (TOOL-*)

These own distinct crates and parallelise; the ones marked *depends* are gated
on a library track above. This is the layer that, per the `tcl`/`f5` pattern
already started, brings **every** Python tool across to Rust.

- **TOOL-TCLPKG** *(new `tcl-pkg` crate; independent)* — **done**: full
  package-manager port — `version` (semver + Tcl-friendly ordering), `manifest`
  (whitelisted-directive parser with safe-mode refusal, no VM), `lockfile`
  (canonical JSON, byte-identical with Python), MVS `resolver`, `cas`
  (worktree-canonical SHA-256 + store/materialise), `fetchers` (gzip-tarball /
  zip / git / path over `ureq`), `registry` (TTL cache + conditional GET),
  `venv`, `docker`, plus `ui`. The `pkg`/`venv`/`docker` CLI stubs in `tcl-cli`
  are wired through to native handlers; help is native clap style. Lockfiles,
  Dockerfiles, and venv scripts diff byte-for-byte against the Python CLI.
  *(XL)*
- **TOOL-REFACTOR** *(owns `tcl-lsp-core::code_actions`)* — **done**: the 5
  remaining transforms (extract/inline variable, if↔switch, switch→dict,
  extract-datagroup) are ported into the new `tcl-lsp-core::refactor` module
  and wired through `refactor_engine_actions`, alongside the pre-existing
  extract/inline proc. `find_command_at` descends registry-resolved
  `ArgRole::Body` words for nested-body support; the data-group action carries
  the rendered tmsh definition on `CodeAction::data_group_definition` (surfaced
  as the LSP `data` field, mirroring `_datagroup_to_code_action`). The
  if↔switch / switch→dict / datagroup outputs are asserted byte-for-byte
  against the live Python oracle; all `tests/test_refactoring.py` decline
  conditions are mirrored. Out of scope: `suggest_datagroup_extraction` (an
  AI-context scanner, not a `CodeAction`). *(M)*
- **TOOL-F5** *(owns `f5-cli`)* — **partial**: the `explain-flow`
  `--tshark` / `--keylog` / `--tshark-filter` L7-enrichment paths landed
  (`tcl_bigip::flow::tshark`, a faithful EK-JSON port of
  `dialects/f5/bigip/flow/tshark.py`, byte-identical to the Python CLI on the
  `--tshark` / `--tshark-filter` paths; graceful degradation when tshark lacks
  EK `--no-duplicate-keys` support is itself parity-matched). Remaining:
  `--simulate` is gated on the native iRule VM (the `tooling.irule_test`
  orchestrator → **RT-VM** / **TOOL-IRULE-TEST**) and stays a clear
  not-implemented error meanwhile; plus dedicated `completion`/`graph` parity
  files. The rest (27/27 verbs, 262 parity tests) is done — the template for the
  other tool ports. *(XS)*
- **TOOL-EXPLORER** *(owns `tcl-explorer`, `tcl-explorer-wasm`; depends on
  RT-WASM)* — **done (for the views)**: the `ssa`-view boolean-render parity bug
  landed (`view_tree::pystr`), and the **`wasm` view now renders WAT** —
  `serialise.rs::serialise_wasm` drives the eval-fallback `wasm_codegen_module`
  (the same emitter `tcl compwasm` uses) and emits the module's WAT plus
  per-function headers, which `render.rs::render_wasm` prints. All TUI views are
  now populated. Refinement (not blocking the view): the rich per-instruction
  explorer shape (`to_explorer_json` — resolved call targets, paired branch
  targets, lane-assigned edges) the *web GUI* uses is not ported, so the wasm
  tab shows flat WAT rather than the gutter-rendered instruction graph; that
  lands with the broader **RT-WASM** emitter work. The Pyodide web server stays
  Python. *(S)*
- **TOOL-FUZZ** *(new `tcl-fuzz` bin; depends on RT-VM)* — **landed**: a seeded
  random Tcl generator (`generator.rs`, scoped to the VM's supported surface so a
  divergence is a real miscompile, not an unimplemented command — pure, bounded
  loops, balanced delimiters), a subprocess differential harness
  (`harness.rs`: `tclvm` subject vs `tclsh` reference, each timeout-killable so a
  hang is a finding not a wedge), a campaign runner with stats (`campaign.rs`),
  and a findings registry (`findings.rs`: JSON + raw `.tcl`, dedup-by-seed,
  categorised, replayable). CLI verbs `run` / `replay` / `summary`. A
  500-iteration campaign over the current VM is clean (498/500 match, 2 skipped,
  0 findings — the loop/array fixes hold under fuzzing). **Remaining:** broaden
  the generator grammar (procs, namespaces, dict, `catch`/`try`, switch) as the
  VM command surface fills in, and add the WASM/Zig backend as a third
  differential arm once **RT-WASM** lands. *(M)*
- **TOOL-DEBUGGER** *(new `tcl-debugger`; depends on RT-VM)* — **working**: a
  record-and-replay step debugger over `tcl-vm`. The RT-VM piece **landed** —
  `tcl-vm` now has a debug-hook seam (`Vm::set_debug_hook`): it fires once per
  source command (keyed on `(line, span-start)`, since `startCommand` is emitted
  only conditionally) with a full `DebugSnapshot` (line, command text, call
  stack, current-frame variables) and honours the returned `DebugAction`. The
  seam is inert (an `Option` check) when no hook is installed — differential
  parity gates stay green. `tcl-debugger`'s `VmBackend` installs a recording
  hook, runs the script once to capture the trace, then serves
  breakpoints / step-in/over/out / continue / stack / variable-inspection by
  navigating the trace with the `DebugController`. Two front-ends drive it: a
  `tcl-debug` interactive CLI
  (`break`/`step`/`next`/`finish`/`continue`/`print`/`stack`/`vars`) and a
  **DAP server** (`tcl-debug --dap`) speaking the Debug Adapter Protocol over
  `Content-Length`-framed stdio — `initialize`/`launch`/`setBreakpoints`/
  `threads`/`stackTrace`/`scopes`/`variables`/`continue`/`next`/`stepIn`/
  `stepOut`/`evaluate`/`disconnect` plus the `initialized`/`stopped`/
  `terminated` events — so editors integrate directly. Known replay-model
  characteristics (documented, not unfinished): script output is produced once
  up front, variable inspection is the current frame's captured scope, and
  `evaluate` resolves captured variables. *(L)*
- **TOOL-IRULE-TEST** *(new `tcl-irule-test`; depends on RT-VM + `tcl-registry`)* —
  **scaffolded**: the crate exists with the portable glue. `topology` ports
  `TopologyFromSCF` — parse a bigip.conf via `tcl-bigip` and emit the `::orch::`
  setup (profiles via name-inference, VIP, pools + members, data-groups,
  attached iRules), parity-checked against the Python generator. `session`
  defines the `SessionPlan` / orchestrator-bootstrap assembly the VM driver
  will run. **Architecture note:** the TMM simulation itself is the ~500 KB of
  Tcl under `tooling/irule_test/tcl/` (orchestrator + TMM shim + command
  mocks); the Rust port runs that Tcl on **`tcl-vm`**, so the live
  event/`run_*`/`assert_*` round-trip is gated on the VM growing the iRule
  command surface (`HTTP::*`, `pool`/`node`, `LB::*`, `when` dispatch,
  `class match`). Remaining: the VM-driven session execution, profile-object
  type resolution, profile-gen, and the mock-stub/registry/event-data codegen.
  Note `tcl-irules` is the BIG-IP reference-extractor, **not** this. *(XL)*
- **TOOL-CLI** *(owns `tcl-cli`; depends on RT-WASM, RT-VM, TOOL-TCLPKG)* —
  **done**: all 26 top-level verbs are ported and dispatched to native engine
  handlers. `dis` (bytecode disassembly via `format_module_asm`) and `compwasm`
  (eval-fallback WASM binary/WAT via the greenfield emitter) landed earlier; the
  `pkg`/`venv`/`docker` verbs are wired through to the `tcl-pkg` handlers that
  landed under **TOOL-TCLPKG**. The dispatch `match` is now exhaustive (the
  not-yet-implemented fallthrough is gone). Note any deeper WASM-emitter
  precision residual is **RT-WASM**'s, not the CLI verb's. *(S glue)*

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

- **clippy hygiene** — a scatter of `too_many_lines` fn-level allows remain
  (codegen emitters, optimiser passes, registry-list fns, CLI command bodies);
  retire alongside the owning track's edits.
- **double taint / CU rebuild** — perf, owned by **SRV-LSP** (build once, share
  `&CompilationUnit`).

## History

The full dated chunk-log — every landed `SYNC-*`, `GAP-AUDIT`, `ARCH*`, `C*`,
and `S*` entry, the per-spec command-tracking tables, and the detailed
sub-plans of work that has shipped — is archived in
[`rust-rewrite-history.md`](rust-rewrite-history.md). It is provenance, not a
plan; this document and the source are authoritative for current status.
