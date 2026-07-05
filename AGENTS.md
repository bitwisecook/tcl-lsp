# AGENTS.md — development guide for AI agents

## Project overview

tcl-lsp is a Tcl Language Server Protocol implementation. The LSP server and
all tooling are native Rust; editor integrations are in TypeScript (VS Code),
Rust (Zed), and Gradle/Kotlin (JetBrains), plus Neovim/Emacs/Helix/Sublime
configs. It supports Tcl 8.4–9.1, F5 iRules/iApps, and EDA tool dialects.

Python has been fully retired on this branch — the product is the Rust
workspace (`rust/`) plus the native binaries it builds.

## Repository layout

The product is a Cargo workspace: ~37 crates under `rust/` (the authoritative
list is `[workspace] members` in the top-level `Cargo.toml`), plus editor
integrations under `editors/` and the Zig WASM runtime under `runtime/zig/`.
The native binaries are the cargo bins `tcl-lsp-server`, `tcl`, `f5-query`,
and `tcl-mcp`.

Key crates under `rust/` and their one-line roles:

```
tcl-lexer / tcl-syntax    Lexer + concrete/red-green syntax tree (CST).
tcl-compiler              IR / CFG / SSA / optimiser / codegen (WASM emitter).
tcl-registry              Command + dialect registry; dialect detection.
tcl-vm / tcl-vm-cli       Bytecode VM + its CLI.
tcl-bigip / tcl-bigip-query   BIG-IP model + the f5-query engine.
tcl-irules                iRules dialect model + analysis.
tcl-lsp-core              LSP feature providers (hover, completion, …).
tcl-lsp-server            Native LSP binary (`tcl-lsp-server`).
tcl-cli                   The `tcl` CLI binary.
f5-cli                    The `f5-query` CLI binary.
tcl-mcp                   MCP server binary (`tcl-mcp`).
tcl-explorer / -wasm      Compiler explorer + its embedded (Rust→WASM) web GUI.
tcl-pkg                   Tcl package manager.
tcl-debugger              Interactive debugger.
tcl-fuzz                  Differential fuzzer.
f5-xc                     iRules → F5 XC translation.
xtask                     Build / codegen / drift-check gate runner.
```

Supporting crates (core types, platform, host, regex, bytecode, runtime-api,
sandbox, LSP db, bpf-tcl-*, irule-test, …) are also listed in `Cargo.toml`.

```
editors/          Editor integrations (VS Code, Zed, JetBrains,
                  Neovim, Emacs, Helix, Sublime).
runtime/zig/      Zig-compiled WASM runtime that the compiler's WASM
                  codegen targets.
runtime/rust/     Rust port of the runtime (leak round-trip + eval suite).
scripts/          Build, release, codegen, and dev automation.
samples/          Sample Tcl, iRules, and BigIP configs.
docs/             Design docs, KCS notes, references, perf reports.
```

The native LSP end-to-end suite lives at
`rust/tcl-lsp-server/tests/*_e2e.rs` (30 suites, run by `cargo test`).

## Prerequisites

- Rust 1.95+ with cargo, via [rustup](https://rustup.rs/) (the Makefile
  Prerequisites block and the `cargo`-missing errors pin 1.95+; the toolchain
  tracks the floating `stable` channel)
- Zig 0.16.0 (for the WASM runtime under `runtime/zig/`)
- Node.js 24+ with npm (for the VS Code TypeScript extension; the npm CLI is
  pinned to v12 via the
  `packageManager` field in `editors/vscode/package.json`; run
  `corepack enable npm` once so `npm` resolves to the pinned version —
  the bare `corepack enable` only shims pnpm/yarn)

### Claude Code on the web — pre-installed toolchains and sources

The SessionStart hook in [`.claude/hooks/session-start.sh`](.claude/hooks/session-start.sh)
prepares remote sessions (containers where `CLAUDE_CODE_REMOTE=true`) with
the language toolchains and Tcl source trees the repo needs. It runs on
local machines as a no-op, so laptops are never touched. Everything listed
here is ready before Claude starts taking instructions — **no manual
`apt install` or curl step is required**.

After the toolchains land it also installs the remaining `test-slow` host
tools via [`scripts/dev/ensure-test-deps.sh`](scripts/dev/ensure-test-deps.sh)
(tclsh, node, kotlinc, emacs, xvfb, tshark, …).

| Tool / source    | Version       | Install path                    | On `PATH` as              |
|------------------|---------------|---------------------------------|---------------------------|
| rsync, xz-utils  | distro        | `/usr/bin/`                     | `rsync`, `xz`             |
| Zig              | 0.16.0        | `/opt/zig-0.16.0/`              | `/usr/local/bin/zig`      |
| Wasmtime         | v43.0.1       | `/opt/wasmtime-43.0.1/`         | `/usr/local/bin/wasmtime` |
| Binaryen         | v123          | `/opt/binaryen-123/`            | `/usr/local/bin/wasm-merge`, `/usr/local/bin/wasm-opt` |
| rustup + Rust    | floating `stable` (currently 1.96.0) | `/root/.rustup`, `/root/.cargo` | `/usr/local/bin/{cargo,rustc,rustup,rustfmt,clippy-driver}` |
| Tcl 8.4 source   | 8.4.20        | `tmp/tcl8.4.20/`                | —                         |
| Tcl 8.5 source   | 8.5.19        | `tmp/tcl8.5.19/`                | —                         |
| Tcl 8.6 source   | 8.6.16        | `tmp/tcl8.6.16/`                | —                         |
| Tcl 9.0 source   | 9.0.3         | `tmp/tcl9.0.3/`                 | —                         |
| tcllib           | 2.0           | `tmp/tcllib-2.0/`               | —                         |
| Tcl regex engine | 9.0.3         | `runtime/zig/vendor/tcl-regex/` | —                         |

Notes on the fetched sources:

- Tcl and tcllib are full source trees (`generic/`, `unix/`, `win/`, `tests/`,
  `library/`, `doc/`, …) pulled as release tarballs from
  `codeload.github.com`. Tarballs are GitHub-CDN cached, smaller than a git
  clone, and friendlier to the upstream Tcl project than hitting
  `tcl.tk`/`sourceforge.net` on every cold session.
