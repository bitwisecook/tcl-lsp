# AGENTS.md — development guide for AI agents

tcl-lsp is a Tcl language server and toolchain: a native Rust workspace under
`rust/` (~45 crates; `[workspace] members` in `Cargo.toml` is the list) that
builds four binaries — `tcl-lsp-server`, `tcl`, `f5-query`, `tcl-mcp` — plus
editor integrations under `editors/` (VS Code, Zed, JetBrains, Neovim, Emacs,
Helix, Sublime) and the WASM runtime the compiler targets under
`runtime/rust/`. It covers Tcl 8.4–9.1, F5 iRules/iApps, and the EDA dialects.
Python is retired on this branch. `rust` is the only active branch;
`legacy-py` is a locked archive — never branch, merge, or tag from it.

Orientation: the crate map and dependency direction in
[project-layout.md](docs/design/contracts/project-layout.md); the compiler
pipeline in [compiler-architecture.md](docs/design/compiler-architecture.md);
what good Rust looks like here in
[engineering-guide.md](docs/design/rust/engineering-guide.md); terms in
[GLOSSARY.md](docs/GLOSSARY.md); every document indexed from
[docs/design/README.md](docs/design/README.md) and
[docs/kcs/README.md](docs/kcs/README.md).

## Gates before every push

```
make rust-check     # fmt + clippy + xtask drift gates; mirrors CI's pr-gate
make prep-pr        # format + codegen + lint/typecheck + smoke tier
```

- `rust-check` is the minimum for Rust-only changes; `prep-pr` is the gate
  before every `git push`. Fix failures, never skip them; commit the
  formatting `prep-pr` applies and re-run. Every "pr-gate bounced on a
  trivial lint" on this repo was a push that skipped this.
