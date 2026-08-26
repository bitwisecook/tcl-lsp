# KCS: How do I build the multi-platform VS Code extension?

> **Audience:** Maintainer
> **Type:** How-To

## Applies to

VS Code

## Question

How do I build the VSIX packages that cover every supported platform, and
how do I add a new platform?

## Before you start

- A Rust toolchain (rustup, 1.95+) and Node.js with npm installed.
- Cross-compilation dependencies for your host: run `make
  ensure-server-cross-deps` (adds the rustup targets, and on Linux the
  cross-linkers plus QEMU).

## Answer

VS Code gets **seven** `.vsix` packages, built from one native
`tcl-lsp-server` binary per platform:

- **Six platform-targeted packages** — `win32-x64`, `win32-arm64`,
  `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64` — each bundles
  only its own platform's binary under `server/<platform>-<arch>/`, built
  with `vsce package --target <platform>`. The Marketplace serves each
  client only the package matching its OS and architecture.
- **One untargeted "universal" package** bundles all seven binaries (the
  six above plus `linux-riscv64`), built with no `--target`. The
  Marketplace falls back to this package for any client with no dedicated
  targeted package — namely riscv64 Linux, since vsce has no `--target`
  string for it — and it also serves as the package for a manual
  "Install from VSIX" side-load.

Either way, the extension launches whichever `server/<platform>-<arch>/`
binary matches the user's machine; it needs no code to know which kind of
package it shipped in.

The universal package carries one thing the six targeted packages do not:
the language server as a WebAssembly module, at
`server/wasm/tcl-lsp-server-wasi.wasm` with its spec packs beside it. That
is the extension's last rung, taken only when no native binary matches at
all, so a targeted package — which by definition ships the binary its
client needs — would carry about 19 MiB nobody could ever run.
`make verify-vsix` asserts the split in both directions: the universal
package must contain it, a targeted package must not.

The Rust-target-to-bundle-directory map is the single source of truth in
`SERVER_TARGET_MAP` in the [`Makefile`](../../Makefile). The bundle
directory name equals Node's `process.platform-process.arch` — for
example `aarch64-apple-darwin` ships as `server/darwin-arm64/` — and, for
six of the seven, is also a valid vsce `--target` string (`VSCE_TARGETS`,
also in the `Makefile`).

### Build locally

1. Build the server for your host's targets: `make server-cross-build`
   (Linux builds the three Linux triples; macOS builds both Darwin
   triples; a Windows host builds both `win32` triples).
2. Smoke-test the binaries: `make server-cross-test` (native arches run
   directly, foreign Linux arches run under QEMU, and anything this host
   cannot run is skipped loudly).
3. Package and verify the universal package: `make package-vsix`. This
   stages every built binary into `server/<dir>/`, stages the WebAssembly
   module into `server/wasm/` (building it with `make lsp-server-wasi`
   only if `rust/tcl-lsp-server-wasi/dist/` has none), and runs `make
   verify-vsix`, which fails if the package contains a `.pyz`, is missing
   a requested binary, or is missing the module.
4. Package and verify the six platform-targeted packages: `make
   package-vsix-targets`. Each needs the corresponding native binary
   already built (step 1); `SERVER_TARGET_MAP` resolves each platform
   name to its triple.

A local `make package-vsix` produces a *partial*-universal `.vsix`
covering only the targets your host can build. To bundle all seven,
build every target first, then `make package-vsix
BUNDLED_TARGETS="$(make -s print-server-targets-all)"`.

### Build for release

The real release artefacts are built by CI: the tag-triggered
`build-server-matrix` job compiles all seven binaries on native runners
(macOS, Ubuntu, and Windows), and `build-vsix` downloads them, then runs
both `make package-vsix BUNDLED_TARGETS="$(make -s
print-server-targets-all)"` (the universal package) and `make
package-vsix-targets` (the six targeted packages). See
[`release-and-publish.md`](../design/contracts/release-and-publish.md).
All seven publish to the VS Code Marketplace from CI's
`publish-vsix-marketplace` job (`secrets.VSCE_PAT` on the protected
`marketplace-vscode` Environment); `make publish-vsix
publish-vsix-targets` remain laptop fallbacks for when that job fails.

### Add a new platform

1. Add one `triple:dir` entry to `SERVER_TARGET_MAP` in the `Makefile`.
2. If vsce supports a `--target` string for the new platform, add it to
   `VSCE_TARGETS` too (also in the `Makefile`) so it gets its own
   targeted package; otherwise it is covered only by the universal
   fallback package, like riscv64 Linux today.
3. Add the triple to the matching runner in the `build-server-matrix`
   matrix in [`ci.yml`](../../.github/workflows/ci.yml), and add any new
   cross-linker to `.cargo/config.toml`.
4. Add the triple to `scripts/test-cross-server.sh` so it is
   smoke-tested.

The extension needs no change: it derives the lookup directory from
`process.platform-process.arch` at runtime, regardless of which package
it was installed from.

## How to tell it worked

`make package-vsix` and each iteration of `make package-vsix-targets` end
with `==> VSIX bundles N/N native server binaries`, followed by either
`==> VSIX carries the WASI language server (server/wasm)` for the
universal package or `==> VSIX is platform-targeted (<platform>) and
correctly omits server/wasm/` for a targeted one. `unzip -Z1
build/tcl-lsp-vscode-*-universal.vsix | grep -E 'server/|\.pyz'` lists
the `server/<dir>/tcl-lsp-server` entries plus `server/wasm/`, with no
`.pyz` (swap in a `*-<platform>.vsix` filename to check a targeted
package — it should list exactly one `server/<dir>/` entry and no
`server/wasm/`). After installing any of the seven `.vsix` files, the
**Tcl Language Server** output channel shows `Using native
tcl-lsp-server: .../server/<platform>-<arch>/tcl-lsp-server`, and
diagnostics and hovers work with no Python on the `PATH`.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Release and publish — the four-layer model](../design/contracts/release-and-publish.md)
