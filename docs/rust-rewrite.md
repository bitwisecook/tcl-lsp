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
| C8    | **Lowering foundations: naming, alias, hooks.** `normalise_qualified_name` added to `naming.rs` (Tcl namespace normalisation). `alias.rs`: `detect_interp_alias`, `resolve_alias`, `expr_alias_names` — alias detection and namespace-aware resolution. `lowering_hooks.rs`: `LoweringCommand` context type, `ArgTokenKind` enum, `try_lower_hook` dispatch for 9 commands: `expr` (→ `ExprEval`), `return` (→ `Return`/`Barrier`), `set` (→ `AssignConst`/`AssignExpr`/`AssignValue`), `incr` (→ `Incr`), `append`/`lappend`/`unset`/`global`/`variable`/`upvar` (→ `Call` with defs). `extract_single_expr_arg` and `parse_decimal_int` helpers. 26 new Rust unit tests, 245 total. | landed |
| C9    | **Command segmenter.** `core/parsing/command_segmenter.py` → `rust/tcl-compiler/src/segmenter.rs`. `SegmentedCommand` type with per-word text, tokens, single-token flags, expansion markers. `segment_commands` splits flat token stream into per-command structures at EOL boundaries, reconstructing word text (VAR→`${name}`, CMD→`[text]`). `segment_commands_with_offset` for body scripts with base-offset lexing. 12 Rust unit tests. | landed |
| C10   | **Lowering engine.** `core/compiler/lowering.py` → `rust/tcl-compiler/src/lowering/` (directory module with `mod.rs` + `structured.rs`). `Lowerer` struct with `lower_to_ir` public API, `lower_script`/`lower_body`/`lower_segmented` pipeline, `lower_command` dispatch (hooks → structured → default). Structured lowerings: `lower_if` (cascaded if/elseif/else clauses), `lower_for` (init/cond/next/body), `lower_while`, `lower_foreach`/`lmap`, `lower_catch`, `lower_try` (on/trap handlers + finally), `lower_switch` (options parsing, pattern/body pairs, fallthrough), `lower_dict` (for/map/set/unset/append/lappend/incr/update/with). `lower_proc` (procedure registration + body lowering), `lower_when` (iRules event handlers with priority numbering), `lower_namespace_eval`. Default lowering with registry-based `ArgRole` queries and alias resolution. 26 new Rust unit tests, 283 total. | landed |
| C11   | **Codegen emitter foundation.** `codegen.rs` refactored into `codegen/` directory module. `CodegenCtx` emission context (literal/LVT tables, instruction stream, label management). `helpers.rs`: `split_list_simple`, `tcl_string_hash`, `tcl_hash_table_order`, `tcl_list_element`, `parse_subst_template`, `regexp_to_glob`, `fold_cmd_args`/`fold_list_cmd`/`fold_dict_create_cmd`, `try_format_fold`. `values.rs`: `push_lit`/`push_lit_no_dedup`, `begin_command`/`end_command`, `split_array_ref`/`is_array_ref`/`is_qualified`, `load_var`/`store_var` (LVT vs stack-based), `emit_incr` (immediate/large/variable amounts), `parse_simple_var_ref`/`parse_braced_scalar_ref`. `expressions.rs`: `emit_expr` (literal validation, string delimiters + backslash escapes, variable loading, short-circuit `&&`/`||`, all binary/unary operator dispatch, ternary branch-around, math function calls as `invokeStk`, `ExprRaw`/`ExprCommand` fallbacks). 98 new Rust unit tests, 381 total. | landed |
| C12   | **Statement emission.** `statements.rs`: `emit_stmt_with_start_cmd` (startCommand wrapping with deferred end labels, generic-invoke tagging), `emit_stmt` (dispatch for all IR statement types: `AssignConst`/`AssignValue`/`AssignExpr`/`Incr`/`ExprEval`/`Call`/`Barrier`/`Return`), `emit_call` (generic command invocation with break/continue loop jumps), `emit_expanded_call` (`{*}` expansion via `expandStart`/`expandStkTop`/`invokeExpanded`), `emit_value_interpolated` (simplified value emission with `${var}` resolution, list/dict constant folding), `push_var_ref` (array key handling for store ops). `CodegenCtx` gains `break_target`/`continue_target` fields for loop compilation. 13 new Rust unit tests. | landed |
| C13   | **Peephole optimisations.** `peephole.rs`: `remove_trailing_pop` (collapse final `pop; done`), `fold_const_push_pop_nops` (constant-folded `push; pop` → 3 nops matching tclsh), `dedup_push_literals` (re-dedup after nop folding), `fold_tail_return_to_done` (proc tail `returnImm 0 0` → `done`), `strip_unused_start_cmd` (remove all startCommand when no generic invoke exists), `fixup_top_level_start_cmd` (remove generic-tagged startCommand in top-level), `strip_nodedup_tags` (clean internal comment markers). 9 new Rust unit tests. | landed |
| C14   | **Layout and formatting.** `Op::size()` — instruction byte sizes for all 150+ opcodes (1/2/3/5/6/9 bytes). `layout.rs`: `optimise_jumps` (iterative 4-byte→1-byte jump shrinking), `resolve_layout` (assign byte offsets, resolve labels). `format.rs`: `esc` (Tcl-compatible literal escaping), `format_function_asm` (full disassembly rendering with literals, LVT, instruction stream, labels, jump targets, jumpTable entries), `format_module_asm`. 14 new Rust unit tests, 417 total. | landed |
| C15   | **Command substitution inline emission.** `core/compiler/codegen/_cmd_subst.py` → `rust/tcl-compiler/src/codegen/cmd_subst.rs`. Static helpers: `unroll_nested_set`, `is_pure_cmd_subst`, `has_command_separator`, `parse_cmd_parts` (quoted/braced/nested argument parsing). `CodegenCtx` methods: `emit_cmd_subst_arg`, `emit_generic_cmd_subst`, `emit_value` (full interpolation with `parse_subst_template` + `STR_CONCAT1`), `emit_inline_cmd_subst` dispatch for `expr`, `incr`, `info exists`, `string` (index/range/equal/compare/length/is/replace), `lindex`, `lrange`, `lreplace`, `linsert`, `regexp`, `list`, `array exists`/`names`/`size`, `dict get`, `catch`. Multi-command scripts fall back to runtime `EVAL_STK`. 22 new Rust unit tests. | landed |
| C16   | **Catch/try control flow emission.** `core/compiler/codegen/_control_flow.py` → `rust/tcl-compiler/src/codegen/control_flow.rs`. `emit_catch_inline` with `beginCatch4`/`endCatch` nesting depth tracking; `emit_catch_body` dispatch for `return`/`error`/`break`/`continue`/`expr`/`try`; `emit_catch_return`/`emit_catch_error` for return-code dispatch; `emit_try_on_error_inline` with nested catch ranges and `-during` option merging; `emit_try_handler_body`; `emit_try_finally_inline` with exception-path merging; `emit_try_body_stmt`/`emit_try_finally_stmt` (no trailing pop); `detect_const_expr_error` for compile-time divide-by-zero. New `CodegenCtx` fields: `catch_depth`, `seen_generic_invoke`, `used_generic_invoke`, `used_inline_cmd_subst`, `expr_func_depth`, `pending_cond_end_label`, `proc_exit_label`, `pending_join_labels`, `current_source_line`. 8 new Rust unit tests. | landed |
| C17   | **Emitter main loop + public API.** `core/compiler/codegen/_emitter.py`, `_bytecoded.py` → `rust/tcl-compiler/src/codegen/emitter/` (directory module). Split by responsibility rather than reproducing Python's mixin composition: `ordering.rs` (`fold_const_branch`, `linearise` RPO + dead-branch elimination, `collect_loop_body`, `reorder_bottom_tested`, `build_loop_context`); `terminator.rs` (`emit_term` for CFG Goto/Branch/Return, `emit_proc_return` with startCommand wrapping + dead-code jumps, `try_emit_jump_table` with Tcl-hash-ordered switch dispatch); `proc_defs.rs` (`is_static_proc`, `emit_one_proc_def`, `emit_pending_proc_defs`, `flush_proc_defs`); `loop_blocks.rs` (`detect_foreach` + `ForeachInfo`, `detect_complex_foreach` + `ComplexForeach` for bodies with Branch terminators); `try_blocks.rs` (`detect_try_finally` CFG pattern detection feeding `emit_try_finally_inline` from C16); `generate.rs` (top-level dispatcher + `GenerateState` struct with `skip_blocks`, `while_end_labels`, `for_init_end_labels`; foreach opcode compilation with complex-body variant emitting step/end at the foreach_end block plus continue/break relabelling and back-edge suppression; while-loop + for-init startCommand wrapping with deferred end labels at the loop-end pop; join-block pops; arm-result preservation); `bytecoded.rs` (registry-backed codegen hook dispatch stub — no hooks wired yet). Public API: `codegen_function`, `codegen_module` re-exported from `codegen::`. Handles: straight-line code, if/else diamonds, switch jump-table dispatch, foreach/while/for-init with the core startCommand wrapping cases, complex-foreach bodies (if/break/continue as first statement), proc returns with dead-code skip jumps, try/finally CFG patterns, constant branch folding. 13 integration tests under `tests/codegen_integration.rs` + per-module unit tests (~488 total). Known layout-precision gaps deferred to C18–C21 (see below). | landed |
| C18   | **startCommand placement precision.** `GenerateState` gains three per-run maps: `foreach_end_labels`, `for_body_end_labels`, and `pending_join_labels`. **Case 1** — foreach startCommand wrapping: when the foreach is not the first command (`cmd_index > 0`), emit a `startCommand` with `fresh_label("cmd_end")` before loading the list args and defer the end label to the `foreach_end` block's trailing pop. **Case 2** — for-body startCommand: a `for_body_*` block whose terminator is a Branch (first statement is an `if`) gets a startCommand whose end label is threaded through the true-branch successor to the `if_end_*` join pop. **Case 3** — complex-foreach if-condition wrapping: body blocks inside a complex foreach with a Branch terminator also get per-body startCommand, with the end label deferred to the `if_end_*` or `if_next_*` join. **Case 4** — constant-folded `if {1}` startCommand in non-proc scripts: in `generate.rs`, before calling `emit_term`, detect branch terminators whose condition folds to `Some(true)` and whose true-branch goto points at `if_end_*`; emit a `startCommand` + defer the end label into `pending_join_labels` keyed by the join block so it is placed before the join pop. This preserves tclsh's "command boundary survives even when the branch is dead" behaviour — `set x 1; if {1} { puts hello }` now byte-matches Python output (9 instructions, 22 bytes, with `startCommand -6 1` at pc 6). **Case 5** — `<cond>` synthetic statement handling: `ir_helpers::expr_has_command` inspects an `ExprNode` for `ExprNode::Command` substitutions; `cfg_lower.rs::lower_if` appends a synthetic `Statement::Call { command: "<cond>", … }` to the dispatch block when the branch condition contains a command substitution; `generate.rs` detects this statement and sets `ctx.pending_cond_end_label` before calling `emit_stmt_with_start_cmd` with the deferred label; `expressions.rs::emit_expr` places that pending label right after emitting an `ExprNode::Command` so the startCommand covers the whole command-substitution body. New fixtures: `foreach-after-set.tcl` (case 1), `for-body-break.tcl` (case 2 — already existed), `if-const-true.tcl` (case 4). The `if-catch-cond.tcl` divergent fixture tracks the pre-existing ExprCommand inline-emission gap (`emit_expr` falls back to `exprStk` with the raw command text as a literal instead of inlining the catch bytecode). Differential harness now reports 20/20 matching (8 exact, 12 semantic, 0 divergent in matching corpus) plus 1 divergent fixture tracking the ExprCommand inline gap. | landed |
| C19   | **Value emission extras.** `CodegenCtx::try_list_expand_concat` (~30 LOC in `codegen/cmd_subst.rs`) compiles `[list {*}$a {*}$b]` as `loadStk a; loadStk b; listConcat`, only matching the exact two-argument form. `CodegenCtx::try_inline_list_with_break_continue` (~60 LOC) compiles `[list arg ... [break] ...]` / `[list arg ... [continue] ...]` as inline jumps with stack cleanup when a loop target is in scope, degrading to literal emission when no target is active (Python-parity). The existing `helpers::try_format_fold` is wired into both `emit_value` (`codegen/cmd_subst.rs`) and `emit_value_interpolated` (`codegen/statements.rs`) so `[format "..." lit ...]` folds to the computed string via `push_lit_no_dedup`. Both paths are also added to `emit_value_interpolated` because Rust's `AssignValue` lowering goes through the interpolated path, not the full `emit_value`. 7 new unit tests (2-var match, single/three-var reject, non-expanded reject, break w/o target, break with target, three-arg list with break). Promoted `list-expand-concat.tcl` and `format-literal.tcl` fixtures from `divergent/` to `matching/`; the differential harness now reports 18/18 corpus entries matching (8 exact, 10 semantic) with an empty divergent corpus. 507 total tests. | landed |
| C20   | **Differential codegen test harness.** `rust/tcl-compiler/tests/differential_codegen.rs` feeds a corpus of Tcl scripts through both the Python emitter (subprocess invocation of `core.compiler.codegen.codegen_module` via `python3 -c`, with `current_dir` set to the repo root) and the Rust pipeline (`lower_to_ir` → `build_cfg` → `codegen_module` → `format_module_asm`). Three-tier equivalence classifier: **Exact** (byte-for-byte match including label names), **Semantic** (matches after stripping `  # label:` comment lines — internal label names differ but bytecode, literals, LVT, and resolved jump PCs agree), and **Divergent** (real codegen gap). Corpus split across two directories: `tests/fixtures/codegen/matching/` (16 fixtures asserted to at least semantic-match) and `tests/fixtures/codegen/divergent/` (2 fixtures tracking known C19 gaps — `list-expand-concat`, `format-literal`). A dedicated progress-reporter test lists divergent fixtures that now match so they can be promoted. Python-oracle probe is cached via `OnceLock`; if `core.compiler.codegen` can't be imported the harness logs a skip and returns `Ok` so the test stays green on sandboxes without the Python build. 9 new integration tests (2 corpus runners + 6 classifier unit tests + 1 pipeline smoke), 500 total. Drives test-coverage for C17 + C18 + C19 and every later codegen-adjacent chunk. | landed |
| C21   | **Registry codegen hooks.** `tcl-registry::CommandSpec` already carries `codegen_hook: Option<CodegenHookId>` from R0 — that infrastructure is left in place for a future ID-indexed dispatch migration, but this chunk follows the same name-based pattern as `lowering_hooks::try_lower_hook`: `codegen/emitter/bytecoded.rs::try_bytecoded(ctx, cmd, args, used_generic_invoke) -> bool` match-dispatches on `cmd` to per-command handlers. Wired into `statements.rs::emit_call` before the generic `invokeStk` fallback. Initial hooks ported from `core/compiler/codegen/bytecoded/`: `lassign list var...` (load list; per-var: push name + `OVER 1` + `LIST_INDEX_IMM i` + `STORE_STK` + `POP`; final `LIST_RANGE_IMM N end` + `POP`); `llength list` (load + `LIST_LENGTH` + `POP`); `array names` / `array size` in non-proc context (invoke fully-qualified `::tcl::array::<sub>` rather than the generic `array` dispatcher). The differential-harness Python driver now calls `core.compiler.codegen.bytecoded.register_all()` before lowering so the Python oracle produces the specialised bytecode too. 7 new unit tests (lassign arity rejection + expected opcode sequence, llength single arg + arity reject, array names FQ emission, array in proc context reject, unknown command fallback). 3 new fixtures (`lassign.tcl`, `llength.tcl`, `array-names.tcl`) in `matching/`. Differential harness now reports 23/23 matching (11 exact, 12 semantic). Additional hooks (`lset`, `linsert`, `lrange`, `dict incr` multi-key, more of the `_dict.py` / `_string.py` family) are left as incremental follow-ups that add one hook + one fixture per commit. | landed |
| C24a  | **Def-use chains over SSA.** `core/compiler/def_use.py` → `rust/tcl-compiler/src/def_use.rs`. `DefKind` (Statement/Phi/Parameter), `UseKind` (Operand/PhiIncoming/Terminator), `DefSite`, `UseSite`, `DefUseChain`, `DefUseResult` keyed by `(variable, version)`. Two-pass `build_def_use_chains(ssa, Option<&CfgFunction>)`: pass 1 collects definitions from phis and statements; pass 2 collects uses from statement operand maps, phi incoming edges, and — when a CFG is supplied — branch-condition reads (via `ExprNode::vars()` and `exit_versions`). Unknown uses synthesise a `Parameter` chain (version 0) or a `Statement` placeholder rooted at the entry block, matching the Python behaviour. Query helpers: `chain_for`, `uses_of`, `is_dead`, `reaching_defs`, `dead_chains`, `total_defs`, `total_uses`. 6 unit tests (empty function, dead def, def with use, phi def with two incoming uses, reaching_defs over multiple versions, synthesised parameter chain). Memory SSA (the second half of the original C24) is deferred. | landed |
| C22   | **Tcl expression evaluator (core subset).** `core/compiler/tcl_expr_eval.py` → `rust/tcl-compiler/src/tcl_expr_eval.rs`. Constant-folding evaluator over `ExprNode`: `TclValue` (Int/Float), `EnvValue` (Int/Float/Str for variable bindings), `Env` (`HashMap<String, EnvValue>`) for the caller-supplied environment. Dispatch covers literals (decimal, hex `0x…`, octal `0o…`, binary `0b…`, all Tcl boolean spellings), variable resolution, all arithmetic with correct int/float promotion, division and modulo with floor-division semantics matching C Tcl 9.0.2 (`r.signum() != y.signum()` correction, sign-follows-divisor for `%`), integer exponentiation with the special `|base| ≤ 1` and negative-exponent rules + overflow-checked square-and-multiply, shifts and bitwise (int-only), numeric and string comparison returning Int(0)/Int(1), short-circuit `&&` / `||` / `and` / `or`, ternary, unary `-` / `+` / `~` / `!` / `not`, and `format_tcl_value` for rendering results back to source text (including the `.0` suffix for integer-valued floats). Math-function dispatch landed in the follow-up commit: `abs`, `int` / `entier` / `wide`, `double`, `bool`, `round` (ties away from zero, matching C Tcl), `ceil` / `floor` (float-returning), variadic `min` / `max` (preserving int width unless a float operand forces promotion), `isqrt` (int-only), `isinf` / `isnan` / `isfinite`, unary-float (`sqrt`, `exp`, `log`, `log10`, `sin`/`cos`/`tan`/`asin`/`acos`/`atan`/`sinh`/`cosh`/`tanh`), and binary-float (`atan2`, `hypot`, `fmod`, `pow`) via concise `unary_float` / `binary_float` closures with domain-error → `None` handling. `rand` / `srand` always return `None` so callers cannot constant-fold a non-deterministic result. Unknown function names also return `None`. Deferred: iRules-specific word operators (`contains`, `starts_with`, `ends_with`, `str_equals`, `matches_glob`, `matches_regex`, `in`, `ni`) — all return `None` so callers fall through to the runtime path. 22 + 16 = 38 unit tests covering literal parsing, arithmetic/float promotion, division by zero, floor-division corner cases, exponentiation rules, comparisons, short-circuit, ternary, unary, bitwise, shifts, variable resolution, command-substitution opacity, and overflow. 529 total tests. | landed |
| C23   | **Side-effects + execution intent (six strips).** Ports `core/compiler/side_effects.py` (`rust/tcl-compiler/src/side_effects.rs`) and `core/compiler/execution_intent.py` (`rust/tcl-compiler/src/execution_intent.rs`) in six commits, each self-contained so the tree is green at every step. **C23a** — vocabulary enums: `StorageType` (5 variants), `StorageScope` (14 variants covering Tcl, iRules, and external-I/O scopes), `ConnectionSide` (5 variants), `SideEffectTarget` (30+ variants spanning variable mutation, iRules data stores, HTTP/TCP/SSL/UDP state, response lifecycle, load balancing, external I/O, DNS, classification/DoS, flow management, application protocols, statistics, F5 security modules, BIG-IP configuration, and interpreter state), and `EffectRegion` bitflags (HTTP_STATE / RESPONSE_LIFECYCLE / GLOBAL_STATE / UNKNOWN_STATE) plus `target_to_region` mapper. **C23b** — `SideEffect` (target, reads, writes, storage_type, scope, connection_side, namespace, dialect, key, subtable) and `CommandSideEffects` (effects Vec, pure/deterministic/dynamic_barrier flags, dialect) with `pure()`/`unknown_write()`/`dynamic_barrier()` constructors and `reads_any`/`writes_any`/`affects_target`/`reads_target`/`writes_target`/`effects_in_scope`/`effects_on_side`/`to_effect_regions` query helpers. **C23c** — `scope_from_varname` routes `static::`/`::var`/`::ns::var`/bare names to the correct scope + namespace; `storage_type_for_command` lifts the registry's 3-variant `StorageType` (Dict/List/Array) into the richer 5-variant compiler-side enum, falling back to Scalar. **C23d** — `classify_side_effects(registry, cmd, args, dialect, callee_summary)` dispatches on callee_summary bridge (EffectRegion → structured effects), registry miss, EVALUATES_CODE/CREATES_BARRIER (dynamic_barrier), PURE/PURE_EVALUATION, assigns_variable_at (Variable effect with inferred scope, storage type, connection side for iRules dialect, read-before-write heuristic for incr/append/lappend), DEFINES_PROCEDURE (ProcDefinition), DESTROYS_VARIABLE (Variable write with scope from first arg), and conservative unknown fallback. `CalleeSummary` struct provides the interprocedural shim. **C23e** — execution-intent enums (`InvocationShape`, `SubstitutionCategory`, `SideEffectClass`, `EscapeClass`) and structs (`CommandSubstitutionIntent`, `FunctionExecutionIntent` keyed by `(block_name, statement_index)`). **C23f** — builders: `categorise_arg` classifies literal/scalar-var/array-var/nested-command/mixed; `shimmer_pressure` weights 1 per var ref, 2 per nested/mixed; `classify_side_effect` bridges to C23d and collapses onto the two-value Pure/MaySideEffect lattice; `classify_escape` consults CREATES_BARRIER trait and argument categories; `parse_command_substitution` uses the Rust lexer to tokenise the inner body of `[…]`, skipping SEP/COMMENT, terminating on EOL/EOF, and rejecting multi-command bodies; `build_function_execution_intent` walks every block's `Statement::AssignValue` and records successful parses. Subcommand-form resolution, structured registry `side_effect_hints` iteration, and protocol-namespace classification (`HTTP::`, `SSL::`, `TCP::`, …) are deferred — they require more registry API surface than the Rust tcl-registry currently exposes. 44 new unit tests total (6+7+7+13+4+15) across the two files. 610 total tests. | landed |
| C24b  | **Memory-SSA (three strips).** Ports `core/compiler/memory_ssa.py` into `rust/tcl-compiler/src/memory_ssa.rs`. **C24b1** — location vocabulary: `MemoryLocationKind` (Local/Upvar/Global/NamespaceVar/ArrayElement/InstanceVar/Unknown), `MemoryLocation` with `new`/`with_qualifier`/`display`, and `AliasSet` (BTreeSet of locations + reason) with `may_alias`/`contains_name`/`names`. **C24b2** — operation types and function container plus alias-detection helpers: `MemoryOpKind` (Def/Use/Phi/Clobber), `MemoryOp` with `new_def`/`new_use`/`new_phi`/`new_clobber` constructors, `MemorySSAFunction` with `aliased_names`/`aliases_for`/`may_alias`, and `detect_upvar` (handles `upvar var local`, `upvar #N var local`, `upvar INT var local`, and `namespace upvar NS …` pair grammars), `detect_global`, `detect_namespace_variable`, `is_clobber`. The Python port defers upvar grammar to `core.analysis.var_scoping`; this Rust port inlines a simplified grammar so the strip lands without the var_scoping port. **C24b3** — `compute_aliases(ssa)` uses union-find with per-root reason aggregation (iterative path compression, `BTreeSet` for reason merges) and `build_memory_ssa(ssa)` walks the dominator tree producing versioned MemoryOps: a memory phi per aliased scalar phi at merge points, a Clobber op on is_clobber statements, a Def per aliased variable defined, and a Use per aliased variable read (with reaching_version = current counter). 21 unit tests covering display parity, detection pair grammars, alias set merging, clobber emission, and full build over a three-statement sequence. 631 tests total. | landed |
| C25   | **SCCP + dataflow graph (four strips).** Ports the SCCP driver (`core/compiler/core_analyses.py::_sccp`) and the dataflow-graph extractors (`core/compiler/dataflow_graph.py`). **C25a** — lattice-join helpers: `compute_predecessors` and `cfg_order` (RPO with unreachable-block append), `join(old, new)` widening to `Overdefined` when either side is Overdefined or when the union exceeds `MAX_CONSTSET_SIZE`, `set_value` generic over `BuildHasher`, plus `cv_eq`/`cv_key` deterministic `ConstValue` comparison. **C25b** — `sccp(cfg, ssa, param_constants)` fixed-point driver returning `SccpResult { values, executable_blocks, executable_edges, constant_branches }`. Processes phis for non-entry blocks using executable incoming predecessors; `Statement::Barrier` widens every tracked value to Overdefined; other statements dispatch through `evaluate_def` (covers AssignConst and AssignExpr via the C22 evaluator with an `Env` built from the current lattice — other statement kinds yield Overdefined). `Terminator::Branch` consults `evaluate_branch` to decide which edges become executable. Post-fixed-point sweep emits `ConstantBranch` diagnostics for reachable branches that fold. **C25c** — dataflow-graph data types: `EdgeKind` (Direct/Phi/Alias/Clobber with `as_str` for stable ser/de), `DataFlowNode`, `DataFlowEdge` (with `direct`/`phi` constructors), `AliasInfo`, `FunctionDataFlowGraph`, and `DataFlowGraph` module container with `total_defs`/`total_uses`/`total_aliases` summaries. **C25d** — `extract_function_dataflow(name, ssa, du, sccp, mem)` builds a `FunctionDataFlowGraph` from the per-function analysis outputs: one node per def-use chain key (sorted for determinism) with lattice rendered as `CONST(value)` / `CONSTSET(…)` / `OVERDEFINED` / `UNKNOWN`, one edge per use site (Direct for Operand/Terminator, Phi for PhiIncoming), AliasInfo records from memory-SSA with stable (kind, name, qualifier) ordering, and pre-computed totals. Deferred: full `extract_dataflow_graph(module)` entry point (needs a CompilationUnit aggregator); `IRAssignValue` + `IRIncr` + foreach-constset evaluation in `evaluate_def`. 31 new unit tests (10+7+5+4 across the four strips). 657 tests total. | landed |
| C26   | **GVN (four strips).** Ports `core/compiler/gvn.py` into `rust/tcl-compiler/src/gvn.rs` as four self-contained strips. **C26a** — value-table types: `ExprKey` alias, `RedundantComputation` diagnostic, `ValueEntry`, `ExprOccurrence`, and `ScopedValueTable` — stack of `ExprKey → ValueEntry` maps with `push_scope`/`pop_scope`/`lookup` (innermost-first)/`insert`/`kill_all`/`scope_depth`/`total_entries`. **C26b** — canonicalisation helpers: `canonicalise_word` uses a left-to-right scanner (avoiding the Python `.replace` chain's `${x}` re-matching quirk — scanner exactly-once replacement with identifier-grammar awareness for both `$name` and `${name}` forms), `build_call_key` assembles `["call", cmd, args…]` keys, `format_expression_text` renders human-readable text, and the three diagnostic-message builders (`full_redundancy_message`, `partial_redundancy_message`, `loop_invariant_message`) return the Python message strings verbatim. **C26c** — statement-level helpers: `is_pure_command` bridges to C23d's `classify_side_effects` and checks `.pure`; `is_worth_reporting` consults the `CSE_CANDIDATE` trait; `statement_writes_state` returns true for `Statement::Barrier` and for `Statement::Call` whose `to_effect_regions` writes set is non-`NONE`; `statement_occurrences` emits one `ExprOccurrence` per pure CSE-candidate `Statement::Call`. **C26d** — `find_redundancies(registry, cfg, ssa, dialect)` walks the dominator tree iteratively (WalkStep Enter/Leave work stack so deep dominator trees don't blow the Rust call stack), pushes/pops `ScopedValueTable` scopes around each block, invokes `statement_writes_state` to call `kill_all` on mutators, and emits `RedundantComputation { code: "O105", … }` diagnostics whenever a lookup hits. Deferred: embedded command-substitution scanning inside argument text (needs TclLexer integration with span tracking); partial-redundancy (O106); loop-invariant (O107); interprocedural pure-proc detection. 29 new unit tests across the four strips (8+8+6+5). 684 tests total, up from 610 at the start of this chunk sequence. | landed |
| C21e  | **Registry codegen hook extensions.** Four follow-up strips on the C21 dispatcher. **C21e (lrange/linsert/lset)**: `lrange list first last` with constant `parse_tcl_index`-able indices emits `LIST_RANGE_IMM`; non-constant indices fall back. `linsert list index element…` emits `LREPLACE4 N 2`. `lset varname ?index…? newvalue` uses `loadScalar1 SLOT; LSET_LIST|LSET_FLAT; storeScalar1 SLOT` in proc context and the stack-based `OVER depth; LOAD_STK; LSET_LIST; STORE_STK` form otherwise. **C21e4 (dict subcommands)** in proc context with a simple (non-qualified) variable name: `dict set var k1 ?k2…? value` → `DICT_SET N slot`, `dict unset var k1 ?k2…?` → `DICT_UNSET N slot`, `dict incr var key ?amount?` → `DICT_INCR_IMM amt slot` (rejects non-integer literal amounts), `dict append var key value` → `DICT_APPEND slot`, `dict lappend var key value` → `DICT_LAPPEND slot`. 6 new fixtures in `matching/`; differential harness now reports 29/29 matching (16 exact, 13 semantic). 16 new unit tests. | landed |
| C22i  | **iRules string operators + list membership + regex.** Completes the full `BinOp::Contains` / `StartsWith` / `EndsWith` / `StrEquals` / `MatchesGlob` / `MatchesRegex` / `In` / `Ni` surface in `tcl_expr_eval.rs` under the `f5-irules` dialect. Module-private helpers: `strip_string_delimiters`, `eval_as_string`, `split_tcl_list`, `glob_match` (inline `*`/`?`/`[abc]` matcher — no external dep), `regex_matches` (via the `regex` crate with a `contains_are_only_feature` guard that bails to `None` on Tcl ARE-specific metacharacters — `\y`/`\Y`/`\A`/`\Z`/`\m`/`\M` word-boundary markers, `(?=…)`/`(?!…)`/`(?<…)` lookaround, and `(?q…)`/`(?c…)`/`(?e…)`/`(?b…)` embedded options so callers fall through to the runtime regex engine for those patterns). 21 new unit tests total across the three strips (i1 + i2 + i3). | landed |
| C24b4 | **var_scoping grammar.** `core/analysis/var_scoping.py` → `rust/tcl-compiler/src/var_scoping.rs`. Public helpers returning *indices* so both the LSP declaration provider and memory-SSA share one source of truth: `global_declaration_indices(args)` filters bare names; `variable_declaration_indices(args)` takes every-other-arg starting at 0 with `$`-filter; `upvar_local_declaration_indices(command, args)` handles `upvar`, `upvar LEVEL`, `upvar #N`, `upvar -N`, lowered `namespace upvar` (args[0]=="upvar") and pre-composed `"namespace upvar"` forms, skipping pairs where either side is a substituted reference. `memory_ssa.rs::detect_upvar`/`detect_global`/`detect_namespace_variable` now call through; signatures unchanged. 15 new unit tests. | landed |
| C25e  | **SCCP / dataflow extensions.** Four strips on top of C25. **C25e1** — `evaluate_def` for `Statement::Incr`: folds through the lattice when base is `Const(Int)` and the amount is absent (+1), a decimal literal (positive or negative), or a `$var`/`${var}` ref that resolves to another `Const(Int)`; overflow / non-integer widens to `Overdefined`. New `resolve_simple_var_ref` bare+braced parser. **C25e2** — foreach/lmap constset extraction: `extract_foreach_elements(list_text)` splits literal `{…}` / `"…"` / bare-word lists (returns `None` for `$`/`[` operands); `resolve_foreach_list_via_lattice(list_text, uses, values)` reads `$var` from the lattice. `evaluate_def`'s `Call` arm for `foreach`/`lmap` with one def + one list arg folds the iteration variable to `Const(String)` (singleton) or `ConstSet` (multiple), widening through `LatticeValue::constset`. **C25e3** — `extract_dataflow_graph(inputs)` module aggregator taking `&[FunctionInputs<'_>]`, sorting by `function_name` for deterministic output. **C25e4** — `Statement::AssignValue` folding: new `fold_assign_value` (plain literal → Const, simple var ref → lattice lookup, command substitution → `try_fold_cmd_subst`) and `try_fold_cmd_subst` (`[list …]` via `fold_list_cmd`, `[format …]` via `try_format_fold`, `[llength LIST]` via foreach-element helpers, `[string length "…"]`, `[expr {EXPR}]` via `parse_expr` + `eval_tcl_expr`). Helper pair `split_head` / `strip_one_level`. 26 new unit tests. | landed |
| C26e  | **GVN extensions.** Four strips on top of C26. **C26e1** — embedded command-substitution scanning: `scan_bracketed_commands(text)` walks argument text left-to-right, skipping braced regions as opaque and handling `\[`/`\]` escapes; `split_cmd_text(inner)` splits into `(cmd, args)` preserving `{…}` / `"…"` delimiters. `statement_occurrences` now also scans `Statement::Call` args and `Statement::AssignValue` value text, emitting an `ExprOccurrence` per pure + CSE-candidate nested command. **C26e2** — loop-invariant detection (O107): `find_loop_invariants(registry, cfg, ssa, dialect)` uses `reachable_from` + `natural_loop_blocks` + `dominates(ssa, ancestor, node)` to enumerate natural loops per back edge, merges loop blocks sharing a header, and reports any pure occurrence whose `variable_uses` are all defined outside the loop with code `"O107"`. **C26e3** — partial-redundancy detection (O106): `OccurrenceEvent::{Occur, Kill}` per-statement events, `collect_function_occurrence_events` walks the function, `transfer_occurrence_keys` applies events to an availability set, and `find_partial_redundancies` runs the classic may/must availability fixed-point then replays events to report occurrences that are may-available but not must-available with code `"O106"`. **C26e4** — intra-module pure-proc detection: `PureProcs` set, `find_pure_procs(registry, cfg_module, dialect)` iterative fixed-point classifier that starts with every user proc assumed pure and drops any proc whose body has an impure statement (handles mutual recursion correctly, propagates impurity through callers), plus `is_pure_with_procs` and `is_worth_reporting_with_procs` that consult the set before falling through to registry metadata. 21 new unit tests. | landed |
| C27a  | **value_shapes.** `core/compiler/value_shapes.py` → `rust/tcl-compiler/src/value_shapes.rs`. `is_pure_var_ref(text)` / `parse_command_substitution(text)` helpers shared by later passes. 7 unit tests. | landed |
| C27b  | **static_loops — bounded iteration inference.** `StaticValue` (Int/Float/Bool/Str), `StaticEnv`, `DEFAULT_MAX_STATIC_LOOP_ITERS=4096`. `evaluate_expr_with_constants` bridges to C22; `simple_var_ref`, `parse_literal_value`, `strip_word_delimiters`, `resolve_switch_subject/pattern` helpers. `exec_statement` handles AssignConst / AssignExpr / AssignValue / Incr / If / Switch; calls / barriers / loops abort. `summarise_static_for(init, cond, next, body, initial, max_iters)` + `summarise_for_statement` convenience wrapper. 11 unit tests. | landed |
| C27c  | **rendered_properties — string content lattice + SSA walk.** `RenderedProperties` bitflags (6 may-bits + 3 provenance-bits + 2 must-bits), `RenderedValueProps { may, must }` with `bottom()` / `top()` / default. `rendered_join(a, b)` (may ∪, must ∩), `analyse_literal(text)` for slash/backslash/CRLF/null + starts-with-slash/dash, `apply_unescape` / `apply_normalised` provenance transitions. **C27c follow-up** — `propagate_rendered_props(cfg, ssa, sccp, registry)` fixed-point SSA walk ordered by `cfg_order`, evaluating AssignConst/AssignValue/AssignExpr/Incr/Call defs plus `Barrier`-widens-to-top. Call dispatch consults `IS_UNESCAPE` (WAS_UNESCAPED + double-escape escalation), `UNNORMALISED_HTTP_GETTER` + `-normalized` arg (FULLY_NORMALISED), and `RETURNS_PATH` (HAS_FORWARD_SLASH) traits. `evaluate_value` handles copy-propagation through pure var refs (version-0 reads widen to `unknown_top()` — all may-props possible — so callers can't treat missing bits as absence); opaque command substitutions start from `unknown_top()` baseline and refine with registry hints. Phi-joins include version-0 incomings modelled as `unknown_top()` so enclosing-scope / undefined-on-this-path values do not narrow the merge. `scan_value_text` scans literal + interpolation segments for leading-char must-bits (STARTS_WITH_SLASH / STARTS_WITH_DASH, including backslash-escaped `\-`). Wired into `FunctionUnit::rendered_props` in `compilation_unit.rs`. 17 unit tests. | landed |
| C27d  | **shimmer — diagnostic types + cycle detection.** `ShimmerWarning`, `ThunkingWarning` with `"S100"`/`"S101"`/`"S102"` codes. `loop_body_blocks(cfg)` via BFS-from-successors-intersected-with-predecessors-of-start (matches Python two-pass algorithm). `blocks_reaching`, `build_successors`. `find_shimmer_warnings` / `find_thunking_warnings` stubs pending TypeLattice follow-up. 5 unit tests. | landed |
| C28   | **Interprocedural summaries + call resolver.** `Arity { min, max }`, `ProcArgTrait` enum, `ConstantReturn`, `ProcSummary` (qualified_name, params, arity, transitive calls, has_barrier/has_unknown_calls/writes_global/pure, effect_reads/effect_writes, returns_constant/constant_return/return_depends_on_params/return_passthrough_param, can_fold_static_calls, per-param traits) + `ProcSummary::unknown(name)` conservative stub, `MethodSummary` (class_name/method_kind/instance vars/calls_my/calls_next), `InterproceduralAnalysis { procedures, methods }`. `resolve_internal_call(command, caller_qname, known)` walks the caller's namespace for bare names, resolves absolute / global-relative names directly. `resolve_call_target` convenience. `namespace_parts_from_proc`. 10 unit tests. Summary-building pipeline (body walk + effect tracking + constant-return inference) is a follow-up. | landed |
| C29   | **Taint analysis — lattice + diagnostic + propagation + inter-proc.** `TaintColour` bitflags (15 colours: TAINTED + 14 mitigating), constant masks `ALL` / `T102_SAFE` / `CRLF_SAFE`, `TaintLattice { colours }` with `clean()` / `tainted()` / `is_tainted` / `join` (taint ∪, mitigations ∩) / `with(colour)` / `sanitised`, `TaintWarning { span, variable, sink_command, code, message, replacement }` (the `replacement` field was added in the IRULE3001–3004 follow-up below). Full `propagate_taints(cfg, ssa, sccp, registry, rendered_props, interproc, dialect)` intra-procedural worklist with SCCP-edge-executability phi semantics, T100/T101/T102 sink detection via `find_taint_warnings`, registry-driven sanitiser / source classification (`EVALUATES_CODE`, `TAINT_SINK`, `UNNORMALISED_HTTP_GETTER`, `WARN_WITHOUT_TERMINATOR`). **C29 follow-up (landed)** — (1) `rendered_props` colouring: `colour_from_rendered` enriches each SSA lattice with `PATH_PREFIXED` + `NON_DASH_PREFIXED` on positive `STARTS_WITH_SLASH` evidence *only*; absence-implies-safe heuristics (CRLF_FREE / NON_DASH_PREFIXED from missing bits) were removed as unsound under phi-joins and opaque command subs. (2) Inter-procedural passthrough transfer via `InterproceduralAnalysis::procedures` + `return_passthrough_param` + `resolve_internal_call`; callers cache a `known_procs` set on `TaintCtx`. Global-write seeding is scoped to procs actually reachable from the current function (via the caller's transitive `calls` closure or a CFG walk of direct callees for `::top`), and seeds only `::`-prefixed names that the function actually reads via `collect_global_reads` (scans every block's entry_versions + statement uses + phi incomings for version-0 reads). (3) iRules dialect sources: when `dialect ∈ {"f5-irules","irules"}`, `HTTP::`/`URI::`/`IP::`/`TCP::`/`UDP::`/`SSL::`/`STREAM::` namespace-prefixed commands are treated as taint sources. `CompilationUnit::with_interprocedural(registry, dialect)` now re-runs `propagate_taints` on every FunctionUnit with the fresh interproc summary + dialect. The inner helpers share a `TaintCtx { registry, interproc, known_procs, caller_qname, dialect }` bundle; join-accumulator uses `Option<TaintLattice>` so mitigations are preserved on first insertion; phi joins include version-0 incomings so the merge stays sound across enclosing-scope reads. 15 unit tests. **C29 follow-up (landed) — path-concat heuristic (W201).** `path_concat.rs::find_path_concat_warnings(cfg, ssa, rendered_props, taints, executable_blocks)` emits `PathConcatWarning { span, variable, code, message, replacement }` for every `Statement::AssignValue` whose SSA def carries both `HAS_FORWARD_SLASH` / `HAS_BACKSLASH` and `HAS_INTERPOLATION` in its rendered-properties may-mask. Pure `$var` aliases and pure `[cmd …]` subs are skipped (structural, not concatenation). Suppression: `PATH_NORMALISED` / `PATH_BOUNDED` on the taint lattice, or a forward-scan of the same block finding the next same-variable assignment equal to `[file normalize $var]` (bare or braced form, optionally quoted). `build_file_join_fix` rebuilds a `[file join segments…]` suggestion when every `/`-split segment is either a simple identifier (`[A-Za-z0-9_.-]+`) or a simple `$var` / `${var}` reference — RHSes with `[`/`]`/`;`/whitespace or mixed segments (e.g. `$name.log`) return `None`. Span preference is argv[2] (the value token) falling back to the full statement span. Wired into `compiler_checks::run_all_checks` as category `"taint"` severity `Warning`. 19 new unit tests (8 file-join-fix forms, 4 file-normalize-of matching, 6 end-to-end detection/suppression, 1 compilation-unit integration via `run_all_checks`). URI-split heuristics and iRules-specific sink codes (IRULE3001–3004) remain for a follow-up. **C29 follow-up (landed) — iRules security sinks (IRULE3001–3004 / 3101 / 3102).** `taint.rs::classify_sink` extended to take `(command, args, dialect)` and delegate to a private `classify_irules_sink` helper under `dialect ∈ {"f5-irules","irules"}`. Emits IRULE3001 on `HTTP::respond` body taint (suppressed by `HTML_ESCAPED`), IRULE3002 on `HTTP::header` / `HTTP::cookie` `insert`|`replace` value taint (suppressed by `CRLF_SAFE`, plus `HEADER_TOKEN_SAFE` when the tainted var occupies arg-index 1 — the header/cookie name position), IRULE3003 on `log` taint (suppressed by `CRLF_SAFE`), IRULE3004 on `HTTP::redirect` URL taint (new `TaintColour::REDIRECT_SAFE = PATH_PREFIXED | PATH_NORMALISED` mask). `find_taint_warnings` gains a `dialect` parameter plumbed through `compiler_checks::run_all_checks`. Per-code suppression runs via `irules_sink_suppressed(code, lattice)` + the `irule3002_name_position_safe` helper. New `find_setter_constraint_warnings(cfg, ssa, taints, executable_blocks)` emits IRULE3101 for `HTTP::uri` / `HTTP::path` setter calls whose value does not start with `/`, with three-case Python parity: literal (no `$`, no `[`) → prefix check; pure var-ref → tainted-and-safe-colour suppression via `PATH_PREFIXED | PATH_NORMALISED | PATH_BOUNDED`; dynamic (interpolation / command sub) → always warns. Command + prefix table hardcoded as `SETTER_CONSTRAINTS`. New sibling module `irules_checks.rs::find_unnormalised_getter_warnings(cu, registry, dialect)` handles the non-taint AST check IRULE3102 (`HTTP::uri` / `HTTP::path` / `HTTP::query` getter without `-normalized`, dialect-gated). Scans `Statement::Call` + `Statement::AssignValue`-with-RHS-command-sub; getter-vs-setter approximation: `args.is_empty() || args.iter().all(|a| a.starts_with('-'))` (TODO noted pending a Rust-side `FormKind` port). `TaintWarning` gains `replacement: Option<String>` (always `None` today, wired through `Diagnostic::from_taint` for future rich-fix support). `Diagnostic::from_irules_check` lowers IRULE3102 as category `"irules"` severity `Warning`; sinks + setter-constraint stay in category `"taint"` severity `Error`. Sink / setter / IRULE3102 command lists are hardcoded pending a Rust-side `CommandSpec::taint_hints` + `OptionSpec` port; IRULE3103 URI-split remains deferred with W202/W203; T103 / T104 / T105 stay deferred. 34 new unit + integration tests (7 classifier / mask, 10 end-to-end sink, 6 setter constraint, 7 IRULE3102, 4 `run_all_checks` integration). 1145 tests total (from 1111). | landed |
| C30   | **Optimiser — types, priorities, pass registry.** `Optimisation { code, message, span, replacement, group, hint_only }` + `new()` constructor, `PassContext<'a> { source, optimisations, interproc }` with `report()`. `opt_priority(code)` table (10-0 scale matching Python `_OPT_PRIORITY`). `PassId` enum (9 variants: BranchFolding / Elimination / ExprSimplify / PatternRecognition / Propagation / StructureElimination / TailCall / UnusedProcs / CodeSinking) with `all()` canonical order + `as_str()`. `run_passes` stub. 6 unit tests. Individual pass bodies deferred (10 follow-up strips). **C30a** — branch-folding pass: split `optimiser.rs` into `optimiser/{mod.rs,branch_folding.rs}`, port `core/compiler/optimiser/_branch_folding.py::optimise_constant_branches` as `branch_folding::run(ctx, cu)` emitting `O101` ("Fold constant expression") for every SCCP-proved [`ConstantBranch`] with the condition replaced by `{1}`/`{0}` (bare `1`/`0` when the source condition is unbraced). Skips switch-dispatch probes (`BinOp::StrEq` + `switch_next_*` block/target names). `run_passes(ctx, cu, passes)` dispatches `PassId::BranchFolding` to the new body; other pass ids remain deferred no-ops. 9 new unit tests covering constant-true / constant-false / nested folds / mixed lattice / Overdefined no-fold / bare condition / switch-dispatch skip / run_passes smoke-test. `optimise_branch_proc_calls` (re-runs expression simplification on branches SCCP can't resolve) is deferred — depends on `expr_simplify` + `propagation` pass bodies. **C30x1** — extend `PassContext` with per-function propagation scratch (`proc_cfgs`, `propagated_branch_uses`, `propagated_use_groups`, `propagated_expr_stmts`, `cross_event_vars`, `next_group` + `alloc_group()`, `ir_module`, `dialect`) + `reset_function_state()` helper. 5 new unit tests. **C30x2** — port `core/compiler/optimiser/_helpers.py` into `optimiser/helpers/{naming, literals, constants, select, tokens, expr_simplify}.rs`; each concern in its own submodule. `naming`: namespace_parts, namespace_from_qualified, resolve_summary_proc_name. `literals`: is_safe_word, is_static_var_word, is_plain_literal, literal_from_constant_str (Literal tag), render_folded_literal, render_static_string_word, format_constant. `constants`: constants_from_versions / _uses / _exit_versions projecting SCCP lattice into a name→Tcl-literal dict. `select`: select_non_overlapping — sort by (start, -priority, -length), drop overlapping rewrites, clear group ids on surviving siblings whose group lost a member. `tokens`: extract_body_text + column_of_offset for source-level body re-indentation. `expr_simplify`: try_fold_expr (via eval_tcl_expr), try_unwrap_expr_in_expr (O115 body detect), substitute_expr_constants (tokenise_expr-driven `$var` replacement with numeric/quoted-string emission), expr_has_command_subst; plus stubs for the deferred AST rewriters (instcombine, strength reduction, strlen / streq simplification) matching the production signatures. 43 new unit tests. **C30b** — port `_unused_procs.py` as `optimiser::unused_procs::run(ctx, cu)`: O124 commenting-out of iRules procs unreachable from any event handler, gated on dialect ∈ {f5-irules, irules}; library-iRule skip; barrier escape hatch; 13 new unit tests. **C30c** — port `_structure_elimination.py` as `optimiser::structure_elimination::run(ctx, cu)`: O112 for constant-condition if / while / for elimination (and switch with literal subject — gated on IR arm/default spans, a pre-existing lowering limitation). Projects SCCP lattice into an `Env` for `eval_tcl_expr`; handles fall-through switch chains. 16 new unit tests. **C30d** (partial) — port the O107 slice of `_elimination.py`: unreachable-block dead-code reporting. O108 / O109 / O126 deferred behind the liveness analyser that's not yet landed. 7 new unit tests. **C30e** (partial) — port the entry-point + three landed helpers of `_expr_simplify.py`: constant folding (O101) + redundant nested-expr unwrap (O115) on standalone `expr` statements; 4 AST-level simplification rewriters (instcombine / strength reduction / strlen / streq) remain stubs. 21 new unit tests. **C28x** (partial) — port the structural half of `interprocedural.py::analyse_interprocedural_ir` as `interprocedural::build_interprocedural_analysis` + `CompilationUnit::with_interprocedural` façade: five-phase pipeline (scan_all_procs → transitive closure → purity fixpoint → effect-region fixpoint → materialise_summaries) populating `calls` (transitively closed), `has_barrier`, `has_unknown_calls`, `writes_global`, `pure`, `effect_reads`, `effect_writes`. Return-value inference + param-trait fields deferred. 9 new unit tests. **C30f** (partial) — port `_propagation.py::optimise_constant_var_refs` as `optimiser::propagation::run(ctx, cu)`: O100 replacement of single-token `$var` call arguments with their SCCP-proved literals, guarded by an integer-or-safe-word value filter that refuses to inline Tcl metacharacter-containing values. Six other propagation modes deferred. 7 new unit tests. **C30a'** — complete the `optimise_branch_proc_calls` half of `_branch_folding.py`: for each branch SCCP could not fold, unwraps one level of `{…}`, runs `substitute_expr_constants` against the per-function lattice, and emits O100 when a substitution produced a text change. 2 new unit tests. **C30g** (partial) — port `_pattern_recognition.py::optimise_incr_idioms` as `optimiser::pattern_recognition::run(ctx, cu)`: O114 rewriting of `set x [expr {$x ± N}]` (commutative Add) to `incr x [N]`, working directly off the AssignExpr AST shape. O119 + O122 deferred. 9 new unit tests. **C30h** (partial) — port the bare-call slice of `_tail_call.py` as `optimiser::tail_call::run(ctx, cu)`: O121 for every proc that ends in a bare self-call (including inside if/switch tail positions). Return-subst variant + O122 loop conversion + O123 accumulator deferred. 5 new unit tests. **C30i** (scaffolding) — `_code_sinking.py` entry point + IR walker. No O125 diagnostics yet — blocked on side-effect classification + per-statement variable-use scanning. 2 new unit tests. **C30j** — port `_manager.py` as `optimiser::manager::{optimise, optimise_with_dialect, optimise_raw}`: thin façade that builds a CompilationUnit, populates interprocedural summaries, runs every landed PassId in PassId::all() canonical order, and applies select_non_overlapping. 6 new unit tests. **C30d-complete** — O109 dead stores + O126 unused-variable assignments using the C24 def-use chains. Distinguishes O109 (later version of the same var has live consumers — the write is shadowed) from O126 (no version has any read, via a textual `$var` scan that catches Return / string-interpolation uses the def-use builder does not track). Scope-alias commands (`global` / `variable` / `upvar` / `namespace upvar`) and cross-event vars are skipped. **C30e4-7** — the four AST-level expression rewriters: `instcombine_expr` (fixpoint over a bottom-up `simplify_node_once`, capped at 16 iterations), `try_strength_reduce_expr` (x+0, x-0, x*0, x*1, x/1, x**2 → x*x, x%pow2 → x & (pow2-1)), `try_strlen_simplify_expr` ([string length OP] == 0 → OP eq ""), `try_eq_ne_string_compare_simplify_expr` (`==`/`!=` against an ExprNode::String literal → `eq`/`ne`). `branch_folding::propagate_into_branches` now cascades substitute → strength_reduce → strlen → streq → instcombine with the first rewriter that changes text winning its diagnostic code (O113 / O117 / O120 / O110 / O100). **C28x-return** — return-value inference: per-proc return classifications (Literal / Passthrough / UsesParam / Other) collapse into `returns_constant` / `constant_return` / `return_passthrough_param` / `return_depends_on_params` / `can_fold_static_calls`. Unlocks proc-call folding in propagation. **C30d-O108** — ADCE fixpoint: a statement-level def is transitively dead when side-effect-free AND every statement-level consumer is itself in the removed set; iterates on top of the O109/O126 baseline. Phi-incoming / terminator uses keep the def alive unconditionally. **C30h-return-subst** — `return [self args]` variant of O121: inspects Return values for a `[cmd args…]` shape whose head matches a self-name variant. **C30f-remainder** — O103 static-proc-call folding (pure procs with constant returns) + O104 return-terminator folding (`return $v` → `return K` when v is SCCP-constant). **C30i-full** — O125 code-sinking detection (hint-only). Pattern: `set X V; <decision>` where the decision condition doesn't reference X, at least one decision body reads X, and no later statement reads X. Walks the IR at the statement level (avoiding the local-offset span issue with re-lowered proc bodies). **C30g-remainder** — O119 multi-set packing hint (3+ consecutive literal `set`) + O122 string-build chain hint (`set s ""` followed by 2+ same-var `append`). Both hint-only. **C28x-param-traits** — per-parameter trait inference (Unused / UsedInCondition / ForwardedToCallee / Passthrough) populating `ProcSummary::param_traits`. Observed via note_params_in_expr (conditions + ExprEval) + text_references_name (Call args), finalised with Passthrough inherited from return_passthrough_param. **C30f-final** — optimise_string_interpolation_var_refs (O100 inlining SCCP constants into `"$x"` arg text) + optimise_load_forwarding (O102 forwarding the single reaching literal def to each Operand use site via def-use chains — fires even when SCCP is Overdefined). optimise_expression_args / optimise_expr_substitutions are subsumed by branch_folding::propagate_into_branches and expr_simplify::run; no separate port needed. **C30h-final** — O122 loop-conversion hint (every self-call is in tail position) + O123 accumulator-candidate hint (non-tail self-call embedded in an expression substitution). Both hint-only. **C30i-multi-opt** — promote O125 from hint-only to a grouped pair of real rewrites: one deletion on the original `set` span, one insertion at the first target-body statement that reads the sunk variable, with the original set's source text + `; ` prepended. Both share a group id via PassContext::alloc_group. Falls back to hint-only when the original statement's span is local (e.g. re-lowered proc body). 1008 cargo test -p tcl-compiler --lib passing; clippy clean. | landed |
| C31   | **compilation_unit facade + compiler_checks aggregator.** `FunctionUnit { name, cfg, ssa, def_use, sccp, memory_ssa }` with `build(name, cfg, registry)` driving the landed pipeline (SSA + def-use + SCCP) and `with_memory_ssa()` on demand. `CompilationUnit { source, ir_module, cfg_module, top_level, procedures, interproc }` with `build_for(source, registry, defer_top_level)` running `lower_to_ir` → `build_cfg` → per-function `FunctionUnit::build`. `function(name)` lookup + `functions()` iterator. `Severity` (Hint/Suggestion/Warning/Error) + unified `Diagnostic { span, code, category, severity, message }`. `run_all_checks(cu, registry, dialect)` collects from SCCP (constant branches), GVN (full + partial + loop-invariant), shimmer + thunking, taint. 8 unit tests. | landed |
| C32   | **Python shim retirement — optimiser entry points.** Landed: PyO3 bindings in `rust/tcl-lsp-rust/src/optimiser.rs` exposing `optimiser_find_optimisations(source, dialect)` / `optimiser_find_optimisations_raw(source, dialect)` / `optimiser_opt_priority(code)` — thin wrappers over `tcl_compiler::optimiser::{optimise_with_dialect, optimise_raw, opt_priority}`. The Python `core.compiler.optimiser._manager::find_optimisations` accepts delegation via `TCL_LSP_RUST_OPTIMISER=1` (opt-in — default keeps the Python pipeline so exact-message and overlap-arbitration tests do not regress during the parity-testing phase). `_materialise_rust_optimisations` helper converts the Rust `(code, message, start, end, replacement, group, hint_only)` tuples back to Python `Optimisation` dataclasses with `Range` / `SourcePosition` values built from a local line-start index. 2 new PyO3 smoke tests. Remaining Python-shim work (the `compiler_checks` / `find_*` analyser entry points, flipping the default once parity is verified) is tracked as a follow-up. **C32-shim (Phase 1)** — PyO3 binding `compiler_checks_run_all(source, dialect)` in `rust/tcl-lsp-rust/src/compiler_checks.rs` exposes `tcl_compiler::compiler_checks::run_all_checks`, returning diagnostic tuples `(code, category, severity, message, start_offset, end_offset)`. Infrastructure only: not wired into a Python consumer yet because (a) shimmer / taint Rust implementations are still stubs (C27d / C29 deferred), (b) the Python compiler-diagnostics pipeline is distributed across several modules (`semantic_graph.py`, `core_analyses.py`, per-check entry points) with no single aggregator to swap 1:1, and (c) the optimiser parity gap (~131 test_optimiser.py failures under `TCL_LSP_RUST_OPTIMISER=1`) indicates broader Rust-side analysis parity work is still pending. 2 new PyO3 smoke tests. Follow-ups: land shimmer / taint Rust bodies, add analyser-level entry points, then flip consumers behind per-class opt-in env vars. | landed |
| C*    | **Compiler migration — deferred analysis bodies.** Remaining work on landed chunks: full shimmer use-site / phi-shimmer / thunking detection (C27d), interprocedural summary-building pipeline (C28), taint propagation + path-concat + URI-split + sink-specific checks (C29), ten optimiser passes (C30a-j), and `connection_scope` + iRules event-flow (C28 follow-up). Each is a focused strip that plugs into the already-landed type surface. | planned |
| S*    | **LSP server migration.** `lsp/` (pygls handlers, workspace orchestration, feature providers) → `rust/tcl-lsp-server/` on `tower-lsp`. This is when `ropey` enters the picture as the document store, the whole pipeline becomes async, and the server ships as a standalone Rust binary. | planned |
| R*    | **Remainder.** `vm/` (bytecode VM, interpreter, REPL), `core/commands/` (command registry), `core/analysis/` (analyser passes), `core/formatting/` (formatter engine), `core/minifier/`, `core/irule_test/`, `debugger/`, `fuzzing/`, `explorer/`, CLI tooling (`scripts/`). A Python interface is kept on top for Claude skills, the MCP server, and other integrations. | planned |
| Sync  | **Rebase the rust rewrite branch onto main HEAD.** The rewrite branch had disjoint history from main; this sync hard-resets the branch pointer to `origin/main` and re-applies every rust-rewrite-unique file (the `rust/` workspace, `Cargo.{toml,lock}`, `rust-toolchain.toml`, the three rust docs, `core/compiler/rust_spans.py`, the `tests/test_rust_*.py` + `tests/test_tokens.py` set) plus the Python-side dispatch shims (`TCL_LSP_RUST_OPTIMISER` / `_GVN` / `_INTERPROC` envs in `core/compiler/{optimiser/_manager,gvn,interprocedural}.py`, the rust primary path in the four `core/parsing/` lexer-adjacent files), splices the `rust-build` / `rust-test` / `rust-lint` / `rust-format` targets into main's `Makefile`, and fixes a stale `core/analysis/analyser.py` reference (split into `_analyser/` on main) plus an out-of-date `core/irule_test/tcl/_registry_data.tcl` codegen output. Also ports main's IEEE 754 special-literal fix (`Inf` / `NaN` / `Infinity` tokenise as `NUMBER` not `FUNCTION`) into `rust/tcl-lexer/src/expr_lexer.rs::Lexer::ident` so the Rust expr-lexer stays parity with the Python fallback. `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test --workspace` + `make rust-build` + `make prep-pr` all green at the sync commit. | landed |
| R2    | **Registry deltas from main.** Adds the three new tcl command specs introduced in main (`registry`, `lseq`, `zlib`) under `rust/tcl-registry/src/commands/tcl/` (the `registry` spec lives in `registry_.rs` to avoid colliding with the crate-level `registry` module). Aligns top-level arity for `fcopy` (`Arity::at_least(2)` → `Arity::new(2, 6)`, matching C Tcl 9.0's two channels + four optional option-pair flags) and `tailcall` (`Arity::at_least(1)` → `Arity::any()`, matching C Tcl 9.0's "no args clears scheduled tailcall, with args replaces it" semantics). 118 tcl specs total (was 115). | landed |
| C39   | **Small codegen fixes from main (audit + per-fix strips).** See the C39 sub-plan below. | landed |
| C35   | **Const-propagate-uplevel.** See the C35 sub-plan below. | landed |
| C34   | **Uplevel-passthrough whole-callee inlining (`inline_uplevel`).** See the C34 sub-plan below. | landed (param-body rewrite deferred) |
| C37   | **Parse-cache address-keying + const-map isolation.** See the C37 sub-plan below. | landed (C37b only; C37a is a Zig-runtime concern, N/A for Rust compiler) |
| C38   | **Namespace-import compile-time resolution.** See the C38 sub-plan below. | landed (data layer; codegen consumer deferred) |
| C36   | **Factory specialisation pass + `subst -nocommands`.** See the C36 sub-plan below. | landed |
| C33   | **`var_escape` flow-sensitive analysis (5 strips).** See the C33 sub-plan below. | planned |

Keep this table current. Mark a row as `landed` in the same commit that
lands the chunk.

## Pending chunks — detailed sub-plans

The chunks below are scoped follow-ups to the `Sync` rebase. Each
table row above maps to one of the sections here. The order
matches the planned execution order: smaller / lower-risk chunks
first to bank confidence, larger restructuring chunks last.

### C39 — Small codegen fixes from main

Audit each main-side codegen / lowering / parser fix landed since
the rewrite branch was created and decide one of three outcomes:

- **Port** — applies to the Tcl-bytecode codegen path that
  `rust/tcl-compiler/` targets. Add a fixture under
  `rust/tcl-compiler/tests/fixtures/codegen/matching/` and a unit
  test next to the change.
- **WASM-only** — the fix lives in `core/compiler/codegen/wasm/`
  and only affects the runtime-interpreter dispatch. Note the
  reason in the per-strip commit message and skip.
- **Already covered** — the Rust port already does the right
  thing; add a regression fixture and skip the port.

Strips (one commit per audit + port):

- **C39a** — `fix(codegen): foreach binds every loop variable per
  iteration` (main `d5437ea0`). Audit
  `rust/tcl-compiler/src/codegen/emitter/loop_blocks.rs::detect_foreach`
  + the foreach emitter — confirm whether the multi-var step uses
  `len(loop_vars)` or `1`. WASM patch was the obvious case; bytecode
  emit *should* already be correct via `FOREACH_START4` / `STEP4`
  semantics, but add a fixture (`foreach-multi-var-step.tcl`) to
  prove it.
- **C39b** — `fix(codegen): continue in a for loop runs the
  next-script before looping` (main `ef4806c5`). Audit
  `codegen/emitter/generate.rs` for-loop dispatch — verify the
  continue target jumps to the post-body / pre-test edge that
  emits the `next` script. Add fixture `for-continue-runs-next.tcl`.
- **C39c** — `fix(codegen): dict create folds key/value pairs into
  the result` (main `74669d92`). The Rust port already has a
  `dict_create` constant-folder in `codegen/helpers.rs::fold_dict_create_cmd`.
  Add fixture proving `[dict create k1 v1 k2 v2]` folds, and a
  matching unit test.
- **C39d** — `fix(codegen): switch -glob dispatches inline via
  tcl_string_match` (main `147a4a74`). WASM-only — the Tcl bytecode
  path already emits `JUMP_TABLE` / `STR_MATCH` opcodes natively.
  Add a fixture proving exact-mode `switch` falls through
  correctly, mark the entry "WASM-only port skipped".
- **C39e** — `fix(codegen): expr min/max variadic, int/entier/wide,
  bool, pow` (main `3cf8f713`). Cross-check
  `codegen/expressions.rs::emit_expr` math-function dispatch —
  ensure variadic `min` / `max` and the int-coercion family emit
  matching `invokeStk` patterns. C22i in the existing chunk log
  already ports the constant-folding side; this strip is the
  emitter side.
- **C39f** — `fix(codegen): expr in/ni list-membership operators`
  (main `89f8e43c`). The C22i chunk in
  `tcl_expr_eval.rs` evaluates `in` / `ni`. Verify
  `codegen/expressions.rs` emits the expected `STR_FIND` / `STR_EQ`
  + `LIST_INDEX` sequence for unfolded operands. Fixture:
  `expr-in-list-runtime.tcl`.
- **C39g** — `fix(codegen): accept 0x/0o/0b expr literals via
  int(value, 0)` (main `fdc1ee70`). Confirm
  `tcl_expr_eval.rs::parse_literal` already handles all three
  prefixes (it does — see C22 chunk log). Fixture:
  `expr-radix-prefixes.tcl`.
- **C39h** — `fix(codegen): lsort with options targets the trailing
  list arg` (main `e43a7985`). Audit `codegen/emitter/bytecoded.rs`
  for an `lsort` hook (none today). Either add a registry
  codegen-hook that picks the trailing arg as the list, or note as
  generic-invoke fallback.
- **C39i** — `fix(codegen): pack args-tail when calling a proc from
  a value context` (main `d341e74f`). Trace
  `codegen/cmd_subst.rs::emit_generic_cmd_subst` — verify the
  `INVOKE_STK4` form is reached when value-context receives ≥4
  args.
- **C39j** — `fix(codegen): double() cast in expr` (main
  `f5506f10`). Verify the `double` math function in
  `tcl_expr_eval.rs::eval_math_function`.

Done definition: each strip lands a fixture or a unit test, the
differential codegen harness still reports zero new divergent
fixtures, and `cargo test -p tcl-compiler --lib` stays green.

**Landed outcome.** Five new matching fixtures added
(`foreach-multi-var.tcl`, `for-continue-runs-next.tcl`,
`dict-create-fold.tcl`, `expr-double-cast.tcl`, plus the existing
single-var `foreach-simple.tcl` already covering C39a's bytecode
path). One new divergent fixture filed:
`divergent/expr-radix-fold.tcl` — `expr {0xFF + 1}` parses but the
Rust pipeline does not constant-fold the binary op (Rust emits 18
instructions, Python 12). Filed as a follow-up, not blocking C39.
Differential matching corpus: 33/33 passing (17 exact + 16
semantic), divergent corpus: 2 (was 1). All other C39 strips fall
into "WASM-only port skipped" (C39d) or "audit confirms parity
already in place" (C39a/b/c/e/f/h/i/j) — a fixture in
`matching/` is the proof for each.

### C35 — Const-propagate-uplevel

**Order swap.** C35 originally listed before C34, but it has a
hard dependency on C34's `IRUpFrame` IR node (the static-body
uplevel case lowers to `IRUpFrame { body: Script, frame_shift: 1 }`
which doesn't exist until C34 lands). Execute C34 first, then
return to C35. The strips below stand unchanged.

Port the const-propagation half of main PR #185 — specifically:

- `lower(barrier): const-propagate braced-literal bodies through
  uplevel/eval` (main `b5e18ce2`)
- `lower(barrier): inherit const-map across nested script scopes`
  (main `c30203da`)
- `lower(barrier): extend eval relaxation to [list ...] shape`
  (main `a080c8d7`)

Affects `rust/tcl-compiler/src/lowering/structured.rs` (the
`lower_eval` / `lower_uplevel` paths) and the const-map carried on
`Lowerer` state.

Strips (one commit each):

- **C35a** — [LANDED] `Lowerer::const_map_stack: Vec<HashMap<String,
  String>>` and `Lowerer::proc_depth: u32` fields. `lower_script`
  pushes a fresh scope; `lower_body` pushes a *clone* of the
  parent scope (matching main `c30203da`). `lower_proc` and
  `lower_when` bump `proc_depth` around their body lowering so
  the const-map is gated to proc / event-handler scopes only —
  top-level / `namespace eval` writes are globals or namespace
  vars whose values can be observed and mutated by other code,
  which would make const-propagating them unsound. `lower_segmented`
  invokes `update_const_map(seg, &stmt)` after each command —
  populates a literal binding for `set name {literal}` shapes
  (rejecting array refs, namespace-qualified names, substituted
  values), invalidates the named entry on assignment-style
  writes, clears the whole map on `Statement::Barrier` /
  `Statement::Block` / `Statement::UpFrame` / structured IR.
  Module-level helpers `set_literal_body` and
  `invalidate_const_map_for` mirror Python's `_set_literal_body`
  and `_invalidate_const_map_for`. 6 unit tests covering disable
  at top level, single-set+resolve, eval-brace lowering, eval-var
  lowering, invalidation on re-assignment, and inheritance into
  catch body.
- **C35b** — [LANDED] `try_lower_uplevel_static` (C34a) gains a
  `TokenType::Var` branch: when the body argument is a `$var`
  that resolves via the const-map to a brace-string literal,
  re-lower the literal as the inlined body. Mirrors main
  `b5e18ce2`'s `_can_relax_uplevel` + `_relax_uplevel` change.
  New `try_lower_eval_static` mirrors the same shape for `eval`,
  producing `Statement::Block` rather than `Statement::UpFrame`
  (eval doesn't shift frames). Wired into `lower_command` as a
  new `"eval"` arm. Two existing interprocedural tests that
  relied on `eval {}` being a barrier were updated to use
  `eval $dyn` since `eval {literal}` is no longer a barrier
  (it's a `Block`). Same change applied to
  `tests/test_rust_interprocedural_delegation.py`.
- **C35c** — [LANDED] `eval_list_literal_body(cmd_text)` parses
  the inner-command of an `eval [list ...]` callsite via
  `segment_commands`, verifies the head is `list` and every arg
  is a single-token literal (`Esc` without `$` / `[`, or `Str`
  which gets re-braced in the synthesised body), and returns
  the joined body text. `try_lower_eval_static` gains a
  `TokenType::Cmd` branch that calls the helper and re-lowers
  the result as a `Statement::Block`. Mirrors main commit
  `a080c8d7`. 3 unit tests: `[list set x 42]` accepted,
  `[list set x $v]` rejected, `[foo arg]` rejected.

Done: a new differential fixture per strip in
`rust/tcl-compiler/tests/fixtures/codegen/matching/`.

### C34 — `inline_uplevel` pass

Port `core/compiler/inline_uplevel.py` (main `25a4340e`, 621 LOC)
to `rust/tcl-compiler/src/inline_uplevel.rs` plus an IR addition
for `IRUpFrame` (currently absent from the Rust port).

Strips:

- **C34a — IRUpFrame IR node.** [LANDED] Adds the
  `Statement::UpFrame { span, frame_shift, body, tokens }` variant
  to `rust/tcl-compiler/src/ir.rs`. The lowering dispatch
  (`lowering::mod::lower_command`) routes `uplevel ?level? {body}`
  through `try_lower_uplevel_static` for the static-body case
  (level is a decimal int or `#N`, body is a brace-string token);
  dynamic forms fall through to default lowering. Codegen emits a
  NOP placeholder with comment `# unhandled: IRUpFrame` matching
  the Python pipeline's expectation that the inline_uplevel pass
  consumes IRUpFrame before codegen runs. Interprocedural analysis
  treats it as a barrier and walks the body. Code-sinking matches
  it alongside `Statement::Catch` for body-walking. 6 unit tests +
  1 matching fixture (`uplevel-static-body.tcl`). Clippy +
  differential corpus + `make prep-pr` clean.
- **C34b — Static (zero-param) passthrough detection.** [LANDED]
  New crate module `rust/tcl-compiler/src/inline_uplevel.rs`.
  `detect_static_passthrough(&Module) -> HashMap<String, Script>`
  for the static-only convenience and the richer
  `detect_passthrough_candidates(&Module) -> HashMap<String,
  PassthroughShape>` for both shapes. Gate matches main's four
  conditions: zero params, body is one `Statement::UpFrame`,
  `frame_shift == 1`, no nested uplevel/upvar/info-frame
  (recursive `body_has_frame_reach` walks if/for/while/foreach/
  catch/try/switch). 6 unit tests.
- **C34c — Single-body-param passthrough detection.** [LANDED]
  Extends the detector with the `ParamBody { param_name }` shape
  (`proc dispatcher {body} { uplevel ?1? $body }`). Recognises both
  the implicit-level (`uplevel $body`) and explicit-level
  (`uplevel 1 $body`) forms; rejects mismatched param names and
  multi-param dispatchers. 4 unit tests.
- **C34d — Per-callsite rewriter.** [LANDED]
  `inline_uplevel_passthrough(&mut Module, &CommandRegistry)` walks
  every script in the module and replaces matched zero-param
  passthrough callsites with `Statement::Block { body, namespace,
  tokens, .. }`. Recurses into structured statements via
  `walk_nested_scripts` so callsites inside if/for/while/foreach/
  catch/try/switch bodies are rewritten too. Idempotent:
  already-inlined callsites no longer match. Adds a new IR
  variant: `Statement::Block { span, body, namespace, tokens }` —
  flat splice of an inline body without a new scope (mirrors
  Python's `IRBlock`). 6 unit tests covering rewriter behaviour
  + idempotency. **Param-body rewrite is gated off** pending a
  future strip that threads source-text access into the pass so
  it can verify the callsite's argument was passed as a
  brace-string literal (the Rust `CommandTokens.argv` carries
  byte spans only, not token kinds).
- **C34e — Pipeline integration.** [LANDED]
  `CompilationUnit::build_for` runs
  `inline_uplevel::inline_uplevel_passthrough(&mut ir_module, registry)`
  immediately after `lower_to_ir` and before `build_cfg`. The
  CFG builder gains an explicit `Statement::Block` arm that
  recursively flattens the body's statements into the current
  control-flow stream so SSA / def-use / codegen see them
  inline. The codegen `emit_stmt` arm walks the block body
  emitting each statement in turn. Differential corpus stays
  green (matching: 34/34, divergent: 2 — unchanged).

Done: 6 fixtures (3 static, 3 param-body), ~25 unit tests.
Differential harness reports same exact-match count as Python
(matching/divergent split unchanged or improved).

### C37 — Parse-cache address-keying + const-map isolation

Port main's two parse-cache fixes (`b64d7a5f`, `49f90130`) into the
Rust lowering layer.

The Rust lowering already has a per-function const-map (see
`Lowerer::const_map`). Main's fix:

- **C37a — Address-keyed body cache.** N/A for the Rust compiler.
  Main commit `b64d7a5f` retitles a Zig runtime cache
  (`runtime/zig/parse_cache.zig`) — there's no analogous Python
  body-cache port, and the Rust lowering registers each procedure
  exactly once into `module.procedures` so re-lowering is not
  observable. The audit confirmed no Rust-side gap.
- **C37b — Fresh const-map for nested procs.** [LANDED]
  `lower_proc` and `lower_when` now push an empty `HashMap` on
  `const_map_stack` around the body lowering (in addition to the
  existing `proc_depth` bump). `lower_body` clones the parent
  scope on entry, so pushing an empty parent first gives the
  nested proc / event-handler body a clean slate independent of
  the outer scope's tracked literals. Mirrors main commit
  `49f90130`. New unit test
  `const_prop_does_not_leak_into_nested_proc` asserts the inner
  `uplevel 1 $body` stays a `Call` / `Barrier` even when the
  outer proc bound `body` to a brace literal.

Done: per-strip fixture; `cargo test -p tcl-compiler --lib`
clean.

### C38 — Namespace-import compile-time resolution

Port `perf(codegen): resolve namespace-import calls at compile time`
(main `ea155a5c`) and `lower/codegen: gate namespace_imports
shortcut on namespace export` (main `2f5cb008`) and `lower:
suppress namespace import/export in dead if branches` (main
`06f42efa`).

Strips:

- **C38a** — [LANDED] `Lowerer::namespace_imports: Vec<(String,
  String)>` plus a detection block in `lower_command` that
  matches `namespace import ?-force? pattern...`, skips
  `{*}`-expanded calls, filters relative patterns, and records
  `(context_namespace, absolute_pattern)` pairs. Surfaced on
  `Module::namespace_imports` at the end of `Lowerer::lower`.
  Mirrors main commit `ea155a5c`'s lowering-side change.
- **C38b** — [LANDED] `Lowerer::namespace_exports` + a
  `namespace export` arm in the same dispatch block. Skips
  `-clear` / option flags, records `(context_namespace, pattern)`
  pairs, surfaces on `Module::namespace_exports`. The codegen-
  side gate (using exports to decide whether an import shortcut
  fires) belongs to a future strip when the Rust compiler grows
  a namespace-import-resolution path; the data layer is in place
  for that consumer. Mirrors main commit `2f5cb008`'s
  lowering-side change.
- **C38c** — [LANDED] `Lowerer::dead_code_depth: u32` counter +
  `static_bool(expr_text)` helper. `lower_if` (in
  `lowering/structured.rs`) detects clauses whose condition is a
  literal `0`/`false`/`no`/`off` or `1`/`true`/`yes`/`on` and
  brackets the dead body with `dead_code_depth += 1` /
  `dead_code_depth -= 1`. A static-true clause latches the flag
  so every later clause + the `else` branch is dead. The
  `namespace import` / `namespace export` collection is gated on
  `dead_code_depth == 0`. Mirrors main commit `06f42efa`. 6 unit
  tests covering recording, relative-pattern skip, `-force` flag,
  export recording, dead `if{0}` branch suppression, dead `else`
  after `if{1}` suppression.

Done: 3 fixtures, ~10 unit tests, differential harness no
regressions.

### C36 — Factory specialisation + `subst -nocommands`

Port the largest single PR-185 component: the factory pass
(`core/compiler/passes/specialise_factories.py`, 559 LOC) plus its
parser dependency (`core/parsing/subst_nocommands.py`, 233 LOC) and
the lowering-layer plumbing (`d4d2cdd5` "materialise [subst
-nocommands {…}] bodies", `2ad4efc9` "resolve `proc \$var body` via
lowering const-map").

Strips:

- **C36a — `subst_nocommands` parser.** [LANDED] New crate
  module `rust/tcl-compiler/src/subst_nocommands.rs`. Implements
  `subst_nocommands(template, &const_map) -> Option<String>` — a
  compile-time evaluator for `[subst -nocommands {template}]` that
  resolves `$var` / `${var}` against the supplied const-map,
  decodes `\…` escapes via `tcl_lexer::backslash_subst`, keeps
  `[…]` literal (the `-nocommands` flag), and refuses (returns
  `None`) for missing vars, array refs, namespace-qualified
  names, unbalanced brackets. Mirrors Python's
  `core/parsing/subst_nocommands.py`. 13 unit tests covering
  every refusal condition + happy paths.
- **C36b — Lowering: `proc $var body` resolution.** [LANDED]
  `lower_proc` first checks for the `$var` shape (proc name
  contains `$`, doesn't contain `[`, single-token VAR, name
  resolves via `const_map_lookup`) and substitutes the literal
  before continuing as a normal static-name proc. Multi-token
  names (`foo_$x`, `$a$b`) and command-substitution names
  (`$name[suffix]`) stay on the runtime / barrier path. The
  inner proc registers under the resolved FQN. Mirrors main
  commit `2ad4efc9`. 4 unit tests covering the resolved /
  no-binding / command-sub / latest-set-wins shapes.
- **C36c — Lowering: materialise `[subst -nocommands {…}]`
  bodies.** [LANDED] New `Lowerer::eval_subst_nocommands_body`
  helper: parses the inner CMD-token text via `segment_commands`,
  verifies `subst` head + a `-nocommands` flag (rejecting
  `-nobackslashes` / `-novariables` whose semantics differ),
  finds the single STR-token positional template, then delegates
  to `crate::subst_nocommands::subst_nocommands` against the
  current const-map. Wired into `lower_proc` alongside the
  existing body-lowering path: when the body is a CMD token and
  the helper succeeds, lower the substituted text via
  `lower_script` instead of `lower_body`. Mirrors main
  `d4d2cdd5`. 3 unit tests covering the happy path,
  missing-var refusal, and `-nobackslashes` refusal.
- **C36d — Factory shape detector.** [LANDED] New crate module
  `rust/tcl-compiler/src/specialise_factories.rs`. `FactoryShape
  { qualified_name, params, name_param, child_params,
  child_body_template }` plus
  `detect_factory_shape(&Procedure) -> Option<FactoryShape>`.
  Recognises the canonical `proc Configure {name default
  description} { proc $name {…} [subst -nocommands {…}] }`
  pattern by matching the proc body's single
  `Statement::Barrier { reason: "dynamic proc name", command:
  "proc", … }` and verifying the `${name}` / `$name` shape, the
  literal child params, and the brace-template body via a
  reused `extract_subst_nocommands_template` helper. 3 unit
  tests: canonical shape detected, multi-statement rejected,
  non-param name rejected.
- **C36e — Per-call factory specialiser.** [LANDED]
  `specialise_factories(&mut Module, &CommandRegistry)` walks
  the module's top-level + each procedure body, recognising
  call sites of any detected factory. For each match it:
  1. resolves the target via namespace-qualified or root
     lookup;
  2. extracts literal arg bindings (`STR` braced text or `ESC`
     bareword without `$`/`[`);
  3. feeds the bindings through `subst_nocommands` against the
     factory's body template;
  4. lowers the materialised body to an `IRScript` via a fresh
     `Lowerer`;
  5. registers a synthesised `Procedure` under the bound
     `name_param` value;
  6. replaces the call site with a no-op `Statement::Block`.
  Recurses into `Statement::Block` containers; structured IR
  (if / for / while / foreach / catch / try / switch) is left
  alone, matching Python's gate. 3 unit tests covering
  successful synthesis, dynamic-arg skip, and multi-call
  recursion.
- **C36f — Per-factory specialisation cap.** [LANDED]
  `DEFAULT_FACTORY_CAP = 64` constant +
  `specialise_factories_with_cap(module, registry, cap)` entry
  point. A `counts: HashMap<String, usize>` tracks per-factory
  rewrites; once a factory's count reaches *cap* further call
  sites stay on the runtime dispatch path. Wired through the
  default `specialise_factories` so existing callers get the
  cap automatically. Pipeline integration: the pass runs
  immediately after `lower_to_ir` in
  `CompilationUnit::build_for`, before the `inline_uplevel`
  pass and before `build_cfg`, so the synthesised procs appear
  in `module.procedures` for every downstream consumer. 1 unit
  test asserts the third call past `cap=2` does not synthesise.

Done: 6 fixtures + 14+ unit tests; the `tcltest` Option pattern
specialises end-to-end through the pipeline.

### C33 — `var_escape` flow-sensitive analysis

Port `core/compiler/var_escape/` (2,243 LOC across 7 files in main)
to `rust/tcl-compiler/src/var_escape/`. This is the single biggest
remaining chunk. Five strips, one per Python sub-module:

- **C33a — Types + lattice (`_types.py`).** Port `EscapeKind`,
  `EscapeReason`, `VarEscapeInfo` data types to
  `rust/tcl-compiler/src/var_escape/types.rs`. ~150 LOC + 6 unit
  tests covering display + lattice merge (LOCAL ∨ FRAME = FRAME).
- **C33b — Static rule audit (`_propagation.py`).** The
  intra-procedural rule engine: walks an `IRScript` and tags
  variables LOCAL / FRAME based on the rules listed in main commit
  `69aa16eb`'s body (upvar #0 / global / variable / dynamic-name
  set / eval-with-literal-body / eval-with-dynamic-body / uplevel
  / info exists). ~700 LOC mirroring the Python module + 25 unit
  tests.
- **C33c — `info` subcommand audit (`_info_subcommands.py`).**
  Smaller module: classifies which `info` subcommands cause a
  proc-pessimistic escape (frame, level, vars, locals) and which
  are safe (`info exists` literal). ~80 LOC + 8 unit tests.
- **C33d — Interprocedural propagation
  (`_interprocedural.py`).** Threads escape sets across call
  edges using the existing
  `interprocedural::InterproceduralAnalysis::procedures` map.
  ~250 LOC + 10 unit tests.
- **C33e — Flow-sensitive SSA-version propagation
  (`_cfg_propagation.py`).** The biggest piece (686 LOC of
  Python). Walks the CFG in reverse-postorder, tags escapes at
  `SSAValueKey = (name, version)` granularity. Wire the result
  into `FunctionUnit::var_escape` in `compilation_unit.rs`. ~700
  LOC + 20 unit tests.

Each strip is independently committable and individually green
under `make prep-pr`. Wire the analysis into
`compiler_checks::run_all_checks` only after C33e lands so
intermediate strips don't leak into Python diagnostics. The
public-API `_api.py` module ports as part of C33e.

