# Development environment

The toolchains a checkout needs, what a remote agent session gets for free,
and where every pinned version is owned. Keep this file short; the scripts it
names are the executable truth.

## Prerequisites

- **Rust** — the floating `stable` channel pinned in `rust-toolchain.toml`;
  `Cargo.toml`'s `rust-version` tracks it (1.98.0, 2026-08-18, as of this
  writing). CI resolves `stable` at run time, so a fresh release can fail
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
| Wasmtime | 48.0.1 | `/opt/wasmtime-48.0.1/`, on `PATH` as `wasmtime` |
| Binaryen | 132 | `/opt/binaryen-132/`, `wasm-opt` / `wasm-merge` on `PATH` |
| wasi-sdk | 34.0 | `/opt/wasi-sdk` (found by `runtime/rust/build.rs`) |
| Rust | floating `stable` | `/root/.rustup`, `/root/.cargo` |
| Tcl + Tk source trees | 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1b0 | `tmp/tcl<ver>/`, `tmp/tk<ver>/` |
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
| Tcl / Tk patchlevels and source tags | `rust/tcl-dialect/data/reference-toolchains.tsv` (the fetch skill and host installer consume it) |
| Wasmtime, Binaryen, wasi-sdk, tcllib (remote) | variables at the top of `.claude/hooks/session-start.sh` |
| Wasmtime, wasi-sdk, tcllib (laptop) | variables near the top of `scripts/dev/ensure-test-deps.sh` |

Changing a minimum version touches all of: `rust-toolchain.toml`, `ci.yml`,
the Makefile's Prerequisites comment block, `README.md`'s requirements, and
this file.

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
| `make rust-check` | Rust PR gate: fmt + clippy + xtask drift gates (mirrors CI `pr-gate`) |
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

- [`AGENTS.md`](../../../AGENTS.md) links here from its prerequisites line.
- [test-tiers-and-ci-gates.md](test-tiers-and-ci-gates.md) — what to run
  once the environment is up.
