# KCS: feature — Selection Range

> **Audience:** User
> **Type:** Functionality

## Summary

Smart expand/shrink selection by syntactic structure.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Shift+Alt+Right to expand selection, Shift+Alt+Left to shrink.
- **Settings**: Toggle with `tclLsp.features.selectionRange`.

## Operational context

Selection ranges are computed from the AST, expanding from the innermost expression outward through arguments, commands, blocks, procs, and namespaces.

## Failure modes

- Selection jumps over syntactic levels after AST changes.

## Example

With the caret placed on `$x` in:

```tcl
proc square {x} {
    return [expr {$x * $x}]
}
```

Pressing Shift+Alt+Right once selects `$x`, again expands to
`$x * $x`, then to the braced expression `{$x * $x}`, then to the
full `[expr {$x * $x}]` command substitution, then to the `return`
command, then to the proc body, then to the whole `proc` command.
Shift+Alt+Left shrinks back through the same levels.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
