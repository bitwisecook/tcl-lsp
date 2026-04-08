# v1.5.1

## New Features

- W125 diagnostic for orphaned control-flow keywords (`else`, `elseif`, `then`,
  `on`, `trap`, `finally`) that appear as standalone commands due to misplaced
  newlines in `if`/`try` statements.
- Isolated minification mode (`--isolated` flag / "Aggressive + Isolated" in
  VS Code) compacts global-scope variable names for self-contained scripts;
  seed maps allow consistent short names across multiple files.

## Improvements

- Corrected arity, subcommand structure, and hover documentation for dozens of
  iRules commands.
- Improved switch case-list handling in the minifier.
- Variable-writing, scope-barrier, and block-terminating command detection now
  driven from registry traits.

## Bug Fixes

- Fixed workspace/executeCommand dispatch for minifyDocument.
- Fixed selection range ordering to sort by span size.
- Fixed clock bootstrap on Tcl 8.5.
- Fixed zipapp builds to use project-managed Python.
- Tightened protocol namespace validation.
