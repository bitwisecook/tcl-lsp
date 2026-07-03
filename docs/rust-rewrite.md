# Python → Rust rewrite

> **Update (2026): completed — Python is fully retired on this branch.**
> The rewrite goal below ("zero Python in the shipping product") has been
> reached. The product is now purely the Rust workspace + native binaries
> (`tcl`, `f5-query`, `tcl-lsp-server`, `tcl-mcp`); the Python source
> (`shared/ compiler/ dialects/ analyser/ server/ tooling/ ai/`, the
> `.pyz` zipapps, `uv`, and the pytest suites) no longer exists here. The
> `lsp_e2e` suite is native (`rust/tcl-lsp-server/tests/*_e2e.rs`, run by
> `cargo test`). The PyO3 facade surface discussed as a *future* public
> API was not shipped on this branch. This document is retained as the
> historical rewrite plan and oracle-parity record; its "Remaining work",
> "not yet", and "🟡/🔴 status" sections describe the migration as it
> stood mid-flight, not the current (completed) state.

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
  tcl-irule-test/         iRule TMM-sim: SCF→orchestrator topology + `LiveSession` running the orchestrator Tcl on tcl-vm (embedded framework)
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

**Main-sync status (last synced 2026-07-01 via PR #732 — `main` merged up
through `v1.11.4`/`#705`, so `main` is once again an ancestor of `rust` and the
behind-count is 0).** The Python-side deltas that PR carried in are all present
in-tree now: `#662` catch/return flow (also delta-ported in **FE-DIAG**),
`#656`/`#661` S110 byte-array corruption (also in **FE-TYPESHIM**), and — new
with this sync — the full **Tcl 9.1** Python surface (see below). The
rust-branch-irrelevant deltas (release notes, CI/security, the `#664`
registry-dump demotion) came across as ordinary merge content. One
language-surface delta is now **half-ported**: the Python side landed with the
merge, but the **Rust crates still lack it**:

- **Tcl 9.1 dialect (`#673`, `main` commit `5d2ae37a`) — Python present, Rust
  pending.** `main` added a `tcl9.1` entry to `KNOWN_DIALECTS`
  (`compiler/registry/dialects.py`) plus three 9.1-only command specs — `timer`
  (`dialects/tcl/timer.py`), `unicode` (`dialects/tcl/unicode_.py`), and the
  `subst -backslashes/-commands/-variables` options (`subst_.py`), all gated on
  the `tcl9.1` dialect. As of the 2026-07-01 sync **all of this now exists in the
  Python tree** (`tcl9.1` is in `dialects.py`; `timer.py`/`unicode_.py` are
  present), but **no Rust crate carries it yet** (`git grep -i tcl9.1 -- rust/`
  is empty). The remaining port is Rust-only: the `tcl-registry` crate needs the
  new dialect flag + command specs, `LexerConfig`/dialect gating must learn
  `tcl9.1`, and the analyser/codegen must treat the new commands and `subst`
  options as dialect-gated. Until that lands, the Rust analyser lags the Python
  oracle's dialect surface. Note this also bears on principle §0 ("C Tcl 9.0.3 is
  the reference standard"): 9.0.3 stays the pinned reference (no 9.1 source tree
  is fetched under `tmp/`), so 9.1 support is a *dialect-flag* addition, not a
  reference-standard bump — but a future task may need to decide whether to
  advance the differential oracle once a 9.1 `tclsh` is available.

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
The plan reflects current source as of 2026-07-01 (`rust` branch), re-verified
that day against the crate source and cross-checked against the Python oracle on
`main`. The **FE-DATAFLOW**, **FE-TYPESHIM**, **FE-VARESCAPE**, **FE-DIAG**
front-end tracks and **SRV-LSP** have landed; their detail moved to the
[history archive](rust-rewrite-history.md) and they survive here only as table
rows (✅ / 🟢).

> **Scope of this plan.** The gaps tracked here are the **tooling, LSP,
> compiler/analysis, and public-API** port — the target is to finish these
> first. The **runtime & execution** layers (WASM codegen **RT-WASM**, the
> bytecode VM **RT-VM**, and the `runtime/rust` tree-walking port), **including
> the tiered plan for bringing the VMs and runtime to C-Tcl parity**, are a
> **separate scope** enumerated in their own index:
> [`design/runtime/runtime-execution-gaps.md`](design/runtime/runtime-execution-gaps.md).
> Their rows survive in the subsystem-status / track-map tables below as
> pointers, but the detail is not duplicated here. The `ai/` **shell** (MCP
> server + Claude skills, prompts, templates) stays Python by design, but `ai/`
> is a **consumer** of the retiring Python engine (7 of 13 files import
> `analyser`/`compiler`/`tooling`/`server.features`/`dialects`), so re-pointing
> it off that engine is real **PYTHON-RETIRE** work under **API-PYO3** — the ai
> *port* is n/a, the ai *re-pointing* is not.

### Vocabulary

One set of terms, used consistently (the older *chunk / phase / slice / strip /
strand / family / wave / pillar / candidate* vocabulary survives only in the
archive):

