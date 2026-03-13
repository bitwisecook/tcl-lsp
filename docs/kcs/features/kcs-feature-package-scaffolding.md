# KCS: feature — Package Scaffolding

## Summary

Generate a Tcl package skeleton with namespace, package provide, and test stubs.

## Surface

vscode-command

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Scaffold Tcl Package Starter`, `Tcl: Insert package require` |

## How to use

- **VS Code**: Run `Tcl: Scaffold Tcl Package Starter` to create a new Tcl package directory with boilerplate files. Run `Tcl: Insert package require` to add a `package require` statement for a known package.

## Operational context

The scaffold creates a standard Tcl package layout with `pkgIndex.tcl`, a main source file with namespace and `package provide`, and optional test files.

## File-path anchors

- `editors/vscode/src/scaffold.ts`

## Failure modes

- Scaffold overwrites existing files without warning.

## Test anchors

- `editors/vscode/src/test/scaffold.test.ts`

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
