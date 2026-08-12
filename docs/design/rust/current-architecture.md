# Rust workspace architecture

How the Rust workspace is laid out today: which crate owns what, the
dependency direction that must not be violated, and the runtime shape of the
native LSP server. Read this before adding a crate, a command fact, or an LSP
feature. The engineering rules that govern *how* code is written live in
[`engineering-guide.md`](engineering-guide.md); the architectural direction the
workspace is converging on is in
[`target-architecture.md`](target-architecture.md).

## Crate graph

Dependency direction is consumer → provider, and no edge may point back up.

```
                  +--------------+
                  |  tcl-lexer   |   spans, tokens, line index, source map, CST
                  +--------------+
                          ^
                          |
                  +--------------+
                  | tcl-registry |   command facts, taint metadata,
                  +--------------+   typed hook IDs, command forms
                          ^
            +-------------+------+-------------+
            |                    |             |
   +---------------+   +---------------+   +-----------------+
   | tcl-compiler  |   | tcl-lsp-core  |   | tcl-lsp-server  |
   |  IR/CFG/SSA   |   | folding,      |   | tower-lsp       |
   |  analyses,    |   | symbols,      |   | binary; holds   |
   |  codegen      |   | diagnostics   |   | the query db    |
   +---------------+   +---------------+   +-----------------+
            ^                  ^                 |
            |                  |                 v
            +---------+--------+        +-----------------+
                                        |  tcl-lsp-db     |  salsa query database
                                        +-----------------+  (incremental analysis)
```

Below the lexer sit the dependency-free vocabulary crates (`tcl-core-types`,
`tcl-version`, `tcl-dialect`) and the host seam (`tcl-platform`,
`tcl-host-native`); beside the compiler sit the execution backends (`tcl-vm`,
`tcl-bytecode`, `bpf-tcl*`, `runtime/rust`), the F5 stack (`tcl-bigip*`,
`tcl-irules`, `f5-xc`), and the tooling crates (`tcl-explorer`, `tcl-cli*`,
`f5-cli`, `tcl-pkg`, `tcl-mcp`, `tcl-fuzz`, `tcl-debugger`, `tcl-irule-test`,
`xtask`). `Cargo.toml` is the authoritative member list, and its `exclude`
block records why each excluded crate (wasm32 cdylibs, `runtime/rust`,
`editors/zed`, the maturin-built `bigip-report-gen/python` sidecar) is out of
the workspace: each needs `unsafe`, a foreign target, or a foreign toolchain,
and the workspace lints `unsafe_code = "forbid"`.

### Ownership rules

- **No command-name tables outside `tcl-registry`.** Compiler, analyser,
  diagnostics, and LSP code ask the registry "which hook?", "is this a taint
  source?", "which argument is a body?". Adding a command to the registry is the
  only place code learns a command-specific fact. A hardcoded `HashSet` of
  command names or a `match cmd.name` outside registry-owned routing is design
  debt, not a pattern to copy.
- **Typed hook IDs.** Lowering and codegen specialisation is selected by a typed
  enum (`LoweringHookId`, `CodegenHookId`) on the matched `CommandSpec` /
  `SubCommand`, and the compiler-side dispatcher matches exhaustively — a new
  variant is a deliberate compile-time error, never a silent fallthrough.
- **Feature logic lives in `tcl-lsp-core`, not in the binary.** The server crate
  wires providers together, handles protocol, cancellation, and progress; it
  does not implement features.
- **No `unsafe`.** Forbidden workspace-wide; a crate that genuinely needs it
  (the wasm32 cdylibs, `runtime/rust`) is excluded from the workspace and gated
  by its own `make` target.

### Authoritative surfaces

| Surface | Crate | Module |
|---|---|---|
| Backslash substitution, tokeniser | `tcl-lexer` | `substitution`, `lexer` / `tokens` |
| Spans / line index / source map | `tcl-lexer` | `span` / `line_index` / `source_map` |
| Byte-exact list, subst, expr, format semantics | `tcl-syntax` | `list` / `subst` / `expr` / `format` |
| Command registry & lookups | `tcl-registry` | `registry` / `commands/` |
| Typed hook IDs, command forms, taint facts | `tcl-registry` | `hooks` / `forms` / `taint` |
| Lowering / codegen hook dispatch | `tcl-compiler` | `lowering_hooks`, `codegen::emitter::bytecoded` |
| IR / CFG / SSA / dataflow | `tcl-compiler` | `ir` / `cfg` / `ssa` / `sccp` / `memory_ssa` |
| Analyser + diagnostics | `tcl-compiler` | `analyser`, `compiler_checks` |
| LSP feature providers | `tcl-lsp-core` | `folding`, `document_symbols`, `hover`, … |
| Incremental query graph | `tcl-lsp-db` | `lib` |

## LSP server runtime

The native server holds no hand-maintained analysis caches. Its runtime state
is a salsa query database plus a document store.

