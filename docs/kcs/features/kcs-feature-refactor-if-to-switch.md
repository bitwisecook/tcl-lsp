# KCS: feature — Refactor: if/elseif to switch

## Summary

Convert an if/elseif equality chain on a single variable into a `switch -exact` statement.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

### Editor (all editors via LSP)

Place the cursor on an `if` command where every branch tests the same variable with `eq` or `==`. Trigger code actions and choose **"Convert to switch on $var"**.

### MCP

Call the `if_to_switch` tool with `source`, `line`, and `character`.

### Claude Code

Use the `refactor` CLI command — it lists if-to-switch when the cursor is on an eligible `if`.

## Before / After

### Before

```tcl
proc handle_method {method} {
    if {$method eq "GET"} {
        set action read
    } elseif {$method eq "POST"} {
        set action create
    } elseif {$method eq "PUT"} {
        set action update
    } elseif {$method eq "DELETE"} {
        set action remove
    } else {
        set action unknown
    }
    return $action
}
```

### After

```tcl
proc handle_method {method} {
    switch -exact -- $method {
        "GET" {
            set action read
        }
        "POST" {
            set action create
        }
        "PUT" {
            set action update
        }
        "DELETE" {
            set action remove
        }
        default {
            set action unknown
        }
    }
    return $action
}
```

The if/elseif chain is collapsed into a `switch -exact` with a `default` clause for the `else` branch.

## Operational context

The refactoring parses each branch's test expression looking for `$var eq "value"` or `$var == "value"` patterns. All branches must test the same variable with an equality operator. Branches using `ne` or `!=` are rejected. An `else` clause becomes `default`.

## File-path anchors

- `core/refactoring/_if_to_switch.py`
- `lsp/features/code_actions.py`

## Failure modes

- Branches test different variables (returns `None`).
- Branch uses `ne` / `!=` (returns `None`).
- Single branch only (returns `None` — not useful as a switch).
- Complex test expressions beyond simple equality.

## Test anchors

- `tests/test_refactoring.py::TestIfToSwitch`

## Samples

- `27-if-to-switch-before.tcl` — before conversion
- `27-if-to-switch-after.tcl` — after conversion

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools overview](kcs-feature-refactorings.md)
