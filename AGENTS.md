# AGENTS.md — development guide for AI agents

## Project overview

tcl-lsp is a Tcl Language Server Protocol implementation. The LSP server and
all tooling are native Rust; editor integrations are in TypeScript (VS Code),
Rust (Zed), and Gradle/Kotlin (JetBrains), plus Neovim/Emacs/Helix/Sublime
configs. It supports Tcl 8.4–9.1, F5 iRules/iApps, and EDA tool dialects.

Python has been fully retired on this branch — the product is the Rust
workspace (`rust/`) plus the native binaries it builds.

## Repository layout

The product is a Cargo workspace: ~45 crates under `rust/` (the authoritative
list is `[workspace] members` in the top-level `Cargo.toml`), plus editor
integrations under `editors/` and the Rust WASM runtime under `runtime/rust/`.
The native binaries are the cargo bins `tcl-lsp-server`, `tcl`, `f5-query`,
and `tcl-mcp`.

Key crates under `rust/` and their one-line roles:

```
tcl-lexer / tcl-syntax    Lexer + concrete/red-green syntax tree (CST).
tcl-compiler              IR / CFG / SSA / optimiser / codegen (WASM emitter).
tcl-registry              Command + dialect registry; dialect detection.
tcl-spectcl               SpecTcl `.tclspec` packs: loader, discovery, cache, install.
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

Supporting crates (core types, platform, userdirs, host, regex, bytecode,
runtime-api, sandbox, LSP db, bpf-tcl-*, irule-test, …) are also listed in
`Cargo.toml`.

```
editors/          Editor integrations (VS Code, Zed, JetBrains,
                  Neovim, Emacs, Helix, Sublime).
runtime/rust/     Rust WASM runtime that the compiler's WASM codegen
                  targets (leak round-trip + eval suite).
