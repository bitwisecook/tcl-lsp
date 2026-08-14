# Release and publish — the four-layer model

Where publishing actually happens, and why the marketplace credentials are
Environment secrets reachable only by a protected, manually-approved job
rather than plain repository secrets available to every workflow run.

## Invariant

**A publish secret used in a workflow must be a GitHub *Environment*
secret on a protected, manually-approved Environment — never a plain
repo/org secret available to every workflow run.**

VS Code and JetBrains publish *from CI*; Package Control (Sublime) and
Zed publish from the maintainer's laptop (they need no token — they push
to a maintainer-owned mirror / open a PR).  A marketplace token may live
in CI only when, stored as an Environment secret, it is reachable solely
by the one job that targets that Environment — which has a required
reviewer and a `v*`-tag-only deployment policy, so it pauses for human
approval and cannot run on a non-tag ref.

This is enforced by:

* Every `secrets.*` reference in `.github/workflows/` appearing only
  inside a job that declares a protected `environment:` (a required
  reviewer + a `v*`-tag-only deployment policy).  A `secrets.*` use in a
  job with no such `environment:` violates the contract.
* The publish secret being an **Environment** secret (e.g. `VSCE_PAT` on
  `marketplace-vscode`, `JETBRAINS_TOKEN` on `marketplace-jetbrains`),
  not a repository or organisation secret.
* The secret being scoped to the publish step's `env:` only, so
  freshly-fetched code earlier in the job never runs with it in scope.
* The published bytes being the exact artefact attached to the GitHub
  Release (checksum-verified against the cosign-signed `SHA256SUMS`),
  not a fresh rebuild.

