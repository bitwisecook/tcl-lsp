# KCS: How do I build the multi-platform VS Code extension?

> **Audience:** Maintainer
> **Type:** How-To

## Applies to

VS Code

## Question

How do I build the universal `.vsix` that bundles a native
`tcl-lsp-server` binary for every supported platform, and how do I add a
new platform?

## Before you start

- A Rust toolchain (rustup, 1.95+) and Node.js with npm installed.
- Cross-compilation dependencies for your host: run `make
  ensure-server-cross-deps` (adds the rustup targets, and on Linux the
  cross-linkers plus QEMU).

## Answer

The extension no longer ships a Python server. Instead the `.vsix` is a
single **universal** package that bundles one native `tcl-lsp-server`
binary per platform under `server/<platform>-<arch>/`, and the extension
launches the one matching the user's machine. The supported set is seven
targets: macOS, Linux, and Windows on x64 and arm64, plus Linux riscv64.

The Rust-target-to-bundle-directory map is the single source of truth in
`SERVER_TARGET_MAP` in the [`Makefile`](../../Makefile). The bundle
directory name equals Node's `process.platform-process.arch` — for
example `aarch64-apple-darwin` ships as `server/darwin-arm64/`.

### Build locally

1. Build the server for your host's targets: `make server-cross-build`
   (Linux builds the three Linux triples; macOS builds both Darwin
   triples; a Windows host builds both `win32` triples).
2. Smoke-test the binaries: `make server-cross-test` (native arches run
   directly, foreign Linux arches run under QEMU, and anything this host
   cannot run is skipped loudly).
3. Package and verify: `make package-vsix`. This stages every built
   binary into `server/<dir>/` and runs `make verify-vsix`, which fails
   if the package contains a `.pyz` or is missing a requested binary.

A local build produces a *partial*-universal `.vsix` covering only the
targets your host can build. To bundle all seven, build every target
first, then `make package-vsix BUNDLED_TARGETS="$(make -s
print-server-targets-all)"`.

### Build for release

The real release artefact is built by CI: the tag-triggered
`build-server-matrix` job compiles all seven binaries on native runners
(macOS, Ubuntu, and Windows), and `build-vsix` downloads them and
packages one universal `.vsix`. See
[`release-and-publish.md`](../design/contracts/release-and-publish.md).
No marketplace token is used in CI — publishing stays on the
maintainer's laptop via `make publish-vsix`.

### Add a new platform

1. Add one `triple:dir` entry to `SERVER_TARGET_MAP` in the `Makefile`.
2. Add the triple to the matching runner in the `build-server-matrix`
   matrix in [`ci.yml`](../../.github/workflows/ci.yml), and add any new
   cross-linker to `.cargo/config.toml`.
3. Add the triple to `scripts/test-cross-server.sh` so it is
   smoke-tested.

The extension needs no change: it derives the lookup directory from
`process.platform-process.arch` at runtime.

## How to tell it worked

`make package-vsix` ends with `==> VSIX bundles N/N native server
binaries`, and `unzip -Z1 build/*.vsix | grep -E 'server/|\.pyz'` lists
the `server/<dir>/tcl-lsp-server` entries with no `.pyz`. After
installing the `.vsix`, the **Tcl Language Server** output channel shows
`Rust mode: using native server .../server/<platform>-<arch>/tcl-lsp-server`,
and diagnostics and hovers work with no Python on the `PATH`.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Release and publish — the four-layer model](../design/contracts/release-and-publish.md)
