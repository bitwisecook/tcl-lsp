# KCS: how to manage Tcl package dependencies

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I add, install, and lock Tcl package dependencies for my project?

## Answer

Use `tcl pkg` to declare dependencies in a manifest, resolve them with
the MVS resolver, and lock the exact versions in a reproducible lockfile.

## Steps

### 1. Create a manifest

```sh
tcl pkg init --name myapp --version 1.0.0
```

This writes `tclpkg.tcl` in the current directory.

### 2. Discover dependencies from source

```sh
tcl pkg discover
tcl pkg discover --add
```

The first command is read-only. The second adds requirements whose names and
minimum versions the analyser and optimiser can prove. It scans procedure and
nested-script bodies, resolves constant variables, interpolations, and safe
builtin command substitutions, and reports dynamic, conditional,
loop-contained, `-exact`, bounded, or alternative requirements for review.

### 3. Add or override dependencies explicitly

```sh
tcl pkg add json 1.0
tcl pkg add http 2.9 --source https://example.org/http.tar.gz
tcl pkg add tcltest 2.5 --dev
```

Use `add` for optional/dynamic requirements discovery cannot prove, packages
which do not appear in source, explicit source URLs, and development-only
dependencies. Each call appends a `require` (or `dev-require`) line to the
manifest.

### 4. Resolve and install

```sh
tcl pkg install
```

The resolver picks the highest version that satisfies every minimum in
the graph (Go-style MVS). Results are written to `tclpkg.lock` and
materialised into `./lib/<pkg>-<ver>/`.

### 5. Verify

```sh
tcl pkg verify
```

Re-checks SHA-256 hashes of every package against the lockfile.

### 6. Reproduce in CI

```sh
tcl pkg sync
```

Installs from the lockfile without re-resolving — refuses to change the
lock. Use this in CI and production builds.

### 7. (Optional) Use a virtual environment

```sh
tcl venv create .venv
source .venv/bin/activate
tcl pkg install          # installs into .venv/lib/
```

The venv pins a specific tclsh and isolates packages from the system.

## Related

- [tcl pkg feature page](features/kcs-feature-tcl-pkg.md)
- [tcl venv feature page](features/kcs-feature-tcl-venv.md)
- [Design: tclpkg architecture](../design/tclpkg-architecture.md)
