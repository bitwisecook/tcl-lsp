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

## Chosen libraries and data structures

Rewrites that drag Python structure into Rust — dataclasses with
dunder methods, hand-rolled line indices when a rope already has one,
thread-local globals ported verbatim — are bad ports. To avoid that
we commit up front to a concrete set of Rust libraries and lean on
them hard. Pick these, document them here, and use them idiomatically
from day one.

- **Buffer storage (LSP layer): [`ropey`](https://crates.io/crates/ropey).**
  Rope for the document store. Standard Rust LSP choice, used by
  Helix, O(log n) edits, cheap slicing, built-in line indexing via
  `Rope::byte_to_line` / `Rope::line_to_byte`. Adopted when the LSP
  server chunks land (R*). The lexer itself does **not** take a
  `Rope` — see below.
- **LSP framework: [`tower-lsp`](https://crates.io/crates/tower-lsp).**
  Async/Tokio, modern, the standard for new Rust LSP servers.
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

Tracked here so we can't forget; every chunk may close items on this
list and should add any new ones it discovers.

- **`LineIndex::from_rope_slice`** — an adapter that wraps
  `RopeSlice`'s own line offsets so the document store can reuse the
  rope's B-tree instead of scanning a flattened `&str`. Deferred
  until the first rope-backed consumer lands (R* chunks).
- **UTF-16 column parity with Python.** Both lexers currently treat
  `character` as byte-offset-within-line. The Python implementation
  happens to be code-point-offset-within-line because Python `str` is
  code-point indexed; the two agree for ASCII and diverge for
  supplementary characters. The LSP specification says `character`
  must be UTF-16 code units. The fix is a coordinated change across
  both lexers and the `LineIndex` lookup; do it before any LSP
  handler that cares (probably alongside the semantic-tokens or
  hover chunk).
- **Rust lexer does not yet handle**: variable substitution (`$` —
  L4), command substitution (`[` — L5), braced strings (`{` — L6),
  quoted strings (`"` — L7), expansion prefix (`{*}` — L8), backslash
  escapes and line continuation (`\` — L9), dialect flags
  (`strict_quoting`, `expand_syntax`, `irules_brace_separator` — L8),
  warning collection (L9), ghost character insertion for error
  recovery (L9), `base_offset` / `base_line` / `base_col` for
  sub-lexing (L5 when command substitution gains nested lexing).
- **Ghost tokens and ghost character insertions for error recovery.**
  The Python lexer refers to these as "synthetic" tokens and
  "virtual" character insertions; we call them **ghost** tokens and
  ghost characters in Rust to avoid collisions with Rust vocabulary
  (`virtual` is a reserved keyword). The concept is the same: a
  ghost entity has no corresponding bytes in the source buffer, but
  participates in the token stream so downstream passes see a
  well-formed structure. The EOF-trailing ghost `EOL` is the one
  ghost token L3 produces; the broader error-recovery story (ghost
  `}`, ghost `]`, ghost `{` inserted at specific offsets to balance
  brace/bracket pairs) arrives with L9.
- **Performance parity.** Not measured yet; the L3 skeleton is
  correctness-first. Benchmark when the Rust lexer becomes the
  default on a real workload.

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
| L4    | Variable substitution in the Rust lexer | planned |
| L5    | Command substitution in the Rust lexer | planned |
| L6    | Brace strings in the Rust lexer | planned |
| L7    | Quoted strings in the Rust lexer | planned |
| L8    | `EXPAND` / dialect flags reshaped into `LexerConfig` | planned |
| L9    | Warnings collection and ghost-character-insertion error recovery | planned |
| L10   | `core/parsing/expr_lexer.py` → Rust | planned |
| L11   | Flip the Rust lexer to the default; keep Python fallback for one release | planned |
| L12   | Remove the pure-Python lexer | planned |
| C*    | Compiler migration | planned |
| S*    | LSP server migration | planned |
| R*    | VM, commands, analysis, formatter, minifier, CLI tooling | planned |

Keep this table current. Mark a row as `landed` in the same commit that
lands the chunk.
