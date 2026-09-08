# Project layout contracts

Where a change belongs. This names the boundaries between the language
pipeline, the analyser, the LSP protocol surface, and the developer tools, so
a change lands in the layer that owns it.

The product is a **Rust workspace** (see
[`Cargo.toml`](../../../Cargo.toml) `[workspace] members`).  Every crate
lives under `rust/`.  Python has been fully retired on this branch — the
crate dependency direction runs from the leaf value types
up to the binaries, and is enforced by cargo's own dependency graph.

| Crate | Role |
|-------|------|
| `tcl-core-types`, `tcl-platform`, `tcl-dialect`, `tcl-version`, `tcl-userdirs` | Graph leaves — value types, platform helpers, the dialect vocabulary (`DialectSet`, `TclVersion`, `DialectProfile`), the version every binary reports, and the per-user cache / config / state directories.  Depend on nothing project-local. |
| `tcl-lexer`, `tcl-syntax` | Tcl lexer + lossless concrete syntax tree (red-green). |
| `tcl-compiler` | Tcl pipeline — IR, lowering, CFG, SSA, passes, optimiser, codegen, WASM emitter, compiler-internal analyses (taint, var-escape, interprocedural). |
| `tcl-registry` | Command / dialect registry (per-command specs, arity, subcommands, arg-roles) + canonical dialect detection. |
| `tcl-bytecode`, `tcl-runtime-api`, `tcl-cmd-core`, `tcl-vm`, `tcl-vm-cli` | Bytecode model, runtime API, per-command runtime, the bytecode VM, and its `tclvm` CLI. |
| `tcl-engine-api`, `tcl-engine-tclvm`, `tcl-cshim` | The engine-neutral Tcl extension interface, the `tcl-vm` engine behind it, and the C Tcl extension shim over it. |
| `tcl-spectcl`, `tcl-spec-hooks`, `tcl-spec-studio` | SpecTcl runtime pack support (`.tclspec` loader, pack discovery, compiled-pack cache, registry insertion), the sandboxed hook host, and the spec-studio field schema / draft model / renderers behind the web UI. |
| `tcl-regex` | Tcl regex engine port. |
| `tcl-bigip`, `tcl-bigip-io`, `tcl-bigip-query`, `tcl-f5mku`, `bigip-report-gen` | F5 BIG-IP object model + config parser, config I/O, the `f5 query` DSL engine, the `f5mku` SecureVault envelope, and the standalone HTML estate report generator. |
| `tcl-irules`, `f5-xc`, `tcl-diagram` | iRules metadata + analysis, the iRules → F5 Distributed Cloud translator, and the LSP-independent structural / control-flow diagram extractor. |
| `tcl-lsp-core`, `tcl-lsp-db` | Pure LSP feature providers (folding, symbols, diagnostics projection, …) + the LSP-side doc/registry DB. |
| `tcl-lsp-server` | Native LSP server binary (`tcl-lsp-server`), on tower-lsp. |
| `tcl-cli`, `tcl-cli-support` | The unified `tcl` CLI binary + shared CLI plumbing. |
| `f5-cli` | The `f5-query` CLI binary (F5 BIG-IP tooling). |
| `tcl-mcp` | Native MCP server binary (`tcl-mcp`) — the tool surface Claude skills / Codex call. |
| `tcl-explorer` | Compiler-explorer verbs behind `tcl explore` (and the embedded web GUI's `tcl explore --serve`). |
| `tcl-pkg`, `tcl-debugger`, `tcl-fuzz`, `tcl-irule-test`, `tcl-sandbox`, `tcl-host-native` | Tcl package manager, interactive debugger, differential fuzzer, iRule-test framework, sandbox, and native host bridge. |
| `tcl-sslictcl` | SslicTcl — declarative offline TLS configuration, certificate, trust, and assessment model. |
| `tcl-test-support` | Shared tclsh discovery and source-tree location for the conformance suites. |
| `bpf-tcl`, `bpf-tcl-ir`, `bpf-tcl-codegen` | Experimental Tcl→BPF backend. |
| `xtask` | Build / codegen / check-gate runner (`cargo xtask …`) — editor-settings and catalog generation, DiagCode-table drift checks, docs index-link validation. |

The four shipped binaries are cargo bins: `tcl` (crate `tcl-cli`),
`f5-query` (crate `f5-cli`), `tcl-lsp-server` (crate `tcl-lsp-server`),
and `tcl-mcp` (crate `tcl-mcp`).

Outside the Cargo workspace:

| Path | Role |
|------|------|
| `editors/` | Editor integrations — VS Code (TypeScript), Zed (Rust cdylib), JetBrains (Kotlin), plus Neovim / Emacs / Helix / Sublime configs. |
| `runtime/rust/` | Rust-compiled WASM runtime (crate `tcl-runtime`) the compiler's WASM codegen targets. |
| `rust/*-wasm`, `rust/tcl-lsp-server-wasi`, `rust/zed-query-check`, `rust/bigip-report-gen/{python,wasm}` | Crates named in `[workspace] exclude` — the wasm32 cdylib facades (`tcl-explorer-wasm`, `tcl-spec-studio-wasm`, `tcl-lsp-server-wasm`, `tcl-vm-wasm`, `bigip-query-wasm`), the WASI LSP transport, the tree-sitter grammar gate, and the PyO3 / WASM report builds.  Their generated glue needs `unsafe`, which the workspace `unsafe_code = "forbid"` lint bans; each has its own `make` target. |
| `samples/` | Sample Tcl, iRules, and BIG-IP configs. |
| `docs/` | Design docs, KCS notes, references, perf reports. |
| `scripts/` | Build, release, capture, and dev automation (shell). |

## Decision rules / contracts

The crate boundaries are the single source of truth, enforced by cargo's
dependency graph (a crate can only use what it declares in
`Cargo.toml`).  Summary of the intended direction:

1. **`tcl-core-types`, `tcl-platform`, `tcl-dialect`, `tcl-version` and
   `tcl-userdirs` are graph leaves.**  No dependency on any higher project
   crate — every other crate may depend on them without cycles.
2. **The compiler stack stays below the analyser-and-up stack.**
   `tcl-lexer`, `tcl-syntax`, `tcl-compiler`, and `tcl-registry` do not
   depend on `tcl-lsp-core`, `tcl-lsp-server`, or the CLI/MCP crates.
3. **The registry is data.**  Dialect spec packs live in `tcl-registry`;
   they describe commands (arity, subcommands, arg-roles) and are
   reload-safe — they do not reach into compiler internals such as
   codegen or the optimiser.
4. **LSP feature logic lives in `tcl-lsp-core`; transport lives in
   `tcl-lsp-server`.**  Feature providers are pure and reusable; the
   server crate owns the tower-lsp wiring and the `ServerCapabilities`
   advertised during `initialize`.

   One crate reaches *up* into `tcl-lsp-core` deliberately: `tcl-spectcl`,
   for `tcl_lsp_core::vfs::SourceStore` alone.  That trait is the single
   seam every closed file the server reads comes through, and `.tclspec`
   discovery is one of those readers, so a browser host that supplies bytes
   instead of a filesystem loads packs from them.  `tcl-lsp-core` is the
   lowest crate that can hold the trait *and* both implementations
   undivided — `tcl-core-types` is `#![no_std]`, and `tcl-platform` bans
   syscalls by charter and already owns the host-filesystem trait.  See
   [lsp-source-store.md](lsp-source-store.md), "Where the trait lives, and
   why".  The edge introduces no cycle, and it is the only reason
   `tcl-spectcl` may name anything in the LSP layer.
5. **Developer tools sit above the compiler, beside the server.**
   `tcl-cli`, `f5-cli`, `tcl-explorer`, `tcl-pkg`, `tcl-debugger`,
   `tcl-fuzz`, and the F5 query/XC crates consume the compiler /
   registry / VM but are not consumed by them.
6. **AI integrations sit above the LSP.**  `tcl-mcp` exposes analysis as
   MCP tools on top of the LSP-core and tooling crates; nothing below it
   depends on it.

## When you add a new module

- New parsing / analysis passes must expose stable, reusable facts.  Put
  them in the crate that matches what the code does, not where the caller
  lives (registry facts → `tcl-registry`; compiler passes →
  `tcl-compiler`; LSP feature glue → `tcl-lsp-core`).
- Editor- or transport-specific adaptation belongs in `tcl-lsp-server`
  (or the editor package), not in `tcl-lsp-core` or `tcl-compiler`.
- A new developer command (CLI verb, codemod, debugger) belongs in the
  relevant tooling crate.  A brand-new binary gets its own crate with a
  `[[bin]]` and is added to `[workspace] members`.

## When you move behaviour between crates

- Remove legacy paths in the same change; do not leave compatibility
  re-exports behind.
- Update all downstream consumers (other crates, `rust/**/tests/`) to
  import the new path directly.
- If a move introduces a new crate-to-crate edge, add the dependency to
  the consumer's `Cargo.toml` and check it does not create an
  unwanted upward edge (compiler depending on the server, registry
  depending on codegen, …).

## Anti-patterns

- A leaf crate (`tcl-core-types`) depending on a higher crate.
- A compiler crate reaching into the LSP or CLI crates for a small
  helper — lift the helper down to a leaf or duplicate it conservatively.
- A registry spec reaching into a compiler internal beyond the
  registry/parsing surface.  Spec data must be reload-safe.
- "Compatibility shim" crates or modules that exist solely so legacy
  code keeps working.  Rewrite the callers.

## File-path anchors

- [`Cargo.toml`](../../../Cargo.toml) — `[workspace] members` and the
  per-crate `[[bin]]` entries (`tcl`, `f5-query`, `tcl-lsp-server`,
  `tcl-mcp`).
- [`AGENTS.md`](../../../AGENTS.md) — the agent guide; links here for the crate map.

## Test anchors

- `rust/tcl-lsp-server/tests/e2e.rs` and its `e2e/` module tree — the native
  LSP end-to-end suite, one module per feature area plus an
  `issue<N>_*` module per pinned regression.
- `rust/tcl-lsp-server/tests/e2e/commands.rs` — the workspace
  `executeCommand` handlers, including the registry-backed ones, driven
  end-to-end against the packaged server.
- `rust/tcl-lsp-server/tests/smoke.rs` and its `smoke/` module tree — the
  per-feature smoke suites (completion, definition, hover, folding, inlay
  hints, signature help, document symbols, diagnostics delivery), sharing
  one test binary the way `e2e.rs` does.

## Discoverability

- [`AGENTS.md`](../../../AGENTS.md) — the agent guide's one-paragraph layout.
- [`shared-utility-contracts-rust.md`](shared-utility-contracts-rust.md) —
  ownership rules for the cross-cutting helpers.
- [`pipeline-lsp-first.md`](pipeline-lsp-first.md) — pipeline layering
  for LSP use cases.
