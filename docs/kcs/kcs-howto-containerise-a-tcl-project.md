# KCS: how to containerise a Tcl project

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I build a Docker image that runs my Tcl project, with the same Tcl
version and the same locked packages I develop against?

## Before you start

- The `tcl` CLI on `$PATH` (see [INSTALL-cli.md](../../INSTALL-cli.md)).
- Docker (or another OCI builder) able to reach `github.com` — the image
  downloads the `tcl` release binary during the build.
- A `tclpkg.lock` in the project, if you want packages installed for you.

## Answer

`tcl docker create` writes a Dockerfile that installs Tcl, installs the
**native `tcl` CLI binary** from a GitHub release, and runs
`tcl pkg install --frozen` against your lockfile. No Python interpreter is
installed or needed — the 1.x-era zipapp is gone.

### 1. Pick a base image and Tcl version

```sh
tcl docker info
```

Lists the Tcl versions with install recipes (8.4, 8.5, 8.6, 9.0), the
base-image families they cover, the release the CLI is pinned to by default,
and the architectures a release asset exists for.

### 2. Generate the Dockerfile

```sh
tcl docker create debian:bookworm-slim --tcl-version 8.6 --entrypoint main.tcl
```

Useful flags:

| Flag | Effect |
|---|---|
| `--venv` | create a `tcl venv` inside the image and install into it |
| `--no-packages` | skip the CLI download and `pkg install` entirely |
| `--no-copy` | omit `COPY . .` (for multi-stage builds) |
| `--cli-version 2.2.1` | pin the tcl-lsp release the CLI comes from |
| `--extra-package tk` | add an OS package |
| `-o path` / `--force` | write elsewhere / overwrite |

### 3. Build and run

```sh
docker build -t myapp .
docker run --rm myapp
```

### 4. (Optional) Repin or float the CLI version

The generated Dockerfile carries the release as a build argument, so you can
change it without regenerating:

```sh
docker build --build-arg TCL_LSP_VERSION=2.2.1 -t myapp .   # pin a release
docker build --build-arg TCL_LSP_VERSION= -t myapp .        # newest release
```

Its default is the release the `tcl` that generated the file was built from,
so an image built from a generated Dockerfile ships the CLI line you develop
with.

## How to tell it worked

The build prints the checksum verification and the version of the CLI it
installed, then the image runs Tcl:

```text
 => [5/8] RUN set -eu; ... sha256sum -c -; chmod +x /usr/local/bin/tcl; tcl --version
 #10 1.7 /usr/local/bin/tcl: OK
 #10 1.7 tcl 2.2.1+g45196df1
```

```sh
$ docker run --rm myapp tcl --version
tcl 2.2.1+g45196df1
$ docker run --rm myapp sh -c 'echo "puts [info patchlevel]" | tclsh'
8.6.13
```

`/usr/local/bin/tcl: OK` is the SHA-256 of the downloaded asset matching the
release's `SHA256SUMS`. The build fails rather than installing an asset that
is unlisted, mismatched, or unable to start.

## Alpine and other musl images

`tcl docker create alpine:…` **fails** as soon as it would install the CLI:

```text
error: the native tcl CLI has no musl release asset, so it cannot run on
alpine (its glibc shim is missing fcntl64 and __res_init). Use a glibc base
image (debian, ubuntu, fedora, rockylinux, …), or build a Tcl-only alpine
image with --no-packages and no --venv.
```

Every published Linux `tcl` asset is glibc-linked, and `gcompat` re-exports
neither `fcntl64` nor `__res_init`, so the binary dies in the dynamic loader.
Either move to a glibc base image, or keep Alpine and drop the CLI:

```sh
tcl docker create alpine:3.21 --tcl-version 8.6 --no-packages
```

That still gives a working `tclsh` image — it just cannot resolve packages or
create a venv inside the container. Vendor them instead (`tcl pkg vendor`)
and `COPY` the result in.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-howto-manage-tcl-packages.md](kcs-howto-manage-tcl-packages.md) —
  declare, resolve, and lock the dependencies this image installs.
- [tcl pkg feature page](features/kcs-feature-tcl-pkg.md)
- [tcl venv feature page](features/kcs-feature-tcl-venv.md)
- [Design: tclpkg architecture](../design/tclpkg-architecture.md)
