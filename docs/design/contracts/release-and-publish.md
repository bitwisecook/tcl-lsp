# KCS: Release and publish — the four-layer model

## Symptom

A new contributor opens the Makefile, sees `publish-vsix` next to a CI
workflow that builds VSIX files, and assumes the publish step should
move into CI ("isn't that what CI is for?").  Or someone adds a
marketplace `secrets.VSCE_PAT` to CI in good faith, not realising the
maintainer is deliberately keeping every marketplace credential off
the GitHub Actions runner.

## Invariant

**No marketplace tokens go into CI.  Publishing to VS Code, JetBrains,
Sublime, and Zed marketplaces always runs from the maintainer's
laptop, using credentials that live in environment variables /
keychain — never on a GitHub Actions runner.**

This is enforced by:

* `grep -rE "secrets\.[A-Z_]+" .github/workflows/` returning nothing.
* The publish targets (`make publish-vsix`, `make publish-jetbrains`,
  `make publish-sublime`, `make publish-zed`) being defined only in
  the Makefile, not in any GitHub Actions workflow.
* Every marketplace SDK invocation living in `scripts/release/`,
  invoked exclusively by `make publish-*` — never by `.github/workflows/`.

CI still does plenty: it builds release artefacts, attests them with
sigstore (using GitHub's built-in OIDC — no token), generates SBOMs,
and attaches everything to a GitHub Release.  But the last hop —
pushing those artefacts to an external marketplace — is always a
local action.

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
│   - check/      wasm_command_parity.py, … (kcs-index-links →   │
│                 cargo xtask; refcount/audit ported to xtask)  │
│   - capture/    tcltest bytecode + result capture            │
│   - release/    tag.sh, publish_*.sh, jetbrains_token.sh, …  │
│   - install/    install.sh, hooks.sh, filter-readme.mjs      │
│   - zipapp-main/  3-line entry-point shims                   │
└─────────────┬────────────────────────────────────────────────┘
              ↓ invoked by both
┌─ CI ─────────────────────────────────────────────────────────┐
│ .github/workflows/*.yml                                      │
│   - pr-gate    runs `make ci-fast` on every PR + main        │
│   - test-py    full Python suite on push to main and tags    │
│   - test-ext   VS Code extension tests on push and tags      │
│   - create-release  + build-server-matrix (native LSP per    │
│       platform) → build-vsix (universal) + build-zipapp      │
│       (matrix) + build-claude-skills + build-jetbrains       │
│       + build-sublime + build-zed + publish-checksums        │
│       — tag-only                                             │
│   - never holds a marketplace token                          │
└─────────────┬────────────────────────────────────────────────┘
              ↓ produces signed artefacts that
┌─ Publishing (local-only) ────────────────────────────────────┐
│ make publish-* → scripts/release/publish_*.sh                │
│   - reads tokens from local env / keychain                   │
│   - pushes to marketplaces; CI never sees these              │
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

## Stable vs pre-release channels (odd/even-minor)

Two release lines run in parallel and the **tag alone** decides which:

| Line | Tags | Cut from | GitHub Release | VS Code Marketplace |
|---|---|---|---|---|
| **Stable** (default) | `v1.x`, `v2.2.0`, … (even 2.x minor) | `main` | normal / `latest` | normal channel |
| **Pre-release** ("for the brave") | `v2.1.x` (odd 2.x minor) | `rust` | `--prerelease` (never `latest`) | `--pre-release` channel |

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
* the Makefile's `VSCE_PRERELEASE_FLAG` adds `--pre-release` to both
  `vsce package` (so the `.vsix` self-describes) and the laptop
  `make publish-vsix`.
* the `publish-vsix-marketplace` CI job adds `--pre-release` to
  `vsce publish`.

No source-file edits or per-version config are needed: tag `v2.1.0`
from `rust`, and the whole pipeline routes it to the pre-release
channel automatically.

## Marketplace token table

Each marketplace's credential lives in exactly one place on the
maintainer's machine.  None of them exist in GitHub Actions secrets.

| Marketplace | Local env var | Source on the laptop | Used by |
|---|---|---|---|
| VS Code Marketplace | `VSCE_PAT` | `~/.vsce/keytar` cache, or interactively from `dev.azure.com` PAT | `make publish-vsix` |
| JetBrains Marketplace | `JETBRAINS_TOKEN` | macOS Keychain via `scripts/release/jetbrains_token.sh` | `make publish-jetbrains` |
| Package Control (Sublime) | (none; uses `git push` to a mirror repo) | `~/.ssh/` ssh keys | `make publish-sublime` |
| Zed Extensions | (none; opens a local PR branch for the maintainer to review and push) | — | `make publish-zed` |

## What CI may do

* Build every release artefact via `make zipapp-X` / `make build-editor-jetbrains`
  / `make build-editor-sublime` / `make build-editor-zed` / `make package-vsix`.
* Sign every artefact with sigstore (OIDC + `github.token` — no
  configured secrets).
* Generate SBOMs (`anchore/sbom-action`).
* Attach everything to a GitHub Release with `gh release upload`
  (uses `github.token`).
* Verify checksums with cosign keyless OIDC signing.

## What CI may NOT do

* Push to any external marketplace (VS Code, JetBrains, Package
  Control, zed-industries/extensions).
* Hold any `secrets.VSCE_PAT`, `secrets.JETBRAINS_TOKEN`, or
  similar.  These never enter the Actions runner.
* Replicate logic that lives in `scripts/release/`.  CI is allowed
  to *invoke* `scripts/release/*.sh`, but not duplicate its body.

If a future change requires CI to publish something for which CI
would need a token, that's a design conversation, not a unilateral
edit.  The "Add a marketplace token to CI" path is closed by design;
opening it requires updating this contract and `AGENTS.md`.

## File-path anchors

- [`Makefile`](../../../Makefile) — `publish-vsix`, `publish-jetbrains`,
  `publish-sublime`, `publish-zed`, `publish-all`, `publish-verify`,
  `publish-flow`, `release-tag`, `release-codeql-gate`.
- [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) —
  builds artefacts, attests them, attaches them to the Release.  No
  `secrets.*` references.
- [`.github/actions/setup-build/action.yml`](../../../.github/actions/setup-build/action.yml)
  and [`.github/actions/sign-and-upload/action.yml`](../../../.github/actions/sign-and-upload/action.yml) —
  composite actions for the CI build/sign tail.
- [`scripts/release/`](../../../scripts/release/) — every script that
  takes a marketplace credential or pushes to a marketplace.
- [`scripts/release/prerelease.sh`](../../../scripts/release/prerelease.sh) —
  the single source of truth for the stable-vs-pre-release decision
  (odd/even-minor convention), read by CI, the Makefile, and `tag.sh`.
- [`scripts/release/github_release.sh`](../../../scripts/release/github_release.sh) —
  the body of the CI `create-release` step: idempotent `gh release create`
  with `--prerelease` for odd-minor 2.x tags.  CI *invokes* it (one-line
  `run:`) rather than inlining the logic.
- [`scripts/release/vsce_publish.sh`](../../../scripts/release/vsce_publish.sh) —
  the body of the CI `publish-vsix-marketplace` step: runs the local vsce
  binary with `--pre-release` for odd-minor 2.x tags.  Still keyless (no
  PAT); CI invokes it via one-line `run:`.
- [`scripts/release/tag.sh`](../../../scripts/release/tag.sh) — creates +
  pushes the annotated tag; enforces the per-channel source branch.
- [`scripts/release/publish_verify.sh`](../../../scripts/release/publish_verify.sh) —
  pre-flight credential check.

## Test anchors

The invariant is machine-verifiable:

```bash
# No marketplace secrets in CI:
! grep -rE "secrets\.[A-Z_]+" .github/workflows/ \
  || (echo "FAIL: a marketplace secret leaked into CI" && exit 1)

# Every marketplace push happens from a make publish-* target:
grep -rln "publish-vsix\|publish-jetbrains\|publish-sublime\|publish-zed" \
  .github/workflows/ | grep -v "scripts/release" \
  && echo "FAIL: CI workflow invokes a publish target" && exit 1 || true
```

## Discoverability

- [`AGENTS.md`](../../../AGENTS.md) "Build, CI, and publishing"
  section — short reference to this contract.
- [`project-layout.md`](project-layout.md) — the seven-concern
  Python layout that the build/CI structure mirrors.
- [`Makefile`](../../../Makefile) `publish-flow` target — prints
  the cheat-sheet on demand.
