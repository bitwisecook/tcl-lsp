# KCS: Command alias resolution

## Purpose

When `interp alias {} name {} target ?args?` creates a command alias in the
current interpreter, the LSP automatically inherits the target command's
argument semantics.  Without this, aliased commands are treated as unknown
and their arguments are not analysed for variable references, expression
bodies, or script bodies, leading to false positives such as W214 (unused
parameter).

## Supported forms

Only aliases in the current interpreter (empty source and target paths)
are tracked:

```tcl
interp alias {} = {} expr              ;# = is now an alias for expr
interp alias {} myeval {} eval         ;# myeval is now an alias for eval
interp alias {} myput {} puts stdout   ;# myput prepends "stdout" to args
```

Aliases targeting child interpreters (`interp alias child ...`) are not
tracked since they do not affect the current interpreter's namespace.

## How it works

### Shared utilities

Alias detection and resolution logic lives in `shared/alias.py`:

- `detect_interp_alias(cmd_name, args)` — parse an `interp alias` call
- `resolve_alias(cmd_name, aliases, namespace)` — namespace-aware lookup
- `expr_alias_names(aliases)` — find aliases targeting `expr`
- `lookup_alias_for_word(word, aliases)` — simple qualified-name lookup

Both the analyser and the IR lowerer call these shared functions.

### Analyser

The analyser detects `interp alias` calls during command processing and
records the mapping from alias name to (target command, prepended args).
When processing a call to an aliased command, the analyser resolves the
alias and uses the target command's argument roles for:

- **EXPR** arguments: parsed as expressions, variable references tracked
- **BODY** arguments: recursed into as Tcl scripts
- **VAR_NAME** arguments: registered as variable definitions
- **PATTERN** arguments: recorded as regex patterns

When the alias has prepended arguments, the argument indices are shifted
accordingly so that the correct arguments are matched to the correct roles.

### IR lowering

The compiler's IR lowering also detects `interp alias` calls and resolves
aliases when lowering commands.  For `expr` aliases with a single braced
argument, this produces `IRExprEval` / `IRAssignExpr` nodes with a proper
expression AST, ensuring the SSA analysis correctly tracks variable reads.

When building the virtual command for hook dispatch, synthetic tokens are
constructed for prepended arguments so that `argv`, `texts`,
`single_token_word`, and `expand_word` all have matching lengths.

This is critical for diagnostics like W214 (unused parameter) which rely
on SSA-based variable use analysis.

### `set var [alias {expr}]` pattern

The `set var [expr {$x + $y}]` pattern is special-cased in the lowering
to produce `IRAssignExpr` instead of `IRAssignValue`.  This optimisation
is extended to cover expr aliases, so `set var [= {$x + $y}]` produces
the same IR when `=` is an alias for `expr`.

## Example

```tcl
interp alias {} = {} expr

proc calculate {x y} {
    set result [= {$x + $y}]   ;# $x and $y correctly tracked as reads
    return $result
}
# No W214 warnings: both x and y are used
```

## Namespace awareness

Alias names are stored fully qualified (e.g. `::=`, `::math::=`)
because `interp alias` creates interpreter-wide commands.  Resolution
mirrors Tcl's command lookup:

1. If the call uses a qualified name (`::math::=`), it is looked up
   directly after normalisation.
2. If the call uses an unqualified name (`=`), the current namespace is
   tried first (`::math::=` inside `namespace eval math`), then the
   global namespace (`::=`).

```tcl
interp alias {} ::math::= {} expr

namespace eval math {
    proc calc {x y} {
        set r [= {$x + $y}]   ;# resolves ::math::= → expr
        return $r
    }
}
```

Global aliases also resolve from inside any namespace:

```tcl
interp alias {} = {} expr

namespace eval utils {
    proc calc {x y} {
        set r [= {$x + $y}]   ;# resolves ::= → expr (global fallback)
        return $r
    }
}
```

## LSP integration

Alias information from `AnalysisResult.command_aliases` is used by
several LSP features:

- **Hover**: shows "Alias for `target`" plus the target command's
  documentation when hovering over an aliased command name.
- **Completion**: aliases are offered as completion candidates with
  detail text showing the target command.
- **Go-to-definition**: follows the alias to the target proc's
  definition (if the target is a user-defined proc).
- **Signature help**: resolves the alias and shows the target command's
  parameter signature, adjusting the active parameter index for any
  prepended arguments.
- **Semantic tokens**: aliased commands are styled as "function" tokens
  (same as user-defined procs), which is correct since they are
  user-defined commands.

## Limitations

- Aliases must appear textually before their use in the same file
  (the analyser and lowerer process commands sequentially).
- Only `interp alias` is tracked; `rename` and runtime alias creation
  via `proc` wrappers are not detected.
- `namespace import` aliases are not tracked — this is a common Tcl
  pattern but requires cross-namespace resolution infrastructure.
- Alias chains (alias-of-alias) are not resolved transitively.
  `interp alias {} a {} b` followed by `interp alias {} b {} expr`
  means `a` targets `b`, not `expr`.
- Aliases targeting child interpreters (`interp alias child ...`) or
  from a child to the master (`interp alias $slave name {} target`)
  are not tracked since they don't affect the current interpreter's
  command table.
- The 4-argument query form (`interp alias {} name {}`) is correctly
  ignored (gated by `len(args) >= 5`).  The deletion form
  (`interp alias {} name {}`) is also ignored for the same reason.
- Dynamic alias names (e.g. `interp alias {} $var {} expr`) are stored
  with the literal `$var` string, not the resolved value.
- Dynamically loaded aliases (via `package require`, `source`,
  `auto_index`, or the `unknown` proc) are invisible to static analysis.
