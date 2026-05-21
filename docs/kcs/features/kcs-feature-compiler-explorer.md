# KCS: feature — Compiler Explorer

## Summary

Interactive web panel showing bytecode disassembly, AST, IR, and compiler passes, plus a structured WebAssembly disassembly view with click-to-source navigation, call and branch target cross-linking, control-flow arrows, and labelled structural ops.

## Applies to

VS Code

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Open in Tcl Compiler Explorer` (Ctrl+Alt+E), or right-click a Tcl file → `Tcl` → `Open in Tcl Compiler Explorer` |
| JetBrains | Right-click a Tcl/iRule file → `Open In Tcl Compiler Explorer`, or open the `Tcl Compiler Explorer` tool window |

## How to use

- **VS Code**: Open a Tcl file and run `Tcl: Open in Tcl Compiler Explorer` from the command palette or press Ctrl+Alt+E. The panel shows bytecode disassembly side-by-side with the source, and updates live as you edit.
- **JetBrains**: Right-click a Tcl/iRule file in the editor or project view and choose `Open In Tcl Compiler Explorer`, or open the `Tcl Compiler Explorer` tool window. The panel tracks the active editor and recompiles when you open or switch to a different Tcl file.

## Operational context

The compiler explorer runs the full compilation pipeline (parse, lower, optimise, codegen) and displays the output at each stage. It uses a Pyodide-powered web panel for interactive exploration.

### WASM disassembly view

The **WASM** and **WASM (Opt)** tabs show a structured per-instruction disassembly. Each instruction carries:

- The Tcl source range of the originating statement — clicking the instruction places the source cursor at the corresponding point (inside an expression, after a semicolon, or at any other nested command location).
- A source-line comment above each group of instructions sharing the same originating statement, so the reader can trace "this command compiled to these ops".
- A resolved target on `call N` (e.g. `call 22 ; ::greet`) — clicking the target label jumps both the disassembly and the source to the callee's definition.
- A resolved target on `br N` / `br_if N` (e.g. `br 0 ; loop_header foreach`) — clicking the target navigates to the matching `block` / `loop` / `if` open, along with its source range.
- A label on `block` / `loop` / `if` opens identifying the Tcl construct that produced them (`foreach`, `while`, `for`, `if`, `catch body`, `switch arm`).
- Orthogonal control-flow arrows in the left gutter showing every branch target, with forward edges drawn solid-blue and back-edges drawn dashed-yellow.

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
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