scripts/          Build, release, codegen, and dev automation.
samples/          Sample Tcl, iRules, and BigIP configs.
docs/             Design docs, KCS notes, references, perf reports.
```

The native LSP end-to-end suite lives at
`rust/tcl-lsp-server/tests/*_e2e.rs` (30 suites, run by `cargo test`).

## Shared semantic owner map

The full, machine-checked owner map is
[`docs/design/contracts/shared-utility-contracts-rust.md`](docs/design/contracts/shared-utility-contracts-rust.md).
The current owners and their semantic axes are:

| Surface | Owner | Axis |
|---|---|---|
| Names / namespaces | `tcl-syntax::naming` + `tcl-cmd-core::namespace` | invariant; #1493 absolute marker |
| Lists | `tcl-syntax::list` | invariant |
| Dicts | `tcl-syntax::value::ValueOps::dict_pairs` + list codec | invariant |
| Numbers | `tcl-syntax::number` | `NumberSyntax` per release |
| Backslash escapes | shared lexer/syntax decoder | `LexerGrammar::escapes` per release |
| Quotes, braces, word spans | `tcl-lexer::ranges` | close rule per release; tmsh mode per dialect |
| Indices | `tcl-cmd-core::index` | grammar-parameterised; number axis |
| Option words / subcommands | `tcl-cmd-core::prefix` + registry tables | release/dialect surface |
| Expr grammar / evaluation | `tcl-syntax::expr` + `RuntimeExprSurface` | per release |
| Command / word segmentation | `tcl-compiler::segmenter` over the red-green CST | `LexerConfig` per document dialect |
| Tcl comment lines | `tcl-lexer::comment_line_starts` plus registry-declared script bodies | resolved `LexerConfig` and registry |
| iRules `when EVENT` blocks | `tcl_syntax::event_handler::{event_handlers, script_commands}` for supplied script regions + `tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver`, with `tcl_irules::when_blocks` wrapper | caller-supplied `LexerConfig`; offset-keyed resolved command identity at the top-level iRules boundary |
| Per-command knowledge | `tcl-registry::CommandSpec` and descriptors | per release/dialect |
| Dialect / release facts | `tcl-dialect::DialectProfile` | resolved profile axis |

Run `cargo xtask owner-resolution` (included in `make rust-check`) after
moving an owner or changing one of these axes. Do not add a new owner-shaped
implementation without updating the contract and its gate.

## The registry is the source of truth — no per-command logic elsewhere

Per-command knowledge lives in **`tcl-registry`** (`CommandSpec` and its
attached descriptors), never as `match cmd_name { "foo" => … }` special-casing
in the compiler, analyser, or LSP.  The compiler / analyser / LSP are **generic
consumers** of registry data; adding or extending a command is editing (or
adding) a spec, not adding a branch to a walker.

When a command needs behaviour the existing `CommandSpec` fields can't express,
**extend `CommandSpec`** — add a field and, if needed, a small descriptor type
or a hook ID the compiler/analyser can dispatch on (see `hooks.rs`,
`side_effects`, `taint_*`, `arg_role_resolver`, `definition_body`) — rather than
teaching a consumer about the command by name.  Established examples:

- **Argument roles** (`arg_roles` / `arg_role_resolver`, `ArgRole`) drive
  variable-write / body / param-list / loop-var / expr highlighting and
  analysis generically — the LSP token walk never names a command.
- **Definition-body grammars** (`definition_body`, `tcl_registry::definer`)
  describe a class/type *definer*'s body: its member sub-keywords (`method`,
  `typemethod`, `constructor`, `variable`, …) with their body / param / var
  layout (`MemberKind::Flat`), the nested-member wrappers (`self`, itcl's
  `public`/`protected`/`private` — `MemberKind::Wrapper`), the flag-keyed forms
  (`property` — `MemberKind::FlagKeyed`), plus implicit member-body variables.
  TclOO, snit **and** [incr Tcl] are pure registry data; the shared walker in
  `tcl-lsp-core/src/oo_body.rs` (folding + semantic tokens) contains **no**
  command names — it dispatches on `MemberKind`, never a keyword.  A new definer
  (xotcl, a bespoke class system) is a new `DefinitionBodyGrammar` + a
  `DefinerFamily` arm, not new walker code.
- Taint / side-effect / const-fold / lowering / codegen behaviour is likewise
  spec-declared and dispatched via typed hook IDs.

Migration debt is tracked, not grandfathered: when a consumer hardcodes a
command, the goal is to move that knowledge into the registry, not to add more
of it in the consumer.  The analyser's snit/OO *body* parser
(`tcl-compiler/src/analyser/oo.rs`) has completed this migration — member
**recognition** (`is_member` / `member`) and **argument layout** (which word is
the name / parameter list / body / variable, via `MemberSpec::indices_for`) come
entirely from the definer's `definition_body` grammar for both TclOO and snit;
the walkers hold no hardcoded member-keyword list or arg-index literal.  What
stays analyser-local is the small, irreducible *semantics* the registry does not
model: routing a member to its `ClassDef` field / `MethodDef` kind, and whether a
body opens a method scope (an object `destructor` and a class-level `initialise`
are structurally identical single-body members — the difference is analyser scope
modelling, not command structure).  Those are documented at their call sites and
are *not* debt.

## Prerequisites

- Rust stable with cargo, via [rustup](https://rustup.rs/).  The toolchain
  tracks the floating `stable` channel pinned in `rust-toolchain.toml`, so
  `Cargo.toml`'s `rust-version` bumps whenever stable does.  Current stable is
  1.97.0, released 2026-07-07.  CI resolves `stable` at run time, so a fresh
  release can fail `pr-gate`'s `cargo clippy -D warnings` on untouched code the
  day it lands — `rustup update` before debugging a clippy failure you cannot
  reproduce locally.
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

After the toolchains land it also installs the remaining host test tools
via [`scripts/dev/ensure-test-deps.sh`](scripts/dev/ensure-test-deps.sh)
(tclsh, node, kotlinc, emacs, xvfb, tshark, …).

| Tool / source    | Version       | Install path                    | On `PATH` as              |
|------------------|---------------|---------------------------------|---------------------------|
| rsync, xz-utils  | distro        | `/usr/bin/`                     | `rsync`, `xz`             |
| Wasmtime         | v47.0.3       | `/opt/wasmtime-47.0.3/`         | `/usr/local/bin/wasmtime` |
| Binaryen         | v132          | `/opt/binaryen-132/`            | `/usr/local/bin/wasm-merge`, `/usr/local/bin/wasm-opt` |
| wasi-sdk         | 33.0          | `/opt/wasi-sdk-33.0/` (symlink `/opt/wasi-sdk`) | — (found by `runtime/rust/build.rs`) |
| rustup + Rust    | floating `stable` (currently 1.97.1) | `/root/.rustup`, `/root/.cargo` | `/usr/local/bin/{cargo,rustc,rustup,rustfmt,clippy-driver}` |
| Tcl 8.4 source   | 8.4.20        | `tmp/tcl8.4.20/`                | —                         |
| Tcl 8.5 source   | 8.5.19        | `tmp/tcl8.5.19/`                | —                         |
| Tcl 8.6 source   | 8.6.16        | `tmp/tcl8.6.16/`                | —                         |
| Tcl 9.0 source   | 9.0.4         | `tmp/tcl9.0.4/`                 | —                         |
| tcllib           | 2.0           | `tmp/tcllib-2.0/`               | —                         |

Notes on the fetched sources:

- Tcl and tcllib are full source trees (`generic/`, `unix/`, `win/`, `tests/`,
  `library/`, `doc/`, …) pulled as release tarballs from
  `codeload.github.com`. Tarballs are GitHub-CDN cached, smaller than a git
  clone, and friendlier to the upstream Tcl project than hitting
  `tcl.tk`/`sourceforge.net` on every cold session.
- The hook is idempotent — warm containers re-run it and finish in seconds.

To bump any of these versions, edit the pinned variables at the top of
[`.claude/hooks/session-start.sh`](.claude/hooks/session-start.sh)
(`WASMTIME_VERSION`, `BINARYEN_VERSION`, `WASI_SDK_VERSION`, `TCLLIB_TAG` /
`TCLLIB_VERSION`; Rust tracks the floating `stable` channel via
`RUST_TOOLCHAIN` and needs no version bump) and, for Tcl, the version/tag maps in
[`.claude/skills/fetch-tcl-source/fetch_tcl_source.sh`](.claude/skills/fetch-tcl-source/fetch_tcl_source.sh).

### Version requirements — sources of truth and update checklist

The **source of truth** for each minimum version:

| Requirement | Source of truth              | File                  |
|-------------|------------------------------|-----------------------|
| Rust        | current stable               | `Cargo.toml` workspace `rust-version` (authoritative, tracks stable); `rust-toolchain.toml` pins the channel; `Makefile` Prerequisites echo it |
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
| `make check-all`   | **Pre-push gate** — full lint + typecheck across **every** language: TypeScript via ESLint + Prettier + tsc, Rust via `cargo fmt --check` + `cargo clippy`, Python via `ruff` + `ty` + `pyright` (`lint-py` + `typecheck-py`). Run before every push. |
| `make install-test-deps` | One-shot setup: install **everything** the full test suite needs (the system toolchain — all of `ensure-test-deps`). The target to run on a fresh checkout before running the heavier suites below. Same platform coverage as `ensure-test-deps`. |
| `make ensure-test-deps` | Install the optional host toolchain (`tclsh9.0`, `node`+`npm`, `kotlinc`, Rust/rustup, Wasmtime, Binaryen, wasi-sdk, emacs, xvfb, …) on Debian/Ubuntu (apt-get), CentOS/RHEL/Rocky/Alma/Fedora (dnf or yum), or macOS (Homebrew). Idempotent. Builds Tcl 9 from `tmp/tcl9.0.4/` since most distros don't package it yet. Skip individual tools with `SKIP_TCLSH=1`, `SKIP_NODE=1`, `SKIP_KOTLINC=1`, `SKIP_RUST=1`, … Run `bash scripts/dev/ensure-test-deps.sh --check` for a non-mutating report of what would be installed. |
| `make ensure-rust-deps` | Install Rust/rustup + the `wasm32-wasip2` target needed by `check-rust` / the WASM build. |
| `make check-rust`  | Rust format check + clippy across the workspace (and the Zed extension). Skip with `SKIP_CHECK_RUST=1`. |
| `make prep-pr`     | The standard local gate: auto-formats code, runs codegen, lint/typecheck, and the smoke tier (`make smoke`). Deep suites run in CI — see "Workflow requirements" below. |
| `make smoke`       | Fast per-module smoke tier (`cargo nextest run --profile smoke`): the `smoke_*` / `*_smoke.rs` subset, seconds warm, reusing the existing dev build. `make smoke-p P=<crate>` scopes it to one crate. |
| `make test-exhaustive` | Manual-only tier: every `#[ignore]`d corpus sweep / fuzz gate / privileged test (`--run-ignored ignored-only`). Never run automatically. |
| `make fuzz`        | tcl-fuzz differential campaign (manual-only; see the fuzz-findings skill). |
| `make test`        | **The CI-mirror test gate** — everything except Emacs: `test-rust` + `test-ext` + `runtime-rust-test` + `zed-query-check`. Use to reproduce CI locally; not required before a PR. |
| `make test-rust`   | `cargo test --workspace --all-features` — includes the native lsp_e2e suite (`rust/tcl-lsp-server/tests/*_e2e.rs`); skip with `SKIP_TEST_RUST=1` |
| `make test-ext`    | VS Code extension integration tests — the single-root suite **and** the multi-root (`test:multi-folder`) suite (xvfb on headless Linux) |
| `make lint-py`     | `ruff format --check` + `ruff check` over every tracked `.py` (versions pinned in the Makefile) |
| `make format-py`   | `ruff format` over every tracked `.py` |
| `make typecheck-py`| `ty` + `pyright` over every tracked `.py`.  Builds `.venv-typecheck`, which installs `f5report` (maturin-compiling the native `_engine`) plus pytest, so both checkers resolve every import for real.  Sublime host APIs are declared by stubs under `typings/`. |
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
| make update-source-data | Explicitly refresh the embedded SslicTcl trust-store/TLS source bundle (network-capable; run only when intentionally updating provenance) |
| make check-source-data | Offline verification of the embedded SslicTcl source bundle, provenance, hashes, licences, and deterministic generated output |
| `make publish-flow` | Print the release + marketplace publish cheat-sheet |
| `make release-prepare V=X.Y.Z` | 2.1.x pre-release: preflight, benchmark the release, regenerate the release-notes graphs, write the perf notes section, verify, commit |
| `make release-verify V=X.Y.Z` | Check `scripts/perf/{results,graphs}` and `RELEASE_NOTES.md` agree for X.Y.Z |

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

### Parallel worktrees and agent build isolation

**Never share one `CARGO_TARGET_DIR` across concurrently-building git
worktrees of this workspace.**  Cargo's unit hashing does not reliably
disambiguate workspace-member crates built from different worktree paths of
the same workspace (same package name/version/features), so concurrent
builds race on the same `deps/` outputs.  Observed failure modes (all real,
from the 2026-07-29 multi-agent session — issue #1052): an xtask binary
linking a **sibling branch's** `libtcl_registry` rlib and failing the drift
gate on a diagnostic that branch doesn't have; phantom `E0603` privacy
errors naming line numbers from a different checkout; alternating
cannot-find/no-variant errors on symbols plainly present in the tree;
a full-suite run reporting failures from a pre-fix `libtcl_compiler` that
had already been fixed.  A green or red produced this way is
**untrustworthy** — the test ran against code that is not in the tree
being tested.

Rules:

- Give each worktree its **own** `CARGO_TARGET_DIR` — the per-worktree
  default (`<worktree-root>/target`) is already correct; the hazard only
  appears when a shared dir is exported to save disk.
- To keep per-worktree dirs affordable, build with `CARGO_INCREMENTAL=0`
  and `CARGO_PROFILE_DEV_DEBUG=0` (~3-4 GB per dir instead of ~15 GB).
- Sharing **`CARGO_HOME`** (the registry/git dependency cache) across
  worktrees is safe — external-crate artefacts do not exhibit the
  collision, only workspace members do.
- Treat any build or test result produced **after an ENOSPC event** as
  untrustworthy until a clean rebuild — disk-full aggravates the
  fingerprint corruption.  `cargo clean -p <crate>` (or `touch`ing the
  crate's `lib.rs`) recovers a wedged crate; neither prevents recurrence
  while a target dir stays shared.

`source scripts/dev/agent-build-env.sh` pins all of this for the current
worktree (`--check` shows what it would set and warns if the current
`CARGO_TARGET_DIR` points elsewhere).  Agent instructions should reference
that helper rather than repeating the env-var incantations.

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

The 2.1.x line has one thing to do **before** the tag: measure the release
and regenerate the release-notes performance graphs.
`scripts/release/rust_release.sh` drives that whole sequence
(`next` → `preflight` → `prepare` → `tag`); `prepare` benchmarks the tree
about to be tagged, commits `scripts/perf/results/X.Y.Z.json` alongside the
re-rendered `scripts/perf/graphs/`, and writes the `## Performance` section
of `RELEASE_NOTES.md` — never hand-edit that section, its four release-asset
URLs are the easiest thing in a release to leave pointing at the previous
version. `verify` (re-render the graphs, diff against the committed ones)
gates the tag, and the `graphs-current` job in `perf.yml` runs the same
check on every push.

## Embedded SslicTcl source data

The report builder embeds all SslicTcl trust-store and TLS source data. The
refresh covers Trust Stores Observatory root programs plus the pinned Chromium
Chrome Root Store (including its textproto constraints). Normal builds,
reports, CI, and report viewing never fetch it. To intentionally refresh
upstream material, run:

~~~text
make update-source-data
make check-source-data
~~~

The refresh is the only network-capable operation and records source URLs,
revisions, retrieval dates, licences, and SHA-256 file hashes in the
rust/tcl-sslictcl/data/provenance.json manifest. make rust-check includes the
offline check. Before a release, refresh the data and verify it with
make check-source-data SOURCE_DATA_MAX_AGE_DAYS=180; a blocked upstream may
only be waived with SSLICTCL_SOURCE_DATA_WAIVER and a documented reason in
the release preparation notes. See
docs/design/contracts/sslictcl-source-data.md.

## WASM command parity

The Rust command specs in **`tcl-registry`** are the **source of truth**
for which Tcl 8.4-9.1 commands exist.  The WASM runtime backing — the
Rust runtime under `runtime/rust/` that the compiler's WASM codegen
targets — must stay aligned with the registry: every command in the
registry needs runtime backing (a real handler, an interpreter-fallback
path, or an explicit "not required" classification such as the
`tcl::mathop::*` prefix-form operators).  The runtime port is a distinct
workstream (`runtime/rust/`).

This contract is enforced by the **`cargo xtask command-backing`** drift
gate (wired into `make xtask-check`): it cross-checks the registry's core
Tcl 9.0 command specs against the runtime's `register_builtin`
registrations, classifies the residue (stdlib fallback / not-required /
known-gap), and writes
[`docs/generated/wasm-command-backing.md`](docs/generated/wasm-command-backing.md).
`--check` fails on report drift, on any unclassified command (a new gap),
or on a stale classification — so the registry and runtime cannot silently
diverge.  A genuinely-missing command that is a real gap goes on the
`KNOWN_UNBACKED` allow-list in `rust/xtask/src/command_backing.rs` until it
gains a handler.

For a walkthrough of how a Tcl script becomes a WASM module (the
6-phase codegen pipeline, per-statement dispatch order, per-command
file layout), see
[`docs/design/compiler/wasm-codegen.md`](docs/design/compiler/wasm-codegen.md).

When you add a Tcl command, add both the registry `CommandSpec` (in
`tcl-registry`) and its runtime backing in the same change so the two
describe the same command surface, arity bounds, and sub-commands.

## Optional WASM extensions

Current Rust WASM builds can enable the `wasm_stdlib` feature in
`runtime/rust/Cargo.toml`. It embeds Tcl scripts, package indices, and the
Tcl-level `tcltest` package in the runtime VFS. This lets a filesystem-backed
interpreter load scripts through normal `source` and `package require`
machinery.

There is not yet a compiler package-require scan, extension selector, variant
runtime artefact, or compiled Tcltest C-tier `test*` command surface. The
runtime's embedded Tcltest files must not be described as a port of those C
commands. Package-driven extension bundling is documented only as **Future
desired state — written and reviewed 2026-08-11** in
[`docs/design/compiler/wasm-extensions.md`](docs/design/compiler/wasm-extensions.md).

## Workflow requirements

Testing is **tiered**: a seconds-fast local smoke tier, the deep suites in
CI, and a manual-only exhaustive tier that never runs automatically.

| Tier | When it runs | What runs |
|---|---|---|
| **smoke** (`make smoke`, `make smoke-p P=<crate>`) | locally, right after every compile; part of `make prep-pr` | the `smoke_*` / `*_smoke.rs` subset — one fast per-module sanity check per crate, seconds warm.  Reuses the dev-profile default-features build you just made (never `--all-features`), so it never forces a recompile |
| **deep** (CI: `rust-tests`, `rust-tests-heavy`, `lsp-e2e`, `test-ext`, `cargo-deny`, `python`) | every PR and every push to `rust`/`main`, automatically | the full workspace suite (native lsp_e2e included), the VM-sim heavies, the VS Code extension, supply-chain audit, Python lint/typecheck.  CI skips work only when its inputs demonstrably didn't change (see "CI redundancy contract" below) |
| **exhaustive / manual-only** (`make test-exhaustive`, `make fuzz`, `make tcltest-sweep[-check]`) | only when a human (or a deliberate scheduled job) invokes it by name | every `#[ignore]`d corpus sweep over `tmp/tcl*`/tcllib, differential-fuzz gates, privileged bpf/kernel tests, fuzz campaigns.  NEVER wire these into `prep-pr`, `test`, `check-all`, or CI |

**Fuzzing is always manual.**  This covers campaigns (`make fuzz` /
`tcl-fuzz`) *and* fuzz-shaped `#[test]`s — anything whose body is
generator-driven or seeded-random exploration (edit storms, permuted-input
loops) is `#[ignore]`d into the manual tier, regardless of how fast it
happens to be today.  Deterministic fixed-input tests covering the same
code stay in CI.  (`tcl-fuzz`'s own plumbing unit tests are ordinary tests
of the fuzzer's code, not fuzzing — they run in CI's `rust-tests-heavy`
partition.)

The two local gates, in the order you run them:

```
make rust-check     # Rust fmt + clippy + drift gates (mirrors CI's pr-gate)
make prep-pr        # format + codegen + lint/typecheck + make smoke
```

`make check-all` (multi-language lint + typecheck) is part of `prep-pr`'s
check phase in spirit; run it directly when you've touched TypeScript or
Python and want just the lint/typecheck surface.

### Before any push

**Format, lint, and smoke must be clean before you push.**

```
make rust-check     # Rust-only changes: the minimum gate
make prep-pr        # the standard gate: format + codegen + lint/typecheck + smoke
```

Failures must be fixed, not skipped — tooling-missing skips must be
deliberate (`SKIP_CHECK_RUST=1`, ...).  Commit any formatting changes
`prep-pr` applies.

**Agent rule — Claude / codex / etc:** `make prep-pr` (which includes
`make smoke`) is the gate before every `git push`.  CI's `pr-gate` job runs
the same fmt + clippy + drift checks, so a local pass means CI passes the
fast gate on the first try.  Running `cargo fmt` after a commit isn't
enough — re-run the gate.  Every "the PR gate bounced on a trivial clippy
lint / format drift" failure on this repo's PRs has been a push that
skipped this step.

### Opening a PR: CI carries the deep suites

Rebase off the branch you are targeting (`rust` for 2.x work, `main` for
1.x stable), fix conflicts, run `make prep-pr`, and open the PR.  **Do not
block on running the full suite locally** — CI runs the deep tier
(`rust-tests` + `rust-tests-heavy` + `lsp-e2e` + `test-ext` + `cargo-deny`
+ `python`) on every PR, in parallel, in a few minutes.  Subscribe to the
PR's activity and fix forward when a deep suite fails.

`make test` (workspace + extension + runtime port + Zed query check) and
`make test-emacs` remain available for reproducing a CI failure locally or
for extra confidence on a risky change — they are no longer a precondition
for opening a PR.  The one gap: **Emacs eglot tests run nowhere in CI**, so
touch `editors/emacs` only with a local `make test-emacs` run.

**Known eglot xfail — upstream bug, not ours.**  The eglot semantic-token
repaint failures are an **expected failure caused by an upstream eglot
painter bug** (issue #333: `eglot--semtok-font-lock-2` stacks
`eglot-semantic-*` faces from stale properties while a semantic-tokens
response is in flight).  The server is verified correct by the spec-correct
reference client
(`rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs`), so a
`test-emacs` failure in that area is NOT a server regression: do not chase
it, do not block a push or PR on it, and do not "fix" it server-side.  See
`editors/emacs/README.md` and `scripts/eglot_test/` for the repro and the
evidence.

### CI redundancy contract

CI avoids re-testing what demonstrably didn't change, and the rules live in
`ci.yml`'s `channel` job (read its header comment before touching them):

- a **tag** whose SHA already went green on a `rust`/`main` push within 24 h
  step-skips the test surface (the release graph still runs);
- a **merge push** whose tree is byte-identical to its already-green PR head
  downgrades tests to a cache-warming build (`--no-run`);
- **docs-only changes** skip the cargo test steps; the `python` and
  `test-ext` suites run only when their input paths changed;
- `cargo-deny` never skips (new advisories arrive against unchanged trees);
- every skip fails safe: API error or ambiguity → run everything.

Keep these properties when editing CI: skips must be **step-level** (jobs
still report success so required checks and the release `needs:` graph stay
intact), verified against **content identity** (tree/SHA, never a label or
commit message), and bounded in time.

### Capturing build / test logs

Long gates (`make test-rust`, `make test-ext`, anything running for more
than a few seconds) MUST have their full output captured to a file under
`/tmp/` rather than only being read via `tail`.  Tailing loses signal: a
failure in the middle of a 10-minute run won't appear in the last 50 lines
if the harness keeps going, and cargo/test summary lines can get pushed
off the bottom by skip-message spam.  The pattern:

```
make test-rust 2>&1 | tee /tmp/test-rust-<branch>.log
# then, when investigating a failure:
grep -nE 'FAIL|ERROR|panicked|error\[|^E ' /tmp/test-rust-<branch>.log
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

### KCS — the six categories

Every KCS note is exactly one of these six types. Pick the category first,
then copy the matching template from [`docs/kcs/templates/`](docs/kcs/templates/README.md).

| Type | The question it answers | Template |
|---|---|---|
| **Issue** | Why is X not working, and how do I fix it? | [`kcs-template-issue.md`](docs/kcs/templates/kcs-template-issue.md) |
| **Q&A** | What is X? / When should I use Y? | [`kcs-template-qa.md`](docs/kcs/templates/kcs-template-qa.md) |
| **How-To** | How do I do X? | [`kcs-template-how-to.md`](docs/kcs/templates/kcs-template-how-to.md) |
| **Functionality** | What does command/feature/tool X do, and how do I use it? | [`kcs-template-functionality.md`](docs/kcs/templates/kcs-template-functionality.md) |
| **Diagnostic** | Per-code page for an E/W/S/T/IRULE diagnostic. | [`kcs-template-diagnostic.md`](docs/kcs/templates/kcs-template-diagnostic.md) |
| **Optimisation** | Per-code page for an O-code optimiser rewrite. | [`kcs-template-optimisation.md`](docs/kcs/templates/kcs-template-optimisation.md) |

Every KCS note starts with a blockquote header naming its audience and
type:

```markdown
# KCS: <short title>

> **Audience:** User | Contributor | Maintainer
> **Type:** Issue | Q&A | How-To | Functionality | Diagnostic | Optimisation
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
    `strength-reduce`, `pattern`, `codegen`). The tables in
    [`docs/kcs/STYLE.md`](docs/kcs/STYLE.md) (rule 11) are the canonical
    vocabulary: `rust/tcl-cli/build.rs` indexes whatever tokens the
    `## Applies to` line carries rather than validating them, so an
    unrecognised tag is silently indexed, not rejected. Per-code
    pages and compiler-internals feature pages must carry the
    compiler-pass tag of the pass that produces the code or the
    facts they consume.
13. **If the answer differs per editor or tool, split it into
    sub-headings** under the answer section, in the same order as
    `## Applies to`. Do not bury per-editor differences in inline
    asides.
14. **A fixed bug that needs no reader action is not a KCS note.** If the
    only thing anyone has to do is be on a current build, the fix belongs
    in the changelog and the release notes — not in the knowledge base,
    where it leaves a reader unable to tell a historical fault from a live
    one. Write the note only when something survives the fix: a version
    range or restart/setting/cache step the reader must act on, a boundary
    that still reports nearby (say which, and why), or a symptom with
    several possible causes the note helps tell apart. The test: delete
    every sentence that only says the bug is fixed, and see whether
    anything actionable is left.

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
| Module ownership or API contract | `docs/design/contracts/` | `shared-utility-contracts-rust.md` |
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

A small set of features cannot follow this pattern because their capability
advertisement is decided during `initialize`, which alters client behaviour
for the session. There are currently no user-configurable restart-required
toggles.

Diagnostics are deliberately not one: the server always advertises the push
model and never advertises `diagnosticProvider`. Its
`textDocument/diagnostic` and `workspace/diagnostic` handlers remain available
for direct requests against the shared cache, but a setting must not attempt to
switch the delivery model after initialisation. See
`docs/design/contracts/lsp-diagnostics-publication.md` for the delivery
contract.

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

See [`docs/design/compiler/syntax-tree.md`](docs/design/compiler/syntax-tree.md)
for the contract.

## Command registry

Command metadata lives on the `CommandSpec` type in `rust/tcl-registry`,
**not** in hardcoded sets scattered across consumer modules. Commands are
defined in the registry's per-dialect spec packs (Tcl, F5 iRules/iApps, Tk,
Expect, …) — **except the EDA vendor libraries**, which ship as bundled
`SpecTcl` loadables in `specs/*.tclspec` and reach a registry only through
`tcl_spectcl::bundled` (`docs/design/spec-packs.md`). Editing an EDA command
means editing the `.tclspec`, not a Rust module.

When a consumer needs to know something about a command (e.g. "is this an
action?", "does this mutate state?"), add a field to `CommandSpec`, a query
method to the registry, and set the flag on the relevant command specs. Do
**not** create an ad-hoc set of command names in the consumer module.

The same rule covers a command's **option** surface, and it has a drift gate:
**`cargo xtask audit-option-dialects --check`** (wired into `make xtask-check`
as `xtask-option-registry-drift`) fails when the tclsh dialect audit probes an
option no `OptionSpec` declares. The audit and the registry described different
surfaces once already — the audit probed `fconfigure -profile` (TIP 656) while
the registry had never heard of it, and the gap was found by hand (issue
#1396). The gate runs no tclsh, so it needs no built Tcl trees. A genuinely
missing option goes on the `KNOWN_UNSPECIFIED` list in
`rust/xtask/src/audit_option_dialects.rs` with its tracking issue, and the gate
fails on a waiver whose gap has closed. See
[`docs/design/compiler/command-registry.md`](docs/design/compiler/command-registry.md)
§ *The audit-registry option-surface gate*.

### Native specs and the SpecTcl DSL must maintain functional parity

Anything a native `.rs` `CommandSpec` (or one of its attached descriptor
types) can express, a `.tclspec` pack must be able to express too — and the
studio must round-trip it.  A registry change is **not complete** until all
four surfaces move together:

1. **Registry** — the new field/descriptor on the type, with its coverage
   witness in `rust/tcl-spec-studio/src/coverage.rs` (an exhaustive
   destructuring plus a `Field` table entry, so the build breaks until the
   studio knows about it).
2. **Loader** — a documented spelling in `rust/tcl-spectcl/src/loader.rs`,
   recorded in the frozen-syntax memo
   (`docs/design/spec-dsl-examples/README.md`) and its coverage matrix.
3. **Renderer** — `render_spectcl.rs` emits the spelling (non-default
   values only), and the `spectcl_roundtrip` gate proves draft → DSL →
   loader → draft loses nothing.
4. **Studio form** — `schema.rs` / `draft.rs` / `help.rs` surface it for
   editing.

The only sanctioned exception is an explicit entry in `render_spectcl.rs`'s
`GAPS` table naming the field, its documented spelling, and why it cannot
round-trip yet (`DraftOpaque` for genuinely opaque function pointers,
`Excluded` for deliberate design exclusions) — a `TODO(spectcl)` comment in
rendered output is the visible trace.  Never let the two spec forms drift
silently: a field that native specs can set but a pack cannot say, with no
`GAPS` entry, is a bug.  New DSL words are **additive** (the `speclib`
version word revs, e.g. 1.0 → 1.1; `VOCABULARY_VERSION` bumps only on
meaning changes) and the loader must keep accepting every older vocabulary.

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

- Test framework: **cargo test** (Rust workspace) + the VS Code extension
  harness.
- Rust tests: `make test-rust` (`cargo test --workspace --all-features`).
- The native LSP end-to-end suite lives at
  `rust/tcl-lsp-server/tests/*_e2e.rs` (30 suites, run by `cargo test`). VS Code
  extension tests: `make test-ext`, which builds the native `tcl-lsp-server` and
  points the extension at it via `TCL_LSP_SERVER_BIN`.
- **Registry contract & behaviour tests**
  (`rust/tcl-registry/tests/registry_sweep.rs`,
  `rust/tcl-registry/tests/registry_commands.rs`): coverage of the whole
  command registry and the iRules event/profile/object graphs.  The
  registry **generates** real Tcl scripts and iRules (`when EVENT { … }`)
  and the tests assert the live analysis — arity (E002/E003), subcommands
  (E001/W001), event scoping (IRULE1001/1002), and event ordering.  See
  [`docs/design/contracts/registry-contract-tests.md`](docs/design/contracts/registry-contract-tests.md).
- **iRule test framework** (`rust/tcl-irule-test`): simulates TMM for testing
  iRules without hardware.  See
  `docs/design/contracts/irule-test-framework.md` for architecture.
- **WASM runtime tests** (`runtime/rust/`): the Rust WASM runtime port carries
  its own leak round-trip + eval suite, gated separately via
  `make runtime-rust-test`.
- **`#[ignore]` policy**: `#[ignore]` has exactly one sanctioned steady-state
  use — a **permanently** expensive or environment-gated test (a corpus sweep
  over `tmp/tcl*`/tcllib, a differential-fuzz gate, a test needing root + a
  live kernel), marked with a reason string saying why and how to run it:
  `#[ignore = "<reason>; run explicitly with --ignored"]`.  These form the
  manual-only tier (`make test-exhaustive`) and are **not** debt.
- **xfail policy**: an expected-failure marker (a test that currently fails
  pending a fix) is only permitted as an intermediate state while a feature is
  under active development. Before a feature is considered ready for release,
  all underlying issues must be fixed and the markers removed. Do not ship
  xfails — fix the root cause instead, and do not confuse an xfail with the
  sanctioned manual-only `#[ignore]`s above.  One standing exception: the
  **eglot semantic-token repaint failures in `make test-emacs` are xfail
  because the defect is upstream in eglot itself** (issue #333, face-stacking
  in `eglot--semtok-font-lock-2`) — the server is proven correct by the
  reference client, so these stay xfail until the upstream fix ships, and
  they are not a release blocker.

## Common tasks

**Format the code:**
```
cargo fmt            # Rust
make format          # TypeScript (Prettier)
```

**Smoke-test after a compile (seconds):**
```
make smoke                    # whole workspace
make smoke-p P=tcl-compiler   # one crate
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
