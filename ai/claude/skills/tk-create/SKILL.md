---
name: tk-create
description: "Create Tk GUI code from a description with proper widget hierarchy. Generates the code, validates with the LSP analyser (including TK-specific checks), and iterates until clean. Use when creating Tk GUIs, generating Tcl/Tk code from descriptions, building Tk widget layouts, or scaffolding Tk applications."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__tk_layout, Read, Write
---

# Tk Create

Generate Tk GUI code from a description, validate with the LSP, iterate until
clean.

## Steps

1. Read `../_prompts/tk_system.md`.
2. Generate the code: `package require Tk` first; `ttk::` widgets where they
   exist, classic canvas/text/listbox/menu otherwise; grid for anything
   non-trivial, and never `pack` and `grid` in one effective `-in` container
   while propagation is on; proper `.parent.child` paths; scrollbars wired
   with `-yscrollcommand` / `-command`; `wm title` and `wm geometry`;
   bindings where useful; braced expressions and bodies.
3. Before adding behaviour, classify each callback (`after` / `after cancel`,
   `fileevent`, `chan event`, `trace`, `bind` / `bindtags`, virtual events,
   widget command and validation prefixes, `wm protocol`) and check the
   event substitutions and appended arguments it actually receives; build
   prefixes with `list` or `namespace code`. Images, fonts, menus, and
   variable-backed options are resources with lifetimes.
4. Write the file and call `mcp__tcl-lsp__analyze` with the contents as
   `source`. Fix errors and warnings — especially TK1001 geometry conflicts,
   TK1002 invalid paths, TK1003 unknown options — and re-validate, up to 5
   iterations; then report what remains.
5. Call `mcp__tcl-lsp__tk_layout` on the final source before claiming
   anything about the hierarchy or layout: report its certainty and
   uncertainty records, never fill dynamic gaps by guessing, and never run
   the Tcl or `wish` as a preview. If the model is partial or unavailable,
   say the layout was not verified.
6. Report the final status with the widget structure the model confirmed.

$ARGUMENTS
