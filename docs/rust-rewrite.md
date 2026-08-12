# Rust workspace engineering guide

The rules every change under `rust/` is measured against: the two
non-negotiable principles, the ratified library and data-structure choices, the
crate layering, and what good code looks like here. Read it before touching the
workspace. The crate-by-crate map of who owns what is in
[`design/rust/current-architecture.md`](design/rust/current-architecture.md).

The product is the Rust workspace plus the native binaries it builds (`tcl`,
`f5-query`, `tcl-lsp-server`, `tcl-mcp`) and the editor extensions that bundle
them. There is no wheel, no zipapp, and no Python in the shipping product.
`editors/zed/` is a standalone Rust crate targeting WASM, intentionally excluded
from the Cargo workspace; it is unrelated to the rest of this guide.

## Non-negotiable principles

Two architectural constraints that override local simplicity when they conflict.
If you find yourself working around one, stop and raise the design question.

### 0. C Tcl 9.0.4 is the reference standard

The lexer, compiler, VM, and runtime must produce behaviour identical to **C Tcl
9.0.4**, the pinned reference release. Every escape sequence, quoting rule,
brace-nesting edge case, and backslash-continuation behaviour is measured
against what `tclsh9.0` produces, and the C source is the reference algorithm —
`tmp/tcl9.0.4/generic/` carries the `tclParse.c` / `tclUtil.c` / `tclExecute.c`
files the implementations mirror.

Verify a behaviour against real `tclsh` 8.4–9.0 (the source trees live under
`tmp/tcl<ver>/`; build a missing one with the `fetch-tcl-source` skill plus
`configure && make` under `unix/`). Version-specific behaviour is gated on
registry / `LexerConfig` dialect flags — `0o` / `0b` integer prefixes are 8.5+,
`{*}` expansion is 8.5+, and so on — never hardcoded to one version. Tcl 9.1 is
a dialect-flag addition, not a bump of the reference standard.

### 1. Time to first semantic tokens is paramount

The single user-visible latency metric that matters is **time from
`textDocument/didOpen` to the first `textDocument/semanticTokens/full`
response** — what the user experiences as "how long until highlighting shows
up". Throughput, incremental-update latency, and memory are all subordinate to
it until time-to-first-tokens is in the single-digit milliseconds for a typical
file.

Consequences:

- **The open → first-tokens path is hot.** Anything on it gets optimised first:
  lexer, tree build, semantic-token encoding, JSON-RPC write. Anything off it
  (incremental edits, diagnostics beyond the first batch, hover, completion) is
  secondary.
- **No lazy initialisation on the hot path.** What the first response needs is
  computed eagerly on open, in parallel with response construction where
  possible.
- **No blocking I/O, no cross-process calls on the hot path.** A feature that
  needs them runs in the background and feeds results in when ready; it never
  gates the first tokens response.
- **Measure before optimising, and measure after.** Cite the numbers; a
  regression beyond run noise is a blocker, no improvement is acceptable.
  [`design/rust/lsp-performance.md`](design/rust/lsp-performance.md) lists the
  harnesses.

### 2. Async through and through

The LSP server is async-first from the protocol handler down to the analysis
pipeline. Every layer above the raw lexer is `async fn`, runs on Tokio, yields
cooperatively, and composes with cancellation: a fresh
`semanticTokens/full` request arriving while an older one is still computing
cancels the older one cleanly rather than queueing behind it.

Consequences:

- **`tower-lsp` is the LSP framework** — `async fn` handlers for every method,
  dispatched on Tokio, so cancellation composes with `tokio::select!` and the
  LSP cancellation token.
- **The lexer is synchronous but `Send`.** It is CPU-bound and fast, holds no
  thread-local or `static mut` state, and is safe to call from any async task.
  Moving a large lex into `spawn_blocking` is the caller's decision, not baked
  into the lexer.
- **Analysis passes are `async fn`** even when their bodies are CPU-bound, so
  they compose with cancellation and can yield between phases. Long phases hit
  `tokio::task::yield_now().await` at coarse checkpoints.
- **Document-store updates use `tokio::sync` primitives.** No `std::sync::Mutex`
  held across an `.await`, no blocking read locks. Read-heavy paths clone an
  `Arc` and drop the lock immediately.
- **No globals, no thread-locals, no singletons.** State lives on an owned
  struct the task holds or receives as a parameter — a flag becomes a field on a
  `Config`, never a `lazy_static` or `thread_local!`.
- **Every handler is cancellable.** Diagnostics, semantic tokens, hover,
  completion, definition, references, formatting, code actions, and inlay hints
  all propagate cancellation; no handler body blocks for more than a few µs
  without an `.await`.

## Chosen libraries and data structures

Each choice serves the two principles above.