- Zig is fetched via the community mirror pool listed at
  [`community-mirrors.txt`](https://ziglang.org/download/community-mirrors.txt);
  the hook shuffles the pool, falls back to `ziglang.org` as the last resort,
  and verifies the x86_64-linux tarball against the published SHA-256.
- The hook is idempotent — warm containers re-run it and finish in seconds.
- The Tcl regex engine sources (14 `.c`/`.h` files, ~150 KB) are fetched into
  `runtime/zig/vendor/tcl-regex/` by `scripts/fetch_tcl_regex.sh`. They are
  not vendored in the repo. The WASM runtime build (`zig build`) **does not**
  fetch them itself — local developers must run the script once after
  cloning. Re-fetch by deleting `runtime/zig/vendor/tcl-regex/.stamp` and
  re-running.

To bump any of these versions, edit the pinned variables at the top of
[`.claude/hooks/session-start.sh`](.claude/hooks/session-start.sh)
(`ZIG_VERSION`, `WASMTIME_VERSION`, `BINARYEN_VERSION`, `TCLLIB_TAG` /
`TCLLIB_VERSION`; Rust tracks the floating `stable` channel via
`RUST_TOOLCHAIN` and needs no version bump) and, for Tcl, the version/tag maps in
[`.claude/skills/fetch-tcl-source/fetch_tcl_source.sh`](.claude/skills/fetch-tcl-source/fetch_tcl_source.sh).
For Zig, refresh `expected_sha` in the hook to match the new x86_64-linux
tarball's SHA-256 from `https://ziglang.org/download/index.json`.

### Version requirements — sources of truth and update checklist

The **source of truth** for each minimum version:

| Requirement | Source of truth              | File                  |
|-------------|------------------------------|-----------------------|
| Rust        | pinned min (1.95+)           | `rust-toolchain.toml` / `Makefile` Prerequisites |
| Node.js     | CI matrix                    | `.github/workflows/ci.yml` |

When changing a minimum version, update **all** of these locations:

- `rust-toolchain.toml` — the pinned Rust channel/version
- `.github/workflows/ci.yml` — `node-version` values (and the Rust toolchain step)
- `Makefile` — Prerequisites comment block at the top
- `AGENTS.md` — Prerequisites section (this file)
- `README.md` — Prerequisites / requirements section

## Build system

The project uses GNU Make. Key targets:

| Target             | Purpose                                  |
|--------------------|------------------------------------------|
| `make rust-check`  | **Rust PR gate** — `check-rust` (cargo `fmt --check` + `clippy`) + `xtask-check` (generated-file / docs-index drift gates via `cargo xtask …`). Mirrors the GitHub Actions `pr-gate` job. |
| `make check-all`   | **Pre-push gate** — full lint + typecheck across **every** language: TypeScript via ESLint + Prettier + tsc, Zig via `zig fmt --check` + `zig build`, Rust via `cargo fmt --check` + `cargo clippy`. On success writes `tmp/check-all.stamp`; the pre-push hook requires this. |
| `make test-slow`   | **Pre-PR gate** — must pass before opening a PR. Runs everything: optional dep check (or install when `AUTO_INSTALL_DEPS=1`) + `capture-bytecode-refs` + `prep-pr` + `check-zig` + `check-rust` + tclpkg (`test-tclpkg-tcl`) + VS Code extension (`test-ext`) + Zig WASM runtime tests (`test-zig`) + Emacs eglot (`test-emacs`) + VSIX smoke (`_prep-pr-smoke`) + the full Rust workspace test suite (`test-rust`, which includes the native lsp_e2e). Drives every phase through `scripts/dev/test-slow-runner.sh`, which keeps going past failures and prints one consolidated PASS/FAIL summary at the end. On success writes the committed `.test-slow.stamp` (CI PR gate) plus the local `tmp/check-all.stamp` and `tmp/test-slow.stamp`. **`git add .test-slow.stamp` and commit it with your PR.** Release-only docs (`RELEASE_NOTES.md`, `docs/sphinx/changelog.md`) are excluded from the fingerprint, so a single green run before merge stays valid through the release tag — the release flow **verifies** this stamp rather than re-running the gate. |
| `make verify-test-slow-stamp` | Verify the committed `.test-slow.stamp` matches the current tree — the same content-fingerprint check the GitHub `test-slow-stamp` PR job runs. Fails loudly if the tree changed since the last green `test-slow`. Note: running **any** make target other than this one, `test-slow`, or `help` deletes `.test-slow.stamp`, so re-run `make test-slow` (which rewrites it) as the final step before committing. |
| `make install-test-deps` | One-shot setup: install **everything** `test-slow` needs (the system toolchain — all of `ensure-test-deps`). The target to run on a fresh checkout before `make test-slow`. Same platform coverage as `ensure-test-deps`. |
| `make ensure-test-deps` | Install the optional `test-slow` toolchain (`tclsh9.0`, `node`+`npm`, `kotlinc`, Rust/rustup, Zig, Wasmtime, Binaryen, emacs, xvfb, …) on Debian/Ubuntu (apt-get), CentOS/RHEL/Rocky/Alma/Fedora (dnf or yum), or macOS (Homebrew). Idempotent. Builds Tcl 9 from `tmp/tcl9.0.3/` since most distros don't package it yet. Skip individual tools with `SKIP_TCLSH=1`, `SKIP_NODE=1`, `SKIP_KOTLINC=1`, `SKIP_RUST=1`, `SKIP_ZIG=1`, … Run `bash scripts/dev/ensure-test-deps.sh --check` for a non-mutating report of what would be installed. |
| `make ensure-rust-deps` | Install Rust/rustup + the `wasm32-wasip2` target needed by `check-rust` / the WASM build. |
| `make capture-bytecode-refs` | Run `scripts/capture/bytecode.sh` to fill in any missing `tests/bytecode_reference/<ver>/*.disasm` files using a locally available `tclsh9.0`. No-op when the corpus is complete; soft-skips with guidance when `tclsh9.0` is missing. |
| `make check-zig`   | Zig format check + compile (`zig fmt --check` + `zig build install`). Skip with `SKIP_CHECK_ZIG=1`. |
| `make check-rust`  | Rust format check + clippy across the workspace (and the Zed extension). Skip with `SKIP_CHECK_RUST=1`. |
| `make install-hooks` | Install the project's git pre-push hook, which refuses pushes unless `make check-all` (or `make test-slow`) has been run against the current worktree. |
| `make prep-pr`     | Pre-PR formatting + fast checks (a subset of test-slow; auto-formats code, runs codegen, lint/typecheck, and `test-rust`).  Use `make test-slow` for the full gate. |
| `make test`        | Run all tests — Rust workspace + VS Code extension + Zig WASM runtime (`test-rust test-ext test-zig`) |
| `make test-rust`   | `cargo test --workspace --all-features` — includes the native lsp_e2e suite (`rust/tcl-lsp-server/tests/*_e2e.rs`); skip with `SKIP_TEST_RUST=1` |
| `make test-ext`    | VS Code extension integration tests (xvfb on headless Linux) |
| `make test-ext-rust` | Build `rust-server`, then run the VS Code extension tests against the native Rust server (`TCL_LSP_SERVER_KIND=rust`, `TCL_LSP_SERVER_BIN` set) |
| `make test-zig`    | Zig WASM runtime unit tests (`zig build test`); skip with `SKIP_TEST_ZIG=1` |
| `make lint`        | Lint / style checks — TypeScript only (`lint-ts`: ESLint + Prettier) |
| `make format`      | Format TypeScript (`format-ts`, Prettier) |
| `make rust-server` | Build the native `tcl-lsp-server` binary (`cargo build -p tcl-lsp-server`; `PROFILE=release|debug`) |
| `make rust-tcl`    | Build the `tcl` CLI (`cargo build -p tcl-cli`) |
| `make rust-f5`     | Build the `f5-query` CLI (`cargo build -p f5-cli`) |
| `make rust-mcp`    | Build the `tcl-mcp` MCP server (`cargo build -p tcl-mcp`) |
| `make rust-clis`   | Build the `tcl` + `f5-query` CLIs (`rust-tcl rust-f5`) |
| `make compile`     | Compile the TypeScript extension         |
| `make build-editor-vsix`        | Build the .vsix VS Code extension (bundles the native `tcl-lsp-server` binaries) |
| `make codegen`     | Regenerate all generated files (editor catalogs + settings + AI prompts) via `cargo xtask` |
| `make publish-flow` | Print the release + marketplace publish cheat-sheet |

The build is organised into **four layers** with a clear separation
between them — see
[`docs/design/contracts/release-and-publish.md`](docs/design/contracts/release-and-publish.md)
for the full contract:

1. **Entry points** — `Makefile` targets + the native cargo bins
   (`tcl-lsp-server`, `tcl`, `f5-query`, `tcl-mcp`) and `cargo xtask`.
2. **Helpers** — `scripts/{build,codegen,check,capture,release,install,dev}/*`.
3. **CI** — `.github/workflows/*.yml` (PR gate + tag-triggered
   artefact build → sign → attach to GitHub Release).
4. **Publishing** — VS Code and JetBrains publish *from CI* behind
   manually-approved Environments; Package Control (Sublime) and Zed
   publish from the maintainer's laptop (they need no token).

**Invariant: a publish secret used in a workflow must be an Environment
secret on a protected, manually-approved Environment.**  A marketplace
token may live in CI only when it is stored as a GitHub *Environment*
secret on an Environment with a required reviewer (and a `v*`-tag-only
deployment policy) — never as a plain repo/org secret available to every
workflow run.  Stored that way, the secret is reachable only by the one
gated publish job that targets that Environment, which pauses for human
approval and cannot run on a non-tag ref.  Concretely:

- **VS Code Marketplace** publishes from CI with `secrets.VSCE_PAT`, an
  Environment secret on `marketplace-vscode` (required reviewer + a
  `v*`-tag-only policy).  The token is scoped to the publish step's `env:`
  so freshly-fetched npm code never runs with it in the environment.
  `make publish-vsix` stays as a laptop fallback (keyless `az login`, or
  `VSCE_PAT`).
- **JetBrains Marketplace** publishes from CI with `secrets.JETBRAINS_TOKEN`,
  an Environment secret on `marketplace-jetbrains` (same protections),
  uploading the released, checksum-verified `.zip` via the Marketplace REST
  API.  `make publish-jetbrains` stays as a laptop fallback.
- **Package Control (Sublime)** and **zed-industries/extensions** need no
  token — they publish by pushing to a maintainer-owned mirror / opening a
  PR — so they stay laptop-only and never enter CI.
- CI otherwise uses only GitHub's built-in `github.token` + sigstore OIDC
  for attestations.

A publish secret stored any other way — a plain repo secret, an org secret,
or one reachable by a job with no protected `environment:` — violates the
contract.

**Stable vs pre-release channels (odd/even-minor).**  Two lines run in
parallel and the tag alone decides which:

- **Stable / default** — `v1.x` and even-minor `v2.x` (`v2.2.0`), cut from
  `main`.  Normal GitHub Release (`latest`) and normal Marketplace channel.
- **Pre-release / "for the brave"** — odd-minor `v2.x` (`v2.1.0`,
  `v2.1.1`, …), cut from `rust`.  Published as a GitHub `--prerelease`
  (never `latest`) and to the Marketplace `--pre-release` channel, so 1.x
  stays the default install until a user opts into pre-releases.

The 2.x rewrite ships its alphas on `2.1.x` and promotes to the stable
`2.2.0` when ready.  `scripts/release/prerelease.sh X.Y.Z` is the single
source of truth (prints `true`/`false`); CI (`create-release`,
`publish-vsix-marketplace`), the Makefile (`VSCE_PRERELEASE_FLAG`), and
`tag.sh` all read it, so nothing per-version needs editing — just
`make release-tag V=2.1.0` from `rust`.

## WASM command parity

The Rust command specs in **`tcl-registry`** are the **source of truth**
for which Tcl 8.4-9.1 commands exist.  The Zig WASM runtime must be
bit-for-bit aligned with the registry — same commands, same sub-commands,
same arity bounds — so that every command in the registry has runtime
backing (a real handler, a trapping stub, or an explicit "not required"
classification).

For a walkthrough of how a Tcl script becomes a WASM module (the
6-phase codegen pipeline, per-statement dispatch order, per-command
file layout), see
[`docs/design/compiler/wasm-codegen.md`](docs/design/compiler/wasm-codegen.md).

Every command must have one of:

- a real Zig handler in `runtime/zig/cmds/*.zig` (visible in
  `runtime/zig/dispatch/tcl_cmd_table.zig`'s `BUILTINS` slice),
- a trapping stub in `runtime/zig/dispatch/tcl_stub_fallback.zig` (raises
  `unsupported command: X`), or
- an explicit "not required" classification (currently only the
  `tcl::mathop::*` prefix-form operators).

### Arity contract

`CmdEntry` in `runtime/zig/dispatch/tcl_cmd_registry.zig` carries
explicit `arity_min: u32` and `arity_max: ?u32` fields (null =
variadic).  Every registration in `cmds/*.zig` must fill them in:

```zig
.{ .name = "set", .arity_min = 1, .arity_max = 2, .handler = &eval_set }
```

These bounds must match the matching `CommandSpec` arity in the Rust
`tcl-registry`.  A `(command, registry-bounds, zig-bounds)` mismatch is a
regression — the Zig side is the one that has to track the registry.

### Sub-command contract

Commands that dispatch on a sub-command word (`string length`,
`dict get`, `clock seconds`, `info body`, …) must declare their
sub-commands as a `SubEntry` slice:

```zig
pub const subcommands: []const reg.SubEntry = &.{
    .{ .name = "length", .arity_min = 1, .arity_max = 1, .handler = &sub_length },
    .{ .name = "index",  .arity_min = 2, .arity_max = 2, .handler = &sub_index  },
    …
};
```

These entries must match the `SubCommand` entries on the parent
`CommandSpec` in the Rust `tcl-registry`.  Sub-command migration is
incremental: commands without a Zig `subcommands` table are known-missing
(tracked, not a regression).  Adding a sub-command to the registry without
the matching Zig entry — or vice versa — is a regression.

### When you change the parity

When you add a Tcl command, add both the registry `CommandSpec` (in
`tcl-registry`) and the runtime backing (a Zig handler + arity, a stub, or
an explicit "not required" classification) in the same change.  Common
reasons the parity moves:

- Adding a new Tcl command (registry spec + Zig handler + arity).
- Migrating a sub-command dispatcher from if-chain to `SubEntry` slice.
- Promoting a silent stub to a real implementation.

Each change should tell a clean improvement story — the registry and the
Zig runtime describe the same command surface, arity bounds, and
sub-commands.

## Optional WASM extensions

The compiler can ship *optional* runtime features the user's program
requests via `package require`.  Today this is implemented as
runtime variants — `zig build` produces both `tcl_runtime.wasm`
(lean) and `tcl_runtime_with_<extname>.wasm` (with the extension's
commands compiled into BUILTINS).  The `tcl-compiler` WASM link/bundle
step picks the right variant based on the `package require` calls it
finds in the merged IR, then `wasm-merge`s it with the user-code module
to produce a single bundled `.wasm`.

The first extension is **Tcltest**: the full Tcl 9 `tcltest` C-tier
`test*` command surface (107 commands across 14 cmd_*.zig files
under `runtime/zig/tcltest/`).  PORTABLE / PARTIAL commands have
functional implementations; NOT-PORTABLE ones (sockets, threads,
fork, native FS hooks) raise an explicit "not supported under WASM"
error.  See
[`docs/design/compiler/wasm-extensions.md`](docs/design/compiler/wasm-extensions.md)
for the contract and full per-cluster file layout.

## Workflow requirements

There are **three distinct gates**, in increasing strictness:

| Gate | Required before | What runs | Enforcement |
|---|---|---|---|
| **`make rust-check`** | Rust-only changes (minimum) | Rust `fmt --check` + `clippy` + `xtask-check` (generated-file / docs-index drift gates).  Mirrors GitHub Actions' `pr-gate` job | fast subset — run before `check-all` |
| **`make check-all`** | every `git push` (minimum) | multi-language lint + typecheck across TypeScript (ESLint + Prettier + tsc), Zig (`zig fmt --check` + `zig build`), Rust (`cargo fmt --check` + `cargo clippy`) | Pre-push hook accepts `tmp/check-all.stamp` matching the current worktree |
| **`make test-slow`** | every PR / merge request | Everything `check-all` runs **plus** the full Rust workspace test suite (native lsp_e2e included), tclpkg, VS Code extension, Zig WASM runtime tests, Emacs eglot, and VSIX smoke | Pre-push hook accepts `tmp/test-slow.stamp` + agent rule: required before opening a PR |

GitHub Actions runs only the fast gate on PRs (the `pr-gate` job, mirrored by
`make rust-check` + the TS/Zig lint in `check-all`).  Everything else is the
responsibility of the local gates above.  CI is **not** a substitute for
either gate.

### Before any push

**Lint and typecheck must be clean before you push.**  Three gate
levels, weakest → strongest:

```
make rust-check     # Rust fmt + clippy + drift gates (mirrors the PR gate)
make check-all      # multi-language lint + typecheck — the push gate
make test-slow      # full suite — required before opening a PR
```

The pre-push hook (installed via `make install-hooks`) recomputes
the worktree fingerprint at push time and accepts the push when
`tmp/check-all.stamp` or `tmp/test-slow.stamp` matches.  Failures must
be fixed, not skipped — tooling-missing skips must be deliberate
(`SKIP_CHECK_ZIG=1`, `SKIP_CHECK_RUST=1`, ...).

**Agent rule — Claude / codex / etc:** `make check-all` is the MINIMUM
gate before every `git push`.  The PR's `pr-gate` job runs the same Rust
fmt + clippy + drift checks, so a local pass means CI will pass on the
first try.  Running `cargo fmt` after a commit isn't enough — the
fingerprint stamp locks the full gate.  Every "the PR gate bounced on a
trivial clippy lint / format drift" failure on this repo's PRs has been a
`push` that skipped this step.

### Before opening a PR

**Rebase off `main`, fix conflicts, then run the full pre-PR gate:**

```
make test-slow
```

`test-slow` is a strict superset of `check-all`.  It runs (serially first,
then the rest in parallel):

1. **capture-bytecode-refs** — fill any missing bytecode reference disasms
2. **prep-pr** — format (Prettier) + codegen (`cargo xtask`) + lint + typecheck
   (tsc) + editor-settings drift check + `test-rust`
3. **check-zig** + **check-rust** — Zig and Rust lint/typecheck
4. **tclpkg** (`test-tclpkg-tcl`) — pure-Tcl package-manager tests
5. **VS Code extension** (`test-ext`) — xvfb if no DISPLAY
6. **Zig WASM runtime** (`test-zig`) — runtime unit tests via `wasmtime`
7. **Emacs eglot** (`test-emacs`)
8. **VSIX smoke** (`_prep-pr-smoke`)
9. **Rust workspace** (`test-rust`) — `cargo test --workspace --all-features`,
   including the native lsp_e2e suite (`rust/tcl-lsp-server/tests/*_e2e.rs`)

On success it writes both `tmp/check-all.stamp` and `tmp/test-slow.stamp`.

Skip variables for missing tooling: `SKIP_TEST_ZIG=1`, `SKIP_TEST_EMACS=1`,
`SKIP_TEST_RUST=1`.  Use these only when the tool genuinely isn't available
on your machine — the gate must still cover everything else.

### Rule for agents

**Agents MUST NOT open a PR (or instruct the user to open one) until
`make test-slow` has completed successfully in its entirety against the
exact worktree being proposed.**  Verify before opening the PR:

```
test "$(cat tmp/test-slow.stamp 2>/dev/null)" = "$(scripts/worktree-fingerprint.sh)"
```

If the check fails (no stamp, or mismatch), re-run `make test-slow` before
proceeding.  Likewise, **agents MUST NOT push** without `make check-all`
having completed cleanly — the pre-push hook will reject the push, and
agents must not bypass the hook with `SKIP_PUSH_GATE=1` or
`git push --no-verify` unless the user has explicitly authorised it.

These rules are non-negotiable: CI covers only the fast Rust fmt/clippy/drift
gate, so the local gates are the only thing standing between a regression and
`main`.

Commit any formatting changes that `make test-slow` applies before creating
the PR (it runs `prep-pr` which auto-formats — re-running test-slow after
any commits is required so the stamp matches the final tree).

### When a PR is created, run test-slow locally

When a PR is opened on this repository — whether by the agent, by the user,
or by the Claude Code UI on the agent's behalf — the agent MUST kick off
`make test-slow` **on its local machine** against the exact tip the PR is
built from, without being asked.  This applies to PRs the agent didn't
open: if the agent learns of a new PR (e.g. via `<github-webhook-activity>`
subscription, a comment, or the user mentioning it), it must immediately
verify the stamp and re-run test-slow locally if the worktree drifted, then
act on whatever fails.

**Do NOT add `test-slow` (or any subset of it beyond the existing fast
PR gate) to `.github/workflows/`.**  CI on this repo intentionally runs
only the fast Rust fmt/clippy/drift gate; the rest of the gate is the
agent's local responsibility.  Don't wire `test-slow` into a GitHub Action, don't trigger
it via `workflow_dispatch`, and don't ask the user to enable it on the
runner — run it on the local machine you're already working in.

### Capturing build / test logs

Long gates (`make test-slow`, `make test-rust`, `make test-ext`, anything
running for more than a few seconds) MUST have their full output captured
to a file under `/tmp/` rather than only being read via `tail`.  Tailing
loses signal: a failure in the middle of a 10-minute run won't appear in
the last 50 lines if the harness keeps going, and cargo/test summary lines
can get pushed off the bottom by skip-message spam.  The pattern:

```
make test-slow 2>&1 | tee /tmp/test-slow-<branch>.log
# then, when investigating a failure:
grep -nE 'FAIL|ERROR|panicked|error\[|^E ' /tmp/test-slow-<branch>.log
```

For background runs use `tee` to a `/tmp/` path, then `grep` the file when
you want a specific signal.  Keep the file around until the PR merges so
you can re-investigate after a CI ping without having to re-run the gate.

## Knowledge base and documentation

The project has two kinds of written content with different purposes, tones,
and locations.

- **KCS notes** (`docs/kcs/`) are small, searchable answers to one question
  each, written in plain English for a named audience (user, contributor, or
  maintainer). They are for people who are trying to get something done.
- **Documentation** (`docs/design/`, `docs/GLOSSARY.md`) is technical
  material — design docs, contracts, interfaces, data-structure references,
  architecture narratives. It describes how the system is built and why.
  Technical jargon is allowed.

If you are not sure where something belongs: if it answers one question a
person would ask out loud, it is a KCS note; if it describes how a module is
structured, what its contract is, or what data flows through it, it is a
design doc.

### KCS — the four categories

Every KCS note is exactly one of these four types. Pick the category first,
then copy the matching template from [`docs/kcs/templates/`](docs/kcs/templates/README.md).

| Type | The question it answers | Template |
|---|---|---|
| **Issue** | Why is X not working, and how do I fix it? | [`kcs-template-issue.md`](docs/kcs/templates/kcs-template-issue.md) |
| **Q&A** | What is X? / When should I use Y? | [`kcs-template-qa.md`](docs/kcs/templates/kcs-template-qa.md) |
| **How-To** | How do I do X? | [`kcs-template-how-to.md`](docs/kcs/templates/kcs-template-how-to.md) |
| **Functionality** | What does command/feature/tool X do, and how do I use it? | [`kcs-template-functionality.md`](docs/kcs/templates/kcs-template-functionality.md) |

Every KCS note starts with a blockquote header naming its audience and
type:

```markdown
# KCS: <short title>

> **Audience:** User | Contributor | Maintainer
> **Type:** Issue | Q&A | How-To | Functionality
```

### KCS style rules

1. One KCS note answers **one** core question. If a note answers two
   questions, split it.
2. Name the audience explicitly at the top: **User**, **Contributor**, or
   **Maintainer**.
3. Write in **British English** (`colour`, `optimiser`, `analyse`).
4. Use the **Oxford comma**: "tokens, ranges, and diagnostics" — not
   "tokens, ranges and diagnostics".
5. Prefer short, plain sentences. Avoid long subordinate clauses.
6. **Do not use acronyms or specialist terms** without linking to the
   glossary. On first use within a note, use the plain name and link the
   glossary term: `[control-flow graph](docs/GLOSSARY.md#cfg)`.
7. Use **exact UI labels** when referring to buttons, menus, or commands.
8. Do not inline contract tables, data-structure references, or API
   signatures. Link to the relevant design doc instead.
9. Keep notes **short** — aim for one screen. If longer is required,
   consider whether it should be a design doc.
10. **Name the file after the question, not the implementation.** Use
    `kcs-issue-lsp-features-are-missing.md`, not
    `kcs-issue-vscode-lsp-startup-logs.md`. Functionality, diagnostic,
    and optimisation notes are named around their stable identifier:
    `kcs-feature-rename.md`, `kcs-diagnostic-w210-variable-read-before-set.md`,
    `kcs-optimisation-o105-constant-var-ref-propagation.md`.
11. **Functionality notes must include at least one concrete example**
    — a before/after code block for a transform, a code pointer
    showing where a diagnostic or hover appears, or a screenshot of a
    visual panel.
12. **Every note lists the editors and tools it applies to**, in an
    `## Applies to` section immediately after the audience/type
    header, as a comma-separated plain-text list (not bullets):
    `VS Code, Zed, JetBrains, Neovim, tcl-lsp CLI`. Use `all-editors`
    when the note runs everywhere; the build script expands it to
    the full LSP editor set. The canonical tag vocabulary covers
    editors (`vs-code`, `zed`, `jetbrains`, `neovim`, `helix`,
    `emacs`, `sublime-text`), tools (`tcl-lsp-cli`, `mcp`,
    `claude-skill`, `copilot-chat`), content kinds (`diagnostic`,
    `optimisation`, `warning`, `refactoring`, `analyser`,
    `transform`), and compiler passes (`lexing`, `lowering`, `cfg`,
    `ssa`, `sccp`, `liveness`, `type-infer`, `gvn`, `cse`, `dce`,
    `licm`, `instcombine`, `ipa`, `memssa`, `dataflow`, `taint`,
    `shimmer`, `tail-call`, `code-sinking`, `unused-procs`,
    `side-effects`, `exec-intent`, `rendered-props`, `const-fold`,
    `strength-reduce`, `codegen`). The vocabulary lives in the `tcl-cli`
    KCS/help data (`rust/tcl-cli`) and is documented
    in [`docs/kcs/STYLE.md`](docs/kcs/STYLE.md) (rule 11). Per-code
    pages and compiler-internals feature pages must carry the
    compiler-pass tag of the pass that produces the code or the
    facts they consume.
13. **If the answer differs per editor or tool, split it into
    sub-headings** under the answer section, in the same order as
    `## Applies to`. Do not bury per-editor differences in inline
    asides.

For the full style guide with worked examples, see
[`docs/kcs/STYLE.md`](docs/kcs/STYLE.md).

### Documentation (non-KCS)

Design docs, contracts, and interface references live under
[`docs/design/`](docs/design/README.md). A design doc may be long, may use
technical jargon freely, and may include type signatures, contract tables,
ownership matrices, and file-path anchors. One contract per file is the
rule of thumb.

Complex terms go in [`docs/GLOSSARY.md`](docs/GLOSSARY.md). KCS notes link
to the glossary instead of defining terms inline; design docs may either
link or define locally.

### Where things live

| Content kind | Folder | Example |
|---|---|---|
| User/contributor answer to one question | `docs/kcs/` | `kcs-issue-lsp-features-are-missing.md` |
| Feature, command, or tool description | `docs/kcs/features/` | `kcs-feature-rename.md` |
| KCS style guide and templates | `docs/kcs/STYLE.md`, `docs/kcs/templates/` | — |
| Architecture and pipeline walkthroughs | `docs/design/` | `compiler-architecture.md` |
| Compiler pass, stage, or analysis internals | `docs/design/compiler/` | `cfg-construction.md` |
| Module ownership or API contract | `docs/design/contracts/` | `shared-utility-contracts.md` |
| Design-doc templates | `docs/design/templates/` | `template-contract.md` |
| Definitions of complex terms | `docs/GLOSSARY.md` | `CFG`, `SSA`, `lattice`, `shimmer` |

### Documentation required for a PR

Any new or changed feature **must** include documentation updates in the
same change:

1. **README.md** — update the relevant section to reflect the new or
   changed behaviour.
2. **KCS note** — create or update a note in `docs/kcs/` using the
   matching template, and add it to the relevant section of
   [`docs/kcs/README.md`](docs/kcs/README.md). For feature changes, update
   the file under [`docs/kcs/features/`](docs/kcs/features/README.md).
3. **Design doc** — if the change introduces or modifies a contract,
   interface, or data-structure, update the relevant file under
   [`docs/design/`](docs/design/README.md) and link it from
   [`docs/design/README.md`](docs/design/README.md).
4. **Glossary** — if the change introduces a new technical term, add it to
   [`docs/GLOSSARY.md`](docs/GLOSSARY.md) with a stable anchor.
5. **Screenshots** — capture screenshots for user-visible changes and
   reference them from the relevant KCS note and `README.md`.

A PR that adds or modifies a feature without these documentation updates
is incomplete and must not be merged.

## Code style

- TypeScript style is enforced by **ESLint + Prettier** (`make lint-ts` / `make format-ts`).
- Rust must pass **`cargo clippy` cleanly**, including the workspace-enabled
  `clippy::pedantic`. **Do not add `#[allow(...)]` / `#[expect(...)]` to silence
  a lint** — an allow is a code smell with a very high bar. Fix the underlying
  issue instead (e.g. `too_many_lines` → extract helpers; `similar_names` →
  rename; `too_many_arguments` → group the parameters into a config/options
  struct). Only when a lint is genuinely wrong *and* no reasonable refactor
  exists may you allow it, with a comment saying why. Pre-existing allows are
  not licence to add more. The one clear pass of the bar: a **config/options
  constructor** where the many parameters *are* the config — `too_many_arguments`
  there is fine to allow (with a one-line comment), because grouping into a
  struct only makes the API worse.
- Use **UK spelling** in identifiers and comments (`normalise`, `optimiser`, `analyse`).
- Keep names explicit; avoid ambiguous single-letter variables outside tiny loops.
- Prefer `match` for enum/token dispatch with 3+ branches.
- **Comments** must be plain, minimal, and only present when they illuminate
  something the code itself does not convey. Do not use banner-style comments
  (`// -----------`, `// --- Text ---`, `// -- [section] ------`). Use a plain
  `// Text` comment instead. Never add standalone dash-separator lines.
- **Copyright headers** go on *our own original source only*: the full
  AGPL-3.0 notice with `Copyright (C) <year> James Deucker (bitwisecook)`,
  placed after any shebang / `-*-` / coding magic first line. **Never** add our
  header to vendored or third-party code — it keeps its own notices and licence
  (e.g. `runtime/rust/vendor/`, `rust/tcl-regex/tests/data/reg.test`). Also skip
  generated files, test fixtures / golden corpora, and `.github/workflows/*`
  (the CI token cannot push header edits to workflows). See `DUAL-LICENSING.md`.
- See `CONTRIBUTING.md` for the full style guide.

## Editor settings codegen

Whenever a diagnostic or optimisation is added, removed, or changed (code,
severity, message, or section), you **must** regenerate the editor settings
catalogues:

```
make gen-editor-settings
```

This is xtask-backed (`cargo xtask gen-editor-settings` + the sibling
`gen-vscode-package` / `gen-jetbrains-catalog` / `gen-ai-diagnostics`
generators) and updates the generated diagnostic tables in VS Code, Neovim,
Zed, Emacs, Helix, Sublime, and JetBrains editor integrations. Commit the
regenerated files alongside the diagnostic/optimisation change — the
`xtask-check` drift gate (and CI) will fail if they are stale.

## LSP feature toggles

Most LSP features use a simple runtime guard pattern: the handler is always
registered, and the handler body checks whether the feature is enabled before
doing work (returning nothing when disabled). This allows features to be
toggled via `didChangeConfiguration` without restarting.

A small set of features cannot follow this pattern because their handler
registration changes the `ServerCapabilities` advertised during `initialize`,
which alters client behaviour irreversibly for the session. These
restart-required toggles are decided at `initialize` time in the native
server (`tcl-lsp-server` / `tcl-lsp-core`). Currently this set includes:

- **`pull_diagnostics_enabled`** — registers `textDocument/diagnostic` and
  `workspace/diagnostic` handlers, which flips `vscode-languageclient` into
  pull mode and disables the push pipeline.

Changing a restart-required toggle at runtime logs a warning but has no
effect until the server process is restarted.

## Lexer token types

The Tcl lexer (`rust/tcl-lexer`) produces tokens with a `TokenType`
enum. Key conventions that affect downstream consumers:

- **`ESC`** — plain word fragment, possibly containing backslash escapes.
  This is the default type for unbraced, unsubstituted text. Standalone
  punctuation like `}` or `]` appearing outside of their structural role
  (i.e. as stray characters) also receives `TokenType.ESC`.
- **`STR`** — braced string `{...}`.
- **`CMD`** — command substitution `[...]`.
- **`VAR`** — variable substitution `$name` or `${name}`.
- **`SEP`** / **`EOL`** — whitespace separator / end-of-line.

When checking for stray punctuation (`}`, `]`), always check
`tok.type is TokenType.ESC` — not just `tok.text`. A `}` with type `STR`
is a structural brace, not a stray character.

See `docs/design/compiler/lexing-segmentation.md` for the full token type
table and lexer contracts.

## Zig runtime layering

The WASM runtime under `runtime/zig/` is organised into role-based
subfolders, each holding small single-responsibility modules.  Callers
should import the specific module they need — the old "everything in
`tcl_obj.zig`" shape is dead.  `valtypes/tcl_obj.zig` still re-exports
the migrated symbols (`obj.is_space`, `obj.list_elem_quote`, …) as a
compat layer so older callers don't break, but new code should go to
the canonical module.

### Folder layout

```
runtime/zig/
├── build.zig
├── tcl_runtime.zig                      (entry point)
├── valtypes/                            (TclObj + value utilities)
├── parse/                               (script tokeniser + subst)
├── interp/                              (eval loop + frames + ns)
├── dispatch/                            (command lookup + diag)
├── stubs/                               (trapping / degraded exports)
├── io/                                  (real I/O + time)
├── cmds/                                (per-command BUILTINS registrations)
└── regex_include/                       (C vendor shim for Spencer regex)
```

### Module reference

| Module | Owns | Reference Tcl 9 analogue |
|---|---|---|
| `valtypes/tcl_chars.zig` | character classification (`is_space`, `is_scan_space`, `is_bareword`, `is_digit`, …) + byte-span comparators (`str_eq`, `str_cmp`, `mem_eq`) | `tclParse.c` `CHAR_TYPE` / `TclIsSpaceProc` / `TclIsBareword` |
| `valtypes/tcl_bs.zig` | backslash decoder — `consume_bs_escape` for one ``\x`` escape; `decode_into` for whole-span decode; `encode_utf8` helper | `tclParse.c` `TclParseBackslash` / `Tcl_UtfBackslash` / `TclCopyAndCollapse` |
| `valtypes/tcl_list_quote.zig` | output side of the list-string contract — `scan_element` + `convert_element` (COMPAT=1), `list_elem_quote` / `list_elem_quote_nth` | `tclUtil.c` `TclScanElement` / `TclConvertElement` / `Tcl_Merge` |
| `valtypes/tcl_list_parse.zig` | input side — `count_elements`, `element_at`, `copy_unbraced_elem` | `tclUtil.c` `TclFindElement` / `Tcl_SplitList` |
| `valtypes/tcl_obj.zig` | TclObj memory model, type dispatch, `try_parse_int` / `try_parse_bool` | `tclObj.c` |
| `parse/tcl_parse.zig` | script / word tokeniser — `parse_command` (flat-array legacy API) and `ParseCommand` (Token-tree API with per-word `braced` flag) | `tclParse.c` `Tcl_ParseCommand` / `Tcl_ParseBraces` / `Tcl_ParseQuotedString` |
| `parse/tcl_subst.zig` | `subst_flagged` — `$var`, `[cmd]`, and `\bs` substitution engine shared by the word expander and the `subst` command; lazy-imports `interp/tcl_interp.zig` for `eval_script` | `tclParse.c` `Tcl_SubstObj` |
| `interp/tcl_interp.zig` | eval loop, proc frame management, and `pub` helpers (`eval_if`, `eval_while`, `eval_for`, `eval_foreach`, `eval_expr_str`, `qualify_name`, …) called by `cmds/` modules; dispatches builtins via `tcl_cmd_table.lookup()` | `tclBasic.c` / `tclExecute.c` |
| `interp/tcl_interp_registry.zig` | child interpreter registry — `interp create`/`eval`/`delete`/`exists`/`slaves` primitives and per-interp `hidden_cmd_table` slot | `tclInterp.c` `ChildCreate` / `ChildEval` |
| `dispatch/tcl_cmd_registry.zig` | `CmdEntry { name, handler }` type and linear-scan `lookup(entries, name_ptr, name_len)` used by `tcl_cmd_table.zig` | local shim |
| `dispatch/tcl_cmd_table.zig` | assembles the `BUILTINS` slice from all `cmds/*.zig` modules via `++` concatenation; exposes `lookup()` called by `eval_command` | local shim |
| `dispatch/tcl_stub_fallback.zig` | fallback dispatch for Tcl core commands without a BUILTINS entry; the `STUB_TRAP` data table names commands that emit `unsupported command: X` via `stubs/tcl_stubs.zig` | local shim |
| `dispatch/tcl_dispatch.zig` | host bridge for compiled-proc calls (consumer) | local shim |
| `dispatch/tcl_diag.zig` | DiagSite / DiagMap — source-location sidecar for runtime traps so stderr `tcl trap: site=<id>` resolves to a file:line:col | local shim |
| `cmds/tcl_cmd_info.zig` | the `info` command — body/args/default/exists/level/frame/commands/procs/functions/… | `tclCmdIL.c` `Tcl_InfoObjCmd` |
| `cmds/tcl_cmd_interp.zig` | the `interp` command — create/eval/delete/alias/hide/expose/target/invokehidden/… | `tclInterp.c` `Tcl_InterpObjCmd` |
| `cmds/tcl_hide.zig` | hidden command table (used by `interp hide` / `interp expose` / `info hidden`) | `tclInterp.c` hidden command table |
| `cmds/tcl_alias.zig` | interp alias table (used by `interp alias`, `rename` across interps) | `tclInterp.c` alias table |
| `cmds/tcl_rename.zig` | `rename` command — remove-or-relocate a command in the BUILTINS registry / user-proc table | `tclBasic.c` `Tcl_RenameObjCmd` |
| `cmds/*.zig` | one file per command group — `var.zig` (`set`/`incr`/`unset`), `scope.zig` (`global`/`variable`/`upvar`), `flow.zig` (`return`/`break`/`continue`/`error`/`catch`), `loop.zig` (`if`/`while`/`for`/`foreach`), `eval.zig` (`eval`/`uplevel`), `proc.zig` (`proc`), `list.zig` (13 list commands), `io.zig` (`puts`/`append`/`format`/`scan`), `chan.zig` (`encoding`/`fconfigure`), `fs.zig` (`file`/`pwd`/`cd`), `subst.zig` (`subst`/`expr`), `regexp.zig` (`regexp`), `inspect.zig` (`info`/`trace`), `namespace.zig` (`namespace`), `interp.zig` (`rename`/`interp`), `stubs.zig` (`auto_*`/`package`) | `tclBasic.c` built-in table |
| `stubs/tcl_stubs.zig` | `unsupported(name)` / `unsupported_sub(cmd, sub)` / `raise(msg)` — routes through the error path so inside `catch` it sets `error_flag` + `error_msg`, outside a catch it writes to stderr and traps | local shim |
| `io/tcl_io.zig` | real `puts` implementation on WASI `fd_write` | `tclIO.c` |
| `io/tcl_chan.zig` | channel registry + `fconfigure` (set / single-option query / no-args dict query) | `tclIO.c` |
| `io/tcl_fs.zig` | string-path manipulation `file` subcommands + WASI `pwd` / `cd` | `tclFileName.c` / `tclFCmd.c` |
| `io/tcl_clock.zig` | `clock seconds` / `clock clicks` / `clock milliseconds` via WASI wall/monotonic clocks | `tclClock.c` |

**Rebuilding the WASM binary:** use `Debug` mode (the default — no `-Doptimize` flag) during development so Zig's safety checks catch pointer bugs early:

```
cd runtime/zig && zig build
```

Use `ReleaseFast` only for release builds:

```
cd runtime/zig && zig build -Doptimize=ReleaseFast
```

Debug builds are ~3× larger but expose real bugs (e.g. `@ptrFromInt(0)` panics, buffer-offset vs address misuse) that are silently masked in release mode.

A few invariants to preserve when adding features:

- **One canonical implementation per algorithm.**  List-element
  quoting, backslash decoding, whitespace classification — each
  lives in exactly one module.  The "third copy in
  `dispatch/tcl_dispatch.zig`" bug that stripped newlines from braced
  proc bodies was exactly the kind of hazard this layering exists to
  prevent.
- **Character classification goes through `valtypes/tcl_chars.zig`.**
  Don't spell out `c == ' ' or c == '\t' …` inline — use
  `chars.is_space` / `chars.is_scan_space`.  Likewise for `is_digit`,
  `is_hex_digit`, `is_bareword`.
- **Braced-vs-unbraced is first-class.**  When a parser produces a
  word, callers must get the `braced` flag (either from
  `parse.Token.braced` or, in callers that still use the flat-array
  form, the old `word_braced[i]`).  Losing that flag along the path
  from parse to substitute is what causes ``\{`` / newline bugs in
  proc bodies passed through `uplevel`.

Reference Tcl source is fetched to `tmp/tcl9.0.3/` (and the matching 8.4,
8.5, 8.6 trees) by the SessionStart hook on web sessions — see
[Pre-installed toolchains and sources](#claude-code-on-the-web--pre-installed-toolchains-and-sources).
Locally, run `bash .claude/skills/fetch-tcl-source/fetch_tcl_source.sh 9.0`
(or `all`). `tmp/tcl9.0.3/generic/` carries the C parser / util files the
Zig ports mirror.

## Codegen and lowering fallback

The lowering hooks in `rust/tcl-compiler` convert high-level Tcl
commands into IR nodes. When a hook encounters a construct it cannot
safely specialise (e.g. `{*}` expansion in a structured command, or a
`subst` template with unsupported backslash forms), it **falls through to
the generic call IR** rather than producing incorrect specialised IR.

This fallback-to-runtime pattern is intentional and preserves correctness.
Helpers that signal "I cannot handle this" (returning `None`/an error) are
not incomplete — they are conservative by design. The runtime interpreter
handles the full Tcl specification; the compiler only inlines what it can
prove is safe.

See `docs/design/compiler/lowering-dispatch.md` for the dispatch hierarchy.

## Word-token closing delimiters

A braced/bracketed/quoted word token follows the *inner-end* convention:
`Token.end.offset` is the last **inner** character and the closing `}` / `]` /
`"` is one past it — **except** an empty `{}` / `[]` / `""`, whose `end` already
sits *on* the closer. Never re-derive the closer as `end.offset + 1` (it
overshoots the empty case by one — issue #527).

- **Command/word ranges come from the concrete syntax tree** the segmenter
  builds. The authoritative command span is the segmented-command / IR
  statement range: its end is the *boundary* (the `SEP`/`EOL` the lexer emits
  after the last word) minus one — covering the closer for braces, brackets,
  quoted, empty `{}`/`""`, and compound (`{a}b`) words, with no source re-scan.
  **Trust it** rather than re-deriving a command's span. The segmenter derives
  this from the canonical red-green CST in `rust/tcl-syntax`
  (`docs/design/compiler/syntax-tree.md`) — the lossless, position-independent
  tree the formatter, minifier, AOT lowering, and per-command tooling build on.

See [`docs/kcs/kcs-issue-highlight-drops-closing-delimiter.md`](docs/kcs/kcs-issue-highlight-drops-closing-delimiter.md)
for the contract.

## Command registry

Command metadata lives on the `CommandSpec` type in `rust/tcl-registry`,
**not** in hardcoded sets scattered across consumer modules. Commands are
defined in the registry's per-dialect spec packs (Tcl, F5 iRules/iApps, EDA).

When a consumer needs to know something about a command (e.g. "is this an
action?", "does this mutate state?"), add a field to `CommandSpec`, a query
method to the registry, and set the flag on the relevant command specs. Do
**not** create an ad-hoc set of command names in the consumer module.

### Argument role resolution order

Three mechanisms assign argument roles (BODY, EXPR, VAR_READ, VAR_WRITE, etc.)
to command arguments. They are evaluated in **priority order**:

1. **`arg_role_resolver`** (dynamic) — a callback that inspects the actual
   argument list and returns a role map. Used for variable-arity commands
   where roles depend on argument count or values (e.g. `set` distinguishes
   read vs write by whether a value argument is present; `if` maps bodies
   and expressions by keyword position).
2. **`arg_roles`** (static) — a fixed `dict[int, ArgRole]` on the spec.
   Sufficient when every call has the same argument layout.
3. **`assigns_variable_at`** (legacy shorthand) — marks a single argument
   index as a variable write. Overridden by the dynamic resolver when one
   exists.

The dynamic resolver takes priority over static fields. When reviewing a
command spec that has both `assigns_variable_at` and `arg_role_resolver`,
the resolver is the authority — the static field is a fallback for consumers
that do not call the resolver.

See `docs/design/compiler/command-registry.md` for the full field reference.

### Compound commands and multi-module handling

Tcl compound commands like `namespace upvar`, `namespace eval`,
`dict for`, `string map`, etc. are tokenised as a base command
(`namespace`, `dict`, `string`) with a subcommand argument. Different
analysis passes handle these at different levels:

- **Subcommand dispatch** in the registry uses `SubCommand` entries on the
  parent spec. The `arg_role_resolver` on the parent inspects the
  subcommand word to assign roles.
- **Variable scoping** has explicit handling for compound forms like
  `namespace upvar`, `dict set`, `dict update`, etc. — distinct from the
  declaration handling for single-word commands (`global`, `variable`,
  `upvar`).
- **Lowering** (in `tcl-compiler`) has per-command hooks that understand
  subcommand structure.

When checking whether a compound command is handled, search all three
layers — not just the one closest to the symptom.

## Testing

- Test framework: **cargo test** (Rust workspace) + Zig `zig build test`
  (WASM runtime) + the VS Code extension harness.
- Rust tests: `make test-rust` (`cargo test --workspace --all-features`).
- The native LSP end-to-end suite lives at
  `rust/tcl-lsp-server/tests/*_e2e.rs` (30 suites, run by `cargo test`) — it is
  **not** pytest, and there is no zipapp build step. VS Code extension tests:
  `make test-ext` (or `make test-ext-rust` to drive the native server via
  `TCL_LSP_SERVER_KIND=rust`).
- **Registry contract & behaviour tests**
  (`rust/tcl-lsp-server/tests/registry_contract_e2e.rs`): front-end-driven
  coverage of the whole command registry and the iRules
  event/profile/object graphs.  The registry **generates** real Tcl
  scripts and iRules (`when EVENT { … }`) and the tests assert the live
  front-end analysis — arity (E002/E003), subcommands (E001/W001), event
  scoping (IRULE1001/1002), event ordering, and the LSP `executeCommand`
  registry handlers.  See
  [`docs/design/contracts/registry-contract-tests.md`](docs/design/contracts/registry-contract-tests.md).
- **iRule test framework** (`rust/tcl-irule-test`): simulates TMM for testing
  iRules without hardware.  See
  `docs/design/contracts/irule-test-framework.md` for architecture.
- **WASM runtime tests** (`runtime/zig/test_*.zig`): unit tests for the Zig
  runtime, run with `cd runtime/zig && zig build test`. Tests that need to
  catch a Tcl-level error or set up a call frame use the fixture in
  `runtime/zig/runtime_test_fixture.zig` — `with_catch(body)` returns the
  raised error message (or `null` on success), `with_interp(body)` pushes a
  fresh global frame around *body*, and `frame.set` / `frame.get` are
  shorthand for `local_set` / `local_get`. Smoke coverage lives in
  `runtime/zig/test_fixture.zig`.
- **xfail policy**: an expected-failure / `#[ignore]` marker is only permitted
  as an intermediate state while a feature is under active development. Before a
  feature is considered ready for release, all underlying issues must be fixed
  and the markers removed. Do not ship xfails — fix the root cause instead.

## Common tasks

**Format the code:**
```
cargo fmt            # Rust
make format          # TypeScript (Prettier)
```

**Run just the Rust tests (includes native lsp_e2e):**
```
make test-rust
```

**Rust lint / clippy + drift gates:**
```
make rust-check      # cargo fmt --check + clippy + xtask drift gates
# or just:  cargo clippy --workspace --all-targets
```

**Run the TypeScript linters:**
```
make lint
```

**Regenerate editor catalogs + settings after a diagnostic/optimisation change:**
```
make codegen
```
