# KCS: feature — Refactor: Extract Variable

## Summary

Extract a selected expression into a named variable, replacing the selection with `$var`.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

### Editor (all editors via LSP)

Select an expression in your code and trigger code actions (Ctrl+. in VS Code, `<leader>ca` in Neovim). Choose **"Extract into variable '$result'"** from the lightbulb menu.

### MCP

Call the `extract_variable` tool with `source`, `start_line`, `start_char`, `end_line`, `end_char`, and optionally `var_name`.

### Claude Code

Use the `refactor` CLI command — it lists extract-variable when a selection is available.

## Before / After

### Before

```tcl
proc greet {name} {
    puts "Hello [string totitle $name]!"
}
```

### After

```tcl
proc greet {name} {
    set title [string totitle $name]
    puts "Hello $title!"
}
```

The `[string totitle $name]` expression is extracted into `$title`, and the original site is replaced with the variable reference.

## Operational context

The refactoring inserts a `set var expr` line before the line containing the selection, then replaces the selected span with `$var`. Edits are applied bottom-to-top by offset so the insertion does not shift the replacement position.

## File-path anchors

- `core/refactoring/_extract_variable.py`
- `lsp/features/code_actions.py`

## Failure modes

- Extracting an expression with side effects changes evaluation order.
- Selection spans multiple statements (returns `None`).
- Empty or whitespace-only selection (returns `None`).

## Test anchors

- `tests/test_refactoring.py::TestExtractVariable`

## Samples

- `25-extract-variable-before.tcl` — before extraction
- `25-extract-variable-after.tcl` — after extraction

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)
