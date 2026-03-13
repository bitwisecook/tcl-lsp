# KCS: feature — Refactor: Inline Variable

## Summary

Inline a single-use `set var value` — replace the one reference with the value and remove the set command.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

### Editor (all editors via LSP)

Place the cursor on a `set var value` command that has exactly one usage site. Trigger code actions (Ctrl+. in VS Code) and choose **"Inline variable '$var'"**.

### MCP

Call the `inline_variable` tool with `source`, `line`, and `character`.

### Claude Code

Use the `refactor` CLI command — it lists inline-variable when the cursor is on an eligible `set`.

## Before / After

### Before

```tcl
proc fetch {url} {
    set timeout 30
    set result [http::geturl $url -timeout $timeout]
    return $result
}
```

### After

```tcl
proc fetch {url} {
    set result [http::geturl $url -timeout 30]
    return $result
}
```

The single-use `$timeout` variable is inlined to its value `30`, and the `set timeout 30` line is removed.

## Operational context

The refactoring uses the semantic model to count references. It only fires when there is exactly one read site (excluding the definition itself). The raw source text of the value token is preserved — including quotes and braces — so the inlined value is syntactically identical to the original.

## File-path anchors

- `core/refactoring/_inline_variable.py`
- `lsp/features/code_actions.py`

## Failure modes

- Variable used more than once (returns `None` — not offered).
- Variable read via `[set var]` form (returns `None` — too complex to inline safely).
- Value expression has side effects that should only execute once.

## Test anchors

- `tests/test_refactoring.py::TestInlineVariable`

## Samples

- `26-inline-variable-before.tcl` — before inlining
- `26-inline-variable-after.tcl` — after inlining

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)
