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

### What a bad port looks like

If your port has any of these, reshape it before asking for review:

- A `#[pyclass]` in the pure crate.
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
      …                                  tokens, lexer, expr_lexer (L2+)
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
| L3    | Rust `Lexer` skeleton (EOF/SEP/EOL/COMMENT/plain ESC) + differential test harness | planned |
| L4    | Variable substitution in the Rust lexer | planned |
| L5    | Command substitution in the Rust lexer | planned |
| L6    | Brace strings in the Rust lexer | planned |
| L7    | Quoted strings in the Rust lexer | planned |
| L8    | `EXPAND` / dialect flags reshaped into `LexerConfig` | planned |
| L9    | Warnings collection and virtual-insertion error recovery | planned |
| L10   | `core/parsing/expr_lexer.py` → Rust | planned |
| L11   | Flip the Rust lexer to the default; keep Python fallback for one release | planned |
| L12   | Remove the pure-Python lexer | planned |
| C*    | Compiler migration | planned |
| S*    | LSP server migration | planned |
| R*    | VM, commands, analysis, formatter, minifier, CLI tooling | planned |

Keep this table current. Mark a row as `landed` in the same commit that
lands the chunk.
