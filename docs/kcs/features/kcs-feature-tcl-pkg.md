# KCS: feature — tcl pkg

> **Audience:** User
> **Type:** Functionality

## Summary

Manage Tcl package dependencies with a deterministic resolver, lockfile,
and content-addressable cache.

## Applies to

all-editors, tcl-lsp CLI

## Question

What does `tcl pkg` do, and how do I use it?

## How to use

### tcl-lsp CLI

```sh
tcl pkg init                     # create a tclpkg.tcl manifest
tcl pkg discover                 # analyse source and report requirements
tcl pkg discover --add           # add safe undeclared requirements
tcl pkg add json 1.0             # add a dependency
tcl pkg install                  # resolve + fetch + lock
tcl pkg list                     # show installed packages
tcl pkg tree                     # dependency tree
tcl pkg info json                # details for one package
tcl pkg verify                   # check integrity hashes
tcl pkg search http              # search the registry
tcl pkg update                   # bump dependency minimums
tcl pkg sync                     # install from lockfile only
tcl pkg outdated                 # show upgradable packages
tcl pkg why json                 # explain why a package is needed
tcl pkg vendor                   # copy packages into vendor/
tcl pkg run                      # run the manifest entry point
tcl pkg remove json              # remove a dependency
```

`discover` recursively analyses the project beside `tclpkg.tcl`. It uses the
full Tcl analyser so requirements inside procedures, methods, namespaces, and
nested scripts are included, then applies the optimiser's constant propagation
and registry-declared pure folds to resolve names such as:

```tcl
set dependency [string cat j son]
package require $dependency 1.3
```

The default is read-only. `--add` appends only deterministic, unconditional
requirements that the minimum-version manifest can represent. Dynamic names,
dynamic versions, guarded optional requirements, `-exact`, and bounded ranges
are reported for review instead of guessed. Installed and generated trees such
as `lib/`, `vendor/`, `.venv/`, and `target/` are excluded. Add anything the
analysis cannot prove with `tcl pkg add NAME VERSION`.

### VS Code

When a `tclpkg.tcl` file is present in the workspace root, the LSP
server auto-detects the project and adds `lib/` to the library paths
for hover, completion, and diagnostics. On a missing-package diagnostic
(W120), a "Install via tclpkg" quick-fix appears alongside the existing
"Add 'package require'" action.

## Options

- `--json` — emit machine-readable JSON output (all subcommands).
- `--manifest PATH` — override `tclpkg.tcl` location.
- `--offline` — never touch the network; use cached data only.
- `--add` — append safe `discover` findings without installing or locking.
- `--no-recursive` — scan only immediate files (`discover` only).
- `--dialect NAME` — override per-file dialect detection (`discover` only).
- `--no-dev` — skip dev-require packages (install only).
- `--frozen` — refuse to change the lockfile (install only).

## Example

### Manifest (`tclpkg.tcl`)

```tcl
package     myapp
version     1.0.0
license     MIT
tcl         >=8.6

require json    1.3.5
require http    2.9.8
dev-require tcltest 2.5.5
```

### Install output

```
  ✓ http                 2.9.8
  ✓ json                 1.3.5
  ✓ tcltest              2.5.5 (dev)
  ✓ wrote tclpkg.lock
```

### Dependency tree

```
myapp
├── http 2.9.8
├── json 1.3.5
└── [dev] tcltest 2.5.5
```

## Related

- [KCS feature index](README.md)
- [tcl venv](kcs-feature-tcl-venv.md) — virtual environments
- [tcl verb CLI](kcs-feature-tcl-verb-cli.md) — the unified CLI
- [Design: tclpkg architecture](../../design/tclpkg-architecture.md)
