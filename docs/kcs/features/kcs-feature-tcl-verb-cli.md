# KCS: feature — Unified Tcl Verb CLI

> **Audience:** User
> **Type:** Functionality

## Summary

The native `tcl` binary provides a single verb-based CLI for optimisation, diagnostics/linting, validation, formatting, symbol/graph extraction, iRules event metadata lookups, legacy-pattern conversion guidance, disassembly, syntax highlighting, WASM compilation, compiler exploration, and KCS help search.

## Applies to

Claude skill, MCP

## How to use

```sh
tcl opt src/ -o build/optimised.tcl
tcl diag src/ mypkg --package-path ./vendor/tcl
tcl lint src/ mypkg --package-path ./vendor/tcl
tcl validate src/
tcl validate src/ --json
tcl format script.tcl -o formatted.tcl
tcl symbols script.tcl --json
tcl diagram script.tcl --json
tcl callgraph script.tcl --json
tcl symbolgraph script.tcl --json
tcl dataflow script.tcl --json
f5 irule event-order rule.irule --json
f5 irule event-info HTTP_REQUEST --json
tcl command-info HTTP::uri --dialect f5-irules --json
tcl find-legacy rule.irule --json
tcl dis script.tcl
tcl compwasm script.tcl -o out.wasm --wat-output out.wat
tcl compwasm script.tcl --codegen-passes native-tier -o out.wasm
tcl highlight script.tcl --force-colour
tcl highlight script.tcl --format html -o out.html
tcl diff old.irule new.irule --show ast,ir,cfg
tcl explore script.tcl --show ir,cfg,opt
tcl explore script.tcl --json --codegen-passes native-lowering,cell-demotion
tcl help taint analysis --dialect f5-irules
tcl help taint --json

# Package management and virtual environments (tclpkg)
tcl pkg init --name myapp --version 1.0.0
tcl pkg discover --add
tcl pkg install
tcl pkg list --json
tcl pkg tree
tcl pkg verify
tcl pkg info json
tcl pkg search json --json

tcl venv create .venv --tcl 8.6
tcl venv info .venv
tcl venv delete .venv
```

![Unified Tcl verb CLI](../../screenshots/30-tcl-verb-cli.png)

## Operational context

- Crate: `rust/tcl-cli` (produces the `tcl` binary); the iRules `event-order` /
  `event-info` verbs live in `rust/f5-cli` (the `f5` binary).
- Build command: `cargo build --release -p tcl-cli`
- Make target: `make rust-cli`
- The KCS help database is indexed into the binary at build time by
  `rust/tcl-cli/build.rs` (no separate `kcs-db` step).
- Shared metadata lookups for `event-info` / `command-info` are provided by the
  reconciled command registry (`tcl-registry` / `tcl-compiler`) and reused by
  CLI and AI consumers.
- Invocation name contract: when invoked as `irule` (symlink/rename), the CLI
  uses `irule` for usage/version text and defaults dialect to `f5-irules`.

## Input resolution contract

- Positional inputs may be:
  - source files (`.tcl`, `.tk`, `.itcl`, `.tm`, `.irul`, `.irule`, `.iapp`, `.iappimpl`, `.impl`, `pkgIndex.tcl`)
  - directories (recursively scanned by default)
  - package names (resolved via `pkgIndex.tcl` scanning)
- `--package-path` adds package search roots.
- `--source` can be repeated for inline source chunks.
- If no inputs are provided and stdin is piped, stdin is consumed as input.

## Verb contracts

- `opt`: combines resolved inputs, applies optimiser rewrites, and emits rewritten Tcl.
- `diag`: runs diagnostics over each resolved document and reports findings.
- `lint`: runs the same diagnostics pass as `diag` with lint-oriented naming.
- `validate`: reports error-severity diagnostics only (non-zero on any error, `--json` supported).
- `format`: reformats resolved source using the shared Tcl formatter and emits rewritten Tcl.
- `symbols`: emits symbol definitions from analyser scope data (`--json` supported).
- `diagram`: emits diagram extraction data from compiler IR (`--json` supported).
- `callgraph`: emits procedure call graph data (`--json` supported).
- `symbolgraph`: emits symbol relationship graph data (`--json` supported).
- `dataflow`: emits taint/effect data-flow graph data (`--json` supported).
- `event-order`: emits events found in source ordered by canonical iRules firing order (`--json` supported).
- `event-info`: emits iRules event metadata and valid command counts for a named event (`--json` supported).
- `command-info`: emits command registry metadata for a named command and dialect (`--json` supported).
- `find-legacy`: emits diagnostics that map to known modernisation rewrites (`--json` supported, detection only — use `opt` to apply rewrites).
- `dis`: compiles resolved source and emits bytecode disassembly.
- `compwasm`: compiles resolved source to a WASM binary (`--wat-output` optional). `--codegen-passes` selects the semantic/AOT codegen optimisation passes the emitter may use — individual pass ids, or the `native-tier` / `all` groups; omitted, no pass runs and the emitter produces the generic lowering. It is distinct from `dis --optimise`, which runs the *source-rewrite* optimiser.
- `highlight`: emits syntax-highlighted output in ANSI or HTML (`--format`, `--no-colour`, `--force-colour`).
- `diff`: compares two inputs at parser AST, lowered IR, and CFG layers (`--show` and `--json` supported).
- `explore`: forwards combined source into compiler-explorer views. `--codegen-passes` applies to the `wasm` views, and the `semanticOptimisations` view lists every pass with the state the shown module was built with.
- `help`: searches the KCS help database embedded in the binary at build time and reports KCS feature matches (`--dialect` optionally narrows matches).

## Exit-code contract

- `0`: command succeeded.
- `1`: diagnostics found for `diag`/`lint`/`validate`, semantic differences for `diff`, or unknown lookup target for `event-info`/`command-info`.
- `2`: input resolution failure or command execution error.
