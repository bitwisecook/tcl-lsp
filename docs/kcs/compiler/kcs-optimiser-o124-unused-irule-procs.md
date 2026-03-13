# KCS: O124 — Comment out unused procs in iRules

## Summary

O124 detects procs defined in an iRule that are never called (directly or
transitively) from any event handler, and suggests commenting them out.

## Operational context

iRules often accumulate dead proc definitions over time — procs that were
once needed but are no longer called after refactoring.  These procs still
consume TMM memory and add maintenance burden.  O124 builds a call graph
from event handlers through `call` indirection and flags unreachable procs.

### When it fires

- Dialect is `f5-irules`.
- The iRule has at least one non-`RULE_INIT` event (e.g. `HTTP_REQUEST`,
  `CLIENT_ACCEPTED`).
- A `proc` definition is not reachable from any event handler via the
  transitive call graph.

### When it does NOT fire

- **Plain Tcl** — only applies to `f5-irules` dialect.
- **Library iRules** — iRules with only procs and at most a `RULE_INIT`
  event (used for setting static variables) are treated as libraries and
  excluded.  These are designed to be `call`-ed from other iRules.
- **Procs only, no events** — also treated as a library.
- **Procs called from `RULE_INIT`** — still considered used since
  `RULE_INIT` is a real event that executes the proc.
- **Dynamic dispatch present** — if any event handler or transitively
  reachable proc contains a dynamic barrier (`eval`, `uplevel`, etc.) or
  has unknown call targets, O124 is suppressed entirely.  The dynamic
  code could invoke any proc at runtime.

## Examples

### Unused proc — fires O124

```tcl
proc helper {x} {
    return [expr {$x + 1}]
}

when HTTP_REQUEST {
    pool my_pool
}
```

`helper` is defined but never called from `HTTP_REQUEST`.  O124 suggests
replacing the proc with:

```tcl
# [O124] Unused proc — 'helper' is not called from any event
# proc helper {x} {
#     return [expr {$x + 1}]
# }

when HTTP_REQUEST {
    pool my_pool
}
```

### Used proc — no diagnostic

```tcl
proc helper {} {
    return 1
}

when HTTP_REQUEST {
    set val [call helper]
}
```

`helper` is called from `HTTP_REQUEST` via `call helper`, so no O124.

### Transitively used proc — no diagnostic

```tcl
proc inner {} {
    return 42
}

proc outer {} {
    return [call inner]
}

when HTTP_REQUEST {
    set val [call outer]
}
```

`outer` is called from `HTTP_REQUEST`, and `inner` is called from `outer`.
Both are reachable — no O124 for either.

### Library iRule — excluded

```tcl
proc utility_a {x} {
    return [string toupper $x]
}

proc utility_b {x y} {
    return "$x:$y"
}

when RULE_INIT {
    set ::debug 0
}
```

This iRule has only procs and `RULE_INIT`.  It looks like a library
intended to be `call`-ed from other iRules.  O124 does not fire.

### Multiple unused procs

```tcl
proc used_helper {} {
    return 1
}

proc dead_code_a {} {
    return 2
}

proc dead_code_b {} {
    return 3
}

when HTTP_REQUEST {
    set val [call used_helper]
}
```

O124 fires for `dead_code_a` and `dead_code_b` but not `used_helper`.

## Disabling O124

O124 can be disabled per-editor or via LSP settings:

| Editor | Setting |
|--------|---------|
| VS Code | `tclLsp.optimiser.O124` → `false` |
| JetBrains | Settings → Tcl LSP → Optimiser → uncheck O124 |
| Any LSP client | Send `workspace/didChangeConfiguration` with `{"optimiser": {"O124": false}}` |
| Inline | `# noqa: O124` comment on the proc line |

The master optimiser toggle (`tclLsp.optimiser.enabled` / `optimiser.enabled`)
also controls O124.

## Algorithm

1. Separate IR module procedures into event handlers (`::when::*`) and
   user procs.
2. Collect event names; if all are `RULE_INIT` (or none exist), classify
   as library and skip.
3. Build call graph from `InterproceduralAnalysis.procedures[*].calls`.
4. BFS/DFS from all event handler procs to find reachable user procs.
5. **Dynamic dispatch guard**: if any reachable proc has `has_barrier`
   (eval, uplevel, etc.) or `has_unknown_calls`, bail out — dynamic
   dispatch could target any proc so we cannot safely flag them unused.
6. For each unreachable user proc, emit an `Optimisation` that replaces
   the proc text with a commented-out version.

## File-path anchors

- `core/compiler/optimiser/_unused_procs.py` — pass implementation
- `core/compiler/optimiser/_manager.py` — wired as module-level pass
- `core/compiler/optimiser/_types.py` — O124 priority (10, highest)
- `core/compiler/interprocedural.py` — call graph used by the pass
- `lsp/server.py` — O124 in `_ALL_OPTIMISATION_CODES`
- `editors/vscode/package.json` — VS Code toggle
- `editors/jetbrains/.../TclLspSettings.kt` — JetBrains toggle

## Test anchors

- `tests/test_optimiser.py::TestUnusedIruleProcs` — 17 tests
- `tests/test_optimiser_coverage.py::TestO124UnusedIruleProcs` — 13 tests

## Failure modes

- False positive on a library iRule that has a non-RULE_INIT event for
  housekeeping (e.g. `CLIENT_ACCEPTED` that only logs).  The proc may be
  `call`-ed from other iRules — user should disable O124 for that file.
- False negative when `eval`/`uplevel` is present in the reachable call
  graph: O124 conservatively suppresses all suggestions even if the
  dynamic dispatch does not actually target any of the unused procs.

## Discoverability

- [Compiler KCS index](README.md)
- [Optimiser feature KCS](../features/kcs-feature-optimiser.md)
- [Pass fact ownership matrix](kcs-pass-fact-ownership-matrix.md)
