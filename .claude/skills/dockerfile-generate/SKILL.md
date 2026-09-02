---
name: dockerfile-generate
description: >
  Generate a Dockerfile for a Tcl project targeting a specific base image and
  Tcl version. Analyses the project (tclpkg.tcl manifest, lockfile, entry
  point, Tk use, C extensions), runs `tcl docker create`, then tailors the
  result. Use when containerising a Tcl application or building a CI image.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

# Dockerfile Generate

`tcl docker create` writes a Dockerfile that installs the requested Tcl
(8.4 / 8.5 / 8.6 / 9.0 — the OS package manager for 8.6 on Debian, Alpine,
and RHEL families, a source build otherwise), fetches the `tcl` CLI zipapp
release asset (`tcl-<ver>.pyz`, run by `python3`), runs
`tcl pkg sync --frozen` from `tclpkg.lock`, and optionally creates a venv.
This skill adds project analysis around it.

## CLI

```bash
tcl docker info                              # base-image families + Tcl versions
tcl docker recipe IMAGE --tcl-version V      # the install recipe it would use
tcl docker create IMAGE [--tcl-version 8.6] [-o Dockerfile] [--workdir /app]
    [--entrypoint main.tcl] [--venv] [--no-copy] [--no-packages]
    [--extra-package PKG]... [--label k=v]... [--env k=v]...
    [--cli-version X.Y.Z] [--force] [--json]
```

Defaults: `debian:bookworm-slim`, Tcl 8.6. An unknown image falls back to
Debian recipes — say so. Confirm the `--cli-version` asset exists on GitHub
Releases before shipping the file.

## Steps

1. **Analyse** — `tclpkg.tcl` (dependencies), `tclpkg.lock` (frozen), the
   entry point (`main.tcl`, `app.tcl`, or `entry` in the manifest), Tk usage
   (needs a display stack), C extensions (needs a compiler), any existing
   Dockerfile (prior intent).
2. **Check the recipe** — `tcl docker recipe IMAGE --tcl-version V`; prefer a
   package-manager install when one exists.
3. **Generate** — `tcl docker create` with the flags the analysis implies:
   `--no-packages` for a pure-Tcl image, `--cli-version` to pin.
4. **Tailor** — Tk: display packages via `--extra-package`; C extensions:
   build tools installed and removed in one layer; tests: consider a
   multi-stage build; `COPY tclpkg.tcl tclpkg.lock` before `COPY . .` for
   layer caching; a non-root user; health check where it makes sense.
5. **`.dockerignore`** if absent: `.git`, `.venv`, `.vscode`, `tmp/`,
   `.claude/`.
6. **Report** — files written, image and Tcl version, `docker build -t <name> .`,
   `docker run --rm <name>`, caveats and manual steps.

$ARGUMENTS
