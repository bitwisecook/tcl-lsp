# Python → Rust rewrite

tcl-lsp is ~360K lines of Python across `core/`, `lsp/`, `vm/`, `debugger/`,
`fuzzing/`, `explorer/`, `ai/`, `scripts/`, and `tests/`. We're incrementally
rewriting all of it in Rust, from the bottom of the stack upward, without
ever breaking the shipping extension. A thin Python interface is kept on
top of the Rust crates for Claude skills, the MCP server, and other
integrations where a small Python surface is genuinely useful.

This is a multi-year project. Every step is a PR-sized change that leaves
`make prep-pr` green and every editor extension working. There is no "big
bang" branch, no pauses for rewrites, and no points at which the Python
build is intentionally broken.

This document explains what we're doing, how we're doing it, and — most
importantly — what a good port looks like. Read it before touching
anything under `rust/`, the PyO3 bindings, or the native-extension bits of
the zipapp builder.

## What we're doing

The eventual end state is:

- All runtime logic lives in a Rust workspace under `rust/`.
- The LSP server is a standalone Rust binary.
- The zipapp build shrinks to a thin Python launcher, or goes away
  entirely once editors can invoke the binary directly.
- A small PyO3 surface survives for Python-first integrations (Claude
  skills, the MCP server, ad-hoc scripts).
- Python tests that exercise observable behaviour stay where they are and
  continue to pass through the PyO3 bridge.

We get there by porting the codebase bottom-up, in dependency order:

1. **Lexer.** `core/parsing/` (lexer, token types, backslash escapes,
   expression sub-lexer). This is the foundation everything else sits on.
2. **Compiler.** `core/compiler/` (IR, CFG, SSA, lowering, codegen, the
   optimiser passes). Each pass can be its own chunk.
3. **LSP server.** `lsp/` (pygls handlers, workspace orchestration,
   feature providers). Once this flips, the zipapp becomes a thin shim.
4. **Remainder.** `vm/`, `core/commands/`, `core/analysis/`,
   `core/formatting/`, `core/minifier/`, `core/irule_test/`, the
   debugger, fuzzing harnesses, the compiler explorer, and CLI tooling.
5. **Python-facing surface** is reduced to the bits that genuinely want
   to stay in Python (AI skills, MCP, scripts).

`editors/zed/` is already a standalone Rust crate targeting WASM and is
unrelated to this rewrite. It's intentionally excluded from the main
Cargo workspace and should be left alone.

## Non-negotiable principles

Two architectural constraints that every chunk is measured against.
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

Consequences for every chunk:

