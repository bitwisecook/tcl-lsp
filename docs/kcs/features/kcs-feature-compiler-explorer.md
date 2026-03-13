# KCS: feature — Compiler Explorer

## Summary

Interactive web panel showing bytecode disassembly, AST, IR, and compiler passes.

## Surface

vscode-command

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Open Compiler Explorer` (Ctrl+Alt+E) |

## How to use

- **VS Code**: Open a Tcl file and run `Tcl: Open Compiler Explorer` from the command palette or press Ctrl+Alt+E. The panel shows bytecode disassembly side-by-side with the source, and updates live as you edit.

## Operational context

The compiler explorer runs the full compilation pipeline (parse, lower, optimise, codegen) and displays the output at each stage. It uses a Pyodide-powered web panel for interactive exploration.

## File-path anchors

- `editors/vscode/src/compilerExplorer.ts`
- `editors/vscode/src/compilerExplorerHtml.ts`
- `explorer/`

## Failure modes

- Panel fails to load if Pyodide CDN is unreachable.
- Stale display after compilation pipeline changes.

## Test anchors

- `tests/test_compiler_explorer.py` (smoke tests via `make test-slow`)

## Screenshots

- `10-compiler-explorer` — bytecode disassembly panel
- `11-compiler-cfg` — control flow graph (pre-optimisation)
- `12-compiler-ssa` — CFG after SSA optimisation
- `13-compiler-optimiser` — optimiser pass output
- `14-compiler-irule` — iRule-specific IR view

![bytecode disassembly panel](../screenshots/10-compiler-explorer.png)
![control flow graph (pre-optimisation)](../screenshots/11-compiler-cfg.png)
![CFG after SSA optimisation](../screenshots/12-compiler-ssa.png)
![optimiser pass output](../screenshots/13-compiler-optimiser.png)
![iRule-specific IR view](../screenshots/14-compiler-irule.png)

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
