# KCS: feature — Runtime Validation

## Summary

Validate Tcl code against a real tclsh interpreter if available on the system.

## Surface

vscode-command

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

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
