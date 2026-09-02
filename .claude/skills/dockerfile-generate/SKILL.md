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
and RHEL families, a source build otherwise), installs the **native `tcl` CLI
binary** for the build's target architecture from a GitHub release, runs
`tcl pkg install --frozen` from `tclpkg.lock`, and optionally creates a venv.
No Python interpreter is installed or needed. This skill adds project analysis
around it.

The CLI layer picks its triple from BuildKit's `TARGETARCH` (falling back to
`uname -m`), verifies the downloaded `tcl-<triple>` against the release's
`SHA256SUMS`, and ends on `tcl --version` — an unlisted asset, a hash
mismatch, or a binary that cannot start fails the build. The release is an
`ARG`, so it is repinnable without regenerating:
`docker build --build-arg TCL_LSP_VERSION=2.2.1 .` (empty resolves the newest
release).

**Alpine cannot carry the CLI.** Every published Linux asset is glibc-linked
and `gcompat` re-exports neither `fcntl64` nor `__res_init`, so
`tcl docker create alpine:…` errors as soon as a CLI verb is wanted. Use a
glibc base, or `--no-packages` (and no `--venv`) for a Tcl-only Alpine image.

## CLI

```bash
tcl docker info                              # families, Tcl versions, CLI targets
tcl docker recipe IMAGE --tcl-version V      # the Tcl install recipe it would use
tcl docker recipe IMAGE --cli                # the native tcl CLI install layer
tcl docker create IMAGE [--tcl-version 8.6] [-o Dockerfile] [--workdir /app]
    [--entrypoint main.tcl] [--venv] [--no-copy] [--no-packages]
    [--extra-package PKG]... [--label k=v]... [--env k=v]...
    [--cli-version X.Y.Z] [--force] [--json]
```

Defaults: `debian:bookworm-slim`, Tcl 8.6, and the release the `tcl` running
the command was built from. An unknown image falls back to Debian recipes —
say so. Confirm the `--cli-version` asset exists on GitHub Releases before
shipping the file.

## Steps

1. **Analyse** — `tclpkg.tcl` (dependencies), `tclpkg.lock` (frozen), the
   entry point (`main.tcl`, `app.tcl`, or `entry` in the manifest), Tk usage
   (needs a display stack), C extensions (needs a compiler), any existing
   Dockerfile (prior intent).
2. **Check the recipe** — `tcl docker recipe IMAGE --tcl-version V`; prefer a
   package-manager install when one exists. `--cli` shows the CLI layer.
3. **Generate** — `tcl docker create` with the flags the analysis implies:
   `--no-packages` for a pure-Tcl image, `--cli-version` to pin.
4. **Tailor** — Tk: display packages via `--extra-package`; C extensions:
   build tools installed and removed in one layer; tests: consider a
   multi-stage build; `COPY tclpkg.tcl tclpkg.lock` before `COPY . .` for
   layer caching; a non-root user; health check where it makes sense.
5. **`.dockerignore`** if absent: `.git`, `.venv`, `.vscode`, `tmp/`,
   `.claude/`.
6. **Report** — files written, image and Tcl version, `docker build -t <name> .`,
   `docker run --rm <name>`, caveats and manual steps. The build needs network
   access to `github.com`, and to `api.github.com` when the release is unpinned.

$ARGUMENTS
