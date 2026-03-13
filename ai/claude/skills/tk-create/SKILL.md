---
name: tk-create
description: >
  Create Tk GUI code from a description. Generates the code with proper widget
  hierarchy, validates it with the LSP, and iterates until clean.
allowed-tools: Bash, Read, Write
---

# Tk Create

Generate Tk GUI code from a user description, validate with LSP, and iterate.

## Steps

1. Read the domain knowledge from `ai/prompts/tk_system.md`
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
4. Validate the generated code:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
5. If there are errors or warnings (especially TK1001 geometry conflicts,
   TK1002 invalid widget paths, or TK1003 unknown options), fix them and
   re-validate (up to 5 iterations)
6. Optionally extract the layout to verify widget hierarchy:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py tk-layout $FILE
   ```
7. Report the final status

$ARGUMENTS
