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
aliases when lowering commands. For `expr` aliases with a single braced
argument, this produces `IRExprEval` / `IRAssignExpr` nodes with a proper
expression AST, ensuring the SSA analysis correctly tracks variable reads.

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

## Limitations

- Aliases must appear textually before their use in the same file
  (the analyser and lowerer process commands sequentially).
- Only `interp alias` is tracked; `rename` and runtime alias creation
  via `proc` wrappers are not detected.
- Alias chains (alias-of-alias) are not resolved transitively.
