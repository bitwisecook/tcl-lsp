# KCS: Python-to-Rust migration (layering, chunks, binding layer)

## Symptom

A contributor needs to port a piece of Python (lexer, compiler pass, LSP
feature) to Rust and is unsure which crate to put it in, how to keep the
Python extension working during the port, how to surface the new Rust code
back to Python, or how the overall migration is sequenced.

## Operational context

`tcl-lsp` is in the middle of an incremental bottom-up migration from Python
to Rust. The order is:

1. **Lexer** (`core/parsing/` → `rust/tcl-lexer/`).
2. **Compiler** (`core/compiler/` → future `rust/tcl-compiler/`).
3. **LSP server** (`lsp/` → future `rust/tcl-lsp-server/`).
4. **Remainder** — VM, commands, analysis, formatting, minifier, CLI,
   explorer. A Python interface is kept on top for Claude skills and MCP.

Each chunk ships as a separate PR and leaves `make prep-pr` green and every
editor extension fully working. No migration PR is allowed to break
end-user behaviour, even temporarily.

## Decision rules / contracts

1. **Two-crate pattern per domain.** For each migrated domain create a
   pure-Rust crate (no `pyo3`) plus, if Python still needs to call into
   it, a sibling PyO3 binding crate that wraps it. The pure-Rust crate
   never depends on `pyo3`. Downstream Rust consumers link against the
   pure crate directly.

2. **Python compatibility lives only in the binding layer.** If the
   current Python API demands thread-local flags, class-level mutable
   state, or any other non-idiomatic Rust construct, implement it in the
   binding crate and hide it from the pure crate. The pure crate gets
   clean `&Config` parameters or equivalent.

3. **Idiomatic Rust is mandatory; transliteration is forbidden.** The
   migration exists to benefit from enums, lifetimes, iterators, `Result`,
   and zero-copy slices. A port that preserves every Python data shape
   has missed the point. Reshape data structures, split or merge modules,
   rename things, and use Rust's ownership model even when it diverges
   from the Python layout.

4. **Small, reviewable chunks.** One chunk = one logical surface (e.g.
   "backslash escape processing", "variable substitution in the lexer",
   "brace strings"). A chunk is at most a few hundred lines of Rust plus
   the Python shim edits. Use a differential test harness to prove parity
   with the Python version on every chunk that replaces real logic.

5. **Soft dependency during rollout.** Until a chunk explicitly flips the
   default, the Python code imports the Rust wheel via a `try/except
   ImportError` and falls back to the Python implementation. A missing
   wheel is never a release-blocking regression during the migration.

6. **Workspace isolation.** `editors/zed/` is a pre-existing Rust crate
   targeting WASM and is excluded from the main Cargo workspace. Never
   fold it in; treat it as an independent artifact.

7. **Wheel hosting = GitHub release artifacts.** Rust wheels are built in
   CI on tagged releases and attached to the GitHub release. They are
   **not** published to PyPI. `scripts/build_zipapp.py` fetches them at
   packaging time. This rule is revisited only when a chunk makes Rust
   mandatory for non-editor installs.

8. **CI wheel matrix.** Release builds produce wheels for linux x86_64,
   linux aarch64, macOS x86_64, macOS arm64, and windows x86_64. PR CI
   builds only linux x86_64 and uses it to run the Python test suite.

## File-path anchors

- `Cargo.toml` — workspace manifest.
- `rust-toolchain.toml` — floating `stable` channel.
- `rust/tcl-lexer/` — pure Rust lexer crate (empty in L0, populated in L1+).
- `rust/tcl-lsp-rust/` — PyO3 binding crate producing the `tcl_lsp_rust`
  Python extension module.
- `rust/tcl-lsp-rust/pyproject.toml` — maturin build backend config.
- `scripts/build_zipapp.py` — see `_pip_install_pure` and
  `_RUST_NATIVE_PACKAGES` for the native-extension preservation rule.
- `.github/workflows/ci.yml` — `rust` job (PR-time fmt/clippy/test + wheel
  build) and `build-rust-wheels-release` job (tagged release multi-platform
  matrix).
- `Makefile` — `rust-build`, `rust-test`, `rust-lint`, `rust-format`
  targets; `$(RUST_STAMP)` chain that builds and installs the wheel.
- `tests/test_rust_bindings_smoke.py` — end-to-end bridge smoke test.

## Chunk log (completed and in progress)

| Chunk | Scope | Status |
|-------|-------|--------|
| L0    | Rust workspace bootstrap (two crates, hello-world `tcl_lsp_rust`, CI, packaging plumbing) | landed |
| L1    | `core/parsing/substitution.py::backslash_subst` → `rust/tcl-lexer/src/substitution.rs` with PyO3 bridge and Python fallback | landed |
| L2    | `core/parsing/tokens.py` → Rust enum/struct + PyO3 wrappers | planned |
| L3+   | Rust `Lexer` skeleton and incremental feature porting | planned |

## Failure modes

- A chunk tries to port too much in one PR and stalls in differential
  testing. Split further.
- A chunk leaks Python concerns into the pure Rust crate (e.g. the pure
  crate imports `pyo3` "just for convenience"). Move it back into the
  binding crate.
- A chunk replaces real logic without a differential test harness, so
  regressions ship silently. Always run the Python and Rust
  implementations in parallel on the full test corpus before flipping the
  default.
- The native extension strip in `_pip_install_pure` widens and deletes
  the tcl-lsp Rust wheel. Extend `_RUST_NATIVE_PACKAGES` rather than
  broadening the strip.

## Test anchors

- `tests/test_rust_bindings_smoke.py` — the permanent end-to-end proof
  that the build/install/import pipeline works.
- Future: a `tests/test_rust_lexer_differential.py` harness introduced in
  L3 that feeds every fixture through both the Python and Rust lexers and
  compares token streams.

## Discoverability

- [KCS index](README.md)
- [Project layout contracts](kcs-project-layout-contracts.md)
- [Lexing contracts](kcs-lexing-contracts.md)
