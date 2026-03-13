# KCS: feature — Refactor: switch to dict lookup

## Summary

Convert a `switch -exact` where every arm sets the same variable (or returns a value) into a `dict create` + `dict get`.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

### Editor (all editors via LSP)

Place the cursor on a `switch -exact` command where every arm follows `set var value` or `return value`. Trigger code actions and choose **"Convert to dict lookup"**.

### MCP

Call the `switch_to_dict` tool with `source`, `line`, and `character`.

### Claude Code

Use the `refactor` CLI command — it lists switch-to-dict when the cursor is on an eligible `switch`.

## Before / After

### Before

```tcl
proc get_color {level} {
    switch -exact -- $level {
        "error"   { set color red }
        "warning" { set color yellow }
        "info"    { set color blue }
        "debug"   { set color grey }
    }
    return $color
}
```

### After

```tcl
proc get_color {level} {
    set _map [dict create "error" red "warning" yellow "info" blue "debug" grey]
    set color [dict get $_map $level]
    return $color
}
```

The switch is replaced with a dict that maps keys to values, and a single `dict get` call performs the lookup.

## Operational context

The refactoring detects two body patterns: `set var value` (all arms set the same variable) and `return value` (all arms return). Only `-exact` mode switches are supported. A `default` arm, if present, is handled with `dict exists` + fallback. Requires at least 2 arms.

## File-path anchors

- `core/refactoring/_switch_to_dict.py`
- `lsp/features/code_actions.py`

## Failure modes

- Switch uses `-glob` or `-regexp` mode (returns `None`).
- Arms have mixed body shapes (some `set`, some `return`) (returns `None`).
- Fewer than 2 arms (returns `None`).

## Test anchors

- `tests/test_refactoring.py::TestSwitchToDict`

## Samples

- `28-switch-to-dict-before.tcl` — before conversion
- `28-switch-to-dict-after.tcl` — after conversion

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)
