# KCS: feature — tcl docker

> **Audience:** User
> **Type:** Functionality

## Summary

Generate a Dockerfile that installs a chosen Tcl version and the native `tcl`
CLI, verified against the release checksums, and installs your locked packages.

## Applies to

tcl-lsp CLI

## Question

What does `tcl docker` do, and how do I use it?

## How to use

### tcl-lsp CLI

```sh
tcl docker info                          # families, Tcl versions, CLI targets
tcl docker recipe debian:bookworm-slim --tcl-version 9.0   # the Tcl install layer
tcl docker recipe debian:bookworm-slim --cli               # the tcl CLI install layer
tcl docker create debian:bookworm-slim --tcl-version 8.6   # write a Dockerfile
tcl docker create alpine:3.21 --tcl-version 8.6 --no-packages   # Tcl only
tcl docker create ubuntu:24.04 --venv --entrypoint main.tcl --force
```

`create` writes a Dockerfile that installs Tcl (the OS package manager for 8.6
on Debian, Alpine, and RHEL families, a source build otherwise), downloads the
`tcl` binary built for the image's architecture, checks it against the
release's `SHA256SUMS`, and runs `tcl pkg install --frozen` when a
`tclpkg.lock` is present. No Python interpreter is installed or needed.

### Choosing the release

The Dockerfile pins the release as a build argument, defaulting to the one the
`tcl` that wrote the file came from. Repin without regenerating:

```sh
docker build --build-arg TCL_LSP_VERSION=2.2.1 -t myapp .   # pin
docker build --build-arg TCL_LSP_VERSION= -t myapp .        # newest release
```

An unpinned build resolves the newest release the first time only: the
instruction text never changes, so a later rebuild reuses the cached layer
until something above it changes or you build with `--no-cache`.

### Alpine and other musl images

`tcl docker create alpine:…` fails as soon as it would install the CLI. Every
published Linux `tcl` asset is glibc-linked, and Alpine's `gcompat` shim
re-exports neither `fcntl64` nor `__res_init`, so the binary cannot start.
Either use a glibc base image, or pass `--no-packages` (and no `--venv`) for a
Tcl-only image. `tcl docker info` lists which families can carry the CLI.

## Options

- `--tcl-version 8.4|8.5|8.6|9.0` — the Tcl to install (default 8.6).
- `-o PATH` / `--force` — output path; overwrite an existing file.
- `--workdir DIR` — container `WORKDIR` (default `/app`).
- `--entrypoint SCRIPT` — the script `CMD` runs under `tclsh`.
- `--venv` — create a Tcl virtual environment inside the image.
- `--no-copy` — omit `COPY . .`, for multi-stage builds.
- `--no-packages` — skip the CLI download and `pkg install` entirely.
- `--extra-package PKG` — extra OS package (repeatable).
- `--label k=v`, `--env k=v` — Docker `LABEL` / `ENV` (repeatable).
- `--cli-version X.Y.Z` — pin the tcl-lsp release; empty resolves the newest.
- `--cli` — on `recipe`, print the CLI install layer instead of the Tcl one.
- `--json` — emit JSON output.

## Example

```sh
$ tcl docker create debian:bookworm-slim --tcl-version 8.6
  ✓ wrote Dockerfile
  base: debian:bookworm-slim  tcl: 8.6
  docker build -t myapp .

$ docker build -t myapp .
 => [5/8] RUN set -eu; ... sha256sum -c -; chmod +x /usr/local/bin/tcl; tcl --version
 #10 1.7 /usr/local/bin/tcl: OK
 #10 1.7 tcl 2.2.1+g45196df1

$ docker run --rm myapp tcl --version
tcl 2.2.1+g45196df1
```

`/usr/local/bin/tcl: OK` is the downloaded asset matching the release's
`SHA256SUMS`. The build fails rather than installing an asset that is
unlisted, mismatched, or unable to start.

## Related

- [tcl pkg](kcs-feature-tcl-pkg.md) — the packages this image installs.
- [tcl venv](kcs-feature-tcl-venv.md) — what `--venv` creates.
- [How to containerise a Tcl project](../kcs-howto-containerise-a-tcl-project.md)