- **CI carries the deep suites.** Rebase on `rust`, run `prep-pr`, open the
  PR, subscribe to its activity, fix forward. Do not block on the full suite
  locally. To reproduce a deep-tier failure: `make test` (workspace,
  extension, runtime port, Zed query check), `make test-spectcl-compat`, and
  for the browser host `make lsp-server-wasm` then `npm run test:web` in
  `editors/vscode`. Emacs runs nowhere in CI — touch `editors/emacs` only
  with `make test-emacs`; its eglot semantic-token repaint failure is an
  upstream xfail (#333), never chase it.
- **Fuzzing is always manual.** Campaigns and fuzz-shaped tests
  (generator-driven or seeded-random bodies) are `#[ignore]`d into
  `make test-exhaustive` regardless of speed; deterministic fixed-input tests
  covering the same code stay in CI. `#[ignore]` has that one sanctioned use,
  always with a reason string; xfails never ship.
- Capture long gate output with `tee` to `/tmp/<gate>-<branch>.log` and
  `grep` it — a `tail` loses mid-run failures.

Tiers, policies, and the CI redundancy contract:
[test-tiers-and-ci-gates.md](docs/design/contracts/test-tiers-and-ci-gates.md).

## Environment

- Rust floating `stable` (`rust-toolchain.toml`), Node 24+ with
  `corepack enable npm`, then `make install-test-deps`. Remote agent sessions
  get all of it — toolchains, Wasmtime / Binaryen / wasi-sdk, the Tcl 8.4.20
  / 8.5.19 / 8.6.16 / 9.0.4 / 9.1b0 and Tk trees under `tmp/`, tcllib 2.0 —
  from `.claude/hooks/session-start.sh` before the first instruction; never
  `apt install` by hand. Versions and their owners:
  [development-environment.md](docs/design/contracts/development-environment.md).
- **Parallel worktrees:** never share a `CARGO_TARGET_DIR` across
  concurrently-building worktrees — cargo serves a sibling branch's rlibs and
  the resulting green or red is untrustworthy (#1052). `source
  scripts/dev/agent-build-env.sh` in each worktree; treat any result after an
  ENOSPC as suspect until a clean rebuild. Symptoms and recovery:
  [the KCS note](docs/kcs/kcs-issue-parallel-worktree-builds-serve-stale-artefacts.md).
- `make help` lists every target. `make codegen` regenerates every generated
  file (editor catalogues and settings, AI prompts, iRule-test data); the
  `xtask-check` drift gates fail when one is stale, so regenerate and commit
  alongside any diagnostic or optimisation change.

## Invariants

### The registry is the source of truth

Per-command knowledge lives in `tcl-registry` (`CommandSpec` and its
descriptors), never as `match cmd_name { "foo" => … }` in the compiler,
analyser, or LSP — those are generic consumers. When a command needs
behaviour the spec cannot express, **extend `CommandSpec`** — a field, a small
descriptor, or a typed hook ID the consumer dispatches on (`hooks.rs`,
`side_effects`, `taint_*`, `arg_role_resolver`, `definition_body`) — rather
than teaching a consumer about the command by name. Argument roles,
definition-body grammars (TclOO, snit, and itcl are pure data over
`MemberKind`; a new class system is a `DefinitionBodyGrammar` plus a
`DefinerFamily` arm, not walker code), taint, side effects, const-fold,
lowering, and codegen all take this shape. Migration debt is tracked, not
grandfathered: a hardcoded name moves into the registry, never multiplies. The
irreducible analyser-local semantics (routing a member to its `ClassDef`
field, whether a body opens a method scope) are documented at their call
sites and are not debt.
[command-registry.md](docs/design/compiler/command-registry.md),
[tcloo-implementation.md](docs/design/contracts/tcloo-implementation.md).

### Command registry

- Commands live in the registry's per-dialect spec packs — except the EDA
  vendor libraries, which ship as bundled `SpecTcl` loadables in
  `specs/*.tclspec` and reach a registry only through `tcl_spectcl::bundled`
  ([spec-packs.md](docs/design/spec-packs.md)). Edit the `.tclspec`, not Rust.
- Add a command's `CommandSpec` and its WASM runtime backing in the same
  change (see *WASM command parity*).
- Argument roles resolve `arg_role_resolver` → `arg_roles` →
  `assigns_variable_at`; the resolver is authoritative. Compound commands
  (`dict for`, `namespace upvar`) are a base command plus a subcommand word,
  handled by registry `SubCommand` entries and by hook IDs in the analyser,
  lowering, and codegen — check the spec's hook IDs before hunting for a
  missing branch.
- Two drift gates in `make xtask-check` keep the spec surface honest:
  `cargo xtask audit-option-dialects --check` (every option the tclsh audit
  probes has an `OptionSpec`; a `KNOWN_UNSPECIFIED` waiver expires when its
  gap closes — #1396) and `cargo xtask callback-inventory --check` (every
  script / command-prefix position sits in exactly one authored tier —
  [callback-surface-inventory.md](docs/design/contracts/callback-surface-inventory.md)).
- Native specs and the SpecTcl DSL keep functional parity: registry field,
  loader spelling, renderer, and studio form move together, or a `GAPS`
  entry says why not
  ([command-spec-studio.md](docs/design/contracts/command-spec-studio.md)
  § *Parity with native specs*).
- A SpecTcl 2.0 pack is evaluated, not walked: write canonical form unless
  repetition is the problem, run `tcl spec export` / `spectcl_expand` and
  read the expansion before shipping, prefer `-available` rows to
  `available?` ([spec-packs.md](docs/design/spec-packs.md) § *Authoring
  rules for SpecTcl 2.0*).

### Shared semantic owners

Names, lists, dicts, numbers, escapes, word spans, indices, option prefixes,
expr, segmentation, comment lines, `when` blocks, dialect facts, the C Tcl
oracles, platform bootstrap, and SslicTcl declarations each have one owner
crate and one semantic axis; never re-derive one, and never add an
owner-shaped implementation without updating the contract and its gate.
`cargo xtask owner-resolution` (in `make rust-check`) enforces it. Owner map:
[shared-utility-contracts-rust.md](docs/design/contracts/shared-utility-contracts-rust.md).

### WASM command parity

Every command in `tcl-registry` needs backing in `runtime/rust/` — a handler,
an interpreter-fallback path, or an explicit not-required classification.
`cargo xtask command-backing --check` cross-checks the two, writes
[wasm-command-backing.md](docs/generated/wasm-command-backing.md), and fails
on an unclassified command; a real gap goes on `KNOWN_UNBACKED` in
`rust/xtask/src/command_backing.rs` until it gains a handler. The
`wasm_stdlib` feature embeds Tcl scripts and the Tcl-level `tcltest` package
in the runtime VFS; it is not a port of the C `test*` commands, and
package-driven extension bundling is future state only. Pipeline:
[wasm-codegen.md](docs/design/compiler/wasm-codegen.md); extensions:
[wasm-extensions.md](docs/design/compiler/wasm-extensions.md).

### Word-token closing delimiters

A braced / bracketed / quoted token's `end.offset` is the last *inner*
character and the closer is one past it — except an empty `{}` / `[]` / `""`,
whose `end` sits *on* the closer. Never re-derive the closer as
`end.offset + 1` (#527). Command and word ranges come from the red-green CST
through the segmenter; the IR statement range is the authoritative span —
trust it rather than re-deriving.
[syntax-tree.md](docs/design/compiler/syntax-tree.md).

### Lexer, lowering, LSP

- A stray `}` or `]` is `TokenType::ESC`; check `tok.kind`, not just
  `tok.text` — a `}` typed `STR` is structural.
  [lexing-segmentation.md](docs/design/compiler/lexing-segmentation.md).
- A lowering hook that cannot safely specialise a construct falls through to
  the generic call IR: the compiler only inlines what it can prove is safe,
  the runtime handles the rest, and a helper returning `None` is conservative,
  not incomplete.
  [lowering-dispatch.md](docs/design/compiler/lowering-dispatch.md).
- LSP feature handlers are always registered and check their enable flag in
  the body, so `didChangeConfiguration` toggles them live; diagnostics stay
  push-only and never advertise `diagnosticProvider`.
  [lsp-feature-providers.md](docs/design/contracts/lsp-feature-providers.md).

## Code style

[CONTRIBUTING.md](CONTRIBUTING.md) is the style guide;
[engineering-guide.md](docs/design/rust/engineering-guide.md) is the Rust
one. Non-negotiables: `clippy::pedantic` clean with **no new `#[allow]`**
(fix the cause; the one pass is a config constructor whose parameters *are*
the config); UK spelling in identifiers and comments; plain minimal comments,
no banners; the AGPL header on our own source only — never on vendored code,
generated files, fixtures, or `.github/workflows/*`.

## Documentation

Two kinds: **KCS notes** (`docs/kcs/`, one plain-English answer per question;
six categories and fourteen rules in [STYLE.md](docs/kcs/STYLE.md)) and
**design docs** (`docs/design/`, contracts and internals, jargon allowed, one
contract per file). A feature change ships with its README, KCS, design-doc,
glossary, and screenshot updates in the same PR
([CONTRIBUTING.md](CONTRIBUTING.md) § *Documentation required for a PR*);
`cargo xtask kcs-index-links` fails on an unindexed note or a broken link.

## Build and release

- Four layers — Makefile + cargo bins, `scripts/*` helpers, CI, gated
  publishing — and one invariant: **a publish secret lives only as an
  Environment secret on a protected, manually-approved Environment** with a
  `v*`-tag-only policy, never as a plain repo or org secret.
  [release-and-publish.md](docs/design/contracts/release-and-publish.md).
- The tag selects the channel: even-minor `v2.x` is stable, odd-minor is
  pre-release; `scripts/release/prerelease.sh` is the single decider.
  `scripts/release/rust_release.sh` (`next` → `preflight` → `prepare` →
  `tag`) benchmarks the release and regenerates the release-notes graphs;
  never hand-edit the `## Performance` section of `RELEASE_NOTES.md`. The
  `release` skill drives the whole flow.
- The embedded SslicTcl trust-store data is refreshed only deliberately
  (`make update-source-data`, then `make check-source-data`):
  [sslictcl-source-data.md](docs/design/contracts/sslictcl-source-data.md).

## Long-running lanes

A lane keeps a tracking document under `docs/design/lanes/`, commits at every
compiling milestone as `wip(<lane>):` with explicitly staged paths, never
pushes (the orchestrator does), and never deletes `.git/index.lock`.
Protocol: [lanes/README.md](docs/design/lanes/README.md).
