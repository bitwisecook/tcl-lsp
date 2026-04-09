# KCS: feature — Runtime Validation

## Summary

Validate Tcl code against a real tclsh interpreter if available on the system.

## Applies to

VS Code

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Run Runtime Validation` |

## How to use

- **VS Code**: Run `Tcl: Run Runtime Validation` from the command palette. Requires a `tclsh` binary on your PATH. The command runs the code through tclsh and reports any runtime errors.

## Operational context

Runtime validation complements static analysis by executing the code in a real Tcl interpreter. This catches issues that static analysis cannot detect, such as runtime type errors or missing packages.

## File-path anchors

- `editors/vscode/src/runtimeValidation.ts`

## Failure modes

- tclsh not found on PATH.
- Script has side effects when executed.

## Test anchors

- `editors/vscode/src/test/runtimeValidation.test.ts`

## Example

Running **Tcl: Run Runtime Validation** on this script:

```tcl
package require Tcl 8.6
set items {one two three}
foreach item $itmes {
    puts $item
}
```

The VS Code output channel shows the tclsh error:

```
can't read "itmes": no such variable
    while executing
"foreach item $itmes { ... }"
```

The static analyser would not have caught this without
`-Wunresolved`; runtime validation confirms the failure.

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
