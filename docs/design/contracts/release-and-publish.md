# KCS: Release and publish — the four-layer model

## Symptom

A new contributor opens the Makefile, sees `publish-vsix` next to a CI
workflow that builds VSIX files, and wonders where publishing actually
happens.  Or someone adds a marketplace `secrets.VSCE_PAT` as a plain
*repository* secret — available to every workflow run — not realising
the maintainer requires every publish secret to be an **Environment**
secret reachable only by a protected, manually-approved publish job.

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
│ Makefile + [project.scripts]                                 │
│   - file-dep-driven gates and artefact builds                │
│   - composes scripts/* into named, dependency-aware targets  │
└─────────────┬────────────────────────────────────────────────┘
              ↓ invokes
┌─ Build / codegen / check / release helpers ──────────────────┐
│ scripts/<purpose>/*.py + *.sh                                │
│   - build/      build_zipapps.py, build_kcs_db.py, …         │
│   - codegen/    catalogs.py, editor_settings.py, port_names.py │
│   - check/      kcs_index_links.py, wasm_command_parity.py, … │
│   - capture/    tcltest bytecode + result capture            │
│   - release/    tag.sh, publish_*.sh, jetbrains_token.sh, …  │
│   - install/    install.sh, hooks.sh, filter_readme.py       │
│   - zipapp-main/  3-line entry-point shims                   │
└─────────────┬────────────────────────────────────────────────┘
              ↓ invoked by both
┌─ CI ─────────────────────────────────────────────────────────┐
│ .github/workflows/*.yml                                      │
│   - pr-gate    runs `make ci-fast` on every PR + main        │
│   - test-py    full Python suite on push to main and tags    │
│   - test-ext   VS Code extension tests on push and tags      │
│   - create-release  + build-vsix + build-zipapp (matrix)     │
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
| Entry points | `make` + `[project.scripts]` | Gates, composed builds, file-dep tracking | Long-form logic |
| Helpers | `scripts/<purpose>/` | Codegen, parity checks, captures, zipapp main shims, release/publish actions | Anything `make` can express as a one-liner |
| CI | `.github/workflows/` | PR gate, tag-triggered artefact build + sign + upload to GH Release | Marketplace credentials, duplicate per-profile recipes |
| Publishing | `make publish-* → scripts/release/*.sh` | Marketplace pushes from the laptop | Anything CI could do without a token |

## The end-to-end flow

A release rolls out in four manual steps from the maintainer's laptop:

```
1. make publish-verify          # checks that local credentials + tools are ready
2. make release-tag V=X.Y.Z     # creates + pushes the annotated tag
                                # (no source-file edits, no commit — hatch-vcs reads the tag)
3. wait for ci.yml to finish on tag refs/tags/vX.Y.Z
                                # CI builds + signs + attaches every artefact to the GitHub Release
4. make publish-all             # local; pushes each artefact to its marketplace
                                # (or pick individual: publish-vsix, publish-jetbrains, …)
```

`make publish-flow` prints this cheat-sheet on demand so a tired
maintainer doesn't have to remember the sequence.

The publish-verify step (`scripts/release/publish_verify.sh`, 179
lines) checks every published-from-laptop credential and tool
non-destructively — it never ships anything.  Designed for a quick
pre-flight check the week before a planned release.

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

* Build every release artefact via `make zipapp-X` / `make build-editor-jetbrains`
  / `make build-editor-sublime` / `make build-editor-zed` / `make package-vsix`.
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

- [`Makefile`](../../../Makefile) — `publish-vsix`, `publish-jetbrains`,
  `publish-sublime`, `publish-zed`, `publish-all`, `publish-verify`,
  `publish-flow`, `release-tag`, `release-codeql-gate`.
- [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) —
  builds artefacts, attests them, attaches them to the Release, and runs
  the two marketplace publish jobs.  Every `secrets.*` reference sits in a
  job that declares a protected `environment:`.
- [`scripts/release/publish_jetbrains_upload.sh`](../../../scripts/release/publish_jetbrains_upload.sh) —
  uploads the released JetBrains `.zip` to the Marketplace REST API
  (invoked by the CI publish job; reuses `jetbrains_token.sh`).
- [`.github/actions/setup-build/action.yml`](../../../.github/actions/setup-build/action.yml)
  and [`.github/actions/sign-and-upload/action.yml`](../../../.github/actions/sign-and-upload/action.yml) —
  composite actions for the CI build/sign tail.
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
- [`project-layout.md`](project-layout.md) — the seven-concern
  Python layout that the build/CI structure mirrors.
- [`Makefile`](../../../Makefile) `publish-flow` target — prints
  the cheat-sheet on demand.
