# KCS: feature — Compiler Explorer

> **Audience:** User
> **Type:** Functionality

## Summary

Interactive web panel showing bytecode disassembly, AST, IR, and compiler passes, plus a structured WebAssembly disassembly view with click-to-source navigation, call and branch target cross-linking, control-flow arrows, and labelled structural ops.

## Applies to

VS Code, JetBrains, tcl-lsp CLI

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Open in Tcl Compiler Explorer` (Ctrl+Alt+E), or right-click a Tcl file → `Tcl` → `Open in Tcl Compiler Explorer` |
| JetBrains | Right-click a Tcl/iRule file → `Open In Tcl Compiler Explorer`, or open the `Tcl Compiler Explorer` tool window |

## How to use

- **VS Code**: Open a Tcl file and run `Tcl: Open in Tcl Compiler Explorer` from the command palette or press Ctrl+Alt+E. The panel shows bytecode disassembly side-by-side with the source, and updates live as you edit.
- **JetBrains**: Right-click a Tcl/iRule file in the editor or project view and choose `Open In Tcl Compiler Explorer`, or open the `Tcl Compiler Explorer` tool window. The panel tracks the active editor and recompiles when you open or switch to a different Tcl file.
- **Standalone GUI** (`tcl explore --serve`, or the published web build): type in the editor pane and it recompiles automatically after a short pause. Press Ctrl+Enter (⌘+Enter on macOS) or click **Compile** in the toolbar to recompile immediately — useful after switching dialect, or to re-run a compile whose source has not changed. The dialect dropdown is filled as soon as the WebAssembly module finishes loading, before any compile has run.

If a single output tab cannot render a result, that tab shows the reason and the rest of the panel still renders — a broken pane no longer blanks the panel or leaves the compile throbber spinning.

## Operational context

The compiler explorer runs the full compilation pipeline (parse, lower, optimise, codegen) and displays the output at each stage.

The pipeline is native Rust. The browser GUI and the in-editor panels compile via a Rust → WebAssembly module (`tcl-explorer-wasm`, built by `make explorer-wasm`): the standalone GUI's `worker.js` loads the wasm module directly, and the VS Code webview / JetBrains JCEF panel bundle the `.wasm` and call `compile()` **in the webview itself** — no LSP `executeCommand` roundtrip. When the wasm module is absent (e.g. a dev build without `make explorer-wasm`), the editor panels degrade gracefully to host-brokered compilation through the LSP server. A native `tcl explore` CLI verb (`--json`, a feature-gated `--tui` ratatui shell, and `--serve` to serve the embedded GUI bundle) renders the same serialised contract.

### WASM disassembly view

The **WASM** and **WASM (Opt)** tabs show a structured per-instruction disassembly. Each instruction carries:

- The Tcl source range of the originating statement — clicking the instruction places the source cursor at the corresponding point (inside an expression, after a semicolon, or at any other nested command location).
- A source-line comment above each group of instructions sharing the same originating statement, so the reader can trace "this command compiled to these ops".
- A resolved target on `call N` (e.g. `call 22 ; ::greet`) — clicking the target label jumps both the disassembly and the source to the callee's definition.
- A resolved target on `br N` / `br_if N` (e.g. `br 0 ; loop_header foreach`) — clicking the target navigates to the matching `block` / `loop` / `if` open, along with its source range.
- A label on `block` / `loop` / `if` opens identifying the Tcl construct that produced them (`foreach`, `while`, `for`, `if`, `catch body`, `switch arm`).
- Orthogonal control-flow arrows in the left gutter showing every branch target, with forward edges drawn solid-blue and back-edges drawn dashed-yellow.

### Optimiser lens (off / on / diff)

The IR, CFG, SSA, bytecode, and WASM tabs each carry an optimiser lens with three modes:

- **off** — the program as written.
- **on** — the program after the optimiser runs.
- **diff** — a localised diff of the two.

The diff compares the *node* (an IR statement, a CFG block, a bytecode instruction), not the rendered text. Byte offsets, source ranges, statement and literal-pool indices, local-variable slots, header tallies, and the box-drawing tree/gutter glyphs all shift whenever the optimiser adds or removes a node, even when the surrounding nodes are untouched. The diff normalises those position-only tokens away so a single rewrite surfaces as a single localised change rather than flagging every following line. Operand values that carry meaning — instruction arities, increment immediates, literal text, variable names — are kept, so genuinely different nodes still differ.

The `tcl-explorer` CLI and TUI render the same offset-free diff via `--opt diff` (for example `tcl-explorer script.tcl --show ir --opt diff`). The web panel does this for the IR/CFG diff and the bytecode "Show optimiser diff" view.

## File-path anchors

- `editors/vscode/src/compilerExplorer.ts`
- `editors/vscode/src/compilerExplorerHtml.ts` (inlines `explorer-core.js` + the Rust → WASM module; in-webview `compile()` with host fallback)
- `editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/CompilerExplorerToolWindowFactory.kt` (JCEF panel; the bundled HTML compiles in-page, LSP path kept as fallback)
- `rust/tcl-explorer/` (pipeline + serialiser), `rust/tcl-explorer-wasm/` (the `wasm-bindgen` cdylib), `rust/tcl-cli/src/commands/explore.rs` + `src/tui.rs` (CLI verb + ratatui TUI)
- `rust/tcl-cli/src/commands/gui.rs` + `rust/tcl-cli/gui/` (the `tcl explore --serve` GUI bundle embedded at build time, incl. `worker.js` and `explorer-core.js`)

## Failure modes

- Web GUI / editor panels fail to compile if the Rust → WASM module is missing or fails to instantiate; the editor panels then fall back to host-brokered compilation via the LSP server.
- Stale display after compilation pipeline changes.

## Test anchors

- `rust/tcl-explorer/` and `rust/tcl-cli/` crate tests (pipeline + `explore` verb)

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
