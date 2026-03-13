# KCS: feature — Unused Variable Detection

## Summary

Detects variables that are set but never read, unused procedure parameters, and dead stores where a value is overwritten before use. Offers quick-fix code actions to remove unused assignments.

## Surface

lsp, mcp, claude-code, all-editors

## Availability

| Context | How |
|---------|-----|
| Any LSP editor | Hint-level diagnostics (faded text) on unused assignments and parameters |
| VS Code settings | Toggle per-code via `tclLsp.diagnostics.W211`, `.W214`, `.W220` |
| MCP | `analyze` / `validate` tools include W211/W214/W220 in results |
| Claude Code | `/irule-validate`, `/tcl-validate` |
| Optimiser | O126 code action removes unused variable assignments; O109 removes dead stores |

## How to use

- **Editor**: Unused variables appear as faded (hint-severity) diagnostics. Hover to see the message. Use the lightbulb quick-fix to apply O126 (remove unused variable) or O109 (remove dead store).
- **Disable diagnostics per code**: Set `tclLsp.diagnostics.W211` to `false` to suppress unused-variable hints. Same for `W214` (parameters) and `W220` (dead stores).
- **Disable the quick-fix code actions**: Set `tclLsp.optimiser.O126` to `false` to disable unused-variable removal, or `tclLsp.optimiser.O109` to `false` for dead-store removal. Disable the entire optimiser with `tclLsp.optimiser.enabled`.
- **Suppress for a single variable**: Prefix the variable name with `_` (e.g., `set _unused [expr {1+1}]`). Variables starting with `_` are excluded from W211 and O126 checks.
- **Suppress for iRules cross-event variables**: Variables that flow across `when` event boundaries (connection-scope variables) are automatically excluded — no manual suppression needed.

## Diagnostic codes

| Code | Severity | Meaning |
|------|----------|---------|
| **W211** | Hint | Variable set but never used — no version of the variable is ever read |
| **W214** | Hint | Unused proc parameter — argument declared but never read in body |
| **W220** | Hint | Dead store — variable set but overwritten before its value is read |

## Optimisation codes

| Code | Meaning |
|------|---------|
| **O126** | Remove unused variable assignments — eliminate `set` statements for variables never read anywhere |
| **O109** | Eliminate dead stores — remove `set` statements whose value is overwritten before read |

## Examples

### W211 + O126 — Variable set but never used

```tcl
proc calculate {x} {
    set temp [expr {$x * 2}]   ;# W211: Variable 'temp' is set but never used
    set result [expr {$x + 1}]
    return $result
}
```

The variable `temp` is assigned but never referenced. The **O126** quick-fix removes the unused assignment:

```tcl
proc calculate {x} {
    set result [expr {$x + 1}]
    return $result
}
```

### W211 + O126 — Multiple unused variables in iRules

```tcl
when HTTP_REQUEST {
    set debug_mode 1              ;# W211 + O126: 'debug_mode' never read
    set log_level "info"          ;# W211 + O126: 'log_level' never read
    set uri [HTTP::uri]
    pool [class match -value $uri equals uri_map]
}
```

After applying O126:

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    pool [class match -value $uri equals uri_map]
}
```

### W211 — Suppressed with underscore prefix

```tcl
proc callback {event data} {
    set _event $event           ;# No warning — underscore prefix signals intent
    puts "Received: $data"
}
```

### W214 — Unused proc parameter

```tcl
proc handler {request response} {   ;# W214: Parameter 'response' is unused
    log local0. "Got request: $request"
}
```

Fix by using the parameter, removing it, or prefixing with `_`:

```tcl
proc handler {request _response} {
    log local0. "Got request: $request"
}
```

### W220 + O109 — Dead store (value overwritten)

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]         ;# W220: Dead store — overwritten on next line
    set uri [string tolower [HTTP::uri]]
    pool [class match -value $uri equals uri_map]
}
```

The first `set uri` is immediately overwritten. The **O109** quick-fix removes it:

```tcl
when HTTP_REQUEST {
    set uri [string tolower [HTTP::uri]]
    pool [class match -value $uri equals uri_map]
}
```

### W211 — iRules cross-event variables (no false positive)

```tcl
when HTTP_REQUEST {
    set client_ip [IP::client_addr]    ;# No W211 — used in HTTP_RESPONSE
}

when HTTP_RESPONSE {
    HTTP::header insert "X-Client" $client_ip
}
```

Variables that flow between `when` events are recognised as connection-scoped and excluded automatically.

### O126 — Side-effect-safe removal only

O126 only removes assignments that have no side effects. Command substitutions that may have side effects are left in place:

```tcl
proc example {} {
    set result [some_command]   ;# NOT removed — [some_command] may have side effects
    set temp "hello"            ;# O126 removes this — pure constant assignment
}
```

## Disabling

### VS Code settings (settings.json)

```json
{
    "tclLsp.diagnostics.W211": false,
    "tclLsp.diagnostics.W214": false,
    "tclLsp.diagnostics.W220": false,
    "tclLsp.optimiser.O126": false,
    "tclLsp.optimiser.O109": false
}
```

### Any LSP editor (initializationOptions or workspace/didChangeConfiguration)

```json
{
    "tclLsp": {
        "diagnostics": {
            "W211": false,
            "W214": false,
            "W220": false
        },
        "optimiser": {
            "O126": false,
            "O109": false
        }
    }
}
```

### Disable all optimiser suggestions

```json
{
    "tclLsp.optimiser.enabled": false
}
```

## Operational context

The analysis uses the compiler's CFG/SSA intermediate representation to trace variable definitions and uses across all reachable code paths. W211 detects variables where *no* version is ever read (entirely pointless assignments). W220 detects individual assignments that are overwritten before any read (dead stores). W214 checks proc parameter names against uses in the function body. Variables starting with `_` and the `args` parameter are excluded by convention. For iRules, connection-scope analysis suppresses false positives on variables shared across `when` events.

O126 runs as a high-priority elimination pass (priority 10, higher than O109 at 8) so that unused-variable removals are preferred over dead-store removals when both apply to the same statement.

## File-path anchors

- `core/compiler/core_analyses.py` — `_unused_variables()`, `_unused_parameters()` analysis
- `core/analysis/analyser.py` — W211/W214 diagnostic emission
- `core/compiler/optimiser/_elimination.py` — O126 unused variable removal, O109 dead store elimination
- `core/compiler/optimiser/_types.py` — O126 priority in `_OPT_PRIORITY`
- `core/compiler/connection_scope.py` — iRules cross-event variable tracking

## Failure modes

- False positive on a variable used via `upvar`, `uplevel`, or trace callbacks (mitigated by IRBarrier/IRCall exclusions).
- W211 not emitted when variable appears used only in unreachable code — SCCP executable-block analysis handles this correctly.
- O126/O109 remove a `set` whose command substitution had side effects — only pure assignments are eligible for removal.

## Test anchors

- `tests/test_upstream_var.py` — comprehensive W211/W210/W220 test suite
- `tests/test_analyser.py` — W214 unused parameter tests
- `tests/test_checks.py` — W211 cross-event variable suppression
- `tests/test_optimiser.py` — O109/O126 elimination tests

## Discoverability

- [KCS feature index](README.md)
- [Diagnostics](kcs-feature-diagnostics.md)
- [Optimiser](kcs-feature-optimiser.md)
- [Code actions](kcs-feature-code-actions.md)
