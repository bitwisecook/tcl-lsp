# KCS: feature — Package Scaffolding

> **Audience:** User
> **Type:** Functionality

## Summary

Generate a Tcl package skeleton with namespace, package provide, and test stubs.

## Applies to

VS Code

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Scaffold Tcl Package Starter`, `Tcl: Insert package require` |

## How to use

- **VS Code**: Run `Tcl: Scaffold Tcl Package Starter` to create a new Tcl package directory with boilerplate files. Run `Tcl: Insert package require` to add a `package require` statement for a known package.

## Operational context

The scaffold creates a standard Tcl package layout with `pkgIndex.tcl`, a main source file with namespace and `package provide`, and optional test files.

## Failure modes

- Scaffold overwrites existing files without warning.

## Test anchors

- `editors/vscode/src/test/scaffold.test.ts`

## Example

Running **Tcl: Scaffold Tcl Package Starter** and entering the
package name `greet` and version `1.0` creates this directory
layout:

```
greet/
├── pkgIndex.tcl
├── greet.tcl
└── tests/
    └── greet.test
```

`greet.tcl` starts with a ready-to-edit namespace declaration:

```tcl
package provide greet 1.0

namespace eval ::greet {
    namespace export hello
}

proc ::greet::hello {name} {
    return "Hello, $name"
}
```

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
