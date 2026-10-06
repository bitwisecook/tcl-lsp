# Development environment

The toolchains a checkout needs, what a remote agent session gets for free,
and where every pinned version is owned. Keep this file short; the scripts it
names are the executable truth.

## Prerequisites

- **Rust** — the floating `stable` channel pinned in `rust-toolchain.toml`;
  `Cargo.toml`'s `rust-version` tracks it (currently 1.99.0, released
  2026-10-01). CI resolves `stable` at run time, so a fresh release can fail
  `pr-gate`'s `cargo clippy -D warnings` on untouched code the day it lands:
  `rustup update` before debugging a clippy failure you cannot reproduce.
- **Node.js 24+** with npm for the VS Code extension. npm is pinned to v12 via
  `packageManager` in `editors/vscode/package.json`; run
  `corepack enable npm` once (bare `corepack enable` only shims pnpm/yarn).
- **Everything else** — `make install-test-deps` runs
  `scripts/dev/ensure-test-deps.sh` (tclsh 8.4–9.1 built from `tmp/`, node,
  kotlinc, Wasmtime, Binaryen, wasi-sdk, emacs, xvfb, tshark, …) on
  Debian/Ubuntu, RHEL-family, or macOS. Idempotent; `SKIP_<TOOL>=1` skips one
  tool; `--check` reports without installing. `make ensure-rust-deps` adds
  the Rust WASM targets and, on macOS, installs the pinned wasi-sdk. Stock
  Apple clang has no WebAssembly backend, so every `wasm32-unknown-unknown`
  entry point sources `scripts/dev/wasm-cc-env.sh`: it selects the owned SDK
  (or an explicit `CC_wasm32_unknown_unknown`) and compiles a tiny C object for
  the exact target before Cargo starts. An executable named `clang` that fails
  that probe is reported as missing by `ensure-test-deps.sh --check`.

## Remote agent sessions

`.claude/hooks/session-start.sh` runs only where `CLAUDE_CODE_REMOTE=true`
(a no-op on laptops) and prepares the container before the agent takes
instructions — no manual `apt install` or `curl` step is ever required. It is
idempotent; a warm container re-runs it in seconds.

| Provided | Version | Where |
|---|---|---|
| Wasmtime | 49.0.2 | `/opt/wasmtime-49.0.2/`, on `PATH` as `wasmtime` |
| Binaryen | 133 | `/opt/binaryen-133/`, `wasm-opt` / `wasm-merge` on `PATH` |
| wasi-sdk | 34.0 | `/opt/wasi-sdk` (found by `runtime/rust/build.rs`) |
| Rust | floating `stable` | `/root/.rustup`, `/root/.cargo` |
| Tcl + Tk source trees | 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 | `tmp/tcl<ver>/`, `tmp/tk<ver>/` |
| tcllib | 2.0 | `tmp/tcllib-2.0/` |
| host test tools | distro | via `ensure-test-deps.sh` |

Tcl and tcllib are full release trees (`generic/`, `tests/`, `library/`, …)
fetched as GitHub tarballs — CDN-cached, smaller than a clone, kinder to
upstream than `tcl.tk` / SourceForge on every cold session. The hook exports
`TCL_LIBRARY` at the fetched Tcl 9 script library.

## Sources of truth for pinned versions

| Pin | Owner |
|---|---|
| Rust channel | `rust-toolchain.toml`; `Cargo.toml` `rust-version` |
| Node.js minimum | `.github/workflows/ci.yml` `node-version`; `NODE_MIN_MAJOR` in `ensure-test-deps.sh` |
| Current upstream Jim oracle revision | `rust/tcl-test-support/jim-reference.txt`; build with `bash scripts/dev/ensure-jim-oracle.sh` |
| Tcl / Tk patchlevels and source tags | `rust/tcl-dialect/data/reference-toolchains.tsv` (the fetch skill, host installer, and `tcl docker` source-build layers consume it) |
| Wasmtime, Binaryen, wasi-sdk, tcllib (remote) | variables at the top of `.claude/hooks/session-start.sh` |
| Wasmtime, wasi-sdk, tcllib (laptop) | variables near the top of `scripts/dev/ensure-test-deps.sh` |
| Rust dependencies, including standalone WASM/VM/editor/Python crates | every tracked `Cargo.lock`; manifests beside each lockfile |
| npm dependencies and npm CLI | the three application `package.json` / `package-lock.json` pairs |
| Python lint/typecheck/test tooling | `Makefile` `RUFF_VERSION`, `TY_VERSION`, `PYRIGHT_VERSION`, `PYTEST_VERSION` |
| Python packaging tools | `rust/bigip-report-gen/python/deploy/*-build-requirements.in` and their hashed `.txt` locks |
| Gradle, Kotlin, IntelliJ SDK and verifier targets | `editors/jetbrains/build.gradle.kts`, `settings.gradle.kts`, `gradle/wrapper/gradle-wrapper.properties` |
| Mermaid | `Makefile` `MERMAID_VERSION`; the browser fallback in `editors/vscode/src/diagramPanelHtml.ts` must match |
| CI actions and tool binaries | SHA pins and tool versions in `.github/`; canonical report workflows under `rust/bigip-report-gen/python/deploy/` |

