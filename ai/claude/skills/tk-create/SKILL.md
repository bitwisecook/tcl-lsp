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
   - Never mix pack and grid in the same parent container
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
8. Optionally extract the layout to verify widget hierarchy by calling `mcp__tcl-lsp__tk_layout`, passing the generated code as `source`
9. Report the final status with a summary of the generated widget structure

$ARGUMENTS
