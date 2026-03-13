# KCS: feature — Unified Tcl Verb CLI

## Summary

`tcl.pyz` provides a single verb-based CLI for optimisation, diagnostics/linting, validation, formatting, symbol/graph extraction, iRules event metadata lookups, legacy-pattern conversion guidance, disassembly, syntax highlighting, WASM compilation, compiler exploration, and KCS help search.

## Surface

claude-code, mcp

## How to use

```sh
python tcl.pyz opt src/ -o build/optimised.tcl
python tcl.pyz diag src/ mypkg --package-path ./vendor/tcl
python tcl.pyz lint src/ mypkg --package-path ./vendor/tcl
python tcl.pyz validate src/
python tcl.pyz validate src/ --json
python tcl.pyz format script.tcl -o formatted.tcl
python tcl.pyz symbols script.tcl --json
python tcl.pyz diagram script.tcl --json
python tcl.pyz callgraph script.tcl --json
python tcl.pyz symbolgraph script.tcl --json
python tcl.pyz dataflow script.tcl --json
python tcl.pyz event-order rule.irule --dialect f5-irules --json
python tcl.pyz event-info HTTP_REQUEST --json
python tcl.pyz command-info HTTP::uri --dialect f5-irules --json
python tcl.pyz convert rule.irule --json
python tcl.pyz dis script.tcl
python tcl.pyz compwasm script.tcl -o out.wasm --wat-output out.wat
python tcl.pyz highlight script.tcl --force-colour
python tcl.pyz highlight script.tcl --format html -o out.html
python tcl.pyz diff old.irule new.irule --show ast,ir,cfg
python tcl.pyz explore script.tcl --show ir,cfg,opt
python tcl.pyz help taint analysis --dialect f5-irules
python tcl.pyz help taint --json
```

![Unified Tcl verb CLI](../../screenshots/30-tcl-verb-cli.png)

## Operational context

- Entry module: `explorer/tcl_cli.py`
- Zipapp entrypoint: `scripts/zipapp_tcl_main.py`
- Build command: `python scripts/build_zipapp.py tcl --version <v> --output <path>`
- Make target: `make zipapp-tcl`
- KCS DB prerequisite for packaging: `make kcs-db`
- Shared metadata lookups for `event-info` / `command-info` are provided by
  `core/commands/registry/info.py` and reused by CLI and AI consumers.
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
- `convert`: emits diagnostics that map to known modernisation rewrites (`--json` supported).
- `dis`: compiles resolved source and emits bytecode disassembly.
- `compwasm`: compiles resolved source to a WASM binary (`--wat-output` optional).
- `highlight`: emits syntax-highlighted output in ANSI or HTML (`--format`, `--no-colour`, `--force-colour`).
- `diff`: compares two inputs at parser AST, lowered IR, and CFG layers (`--show` and `--json` supported).
- `explore`: forwards combined source into compiler-explorer views.
- `help`: searches `core/help/kcs_help.db` and reports KCS feature matches (`--dialect` optionally narrows matches).

## Exit-code contract

- `0`: command succeeded.
- `1`: diagnostics found for `diag`/`lint`/`validate`, semantic differences for `diff`, or unknown lookup target for `event-info`/`command-info`.
- `2`: input resolution failure or command execution error.

## File-path anchors

- `explorer/tcl_cli.py`
- `core/analysis/semantic_graph.py`
- `core/commands/registry/info.py`
- `tests/test_core_lift_consumers.py`
- `scripts/zipapp_tcl_main.py`
- `scripts/build_zipapp.py`
- `Makefile`