- **Buffer storage (LSP layer): [`ropey`](https://crates.io/crates/ropey).**
  O(log n) slicing so the hot path can flatten a range into a `&str` without an
  O(n) copy; `Arc`-shareable handles so concurrent async readers do not contend;
  built-in line indexing.
- **LSP framework: [`tower-lsp`](https://crates.io/crates/tower-lsp).** Async to
  the core. `lsp-server` (rust-analyzer's) is synchronous and would need an
  async layer built on top; `async-lsp` has a smaller ecosystem.
- **Incremental engine: [`salsa`](https://crates.io/crates/salsa).** A
  demand-driven memoised query graph with exactly the cascade / invalidate /
  rebuild semantics the analysis layer needs. Lives in `tcl-lsp-db`.
- **Errors: [`thiserror`](https://crates.io/crates/thiserror)** in library
  crates, [`anyhow`](https://crates.io/crates/anyhow) in binaries.
- **CLI parsing: [`clap`](https://crates.io/crates/clap)** with `derive`.
- **Logging: [`tracing`](https://crates.io/crates/tracing)** +
  `tracing-subscriber`.

### Spans threaded through everything

The single most important invariant: **every positional entity carries a
[`Span`], not inline position data**. Tokens, IR nodes, CFG nodes, diagnostics,
refactoring ranges, semantic-token outputs — all of them. A [`Span`] is two
`u32`s (inclusive start, exclusive end, byte offsets into the source): 8 bytes,
`Copy`, no lifetime, trivially storable.

To get from a span back to anything human-readable, callers thread a
[`SourceMap`] — a `&str` source buffer bundled with a [`LineIndex`]:

```rust
let source_map = SourceMap::new(source);
let tokens = Lexer::new(source).tokenise_all()?;
for tok in &tokens {
    let text = source_map.text(tok.span);
    let (start, end) = source_map.range_positions(tok.span);
    println!("{:?} {:?} {}-{}", tok.kind, text, start.line, end.line);
}
```

This matches rust-analyzer (`TextRange`), swc (`Span`), and tree-sitter
(`Range`). Consequences of the rule:

- **Tokens have no lifetime.** A `Vec<Token>` is a plain buffer: serialisable,
  sendable, cacheable, diffable, with no lifetime bookkeeping.
- **IR / CFG nodes carry `Span`s.** Passes rewrite nodes freely; positions stay
  deferred to the `SourceMap` and are computed only at diagnostic emission or
  LSP response formatting.
- **Diagnostics carry `Span`s.** LSP `Range` values are derived on publish, not
  stored.
- **Sub-lexing inherits the parent `SourceMap`.** It never builds its own line
  index, so every span downstream lives in one coordinate system.
- **UTF-16 column conversion happens once**, inside
  [`LineIndex::position_at`] and its UTF-16 sibling, so every downstream entity
  gets correct LSP positions for free.

### Position infrastructure — lexer layer vs document layer

- **At the lexer layer** (`rust/tcl-lexer/`) the lexer consumes a `&'src str`
  and produces `Token`s carrying only a `Span`. It owns a [`SourceMap`] — a
  zero-allocation wrapper over `(source, LineIndex)` — borrowable via
  `Lexer::source_map()` or takeable via `Lexer::into_source_map()`.
  [`LineIndex`] is a `Box<[u32]>` with O(log n) `partition_point` lookups.
- **At the document layer** (`rust/tcl-lsp-server/`) the store owns the text and
  its index as shared handles, flattens the affected range into a `&str`, wraps
  it in a [`SourceMap`], and hands that to the lexer via
  `Lexer::with_source_map`. No `LineIndex` is built twice.

The seam between the two is one cheap flatten plus a shared [`SourceMap`]: the
lexer's public API is `&str`-based, the document store's is not, and neither
type leaks into the other.

[`Span`]: ../rust/tcl-lexer/src/span.rs
[`LineIndex`]: ../rust/tcl-lexer/src/line_index.rs
[`LineIndex::position_at`]: ../rust/tcl-lexer/src/line_index.rs
[`SourceMap`]: ../rust/tcl-lexer/src/source_map.rs

## Layered crates, ordered by dependency

Two kinds of crate: **pure library crates** own product behaviour and are what
the server and CLI binaries link against; **binary crates** are entry points and
depend on pure crates. The dependency direction is fixed and must not be
violated:

1. `tcl-lexer` owns source text, spans, line indexes, and tokenisation.
2. `tcl-registry` owns command, dialect, argument, taint, effect, documentation,
   and hook metadata.
3. `tcl-compiler` owns parsing above the lexer, IR, CFG, SSA, analysis,
   lowering, optimisation, and codegen algorithms.
4. `tcl-lsp-core` owns pure LSP feature providers: folding, document symbols,
   hover, completion, references, rename, semantic tokens, diagnostics
   projection, and code actions.
5. `tcl-lsp-server` owns the `tower-lsp` binary, the async document store,
   request routing, cancellation, progress, and protocol plumbing.

No LSP feature provider lands in a binary crate; feature logic belongs in
`tcl-lsp-core`, and the server wires it in.

### Command facts live in the registry

The registry is the source of truth for command facts. The compiler, analyser,
LSP providers, and diagnostics own *algorithms*, never independent command
tables. Registry-owned facts include:

- command names, aliases, dialects, arity, subcommands, option forms, and
  command forms;
- argument roles, including dynamic resolvers for count- and
  subcommand-dependent calls;
- lowering and codegen hook identifiers;
- taint sources, taint sinks, sanitiser roles, setter constraints, and
  protocol-specific sink shapes;
- side-effect summaries, variable read/write summaries, and storage effects;
- help snippets, hover text, examples, KCS links, and editor setting catalogue
  facts.

Compiler and analyser code asks the registry a precise question: "what command
form is this call?", "which argument indices are bodies?", "does this call write
a variable?", "which lowering hook applies?". Hook identifiers are part of the
registry contract — typed enums or generated constants, never bare `u16`s.

## Always shippable, small changes

Every PR leaves the extension fully working: `make prep-pr` passes, every editor
package still builds, and no existing test regresses.

One change = one logical surface. Good: "implement brace-string lexing", "add
the branch-folding optimiser pass", "port `backslash_subst`". Bad: "port the
whole lexer" (too big to review for correctness), "rewrite `tcl-compiler`",
"do the lexer and the IR together because they share a data structure" (split
the data structure out first).

Every change that replaces real logic ships with a **differential test**: feed
the same inputs through the implementation and the oracle — **C Tcl 9.0.4**
(`tclsh9.0`) for runtime and language behaviour — and assert identical output.
In-crate `*_parity.rs` harnesses are the standard shape. Do not land until the
harness is green across the whole corpus.

## What good code looks like here

The point of Rust is enums, lifetimes, iterators, `Result`, zero-copy slices,
and an ownership model that catches bugs at compile time. Code that mirrors the
shape of a reference implementation instead of using those tools has missed the
point: reshape the design, rename things, split or merge modules.

### Data structures

- **Enums for sum types.** String sentinels and type tests become `match`.
- **Structs with named fields for product types**, deriving `Clone, Copy, Debug,
  Eq, PartialEq, Hash` where appropriate.
- **Positional entities carry a `Span`**, never a start/end position pair — see
  *Spans threaded through everything*.
- **`&str` and `Cow<'_, str>`** wherever a borrow works. The caller usually owns
  the buffer; an allocation per token is waste.
- **`Option<T>` for "may be absent", `Result<T, E>` for "may fail".** No
  sentinel values, and never `Option<Option<T>>` because one `None` means
  "absent" and the other means "error".
- **`SmallVec`, `Cow`, `Arc` where they genuinely help** — not by default.

### Control flow

- **Iterators over stateful classes.** A lexer is an
  `Iterator<Item = Result<Token, LexError>>`, not an object with `get_token()`.
- **`match` over `if let` chains**, and exhaustive matches over wildcard arms
  that silently swallow future variants.
- **Flat function bodies.** Early returns are fine; deep nesting wants helpers.

### Errors

- All errors go through **`thiserror::Error`** in pure crates. No panics for
  recoverable conditions; malformed input is a `Result`, not a crash.
- Non-fatal warnings are collected onto the result value, never onto a global.

### Configuration

- Flags are **fields on a `Config` struct** passed to constructors. No
  `lazy_static`, no `thread_local!`, no module-level mutable state.

### Modules and naming

- **Split by responsibility, not by line count.** A module with one 300-line
  function is fine; a module with eight unrelated 50-line functions is not.
- **Break monster dispatch functions into per-case handlers.** The top-level
  loop should read as a dispatcher, with deferred state in an explicit struct
  rather than ten loose local maps.
- **UK spelling** (`normalise`, `optimiser`, `analyse`) in identifiers and
  comments, matching the rest of the repo.
- **Doc comments describe invariants and non-obvious decisions.** Don't
  paraphrase the code, don't add banner dividers. Every public item gets one —
  `#![deny(missing_docs)]` is on.

### Tests

- Unit tests live next to the code (`#[cfg(test)] mod tests`); integration tests
  go under the crate's `tests/` when they need multiple modules.
- Prefer assertions that state the actual invariant over golden files for
  things that are cheap to compute.

### Smells to fix before review

- An IR node, CFG node, or diagnostic storing `SourcePosition`s instead of a
  `Span`.
- A second line-index implementation. There is one `LineIndex`, owned by the
  `SourceMap`; everything else borrows it.
- A `String` field where `&'src str` would borrow the caller's buffer.
- A configuration flag translated into a `static mut` or `lazy_static`.
- A match arm reproducing a three-arm ladder verbatim when two arms share a
  body.
- An `unwrap()` on a hot path, or a panic in a parser crate for malformed input.
- A command-name table in the compiler, analyser, LSP, or diagnostics layer when
  the fact belongs in `tcl-registry`.
- A comment that says "TODO: make this idiomatic later". Do it now.

## Reference layout

Crate granularity, roughly in dependency order (`Cargo.toml` is authoritative):

```
Cargo.toml                workspace manifest        rust-toolchain.toml  channel = "stable"
rust/
  # --- shared vocabulary / host seam (leaf) ---
  tcl-core-types/         dependency-free shared vocabulary (Code, Completion, opaque handles)
  tcl-version/            the ordered Tcl release enum      tcl-dialect/  dialect sets, grammar profiles
  tcl-platform/           host-capability seam (Filesystem/Clock/Env/StdIo/Sockets/Process)
  tcl-host-native/        std-backed NativeHost (full-capability Host impl)
  tcl-cmd-core/           portable Tcl command logic (string/list/dict/…) generic over ValueOps
  tcl-runtime-api/        runtime-state contract (handles, role traits, CompileService)
  # --- lexer / syntax ---
  tcl-lexer/              position-aware lexer (Span, LineIndex, SourceMap, CST) for Tcl + dialects
  tcl-syntax/             shared parse-tree + byte-exact semantics (lists, subst, expr, format)
  # --- registry (single source of truth) ---
  tcl-registry/           command metadata: ArgRole, Arity, Traits, taint, hooks, BytePayloadSpec,
                          commands/{tcl,irules}/*.rs (one file per command)
  # --- compiler + execution ---
  tcl-bytecode/           Tcl 9 bytecode artifact types (opcodes, FunctionAsm/ModuleAsm, layout, disasm)
  tcl-compiler/           IR, lowering, CFG, SSA, dataflow (sccp/intervals/memory_ssa), type_infer,
                          shimmer, var_escape, optimiser, inlining, analyser, irules_checks, codegen/{,wasm}
  tcl-regex/              pure-Rust port of Tcl 9's Henry-Spencer ARE engine (drives both runtimes)
  tcl-vm/                 native Rust bytecode VM             tcl-vm-cli/  the `tclvm` binary
  bpf-tcl-ir/ bpf-tcl-codegen/ bpf-tcl/                       eBPF backend
  # --- F5 dialect crates ---
  tcl-bigip/              BIG-IP object model + config parser  tcl-bigip-io/    UCS archive + path resolver
  tcl-bigip-query/        BIG-IP query DSL                     tcl-irules/      object-ref extractor
  f5-xc/                  BIG-IP → F5-XC translator + XC diagnostics
  # --- LSP ---
  tcl-lsp-core/           pure LSP feature providers (folding, symbols, diagnostics, inlay_hints)
  tcl-lsp-db/             salsa incremental DB (file_analysis_incremental, semantic_tokens, lattices)
  tcl-lsp-server/         tower-lsp binary (async document store, request routing, cancellation)
  # --- tooling ---
  tcl-explorer/           compiler-explorer pipeline + serialiser (CLI/TUI/WASM consume this)
  tcl-diagram/            flow/diagram extraction              tcl-spec-studio/  registry spec studio
  tcl-cli-support/        shared CLI plumbing for the native tcl / f5 CLIs
  tcl-cli/                native `tcl` CLI (incl. `tcl explore --serve`)   f5-cli/  native `f5-query` CLI
  tcl-pkg/                package manager (manifest/resolver/lockfile/CAS/venv/docker)
  tcl-sandbox/            sandboxed execution seam             tcl-f5mku/  F5 master-key utility
  tcl-mcp/                native MCP server (the binary the Claude skills call)
  tcl-fuzz/               differential fuzzer (seeded generator + any-backend-pair harness + findings)
  tcl-irule-test/         iRule TMM-sim: SCF→orchestrator topology + LiveSession over tcl-vm
  tcl-debugger/           record-and-replay step debugger over tcl-vm + the `tcl-debug` CLI + DAP
  xtask/                  cargo-xtask build/release verbs (kcs-index-links, diag-tables, …)
runtime/
  rust/                   Rust WASM runtime (out-of-process runtime for compiled scripts)
.github/workflows/ci.yml  rust job + rust-gate (cargo tests + native lsp_e2e)
Makefile                  rust-build/test/lint/format; check-rust; test-rust
```