CI still does plenty besides publishing: it builds release artefacts,
attests them with sigstore (using GitHub's built-in OIDC — no token),
generates SBOMs, and attaches everything to a GitHub Release.  The
publish jobs then push those same signed artefacts to the marketplaces
behind the approval gate.

## The four layers

```
┌─ Developer entry points ─────────────────────────────────────┐
│ Makefile                                                     │
│   - file-dep-driven gates and artefact builds                │
│   - composes cargo + scripts/* into named, dep-aware targets │
└─────────────┬────────────────────────────────────────────────┘
              ↓ invokes
┌─ Build / codegen / check / release helpers ──────────────────┐
│ cargo (native binaries) + cargo xtask (codegen/check gates)  │
│   + scripts/<purpose>/*.sh                                   │
│   - cargo build -p {tcl-cli,f5-cli,tcl-lsp-server,tcl-mcp}   │
│   - cargo xtask gen-editor-settings / gen-editor-catalogs /  │
│                 diag-tables / kcs-index-links                │
│   - capture/    tcltest bytecode + result capture            │
│   - release/    tag.sh, publish_*.sh, jetbrains_token.sh, …  │
│   - install/    install.sh, hooks.sh                         │
└─────────────┬────────────────────────────────────────────────┘
              ↓ invoked by both
┌─ CI ─────────────────────────────────────────────────────────┐
│ .github/workflows/*.yml                                      │
│   - pr-gate    fast Rust gate (cargo test lsp_e2e) on PRs    │
│   - test-ext   VS Code extension tests on push and tags      │
│   - create-release  + build-vsix + build native binaries     │
│     (tcl / f5-query / tcl-lsp-server / tcl-mcp, cross-matrix) │
│     + build-claude-skills + build-jetbrains + build-sublime  │
│     + build-zed + publish-checksums       — tag-only         │
│   - publish-vsix-marketplace + publish-jetbrains-marketplace │
│     publish from CI behind a protected Environment           │
└─────────────┬────────────────────────────────────────────────┘
              ↓ produces signed artefacts that
┌─ Publishing ─────────────────────────────────────────────────┐
│ CI (behind a protected, approval-gated Environment):         │
│   - VS Code   → secrets.VSCE_PAT      on marketplace-vscode  │
│   - JetBrains → secrets.JETBRAINS_TOKEN on marketplace-jetbrains │
│ Laptop (no token needed):                                    │
│   - make publish-sublime → push to the mirror repo           │
│   - make publish-zed     → open a PR on zed-industries       │
│ make publish-vsix / publish-jetbrains remain laptop fallbacks│
└──────────────────────────────────────────────────────────────┘
```

Each layer does one job:

| Layer | Tool | Owns | Does NOT own |
|---|---|---|---|
| Entry points | `make` | Gates, composed builds, file-dep tracking | Long-form logic |
| Helpers | `cargo` / `cargo xtask` / `scripts/<purpose>/` | Native binary builds, codegen, parity checks, captures, release/publish actions | Anything `make` can express as a one-liner |
| CI | `.github/workflows/` | PR gate, tag-triggered artefact build + sign + upload to GH Release | Marketplace credentials, duplicate per-profile recipes |
| Publishing | `make publish-* → scripts/release/*.sh` | Marketplace pushes from the laptop | Anything CI could do without a token |

## The end-to-end flow

A release rolls out from a single annotated tag; VS Code and JetBrains
publish from CI behind the approval gate, and only Sublime + Zed run from
the laptop:

```
1. make publish-verify          # checks that local credentials + tools are ready
2. make release-tag V=X.Y.Z     # creates + pushes the annotated tag
                                # (no source-file edits, no commit — hatch-vcs reads the tag)
3. wait for ci.yml on refs/tags/vX.Y.Z; it builds + signs + attaches every
   artefact to the GitHub Release, then PAUSES the publish-vsix-marketplace
   and publish-jetbrains-marketplace jobs for approval — approve the
   marketplace-vscode and marketplace-jetbrains deployments when prompted
4. make publish-sublime publish-zed   # local; Sublime + Zed only (no token)
```

`make publish-flow` prints this cheat-sheet on demand so a tired
maintainer doesn't have to remember the sequence.  `make publish-vsix` /
`make publish-jetbrains` remain laptop fallbacks if a CI publish job fails.

## The 2.1.x pre-release sequence is a program, not a procedure

Step 2 above is the *primitive*.  For the `rust` pre-release line there is
work that must happen before it — the release-notes performance graphs —
and every part of it used to be a step someone remembered:

```
scripts/release/rust_release.sh next patch      # -> the next version
scripts/release/rust_release.sh prepare X.Y.Z   # everything before the tag
#   ...open + merge the notes PR against `rust`, pull, then:
scripts/release/rust_release.sh tag X.Y.Z       # verify, then tag.sh
```

| Step | Script | Produces |
|---|---|---|
| `preflight` | `rust_release.sh` | nothing — asserts branch, clean tree, free tag, ordering, channel |
| `perf` | `perf_release.sh` → `scripts/perf/` | `results/X.Y.Z.json`, the `MANIFEST.toml` entry, re-rendered `graphs/` |
| `notes` | `perf_notes.py` | the `## Performance…` section of `RELEASE_NOTES.md` |
| `verify` | `rust_release.sh` | nothing — re-renders the graphs and diffs them against the committed ones |
| `tag` | `rust_release.sh` → `tag.sh` | the annotated tag, which triggers everything above |

The invariant `verify` enforces: **`scripts/perf/graphs/` must be exactly
what `scripts/perf/results/` renders to with the release highlighted.**
`report.py` is byte-deterministic, so re-rendering and diffing is a real
check rather than an approximation, and it runs again inside `tag` — a
graph set that has drifted from its inputs cannot reach a release.

Two things this deliberately leaves to a human: the prose changelog
(a judgement, not a derivation) and pushing the notes branch / opening
the PR (`prepare` stops at a local commit and prints both commands,
the same way `publish_zed.sh` stops before touching an external repo).

The measurement host matters.  `scripts/perf/README.md` documents that CI
and macOS take different code paths, so the *committed* result — taken
wherever the maintainer ran `prepare` — is the release record.
`perf.yml` still benchmarks the tag on its own runner for the trend line,
but renders and attaches the committed result when there is one.

The publish-verify step (`scripts/release/publish_verify.sh`, 179
lines) checks every publish credential and tool non-destructively — it
never ships anything.  Designed for a quick pre-flight check the week
before a planned release.

## Stable vs pre-release channels (odd/even-minor)

Two release lines run in parallel and the **tag alone** decides which:

| Line | Tags | Cut from | GitHub Release | VS Code Marketplace | JetBrains Marketplace |
|---|---|---|---|---|---|
| **Stable** (default) | `v1.x`, `v2.2.0`, … (even 2.x minor) | `main` | normal / `latest` | normal channel | Stable channel |
| **Pre-release** ("for the brave") | `v2.1.x` (odd 2.x minor) | `rust` | `--prerelease` (never `latest`) | `--pre-release` channel | `eap` channel |

This is the VS Code Marketplace
[odd/even-minor convention](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#prerelease-extensions):
from major 2 onward an **odd** minor is a pre-release and an **even**
minor is stable.  The 2.x rewrite ships its alphas on `2.1.x`
(`2.1.0`, `2.1.1`, …) and promotes to the stable `2.2.0` when ready.
The 1.x line predates the convention and is always stable, so it stays
the default download and the default Marketplace install for everyone
who has not opted into pre-releases.

**`scripts/release/prerelease.sh X.Y.Z` is the single source of truth.**
It prints `true`/`false`, and every pre-release switch reads it so CI,
the Makefile, and `tag.sh` can never disagree:

* `tag.sh` expects an odd-minor 2.x tag to be cut from `rust`, an even /
  1.x tag from `main` (override with `ALLOW_NON_MAIN_RELEASE=1`).
* the `create-release` CI job adds `--prerelease` to `gh release create`.
* the `publish-vsix-marketplace` CI job (and the laptop `make publish-vsix`)
  add `--pre-release` to `vsce publish` for odd-minor 2.x tags.
* the `publish-jetbrains-marketplace` CI job sets `JETBRAINS_CHANNEL=eap`
  for odd-minor 2.x tags, so pre-release plugins land on the JetBrains
  `eap` channel instead of the default Stable channel.

No source-file edits or per-version config are needed: tag `v2.1.0`
from `rust`, and the whole pipeline routes it to the pre-release
channel automatically.

## Marketplace credential table

VS Code and JetBrains publish from CI using an **Environment** secret on
a protected, approval-gated Environment.  Sublime and Zed need no token.
The laptop publish targets remain as fallbacks.

| Marketplace | Primary path | Credential | Fallback |
|---|---|---|---|
| VS Code Marketplace | CI job `publish-vsix-marketplace` | `secrets.VSCE_PAT` (Environment secret on `marketplace-vscode`) | `make publish-vsix` (keyless `az login`, or local `VSCE_PAT`) |
| JetBrains Marketplace | CI job `publish-jetbrains-marketplace` | `secrets.JETBRAINS_TOKEN` (Environment secret on `marketplace-jetbrains`) | `make publish-jetbrains` (token via Keychain / `jetbrains_token.sh`) |
| Package Control (Sublime) | `make publish-sublime` (laptop) | none — `git push` to the mirror repo | — |
| Zed Extensions | `make publish-zed` (laptop) | none — opens a local PR branch | — |

Both CI publish jobs target a protected Environment (required reviewer +
`v*`-tag-only deployment policy), so they pause for manual approval and
the secret is reachable by no other job.

## What CI may do

* Build every release artefact: the native binaries (`tcl`, `f5-query`,
  `tcl-lsp-server`, `tcl-mcp`) via `cargo build` / the cross-build targets,
  plus `make build-editor-jetbrains` / `make build-editor-sublime` /
  `make build-editor-zed` / `make package-vsix package-vsix-targets` and
  the Claude-skills zip. The VS Code artefact is seven VSIX packages: one
  untargeted universal package bundling every native `tcl-lsp-server`
  binary (the Marketplace's fallback for riscv64 Linux, and the artefact
  for a manual side-load), plus six platform-targeted packages built with
  `vsce package --target <platform>`, each bundling only its own binary —
  no `.pyz`. The JetBrains artefact is one universal plugin bundling every
  native `tcl-lsp-server` binary except riscv64 Linux (no official
  JetBrains IDE build targets it), since JetBrains Marketplace has no
  per-platform equivalent of vsce's `--target` yet.
* Sign every artefact with sigstore (OIDC + `github.token` — no
  configured secrets).
* Generate SBOMs (`anchore/sbom-action`).
* Attach everything to a GitHub Release with `gh release upload`
  (uses `github.token`).
* Verify checksums with cosign keyless OIDC signing.
* Publish VS Code / JetBrains to their marketplaces — but **only** from a
  job that targets a protected, approval-gated Environment, using that
  Environment's secret, publishing the Release's checksum-verified
  artefact.

## What CI may NOT do

* Reference a marketplace `secrets.*` from a job that does **not** declare
  a protected `environment:` (required reviewer + `v*`-tag-only policy).
* Store a publish token as a plain **repository** or **organisation**
  secret (available to every workflow), rather than an Environment secret.
* Publish Sublime or Zed (they push to a mirror / open a PR — laptop-only).
* Replicate logic that lives in `scripts/release/`.  CI is allowed
  to *invoke* `scripts/release/*.sh`, but not duplicate its body.

Changing which marketplaces publish from CI, or how their secrets are
stored, is a design conversation: it requires updating this contract and
`AGENTS.md` together.

## File-path anchors

- [`Makefile`](../../../Makefile) — `publish-vsix`, `publish-vsix-targets`,
  `publish-jetbrains`, `publish-sublime`, `publish-zed`, `publish-all`,
  `publish-verify`, `publish-flow`, `release-tag`, and the pre-release
  sequence `release-perf`, `release-notes-perf`, `release-verify`,
  `release-prepare`, `release-rust-tag`.
- [`scripts/release/rust_release.sh`](../../../scripts/release/rust_release.sh) —
  the 2.1.x pre-release driver (`next` / `preflight` / `perf` / `notes` /
  `verify` / `prepare` / `tag`).
- [`scripts/release/perf_release.sh`](../../../scripts/release/perf_release.sh)
  and [`scripts/release/perf_notes.py`](../../../scripts/release/perf_notes.py) —
  the release-notes performance graphs and the section that embeds them.
- [`.github/workflows/perf.yml`](../../../.github/workflows/perf.yml) —
  benchmarks every push and tag; attaches `perf-*.svg` / `perf-summary.md`
  to the GitHub Release, which is what the notes link to.
- [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) —
  builds artefacts, attests them, attaches them to the Release, and runs
  the two marketplace publish jobs.  Every `secrets.*` reference sits in a
  job that declares a protected `environment:`.
- [`scripts/release/publish_jetbrains_upload.sh`](../../../scripts/release/publish_jetbrains_upload.sh) —
  uploads the released JetBrains `.zip` to the Marketplace REST API
  (invoked by the CI publish job; reuses `jetbrains_token.sh`).
- [`.github/actions/sign-and-upload/action.yml`](../../../.github/actions/sign-and-upload/action.yml) —
  the composite action for the CI build/sign tail (the only composite action
  in the tree).
- [`scripts/release/`](../../../scripts/release/) — every script that
  takes a marketplace credential or pushes to a marketplace.
- [`scripts/release/publish_verify.sh`](../../../scripts/release/publish_verify.sh) —
  pre-flight credential check.

## Test anchors

The invariant is machine-verifiable.  Every job that references a
marketplace `secrets.*` must declare a protected `environment:`:

```bash
# Each workflow job that uses secrets.VSCE_PAT / secrets.JETBRAINS_TOKEN
# must bind to an Environment (manual gate).  scripts/release/check_publish_env.py
# parses ci.yml and fails if any such job lacks an `environment:` key.
python3 scripts/release/check_publish_env.py

# Sublime and Zed are never published from CI:
! grep -rE "publish-sublime|publish-zed" .github/workflows/ \
  || (echo "FAIL: Sublime/Zed must publish from the laptop" && exit 1)
```

## Discoverability

- [`AGENTS.md`](../../../AGENTS.md) "Build, CI, and publishing"
  section — short reference to this contract.
- [`project-layout.md`](project-layout.md) — the Rust workspace crate
  layout that the build/CI structure mirrors.
- [`Makefile`](../../../Makefile) `publish-flow` target — prints
  the cheat-sheet on demand.