Dependency upgrades preserve host compatibility: `@types/vscode` follows the
supported VS Code API floor, and TypeScript stays on 6.0.x while the latest
`typescript-eslint` peer range requires `<6.1`. The WASI transport requires
the Preview 1 `wasi` 0.11 API; the 0.12+ component API is not an in-place
replacement. Browser crypto requires both the `getrandom` 0.2 `js` and 0.4
`wasm_js` backends for the dependency generations that consume them. Keep
wasm-bindgen's exact report/query pins and every CI CLI pin on the same
release when refreshing all twelve Cargo roots.

Audit every Rust root with `bash scripts/dev/cargo-deny-all.sh` and each
application lockfile with `npm audit`. An existing green test run does not
replace a fresh advisory check. Open VSX uses the same `@vscode/vsce` release
as VS Code packaging through an override: its older nested packager pulled
in the vulnerable `braces` chain (GHSA-vfj7-8cjw-p6xm).

Changing a minimum version touches all of: `rust-toolchain.toml`, `ci.yml`,
the Makefile's Prerequisites comment block, `README.md` § *Building and
contributing*, and this file.

## Resolution interpreter matrix

`bash scripts/dev/run-resolution-oracles.sh` requires every manifest-pinned C
Tcl release and the current Jim reference. It also checks compiler source rewrites against standalone file execution,
preserving expected success/error outcomes and dialect divergences. Set `TCL_LSP_TCLSH84` through
`TCL_LSP_TCLSH91` and `TCL_LSP_JIMSH`, or install their validated PATH names.
Build C references with `make ensure-tcl-deps` and Jim with
`bash scripts/dev/ensure-jim-oracle.sh`. The Jim manifest pins the inspected
upstream revision and capabilities, with explicit divergent vector expectations.
Missing interpreters never count as passed comparisons. Feature absence is an
explicit asserted vector outcome.

## Build isolation for parallel agents

Never share one `CARGO_TARGET_DIR` across concurrently-building worktrees of
this workspace. `source scripts/dev/agent-build-env.sh` pins a per-worktree
target dir with the cheap profile flags; the symptoms, the recovery, and why
sharing `CARGO_HOME` is fine are in
[the parallel-worktree KCS note](../../kcs/kcs-issue-parallel-worktree-builds-serve-stale-artefacts.md).

## Build entry points

`make help` lists every target. The ones an agent reaches for:

| Target | Purpose |
|---|---|
| `make rust-check` | Rust fast worker: fmt + Clippy/test for the workspace/default graph and report-WASM x509-only graph, plus xtask drift gates (aggregated by CI `pr-gate`) |
| `make prep-pr` | pre-push gate: format + codegen + lint/typecheck + smoke |
| `make check-all` | lint + typecheck across TypeScript, Rust, Python |
| `make smoke`, `make smoke-p P=<crate>` | the smoke tier |
| `make test` | workspace + extension + runtime port + Zed query check. CI also runs `make test-spectcl-compat` and the browser host (`make lsp-server-wasm`, then `npm run test:web` in `editors/vscode`), which have no umbrella target |
| `make codegen` | regenerate every generated file via `cargo xtask` |
| `make rust-server` / `rust-tcl` / `rust-f5` / `rust-mcp` | build one native binary |
| `make build-editor-vsix` | the VS Code package (bundles the native servers + the WASI fallback) |

The four build layers (entry points → `scripts/` helpers → CI → gated
publishing) and the publish-secret invariant are in
[release-and-publish.md](release-and-publish.md).

## File-path anchors

- `.claude/hooks/session-start.sh`, `.claude/settings.json`
- `scripts/dev/ensure-test-deps.sh`, `scripts/dev/agent-build-env.sh`,
  `scripts/dev/wasm-cc-env.sh`, `scripts/dev/tcl-reference-toolchains.sh`
- `.claude/skills/fetch-tcl-source/fetch_tcl_source.sh`
- `rust-toolchain.toml`, `Cargo.toml`, `.github/workflows/ci.yml`

## Discoverability

- [`AGENTS.md`](../../../AGENTS.md) links here from its *Environment* section.
- [test-tiers-and-ci-gates.md](test-tiers-and-ci-gates.md) — what to run
  once the environment is up.