- **Benchmark against `perf_track.py` before and after.** The
  chunk commit message cites the numbers. Regressions (beyond run
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
  we _have_ the measurement and can see when each chunk starts
  showing up.

### 2. Async through and through

The Rust LSP server is async-first from the protocol handler down
to the analysis pipeline. Every layer above the raw lexer is
`async fn`, runs on Tokio, yields cooperatively, and composes with
cancellation. This is how we get responsiveness: a fresh
`textDocument/semanticTokens/full` request while an older one is
still computing should cancel the older one cleanly, not wait for
it to finish.

Consequences for every chunk:

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
  `tokio::task::spawn_blocking` or splits the work across chunks —
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
  the LSP server chunks land (R*). The lexer itself does **not**
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
- **IR / CFG nodes (future chunks) carry `Span`s, not
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
  line index; when the document layer needs to lex a chunk it
  flattens the affected rope range into a `&str` (cheap: most ranges
  are a single contiguous chunk), wraps it in a [`SourceMap`], and
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

The L0–L11 lexer migration is complete: every token kind is
implemented in Rust, the expression sub-lexer is ported, and
`TclLexer.tokenise_all()` dispatches to Rust for the default
config. **L12 (remove the Python lexer fallback)** is blocked on
the items below — each one is a feature the Python lexer supports
that the Rust lexer does not yet, causing the Python fallback to
kick in for specific configurations.

**Remaining Python fallback trigger** (after L13):

- **Ghost character insertion for error recovery.** The Python
  lexer's `virtual_insertions: dict[int, str]` parameter injects
  missing delimiters (`}`, `]`, `{`) at specific offsets so
  downstream passes see a well-formed structure. Only used by the
  recovery path (`core/parsing/recovery.py`) and one call site in
  `lsp/features/semantic_tokens.py`. The Rust `Lexer` has no
  equivalent — porting it belongs to the C* compiler phase when
  the recovery module moves to Rust (the two callers will
  naturally follow). Until then, the Python fallback handles this
  single edge case.

**Non-blocking deferred items (can be fixed independently):**

- **`\<CR>` line tracking.** The Rust `LineIndex` only counts
  `\n` for line boundaries. The Python lexer's incremental
  counter also advances on `\r` inside backslash continuations.
  Bare-CR inputs show a minor position drift. Real-world Tcl
  files use `\n` or `\r\n`; bare `\r` is essentially
  non-existent.
- **UTF-16 column parity.** Both lexers treat `character` as
  byte-offset-within-line. The LSP specification says `character`
  must be UTF-16 code units. The fix is a coordinated change in
  `LineIndex::position_at`. Do before any Rust LSP handler that
  cares (hover, go-to-definition, etc.).
- **`LineIndex::from_rope_slice`.** Adapter that pulls line
  offsets from the rope's B-tree instead of scanning a flattened
  `&str`. Deferred until the first rope-backed consumer lands.
- **Performance measured.** L11 benchmark shows **1.8–2.4×
  speedup** on the full open-to-semantic-tokens LSP pipeline
  (from ~220 ms to ~130 ms) and **~10× speedup** at the
  primitive lexer level through the PyO3 bridge. Further gains
  come from porting the compiler and analyser to Rust (eliminating
  the Python→Rust→Python round-trip for each token).

## How we're doing it

### Two crates per domain

For each migrated domain we create a **pure Rust crate** and, if Python
still needs to call into it, a sibling **PyO3 binding crate** that wraps
it. The first pair are `rust/tcl-lexer/` and `rust/tcl-lsp-rust/`.

- **`rust/tcl-lexer/` — pure Rust**. No `pyo3` dependency. Uses borrowed
  `&str`, `thiserror`, iterators, enums, `Result`. Clippy-pedantic.
  Future Rust consumers (the compiler crate, the LSP server binary, a
  standalone CLI) link against this crate directly.

- **`rust/tcl-lsp-rust/` — PyO3 bindings**. This is the **only** crate
  that knows about Python. It owns every `#[pyclass]` wrapper, every
  `PyErr` translation, and any back-compat shim needed to mimic the
  current Python API surface. The underlying Rust crates stay Python-
  agnostic.

Each new domain gets a new pure crate (e.g. `rust/tcl-compiler/`,
`rust/tcl-lsp-server/`) under the same workspace. The PyO3 binding crate
is shared: it aggregates re-exports from whichever pure crates currently
need a Python surface.

### Python compatibility lives only in the binding layer

If the current Python API demands something awkward — thread-local flags,
class-level mutable state, stringly-typed kwargs, magic singleton modules
— the binding crate implements the awkwardness and hides it from the
pure crate. The pure crate gets clean `&Config` parameters or equivalent,
returns `Result<T, Error>`, and never has to apologise for Python.

This rule is non-negotiable. A pure crate that imports `pyo3` "just for
this one function" is a sign that the binding crate needs another wrapper
type, not that the rule should bend.

### Always shippable, small chunks

Every PR leaves the extension fully working. The CI gate is the same gate
as every other PR in the repo: `make prep-pr` must pass, all editor
packages must still build, and no existing test is allowed to regress.

One chunk = one logical surface. Good examples:

- "Port `backslash_subst`." (~100 LOC Python → Rust + PyO3 wrapper + shim.)
- "Port `TokenType` / `SourcePosition` / `Token`."
- "Port brace-string lexing in the Rust lexer."
- "Port the branch-folding optimiser pass."

Bad examples:

- "Port the whole lexer." (Too big, parity can't be reviewed.)
- "Rewrite `core/parsing/`." (Even worse.)
- "Port the lexer and the compiler IR together because they share a
  data structure." (Split the data structure out first.)

Each chunk that replaces real logic needs a differential test: run the
Python and Rust implementations in parallel on every fixture and assert
identical output. The lexer chunk (L3 onward) introduces
`tests/test_rust_lexer_differential.py` for this. Use the same pattern
for the compiler.

### Soft dependency during rollout

Until a chunk explicitly flips the default, the Python code imports the
Rust wheel via `try: from tcl_lsp_rust import … except ImportError`.
A missing wheel is a performance no-op, not a regression. This lets a
developer work on fresh clones without `make rust-build`, and it lets
releases ship even if a platform wheel fails to build.

Once a chunk flips the default (the Rust implementation becomes the
preferred path and the Python fallback is just there as a safety valve),
the pure-Python fallback is kept for exactly one release cycle and then
removed outright in a follow-up chunk. Do not let fallbacks accumulate.

### Packaging and CI

- The main `pyproject.toml` stays on `hatchling`. The Rust wheel is built
  by `maturin` from its own `rust/tcl-lsp-rust/pyproject.toml` and is a
  **separate** distribution. No mixed hatchling/maturin hybrid.
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
- Use UK spelling (`normalise`, `optimiser`, `analyse`) in identifiers
  and comments, matching the rest of the repo.
- Doc comments describe invariants and non-obvious decisions. Don't
  paraphrase the code. Don't add banner-style dividers.
- Every public item gets a doc comment. `#![deny(missing_docs)]` is on.

### Tests

- Unit tests live next to the code they cover (`#[cfg(test)] mod tests`).
  Integration tests go under `tests/` inside the crate when they need
  multiple modules.
- Every chunk that replaces real Python logic ships with a differential
  test harness: feed the same inputs through both implementations and
  assert identical outputs. Do not flip any default until the
  differential harness is green across the whole corpus.
- Avoid golden-file tests for things that are cheap to compute. Prefer
  assertions that state the actual invariant.
- **Test audit.** Every chunk classifies the pytest tests it touches as
  **ported** (Rust has equivalent coverage), **bridge-only** (Python-
  specific behaviour — kept in pytest, not ported), **remove at end**
  (low-value, flagged inline with an `AUDIT:` comment and tracked for
  deletion when the Python layer is retired), or **deferred** (covered
  by a later chunk). The living audit lives in
  [`rust-rewrite-test-audit.md`](rust-rewrite-test-audit.md); update
  the relevant section in the same commit that lands the chunk. No
  pytest test is deleted during the rewrite — the Python suite is the
  behavioural oracle for every chunk, and only comes out when the
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
      codegen.rs                         Op, Instruction, LiteralTable, FunctionAsm (C4)
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
      hooks.rs                           LoweringHookId, CodegenHookId, ArgTypeHint
      commands/tcl/*.rs                  one file per Tcl command (114 ported)
      commands/irules/*.rs               one file per iRules command (1015 ported)
  tcl-lsp-rust/                          PyO3 binding crate
    Cargo.toml
    pyproject.toml                       maturin build backend
    src/
      lib.rs                             #[pymodule] tcl_lsp_rust
scripts/
  build_zipapp.py                        _RUST_NATIVE_PACKAGES strip rule
.github/workflows/ci.yml                 rust job + release wheel matrix
Makefile                                 rust-build/test/lint/format
tests/test_rust_bindings_smoke.py        end-to-end bridge smoke test
```

## Chunk log

| Chunk | Scope | Status |
|-------|-------|--------|
| L0    | Rust workspace bootstrap: two crates, hello-world `tcl_lsp_rust`, CI, packaging plumbing | landed |
| L1    | `core/parsing/substitution.py::backslash_subst` → `rust/tcl-lexer/src/substitution.rs` with PyO3 bridge and Python fallback | landed |
| L2    | `core/parsing/tokens.py` → `rust/tcl-lexer/src/tokens.rs` (`TokenType`, `SourcePosition`, `Token<'src>`) plus PyO3 wrappers preserving singleton/identity semantics; new `tests/test_tokens.py` contract test | landed |
| L3    | Rust `Lexer` skeleton (EOF/SEP/EOL/COMMENT/plain ESC) + span-first architecture (`Span`, `LineIndex`, `SourceMap`) + differential test harness + committed library choices (`ropey`, `tower-lsp`, `thiserror`/`anyhow`, `clap`, `tracing`) | landed |
| L4    | Variable substitution in the Rust lexer (`$name`, `${name}`, `$arr(idx)`, `$ns::var`, bare `$`) + `SourceMap::token_text` for Python-parity text extraction + try/catch-based differential harness filter | landed |
| L5    | Command substitution in the Rust lexer (`[…]` with bracket nesting, brace/quote aware, embedded `${…}` sub-scan, backslash-pair escapes) + per-kind content stripping in `SourceMap::token_text` | landed |
| L6    | Braced strings in the Rust lexer (`{…}` at word boundaries, balanced nesting, backslash-pair escapes, Python-parity `newword` predicate) + `token_text` `Str` stripping + dynamic harness harvesting ~200 new L6-eligible inputs | landed |
| L7    | Quoted strings in the Rust lexer (`"…"` with `$` / `[` interpolation, `in_quote` propagation, mid-word quote as bare word) + `Token::content_offset` for per-kind prefix stripping | landed |
| L8    | `{*}` expansion prefix + `LexerConfig::expand_syntax` dialect flag | landed |
| L9    | Backslash escapes in bare words, quoted strings, and comments — drains the last deferred character. Warning collection and ghost-character-insertion are deferred to a later pass. | landed |
| L10   | `core/parsing/expr_lexer.py` → Rust. Rust implementation ported and tested; Python dispatch wraps Rust `PyExprToken` into Python-native `ExprToken` (via value→enum-member dict) so downstream `tok.type is ExprTokenType.X` works. | landed |
| L11   | Flip `TclLexer.tokenise_all()` to Rust for the default config (~17× speedup). Non-default configs (virtual insertions, strict quoting, base offsets, `expand_syntax=False`) fall back to the Python lexer. | landed |
| L12   | Expand Rust lexer coverage: base offsets, `expand_syntax=False`, `irules_brace_separator` (ghost SEP injection), warning/strict infrastructure, `lexer_tokenise_with_config` PyO3 entry point. | landed |
| L13   | Wire strict-quoting emission points: `warn_or_error` calls in `parse_var`, `parse_command`, `parse_brace`, `parse_quoted` for all 14 Python strict-mode raise sites. `ValueError` → `TclParseError` conversion in the Python shim. Python fallback now only triggers for virtual insertions. | landed |
| C0    | **Compiler crate bootstrap + IR data structures.** `rust/tcl-compiler/` crate: expression AST (`BinOp`, `UnaryOp`, `ExprNode` enum with `vars()`, `render_expr`, `expr_text`), IR types (`Statement` enum with 15 variants, `Script`, `Procedure`, `Module`, `CommandTokens`, helper types). Every IR node carries a `Span` (not inline `SourcePosition` pairs). 41 unit tests. Wired into `tcl-lsp-rust` binding crate via `compiler_version()`. | landed |
| C1    | **Expression parser.** Pratt parser (`core/parsing/expr_parser.py`) → `rust/tcl-compiler/src/expr_parser.rs`. Converts expression token stream (from L10 Rust lexer) into `ExprNode` AST (from C0). Includes `naming::normalise_var_name`. 53 Rust unit tests + 85-case Python differential test harness (`test_rust_expr_parser_differential.py`). PyO3 bindings: `parse_expr_render`, `parse_expr_vars`, `parse_expr_tag`. Full ExprNode bridging deferred until lowering moves to Rust. | landed |
| C2    | **CFG data structures + graph utilities.** Control-flow graph types: `Terminator` (Goto/Branch/Return), `Block`, `Function`, `CfgModule`, `LoopNode`. Utility methods: `predecessors()`, `reachable_blocks()`, `reverse_postorder()`, `successors()`. 18 Rust unit tests covering diamond/loop/unreachable topologies. CFG builder (with command-registry dependencies) deferred to a later chunk. | landed |
| C3    | **SSA data structures + dominator algorithms.** SSA types: `Phi`, `SsaStatement`, `SsaBlock`, `SsaFunction`. Algorithms: `compute_dominators` (iterative dataflow), `compute_idom` (immediate dominators), `compute_dominance_frontier`, `build_dom_tree`, `compute_phi_vars` (iterated DF algorithm), `defs_of` (variable definition extraction from IR statements). 22 Rust unit tests covering linear/diamond/loop topologies. Full SSA rename pass deferred until `_uses` scanner is ported. | landed |
| C4    | **Codegen types.** `Op` enum (150+ Tcl 9.0.2 bytecode opcodes), `Instruction`, `Operand`, `LiteralTable` (dedup intern pool), `LocalVarTable` (slot interning), `FunctionAsm`, `ModuleAsm`. Operator mapping (`BinOp`/`UnaryOp` → `Op`), index parsing (`parse_tcl_index`), `string is` class tables. 15 Rust unit tests. | landed |
| C5    | **Type lattice + analysis result types.** `TclType` (10 intrep variants), `TypeLattice` (Unknown/Known/Shimmered/Overdefined), `type_join` with numeric promotions. SCCP lattice: `LatticeValue` (Unknown/Const/ConstSet/Overdefined), `ConstValue`. Diagnostic types: `DeadStore`, `ConstantBranch`, `ReadBeforeSet`, `UnusedVariable`. Composite: `FunctionAnalysis`, `ModuleAnalysis`. 21 Rust unit tests. | landed |
| R0    | **Command registry crate.** `rust/tcl-registry/` — single source of truth for all command metadata. `CommandSpec` with `Traits` bitflags (u64, 38 bits replacing ~35 booleans), `SubCommand`, `ArgRole` (12 variants), `Arity`, `DialectSet` bitflags, `HoverSnippet`, `SideEffect`, hook IDs, `CommandRegistry` facade with trait-indexed queries and `arg_indices_for_role`. `TclType` moved here from `tcl-compiler` (canonical home). 11 command specs ported (for, if, while, foreach, set, incr, puts, proc, eval, expr, dict with 19 subcommands). 15 registry tests. Compiler re-exports `TclType`. | landed |
| R2    | **iRules dialect.** 1,015 iRules commands auto-generated from Python specs. `CommandRegistry::load_irules()` for lazy dialect loading. `DialectSet::IRULES` on all specs. Namespace separator `::` mapped to `__` in module names with `#![allow(non_snake_case)]`. Collision handling for duplicate Rust names. 3 new registry tests (load, dialect filter, idempotent). | landed |
| C6    | **Variable-reference scanner + SSA rename pass.** `VarReferenceScanner` (`core/compiler/var_refs.py`) → `rust/tcl-compiler/src/var_refs.rs`: LRU-cached scanner extracting variable names from Tcl words/scripts via the Rust lexer, with optional `ArgRole::VarRead` resolution. `uses_of()` function: determines which variables each IR statement reads, handling expression vars, word substitutions, body exclusion, and `dict with/update` special cases. `build_ssa()`: completes the SSA rename pass — iterative dominator-tree walk assigning version numbers to definitions and uses, filling phi-node incoming edges. `ir_helpers.rs`: recursive `defs_from_ir_script` (walks structured IR for catch/try merge-point invalidation) and `defs_from_expr` (extracts VarWrite defs from expression command substitutions via registry queries). 26 new Rust unit tests (scanner caching, SSA rename on linear/diamond/loop CFGs, uses_of per statement type, IR def extraction). | landed |
| C7    | **CFG builder.** `core/compiler/cfg.py::_CFGBuilder` → `rust/tcl-compiler/src/cfg_builder/` (directory module with `mod.rs` + `cfg_lower.rs`). `CfgBuilder` struct with block allocation, terminator management, and `lower_script` dispatch. Per-construct flattening: `lower_if` (cascaded branches), `lower_for` (init→header→body→step loop with `LoopNode` metadata), `lower_while` (header→body loop), `lower_foreach` (synthetic iteration-variable defs + opaque branch), `lower_switch` (exact-mode chain of `StrEq` branches; glob/regexp → barrier), `lower_try` (body→handlers→finally→end). Opaque handlers for `catch` (always-opaque with recursive `defs_from_ir_script`), `dict for/map` (qualified-command barrier), deferred `try` (top-level opaque call). Public API: `build_cfg(module, defer_top_level)` and `build_cfg_function(name, script, inline_loops)`. 14 new Rust unit tests. | landed |
| C*    | **Compiler migration (continued).** `core/compiler/` (lowering, codegen emitter, optimiser passes) → `rust/tcl-compiler/`. Each pass can be its own chunk. | planned |
| S*    | **LSP server migration.** `lsp/` (pygls handlers, workspace orchestration, feature providers) → `rust/tcl-lsp-server/` on `tower-lsp`. This is when `ropey` enters the picture as the document store, the whole pipeline becomes async, and the server ships as a standalone Rust binary. | planned |
| R*    | **Remainder.** `vm/` (bytecode VM, interpreter, REPL), `core/commands/` (command registry), `core/analysis/` (analyser passes), `core/formatting/` (formatter engine), `core/minifier/`, `core/irule_test/`, `debugger/`, `fuzzing/`, `explorer/`, CLI tooling (`scripts/`). A Python interface is kept on top for Claude skills, the MCP server, and other integrations. | planned |

Keep this table current. Mark a row as `landed` in the same commit that
lands the chunk.
