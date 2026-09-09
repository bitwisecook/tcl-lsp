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

- **VS Code**: Run `Tcl: Scaffold Tcl Package Starter` to create a new Tcl package directory with boilerplate files. It asks for the package name, the initial version, and the first exported command name, then writes the package into the workspace folder. Run `Tcl: Insert package require` to add a `package require` statement for a known package.

## Operational context

The scaffold creates a standard Tcl package layout: a `pkgIndex.tcl` loader, a `src/` source file carrying the namespace, the exported command, and `package provide`, `tcltest` test files with a runner, a GitHub Actions workflow, and a README.

## Failure modes

- No workspace folder is open — the command reports this and writes nothing.
- A directory with the package name already exists — the command warns and writes nothing.

## Test anchors

- `editors/vscode/src/test/scaffold.test.ts`

## Example

Running **Tcl: Scaffold Tcl Package Starter** and entering the
package name `greet`, version `0.1.0`, and command name `hello`
creates this directory layout:

```
greet/
├── .github/
│   └── workflows/
│       └── ci.yml
├── README.md
├── pkgIndex.tcl
├── src/
│   └── greet.tcl
└── tests/
    ├── greet.test.tcl
    └── run.tcl
```

`src/greet.tcl` starts with a ready-to-edit namespace declaration:

```tcl
# greet -- generated Tcl package starter
package require Tcl 8.6

namespace eval ::greet {
    namespace export hello
}

proc ::greet::hello {name} {
    return "Hello, $name"
}

package provide greet 0.1.0
```

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
