# KCS: feature — Unused Variable Detection

> **Audience:** User
> **Type:** Functionality

## Summary

Detects variables that are set but never read, unused procedure parameters, and dead stores where a value is overwritten before use. Offers quick-fix code actions to remove unused assignments.

## Applies to

all-editors, MCP, Claude skill, warning

## Availability

| Context | How |
|---------|-----|
| Any LSP editor | Hint-level diagnostics (faded text) on unused assignments and parameters |
| VS Code settings | Toggle per-code via `tclLsp.diagnostics.W211`, `.W214`, `.W220`; change how prominent each is via `tclLsp.diagnosticSeverity.W211` (etc.) |
| MCP | `analyze` / `validate` tools include W211/W214/W220 in results |
| Claude Code | `/irule-validate`, `/tcl-validate` |
| Optimiser | O126 code action removes unused variable assignments; O109 removes dead stores |

## How to use

- **Editor**: Unused variables appear as faded (hint-severity) diagnostics. Hover to see the message. Use the lightbulb quick-fix to apply O126 (remove unused variable) or O109 (remove dead store).
- **Make them more prominent**: The default hint severity renders as a faint underline (the "three dots" some users find hard to spot). Raise it per code with `tclLsp.diagnosticSeverity.W211` set to `"warning"`, `"error"`, `"information"`, or `"hint"` (the default). This changes only how the editor displays the diagnostic — the analysis is unchanged. Any diagnostic code can be re-levelled the same way (`tclLsp.diagnosticSeverity.<CODE>`).
- **Disable diagnostics per code**: Set `tclLsp.diagnostics.W211` to `false` to suppress unused-variable hints. Same for `W214` (parameters) and `W220` (dead stores).
- **Disable the quick-fix code actions**: Set `tclLsp.optimiser.O126` to `false` to disable unused-variable removal, or `tclLsp.optimiser.O109` to `false` for dead-store removal. Disable the entire optimiser with `tclLsp.optimiser.enabled`.
- **Suppress for a single variable**: Prefix the variable name with `_` (e.g., `set _unused [expr {1+1}]`). Variables starting with `_` are excluded from W211 and O126 checks.
- **Suppress for iRules cross-event variables**: Variables that flow across `when` event boundaries (connection-scope variables) are automatically excluded — no manual suppression needed.
- **Interpreter special variables are exempt**: A write to a runtime-consumed special variable (`set auto_path …`, `set env(X) …`, `set tcl_precision …`) is never flagged W211/W220, because the interpreter reads it even when your script does not. See [Special variable recognition](kcs-feature-special-variables.md).

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

## Tricky scopes are recognised, not flagged

A variable can be *used* through a Tcl surface that single-file dataflow does
not see in the same frame. These are recognised, so they never draw a false
unused / read-before-set hint:

- **`upvar` pass-by-reference** — `upvar 1 $varName local` (dynamic target) and
  `upvar 1 caller local` (literal) are treated identically: reading the alias is
  not read-before-set, and writing through it is not a dead store. This is the
  standard accessor idiom (`upvar 1 $arrayName arr; return $arr($key)`) and no
  longer fires (issue #941).
- **Cross-scope globals** — a global assigned at the top level (`set cfg 1`) and
  read only inside a proc (`global cfg; … $cfg`, or `$::cfg`) is used, not a
  dead store; the reverse (a proc writing a global a top-level read consumes) is
  suppressed too.
- **Namespaces, `variable`, TclOO, traces, `eval`, `scan`/`regexp` out-vars,
  `dict with`** — a namespace `variable` shared across procs, a TclOO instance
  `variable` / `my variable`, a write-traced variable, a variable read inside an
  inlined `eval` body, and names written by an out-var command are all counted
  as uses.

## Failure modes

- W211 not emitted when a variable appears used only in unreachable code — SCCP executable-block analysis handles this correctly.
- O126/O109 remove a `set` whose command substitution had side effects — only pure assignments are eligible for removal.
- `uplevel 0 {…}` (evaluate in the *current* frame) is treated as a separate frame, so a variable used only inside its body may still be flagged. Rare; tracked as a residual (`uplevel #0` and `uplevel 1` correctly denote a different frame).

## Discoverability

- [KCS feature index](README.md)
- [Diagnostics](kcs-feature-diagnostics.md)
- [Optimiser](kcs-feature-optimiser.md)
- [Code actions](kcs-feature-code-actions.md)