- **`tcl-lsp-db` (salsa 0.27) query database.** `Backend` holds an
  `Arc<Mutex<TclDatabase>>`. Inputs are the per-URI `SourceFile { text, dialect,
  path, … }`, the shared `AnalyserConfig`, and the `Project` (the workspace's
  file set). Tracked queries wrap the pure providers: `file_analysis` /
  `file_analysis_incremental`, `compiler_check_diagnostics`, `document_symbols`,
  `semantic_tokens`, `folding_ranges`, and the project-level indexes
  (`project_diagnostics`, `project_class_index`, `project_proc_var_index`,
  `project_command_arities`). The static command registry is a durable field on
  the db (read through the `TclDb` trait), not a salsa input, so queries need no
  `CommandRegistry: PartialEq`. `AnalysisResult` derives `PartialEq`, giving
  salsa's no-`unsafe` `Update` fallback early cutoff.
- **One invalidation point.** `did_open` / `did_change` set the `SourceFile`
  input; everything downstream invalidates by dependency. Reads clone the db
  handle onto a `spawn_blocking` worker and catch `salsa::Cancelled`.
- **Interned keys are garbage-collected, and that is load-bearing.** Six
  interned structs key on per-keystroke content; the invariant that keeps them
  from leaking is written up in [`salsa-interned-gc.md`](salsa-interned-gc.md).
  Read it before touching durability or interning in `tcl-lsp-db`.
- **Diagnostics are async, debounced, and tiered.** `schedule_diagnostics`
  spawns the work behind a 50 ms debounce and returns immediately, so the
  message loop is never blocked on analysis. See
  [`lsp-performance.md`](lsp-performance.md) for the tiering and the latency
  contract, and [`incremental-analysis.md`](incremental-analysis.md) for the
  per-item walk the queries sit on.
- **Document snapshots are shared, not copied.** `DocumentState` carries
  `text: Arc<str>` and an `Arc`-backed `LineIndex`, installed together as one
  build-and-swap revision, so a reader that crosses an `.await` observes a
  single consistent revision without holding a lock.

## CLI crates

The two native CLIs are thin `clap` + I/O shells over the same pure engines the
server links:

| Crate | Role |
|---|---|
| `tcl-cli` | bin `tcl` — command tree + verb dispatch (incl. `tcl explore --serve`) |
| `f5-cli` | bin `f5-query` — command tree + verb dispatch |
| `tcl-cli-support` | shared plumbing: input resolution, output writers, per-dialect registry cache, syntax highlighter, `difflib`, and the `chrome` module |
| `tcl-bigip-io` | F5 input layer: UCS archives (gzip-tar + OpenPGP-symmetric decrypt, pure Rust, all in-memory), the `read_path` / `load_paths` resolver, passphrase resolution |
| `tcl-bigip-query` | the jq-flavoured `f5 query` DSL: front-end, value model, evaluator, builtins, projection over the typed BIG-IP model, edit plans, renderers |

Where a pure core already exists (`tcl_lsp_core::{minify,formatting}`,
`tcl_compiler::optimiser`, `tcl_registry`), a verb calls it rather than
duplicating it. Engine crates stay I/O-free: typed in → typed out, with file and
stdout handling confined to the binary.

**Chrome (terminal styling) — `tcl_cli_support::chrome`.** `anstream` +
`anstyle` + `tabled`; auto-detects a TTY and honours `NO_COLOR` /
`CLICOLOR_FORCE`. It drives stderr, error messages, and decorative surfaces
**only** — never a verb's stdout. Piped output stays plain so scripted consumers
and golden tests remain byte-stable.

**Adding a verb.** Resolve inputs via `tcl_cli_support::read_input_documents`
(+ `combine_sources`), call the pure engine, format the output (field order and
separators are part of the contract; JSON via field-ordered structs), write via
`write_text_output` / `write_highlighted_output`, and add a golden test to the
matching `tests/cli_parity.rs` suite.

## Where to add a new fact

| Fact | Home |
|---|---|
| New command | `tcl-registry/src/commands/<dialect>/<name>.rs` |
| Lowering specialisation | new `LoweringHookId` variant + arm in `tcl_compiler::lowering_hooks::dispatch_lowering_hook` |
| Codegen specialisation | new `CodegenHookId` variant + arm in `tcl_compiler::codegen::emitter::bytecoded::dispatch_codegen_hook` |
| Taint source (top-level command) | stamp `Traits::TAINT_SOURCE` on the matching `CommandSpec` |
| Taint source (subcommand-shaped, e.g. `chan gets`) | stamp `Traits::TAINT_SOURCE` on the matching `SubCommand` |
| iRules option-driven check | declare the option in the registry (`OptionSpec`); the consumer reads `spec.options` |
| Side-effect summary | populate `side_effects: &[SideEffect { … }]` on the spec |

## Related

- [`engineering-guide.md`](engineering-guide.md) — the engineering rules for
  the workspace: the non-negotiable principles, library choices, and what good
  Rust looks like here.
- [`target-architecture.md`](target-architecture.md) — the zero-copy,
  single-parse, cascading, MVCC destination.
- [`incremental-analysis.md`](incremental-analysis.md) — the per-item analyser
  walk, the query graph, and the fallback contract.
- [`incremental-analysis-experiments.md`](incremental-analysis-experiments.md) —
  the corpus and the measurements behind that design.
- [`lsp-performance.md`](lsp-performance.md) — how the server hits its latency
  targets, and how to measure it.
- [`salsa-interned-gc.md`](salsa-interned-gc.md) — the interning invariant the
  edit path depends on.
