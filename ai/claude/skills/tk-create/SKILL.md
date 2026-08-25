---
name: tk-create
description: "Create Tk GUI code from a description with proper widget hierarchy. Generates the code, validates with the LSP analyser (including TK-specific checks), and iterates until clean. Use when creating Tk GUIs, generating Tcl/Tk code from descriptions, building Tk widget layouts, or scaffolding Tk applications."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__tk_layout, Read, Write
---

# Tk Create

Generate Tk GUI code from a user description, validate with LSP, and iterate until clean.

## Steps

1. Read the domain knowledge from `../_prompts/tk_system.md`
2. Generate Tk GUI code based on the user's description. Requirements:
   - Always start with `package require Tk`
   - Use ttk:: themed widgets where available (ttk::button, ttk::label, ttk::entry,
     ttk::combobox, ttk::treeview, ttk::notebook, ttk::progressbar, ttk::separator)
   - Use classic widgets where no ttk equivalent exists (canvas, text, listbox, menu)
   - Prefer grid geometry manager for complex layouts
   - Do not mix `pack` and `grid` in one effective `-in` container while
     propagation is enabled; `place` does not claim the container
   - Use proper widget pathname hierarchy (.parent.child)
   - Connect scrollbars with -yscrollcommand and -command options
   - Include wm title and wm geometry for the main window
   - Add event bindings where appropriate
   - Use braced expressions and braced script bodies
3. Write the generated code to a `.tcl` file
4. Validate the generated code by calling `mcp__tcl-lsp__analyze`, passing the generated code as `source`
5. If the tool fails (e.g. parse error), report the error and adjust the generated code
6. If there are errors or warnings (especially TK1001 geometry conflicts,
   TK1002 invalid widget paths, or TK1003 unknown options), fix them and
   re-validate (up to 5 iterations)
7. If validation still fails after 5 iterations, report remaining issues and explain what could not be resolved
8. Call `mcp__tcl-lsp__tk_layout` with the generated source before making a
   claim about the final widget hierarchy or layout. Treat its result as a
   static, versioned model: report certainty and uncertainty records, and do
   not fill dynamic gaps by guessing.
9. If the model is partial or unavailable, report that the layout was not
   verified. Do not execute the generated Tcl or invoke `wish` as a preview.
10. Report the final status with a summary of the widget structure that the
    structured model actually confirmed.

## Callback and resource discipline

When adding behavior, classify the callback before generating it. Account for
`after`/`after cancel`, `fileevent`, `chan event`, `trace`, `bind` and
`bindtags`, virtual events, widget command/validation prefixes, `wm protocol`,
and namespace-safe/list-built prefixes. Check event substitutions and the
callback's actual appended arguments; do not use one generic callback shape
for every option. Treat images, fonts, menus, and variable-backed options as
resources whose creation and lifetime may be dynamic. The static model may
mark these relationships uncertain; do not claim a complete resource or event
graph unless `tk_layout` explicitly provides it.

$ARGUMENTS