- **Stage** — a dependency layer (1 Front-end → 2 Runtime → 3 Server →
  4 Tooling → 5 Public API). Stages are ordered; tracks within a stage are not.
  **Stage 2 (Runtime & execution) is tracked in its own scope** — see
  [`design/runtime/runtime-execution-gaps.md`](design/runtime/runtime-execution-gaps.md).
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
| Optimiser passes | `tcl-compiler::optimiser`, `tcl-compiler::inlining` | ✅ | every O-code pass + the **full inliner** (v0/verbatim **and** v3 α-rename + parameter binding + return-as-break wrap, `tcl-compiler::inlining{,::rename}`) landed (see [history](rust-rewrite-history.md)). Out of scope: v3's execution-differential verification is owned by its consumer (**RT-WASM**); the optional non-correctness O110 rewrites are gated on the iRules `MatchesGlob`/`MatchesRegex` expr operators → **FE-OPT** |
| Bytecode codegen | `tcl-compiler::codegen` | 🟢 | state-mutating statement-position specialisations + `expr` const-fold + byte-wise `esc` + the `set x [cmd]` inline re-land landed (byte-true vs tclsh9.0; VM opcodes implemented to match); residual: bare-statement `string`/`regexp`/`lindex`/`lreplace` (value-discarded) → **FE-CODEGEN** |
| Analyser diagnostics | `tcl-compiler::analyser` | ✅ | every family ported + verified (E001/W125/IRULE5005, snit, OO body-walks, W307/W308, C44 path-sensitivity + IRULE5002/5004/2001 quick-fixes, `when`-body gating, source-style/W108, #662 lockstep fixes); `ProcArgTrait::DynamicNameLocal` added so caller-side W211/W214/dead-store false positives stay suppressed (parity-audit gap #6, 2026-06-25) — see [history](rust-rewrite-history.md). The two consumer-wiring residuals (per-check config toggles, flow-warning code actions) landed under **SRV-LSP** |
| F5 dialect diagnostics | `tcl-compiler::analyser::tk_checks`, `tcl-bigip::{validator,apl}`, `f5-xc` | ✅ | all four families ported & consumer-wired: TK1001-3 (analyser), BIGIP6001-11 + IAPP7001-3 (routed into the native server via `f5_dialect_diagnostics`, push+pull), and XC100-301 (new **`f5-xc`** crate — `translate_irule` IR-walker + `get_xc_diagnostics`, parity-tested vs the Python oracle; opt-in `xcDiagnostics` toggle wired into the `f5-irules` diagnostics path) — see [history](rust-rewrite-history.md) → **FE-DIAG-F5** |
| WASM codegen + runtime | `tcl-compiler::codegen::wasm`, `runtime/zig`, new `tcl-wasm` | 🟡 | **separate scope** → [`design/runtime/runtime-execution-gaps.md`](design/runtime/runtime-execution-gaps.md) §1 (RT-WASM). Headline: eval-fallback emitter + `tcl compwasm` wiring landed; ~1.5 K Rust LOC vs the ~20.6 K-LOC / 49-module Python emitter — the largest single gap |
| Bytecode VM | `tcl-vm` | 🟡 | **separate scope** → [`design/runtime/runtime-execution-gaps.md`](design/runtime/runtime-execution-gaps.md) §2 (RT-VM). Headline: differential `bug_*` cmd-tests all closed (2026-06-25); 98/39/54 of 191 opcodes; 28/59/10 of 97 tcltest stems; TclOO/coroutine still VM-absent |
| Regex engine (ARE) | `tcl-regex` | ✅ | pure-Rust port of Tcl 9's Henry-Spencer ARE engine (no C FFI, no `unsafe`). Passes `reg.test` 544/544 + the `regexp.test` command corpus (engine-relevant cases) as Rust cargo tests vs the real engine. Drives **both** runtimes via the `cmd-core` `RegexEngine` provider — the VM (replacing the `regex` crate) and `runtime/rust` (replacing the C Henry-Spencer engine: `build.rs`/FFI/`regex_shim` removed, so `regexp` now works on wasm32 too). C consumers link the `runtime/rust` C-ABI shim (`regex_capi`, `TclReComp`/`TclReExec`/…). Residual: cmd-plumbing `-about`/`regsub -command`/`-start`-validation gaps live in `tcl-cmd-core`. See [rust-regex-port.md](design/runtime/rust-regex-port.md) |
| LSP server / core / db | `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db` | ✅ | #670 bulk + the two consumer-wiring residuals (GAP-C1 per-check config toggles; IRULE5002/5004 flow-warning code actions) landed; BIG-IP find-references / document-links / code-action providers + "Generate docstring" parity landed (parity-audit gap #8, 2026-06-25) — see [history](rust-rewrite-history.md). The document-store / per-edit-incrementality work is its own **SRV-INCREMENTAL** track (the rope was measured and demoted; design in [`design/srv-incremental/`](design/srv-incremental/README.md)) |
| Document store / incrementality | `tcl-lsp-db`, `tcl-compiler`, `tcl-lsp-server`, `tcl-lexer` | 🟢 | persisted incremental `LineIndex` (Task 1), per-function check memo (2a), incremental interprocedural-taint memo (2b), **the full cross-file cascade (Task 6 — W123 + arity, per-symbol `command_arity` early-cutoff, corpus-scale multi-file fuzzer)**, **Task 4 (per-procedure `optimise_unit` memo)**, and **Task 3 (incremental per-item IR lowering, `lower_proc_body` memo) gated v1** all landed byte-identical (full-corpus-verified); **Tasks 5 (windowed re-lex) + 7 (rope store) dropped — rope-dependent, removed from scope 2026-06-30**; residual: broaden the Task 3 body-cache eligibility gate → **SRV-INCREMENTAL** (see [`design/srv-incremental/`](design/srv-incremental/README.md)) |
| `tcl` CLI | `tcl-cli` | ✅ | all 26 verbs ported & dispatched (`dis`/`compwasm` + `pkg`/`venv`/`docker` wired via TOOL-TCLPKG) → **TOOL-CLI** |
| `f5-query` CLI | `f5-cli`, `tcl-bigip*`, `tcl-irules` | 🟢 | `explain-flow --tshark/--keylog/--tshark-filter` + `--simulate` (iRule run live on `tcl-vm` via `tcl-irule-test`) landed; residual: `f5 irule lint/context/trace/pgo` sub-verbs not implemented (parse + exit 2), SSH/scp fetch transport not ported (REST works; falls back to Python CLI), `registry-dump --section commands` not implemented → **TOOL-F5** |
| Formatter / minifier / diagram | `tcl-lsp-core`, `tcl-cli` | 🟢 | minifier + diagram byte-parity; formatter engine ported, residual: the **docstring rewriter** is unimplemented (config flags carried but not engine-consumed) |
| Refactoring transforms | `tcl-lsp-core::code_actions` | ✅ | all 7 transforms ported (`tcl-lsp-core::refactor`), byte-parity vs the Python oracle → **TOOL-REFACTOR** |
| Compiler explorer | `tcl-explorer`, `tcl-explorer-wasm` | 🟢 | `wasm` view renders the eval-fallback emitter's WAT; rich per-instruction web-GUI shape (`to_explorer_json`) ported (`tcl_explorer::wasm_explorer`: resolved call/branch targets, block-pairing, ranges) — densifies automatically as RT-WASM emits real instructions → **TOOL-EXPLORER** |
| Package manager (`tclpkg`) | `tcl-pkg` | ✅ | full port (manifest/resolver/lockfile/CAS/fetchers/venv/docker) + wired `pkg`/`venv`/`docker` CLI → **TOOL-TCLPKG** |
| Differential fuzzer | `tcl-fuzz` | 🟢 | campaign runner + seeded generator + findings registry land (`tclvm` vs `tclsh`); generator grammar broadened to procs/namespaces/dict/`catch`/`try`/`switch` (RT-VM-gated work done, 1.5 K-iter campaign @ 0 findings); WASM-runnability arm landed (`wasm-check`: compile→`wasmtime`, 600-program campaign clean); WASM **value**-differential arm landed (`wasm-diff`: in-process wasmtime with a `tcl-vm`-backed eval-fallback host, fuel-bounded `WasmHang` detection — verifies control-flow codegen, already caught a non-terminating-loop bug the runnability arm can't); residual: re-back that arm with the **real linked Zig runtime** for a full value differential, gated on **RT-WASM** → **TOOL-FUZZ** |
| Debugger | `tcl-debugger` | ✅ | record-and-replay step debugger over `tcl-vm` (VM debug-hook seam) with a `tcl-debug` CLI **and** a DAP server for editors (`--dap`): breakpoints, step in/over/out, continue, stack/scopes/variables, evaluate → **TOOL-DEBUGGER** |
| iRule test framework | `tcl-irule-test` | 🟢 | SCF→orchestrator topology generator + `LiveSession` running the TMM-sim orchestrator live on `tcl-vm` (load iRule, fire events, read pool/logs/decisions; 14 integration tests green); framework Tcl embedded for self-contained consumers. Residual: auto-broadening coverage **plus** the session's `event dispatch` / `class match` handlers (not yet implemented) → **TOOL-IRULE-TEST** |
| PyO3 public API + retirement | `tcl-lsp-py`, `xtask` | 🟡 | designed public surface **landed** (`parse_tcl`/`compile_tcl`/`analyse_tcl`/`format_tcl`/`parse_bigip_config`/`query_bigip` facades + `TclLspError` hierarchy, `tcl-lsp-py::public`); residual: TEST-MIGRATE; `scripts`→`xtask`; PYTHON-RETIRE → **API-PYO3** |
| `ai/` (MCP + skills) | — | n/a (port) / 🟡 (re-point) | shell stays Python by design, but `ai/` imports the retiring engine (`analyser`/`compiler`/`tooling`/`server.features`/`dialects`) — re-pointing it onto Rust is **PYTHON-RETIRE** work → **API-PYO3** |

### Track map (dependency order)

| Stage | Track | Owns | Depends on | Size |
|---|---|---|---|---|
| FE | **FE-DATAFLOW** ✅ | `tcl-compiler::{sccp,intervals,interval_bounds,memory_ssa,ssa}` | — | M |
| FE | **FE-TYPESHIM** ✅ | `tcl-compiler::{type_infer,value_shapes,rendered_properties,shimmer}` | — | M |
| FE | **FE-VARESCAPE** ✅ | `tcl-compiler::var_escape` | — | M |
| FE | **FE-OPT** ✅ | `tcl-compiler::optimiser`, `inlining` | — | L |
| FE | **FE-CODEGEN** 🟢 | `tcl-compiler::codegen` (non-wasm) | — | M |
| FE | **FE-DIAG** ✅ | `tcl-compiler::analyser`, `irules_checks` | — | M |
| FE | **FE-DIAG-F5** ✅ | `tcl-compiler::analyser::tk_checks`, `tcl-bigip::{validator,apl}`, `f5-xc` (all four families ported + consumer-wired) | `tcl-bigip`, `f5-xc` | L |
| RT | **RT-WASM** 🟡 *(separate scope — [runtime-execution-gaps.md](design/runtime/runtime-execution-gaps.md))* | `tcl-compiler::codegen::wasm`, `runtime/zig`, `tcl-wasm` bin | FE-CODEGEN | L |
| RT | **RT-VM** 🟡 *(separate scope — [runtime-execution-gaps.md](design/runtime/runtime-execution-gaps.md))* | `tcl-vm`, `tcl-vm-cli` (`tclvm` bin) | `tcl-bytecode` | L |
| SRV | **SRV-LSP** ✅ | `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db` | FE-DIAG, FE-DATAFLOW | L |
| SRV | **SRV-INCREMENTAL** 🟢 | per-edit pipeline: Tasks 1/2a/2b (`LineIndex` + per-function check + interproc-taint memos), Task 6 (cross-file cascade, incl. per-symbol `command_arity` cutoff + corpus fuzzer), Task 4 (`optimise_unit` memo), and Task 3 (per-item IR-lowering `lower_proc_body` memo, gated v1) all landed byte-identical; **Tasks 5 + 7 dropped (rope-dependent, 2026-06-30)**; residual: broaden the Task 3 body-cache eligibility gate | FE-LEX (structural-state index), SRV-LSP | L |
| TOOL | **TOOL-TCLPKG** ✅ | `tcl-pkg` crate | — | XL |
| TOOL | **TOOL-REFACTOR** ✅ | `tcl-lsp-core::code_actions` | SRV-LSP | M |
| TOOL | **TOOL-F5** 🟢 | `f5-cli` | RT-VM, TOOL-IRULE-TEST | S |
| TOOL | **TOOL-EXPLORER** 🟢 | `tcl-explorer`, `tcl-explorer-wasm` | RT-WASM | S |
| TOOL | **TOOL-FUZZ** 🟢 | `tcl-fuzz` (bin) | RT-VM | M |
| TOOL | **TOOL-DEBUGGER** ✅ | `tcl-debugger` | RT-VM | L |
| TOOL | **TOOL-IRULE-TEST** 🟢 | `tcl-irule-test` | RT-VM, `tcl-registry` | XL |
| TOOL | **TOOL-CLI** ✅ | `tcl-cli` | RT-WASM, RT-VM, TOOL-TCLPKG | S |
| API | **API-PYO3** 🟡 | `tcl-lsp-py`, `scripts`→`xtask`, `tests` | everything above | L |

---

### Stage 1 — Front-end residuals (FE-*)

The front-end crates are largely ported; what remains is precision and
soundness. These tracks own **disjoint modules** within `tcl-compiler`, so they
parallelise cleanly.

#### FE-CODEGEN — bytecode codegen
Owns `tcl-compiler::codegen` (non-wasm). The state-mutating statement-position
specialisations, integer `expr` const-folding, byte-wise disassembly escaping,
the `set x [cmd]` pure-command-substitution assign, the non-proc `dict` mutators
(ensemble `invokeReplace`), and `{*}` expansion inside command substitutions
have all landed byte-true vs tclsh 9.0 (their bytecode-VM opcode counterparts
implemented in `tcl-vm` so codegen runs end-to-end; detail in the
[history archive](rust-rewrite-history.md)). **Residual:**
- **open** statement-position specialisations for the *value-returning*
  commands used as bare statements — `string` / `regexp` / `lindex` /
  `lreplace` (result discarded; the value-position inline forms exist, so this
  is value-emit + `pop`, gated on threading the per-arg braced-flag through the
  hook). Low frequency; `regexp` with match-vars is the one with real
  statement-position semantics.

#### FE-DIAG-F5 — F5 dialect diagnostics
Owns new analyser slices on `tcl-bigip` / `f5-xc` and the tk dialect. The
per-family port decision (the track's explicit "decide Python-only vs Rust
per family") is made: **TK1001-1003**, **BIGIP6001-6011**,
**IAPP7001-7003**, and **XC100-301** are all landed (detail in the
[history archive](rust-rewrite-history.md) → *FE-DIAG-F5*).
- **landed** **XC100-301** (BIG-IP→F5-XC translator) — the gating subsystem
  (the `translator.py` ~1.2 K LOC IR-walker + the 13-type `xc_model` +
  `mapping`) is ported to the new **`f5-xc`** crate
  (`f5_xc::{translator,model,mapping,diagnostics}`): `translate_irule` walks
  the IR from `lower_to_ir_with_config(.., "f5-irules")` and
  `get_xc_diagnostics` emits the XC100-301 hints. The IR-walk, condition
  decomposition, and XC-construct extraction are verified byte-for-byte
  against the Python oracle by a 38-case differential parity harness
  (`f5-xc/tests/parity.rs` ⇐ `f5-xc/tests/gen_fixture.py`). The registry-driven
  translatability gates (`is_xc_never_translatable` /
  `is_xc_translatable_override`) landed on `tcl-registry`. The **opt-in**
  native-server consumer-wiring also landed: `get_xc_diagnostics` is surfaced
  through both the push and pull diagnostics paths for `f5-irules` documents
  under the default-off `xcDiagnostics` feature toggle (`lift_xc_diagnostics`,
  filtered by disabled codes + `# noqa` / file suppression), the analogue of
  Python `server/features/diagnostics.py`'s `xc_diagnostics_enabled` gate.

Consumer wiring: TK1001-1003 run inside the analyser, so they already
surface through the native server's diagnostics path (the
`TCL_LSP_DIAG_BACKEND=rust` bridge). The BIG-IP and iApp model-level
validators (`validate_bigip_source` /
`validate_iapp_{presentation,implementation}`) are **now routed into the
native server** (`tcl-lsp-server`'s `f5_dialect_diagnostics` file-type
dispatch — landed, see [history](rust-rewrite-history.md) → *FE-DIAG-F5*):
a BIG-IP-config document publishes `BIGIP6001-6011` and an iApp APL
presentation publishes `IAPP7001-7003` (with sibling-implementation
cross-checking) on both the push and pull diagnostics paths, the analogue
of `server/diagnostics_pipeline.py`'s `_publish_bigip_diagnostics` /
`_publish_apl_diagnostics` dispatch. The previously-deferred `f5-xc` (BIG-IP→
F5-XC) translator has since landed (the `XC100-301` bullet above), so this track
carries no remaining residual.

### Stage 2 — Runtime & execution (RT-*) — separate scope

The **runtime & execution** tracks — **RT-WASM** (WASM codegen emitter +
`tcl-wasm` bundling), **RT-VM** (the `tcl-vm` bytecode VM), and the
`runtime/rust` tree-walking port — are enumerated in their own index, together
with the **tiered capability-ladder plan** for bringing the VMs and runtime to
C-Tcl 9.0.3 parity:

> [`design/runtime/runtime-execution-gaps.md`](design/runtime/runtime-execution-gaps.md)

That index is the single entry point for the runtime scope. It links the live,
regenerable trackers (VM opcode coverage in
[`design/runtime/tclvm-opcode-status.md`](design/runtime/tclvm-opcode-status.md),
per-stem tcltest parity in
[`design/runtime/rust-vm-tier-parity.md`](design/runtime/rust-vm-tier-parity.md),
the tree-walking-port breakdown in
[`design/runtime/rust-runtime-port.md`](design/runtime/rust-runtime-port.md)) and
the tiered delivery plan (the capability ladder in
[`design/runtime/tcl-test-tiers.md`](design/runtime/tcl-test-tiers.md), the
`tcltest` bring-up plan in
[`design/runtime/tcltest-bringup.md`](design/runtime/tcltest-bringup.md), and the
Zig-runtime roadmap in
[`design/runtime/zig-runtime-roadmap.md`](design/runtime/zig-runtime-roadmap.md)).
The landed runtime work (the 2026-06-19 parity push, the 2026-06-21/22
follow-ons, and the 2026-06-25 differential-cmd-test closures) is in the
[history archive](rust-rewrite-history.md).

### Stage 3 — LSP server (SRV-LSP) — complete

**SRV-LSP has landed** (`tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db`). The
#670 bulk (incremental salsa reanalysis, UTF-16 `position_encoding`, registry/CU
sharing + debounce, `spawn_blocking` panic containment, the `semantic_tokens`
token memo, `codeLens/resolve`, inlay type hints) and the two consumer-wiring
residuals handed over from **FE-DIAG** — GAP-C1 per-check config toggles and the
IRULE5002/5004 flow-warning code actions — are all shipped; the detail is in the
[history archive](rust-rewrite-history.md).

The **BIG-IP LSP surface** (parity-audit gap #8) is now substantially closed
(2026-06-25): the dialect-specific find-references (`tcl-bigip::refs`),
document-links (`tcl-bigip::links`), and code-action providers replaced the
generic-Tcl fallbacks, the "Generate docstring" action reached parity with the
Python `generate_stub`, and `tcl-lsp-server`'s `execute_command` /
document-lifecycle / diagnostic-core internals gained direct coverage. The
residual is the handful of BIG-IP `execute_command` verbs that delegate to
`tcl-bigip-query` (e.g. `renamePartition`, `writeRuleBack`) — out of this
branch's scope.

The rope-backed `DocumentState` that previously sat here is **demoted**: a measured
experiment put it at ~0.02% of per-edit latency. The document-store /
per-edit-incrementality work is now the **SRV-INCREMENTAL** track (below), with the
rope as an optional, gated final step.

#### SRV-INCREMENTAL — making the per-edit pipeline incremental
Owns finishing end-to-end per-edit incrementality across `tcl-lsp-db`,
`tcl-compiler`, `tcl-lsp-server`, and `tcl-lexer` — *within a file and across the
project*. Builds on the largely-shipped per-item analyser firewall
([`design/rust/incremental-analysis.md`](design/rust/incremental-analysis.md)) and
depends on **FE-LEX** (the landed structural-state index that bounds the dirty
re-lex region) and **SRV-LSP**. Full measurement, the cross-file cascade design,
and the task breakdown live in
[`design/srv-incremental/README.md`](design/srv-incremental/README.md); the
headline:

- **Measured, not asserted.** `tail_profile` on `linalg.tcl` (warm db, single-char
  body edit) puts warm per-edit latency at ~411 ms, of which **whole-file
  `run_all_checks` is ~405 ms**. Buffer apply — the rope's slice — is ~85 µs
  (**0.02%**). Two workspace-excluded harnesses
  ([`design/srv-incremental/experiment/`](design/srv-incremental/experiment/))
  measure both halves.
- **The prize was per-procedure check incrementality — now landed.** The firewall
  made the analyser walk + per-proc lattices incremental, but `run_all_checks` /
  `optimise_unit` had re-run over the **whole unit** every edit (~99% of latency).
  Memoising them per-proc (keyed on the offset-invariant `FnLatticeKey` the
  lattices already use) was the highest-leverage change and shipped as Tasks
  2a/2b/4 — warm `compiler_check_diagnostics` fell ~445 → ~83 ms; an unrelated
  body edit now re-checks and re-optimises exactly one proc.
- **Cross-file cascade — now landed (Task 6).** The `WorkspaceIndex` had been off
  the salsa graph (`resolve_proc_call` per-file, editing file A recomputed nothing
  in file B). A `Project` salsa input now lifts the project signature table into
  salsa so cross-file resolution / arity are tracked edges — reverse-dependency
  invalidation for free, bounded by Tcl's dynamic dispatch, with the per-symbol
  `command_arity` early-cutoff so an unrelated proc's signature edit no longer
  wakes a file.
- **Task status** (each independently shippable, each fuzzer-gated):
  - **landed** (1) persisted incremental `LineIndex` on the `String` store;
    (2) **the prize** — per-proc `run_all_checks` / `optimise_unit` salsa memo
    (`function_checks` 2a + `proc_taint_solve` / `proc_summary_cascade` 2b);
    (4) the per-procedure `optimise_unit` memo (`function_optimisations` +
    whole-module `finalise_optimisations`, byte-identical); (3) **Approach A —
    incremental per-item IR lowering** (`lower_proc_body` memo keyed on
    `ProcBodyKey`, gated by `file_body_cache_eligible`, byte-identical
    incremental == fresh, corpus-verified); (6) the cross-file cascade (project
    signature table in salsa — W123 + cross-file arity, per-symbol
    `command_arity` early-cutoff, corpus-scale multi-file differential fuzzer).
  - **dropped 2026-06-30 (rope-dependent):** (5) windowed re-lex via
    `reparse_window` / the structural-state index; (7) rope store +
    chunk-addressable `SourceFile` input. The `String` store is retained.
  - **open (residual):** broaden the Task 3 body-cache eligibility gate
    (`file_body_cache_eligible` conservatively disqualifies bodies touching
    `namespace`/`interp`/`rename`/OO/`apply`/nested-proc so the per-item memo
    stays byte-identical to a whole-module lowering; widening it recovers more
    warm-edit reuse).
- **Exit criterion:** met for the shipped tasks — every landed task is gated by
  an `incremental == fresh` differential fuzzer (in-crate + corpus `--ignored`)
  asserting byte-identity with a from-scratch build. The rope (7) would only
  re-open if its measured 0.02% per-edit slice grew *and* many-small-doc memory
  stayed under ~1.2× — otherwise the `String` store is retained.

### Stage 4 — Tooling (TOOL-*)

These own distinct crates and parallelise; the ones marked *depends* are gated
on a library track above. This is the layer that, per the `tcl`/`f5` pattern
already started, brings **every** Python tool across to Rust. **Most of the
stage has landed** — the landing logs are in the
[history archive](rust-rewrite-history.md) and the tracks survive in the
subsystem-status / track-map tables above. Only the 🟢 tracks carry residuals:

- **TOOL-EXPLORER** 🟢 *(depends on RT-WASM)* — the rich per-instruction web-GUI
  shape (`to_explorer_json`) has **landed** (2026-06-22):
  `tcl_explorer::wasm_explorer` ports the Python serialiser over the Rust WASM
  IR — resolved `call` target labels (imports + internal functions), `br`/`br_if`
  targets resolved against the enclosing structured construct, block-pairing
  indices (`openIdx`/`elseIdx`/`endIdx`) for edge layout, per-instruction
  source ranges, and indent. It is wired into `serialise_wasm` alongside the
  WAT text the TUI renders. The graph is *sparse* under the current
  eval-fallback tier (most leaf commands are a single `call` to the imported
  `tcl_eval`) and densifies automatically as the real **RT-WASM** emitter emits
  more instructions — no further explorer work is required for that. The Pyodide
  web server stays Python. *(S)*
- **TOOL-FUZZ** 🟢 *(depends on RT-VM)* — the RT-VM-gated generator broadening
  has **landed** (2026-06-22): the seeded generator now emits `proc`
  definitions + calls, `namespace eval`, the `dict` ensemble (value ops +
  mutators), `switch`, `catch`, and `try`/`on error`/`finally`, all over the
  surface RT-VM implements (validated by a 1.5 K-iteration `tclvm`-vs-`tclsh9.0`
  campaign at 0 findings — detail in the
  [history archive](rust-rewrite-history.md)). The **WASM-runnability arm** of
  the third backend has also **landed** (2026-06-22): the `wasm-check`
  subcommand compiles each generated program to the eval-fallback WASM module
  and runs it under `wasmtime` (with the proven `tcl_*` host stub), flagging
  codegen panics and modules that fail to instantiate or trap — a 600-program
  campaign is clean. Residual, **gated on RT-WASM**: upgrade that arm from
  *runnability* to a *value* differential against `tclsh`, which needs the
  interpreter-backed host (the eval-fallback `tcl_eval` stub doesn't evaluate
  Tcl); the arm swaps the stub for the real host in place when it lands. *(M)*
- **TOOL-IRULE-TEST** 🟢 *(depends on RT-VM + `tcl-registry`)* — the orchestrator
  runs **live** on `tcl-vm` (`LiveSession`: load iRule, fire events, read
  pool/logs/decisions; 14 integration tests incl. live routing/reject all
  green). Coverage broadens automatically as the VM command surface grows;
  residual: the session's `event dispatch` / `class match` handlers are not yet
  implemented (`tcl-irule-test/src/session.rs`). Note `tcl-irules` is the BIG-IP
  reference-extractor, **not** this. *(XL)*
- **TOOL-F5** 🟢 *(depends on RT-VM, TOOL-IRULE-TEST)* — the `f5` verbs
  (`event-order`/`extract`/`format`/`minify`/`event-info`, `explain-flow`,
  `--simulate`) landed. Residuals in `f5-cli`: the `irule lint`/`context`/
  `trace`/`pgo` sub-verbs are not implemented (arg-parse then error + exit 2);
  the SSH/scp fetch transport is not ported (`--transport rest` works, SSH falls
  back to the Python CLI); `registry-dump --section commands` is not implemented.
- **Formatter — docstring rewriter** 🟢 — the formatter engine, minifier, and
  diagram extractor are byte-parity ported (`tcl-lsp-core::{formatting,minify}`,
  `tcl-cli`); residual: the docstring rewriter is unimplemented — its config
  flags are carried through `formatting::config` but the engine does not consume
  them.

**TOOL-TCLPKG**, **TOOL-REFACTOR**, **TOOL-DEBUGGER**, and **TOOL-CLI** are ✅
complete (their landing logs are in the
[history archive](rust-rewrite-history.md)).

### Stage 5 — PyO3 interfaces & Python retirement (API-PYO3 — last)

#### API-PYO3
Owns `tcl-lsp-py`, the `scripts`→`xtask` migration, and `tests`. **This is the
final track** — every consumer above must port first.
- **landed (2026-06-22)** the designed public PyO3 surface — built as
  `tcl-lsp-py::public`, additive alongside (not replacing) the legacy
  soft-dependency shims. The six narrow facades (`parse_tcl` →
  `ParseResult`, `compile_tcl` → `CompilationUnit`, `analyse_tcl` →
  `AnalysisResult`, `format_tcl` → `str`, `parse_bigip_config` →
  `BigipConfig`, `query_bigip` → `QueryResult`) take `source/options in,
  structured result out` over the layered crates (`tcl-lexer`,
  `tcl-compiler`, `tcl-lsp-core::formatting`, `tcl-bigip`,
  `tcl-bigip-query`) and resolve every span to a `(line, character)`
  position at the boundary. Paired with the typed error hierarchy
  `TclLspError → TclParseError / TclCompileError / TclAnalysisError /
  BigipParseError / BigipQueryError / UnsupportedFeatureError`, each
  raised instance carrying `code` / `message` / `uri` / `range`,
  translated at the facade boundary (the pure crates stay `pyo3`-free).
  Acceptance test: `tests/test_public_pyo3_api.py` (30 cases, runs against
  the built wheel; `importorskip`-guarded). Detail in the
  [history archive](rust-rewrite-history.md). Still **open**: the legacy
  shim set (~39 `#[pyfunction]`s across `tokens` / `expr_lexer` /
  `compiler_checks` / `interprocedural` / `gvn` / `compilation_unit` /
  `signature_scan` / `optimiser` / `analyser` / `registry` / `bigip` + the
  folding / document-symbol feature bindings) stays until its in-tree
  Python importers retire under PYTHON-RETIRE.
- **partial** TEST-MIGRATE — the **porting** half is **complete** (the
  **deletion** half stays gated on PYTHON-RETIRE). All 473 `tests/test_*.py`
  files are now classified (ported / bridge-only / remove-at-end / deferred)
  in the [test audit](rust-rewrite-test-audit.md#test-migrate--full-pytest-suite-classification-all-473-files):
  every behaviour portable to a landed crate is Rust-covered (per-module
  `#[cfg(test)]`, the `tcltest` reference sweeps, or `*_parity.rs`
  differentials), and the un-ported remainder is attributed to a named
  unlanded track (RT-WASM / RT-VM / SRV-ROPE / PKG / PGO — *deferred*) or to
  Python-binding / `ai/` glue (*bridge-only*). The `test_fp_*` battery is the
  analyser acceptance gate (C41 ✅, ~1,000 analyser `#[test]`s). The cleanly
  portable pure-logic gaps that still had zero Rust unit coverage were ported
  in the closing pass — `tcl-bigip::policy_eval` (LTM policy evaluator),
  `tcl-bigip::validator` (BIGIP6003–6009), `tcl-registry::bigip` (object-kind
  resolution), plus VM helper modules (`cmd_string`/`subst`/`exec`). The
  **deletion** of the `test_*.py` is the terminal PYTHON-RETIRE sweep — gated,
  since the pytest suite is the behavioural oracle while the Python layer ships.
- **partial** rewrite `scripts/` build/release as `cargo xtask` (eliminate the
  Python toolchain dependency). Full triage of the ~110 `scripts/` tools —
  which retire with Python vs which survive and how — is in
  [`design/rust/scripts-retirement-triage.md`](design/rust/scripts-retirement-triage.md).
  The check verbs now have a `make xtask-check` target
  (`cargo xtask kcs-index-links` + `refcount-contract` + `diag-tables --check`),
  verified locally; the CI
  step that runs it in the Rust-capable `rust-tests` job (`rust-gate.yml` +
  `ci.yml`; the Python-only `ci-fast` job has no Rust toolchain) is **prepared,
  pending a workflow-scoped push**. Until then `kcs-index-links` stays wired as
  the Python `lint-py` gate. The `rust/xtask` crate + `cargo xtask` alias scaffold
  **landed**, with five scripts ported and parity-checked against the Python
  originals: `refcount-contract` (⇐ `scripts/check/refcount_contract.py`)
  and `kcs-index-links` (⇐ `scripts/check/kcs_index_links.py`) — byte-for-byte
  identical stdout/stderr + exit codes; `version` (⇐ `scripts/print_version.py`),
  whose `git describe` → setuptools-scm scheme is unit-pinned against real
  `setuptools_scm` outputs; `tzdata-bundle` (⇐
  `scripts/build/tzdata_bundle.py`), whose packed `TZBL` artifact is byte-for-byte
  identical for both the verbatim and `--trim`-window paths (the `TZif` v1
  trimmer included); and `audit-option-dialects` (⇐
  `scripts/check/audit_option_dialects.py`), which probes every `OptionSpec`
  dialect gate against the built tclsh 8.4/8.5/8.6/9.0 trees — the
  `tmp/option_dialect_audit.json` artifact **and** the console log are
  byte-for-byte identical to the Python (verified against the one tclsh tree
  built in the dev env; a hand-rolled `json.dumps(indent=2)` emitter and a
  `repr()`-faithful diagnostic formatter keep the bytes exact, and the probe
  table / version order are transcribed 1:1). **Remaining is the bulk: only 6 of
  ~26+ scripts are ported.** Triage decisions (per
  [`scripts-retirement-triage.md`](design/rust/scripts-retirement-triage.md)):
  the `refcount_contract` / `audit_option_dialects` / `tzdata_bundle` Python
  originals were **deleted 2026-07-01** (fully orphaned) — `refcount-contract` is
  now in `make xtask-check`, `audit-option-dialects` is an on-demand `make`
  target, `tzdata_bundle` was orphaned. `kcs_index_links.py` stays wired in
  `lint-py` until the CI workflow step lands (then flip + delete);
  `print_version.py` is **kept** (live `:=` for the wheel filename; retires with
  the Python packaging). The **artifact generators** (`codegen/*`
  editor catalogs/settings + `port_names`, `dev/gen_query_builtins_doc`,
  `build/kcs_db`, the `gen_f5_query_*` / `gen_bigip_model_rust` /
  `registry-audit/*` families) are **port-to-Rust** targets so the shipped
  artifacts keep regenerating off the Rust registry. The `build/zipapps.py` +
  `zipapp-main/*` Python distribution **retires with Python** once native-binary
  distribution reaches parity. The `dev/*` measurement + `*_differential` /
  `diag_parity` tools are Bucket A (retire with Python, not ported). Then flip
  the Makefile/CI invocations to the xtask verbs and retire the ported originals.
  (`bigip_kind_differential.py` stays Python until retirement — it is a
  Python-vs-Rust differential oracle, not a toolchain script.)
- **open** PYTHON-RETIRE — delete `compiler/`, `analyser/`, `server/`, and the
  ported `tooling/` subtrees once their consumers are Rust. **`ai/` is one such
  consumer:** its shell (MCP server + Claude skills) stays Python by design, but
  it imports the in-tree engine (`analyser`/`compiler`/`tooling.refactoring`/
  `server.features`/`dialects`) well beyond the six landed `tcl-lsp-py::public`
  facades, so this task must first re-point `ai/` onto Rust — either by widening
  the PyO3 surface to cover its call set or by thinning `ai/` to the public
  facades / a subprocess LSP. Deleting the engine before that re-point breaks the
  MCP tools.

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
