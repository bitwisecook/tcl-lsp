---
name: release
description: >
  Run the full release workflow: validate the release branch (2.x pre-releases
  are cut from `rust`, 1.x stable from `main`), test, generate changelog, tag,
  push, then let CI build and publish — approving the marketplace Environments
  with `gh` from the release laptop. Asks for patch/minor/major if not
  specified. Tag-only — no source-file edits, no commit on the release branch.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion
---

# Release

Orchestrates a full release of tcl-lsp. This skill is internal to the project.

The release is **tag-only**: every version literal in the tree is derived
from the latest annotated tag (via `hatch-vcs` for the Python wheel and via
the Makefile + `git describe` for every editor build). To cut a release we
push a `vX.Y.Z` tag — there is no source-file bump and no commit on the
release branch. RELEASE_NOTES.md is the one exception, and it lands via a PR
before tagging.

## Two release lines

There are two, and they are cut from **different branches**:

| Line | Branch | Channel | GitHub release |
| --- | --- | --- | --- |
| **1.x — stable** | `main` | VS Code stable / JetBrains stable | latest |
| **2.x — pre-release** (the Python → Rust rewrite) | `rust` | VS Code pre-release / JetBrains eap | pre-release, never "latest" |

Nothing declares which line you are on except the version: `scripts/release/prerelease.sh`
is the single source of truth, and it says a tag is a pre-release when
**major ≥ 2 and minor is odd** (so `v2.1.8` is a pre-release; `v2.2.0` would not
be). CI reads that same script to decide the GitHub release's prerelease flag and
the marketplace channel — you never pass a flag by hand.

The rewrite is where the work is, so in practice a release is usually a **2.1.x
pre-release cut from `rust`**. Do not "fix" this by tagging `main`: `main` does
not contain the 2.x work, and tagging it would ship a stale tree under a new
version.

## Workflow

Follow these steps **in order**. Stop and report on failure at any step.

### 1. Pick the line, then guard the branch

Work out the line from the version being cut, and require the matching branch:

```bash
# The branch this line releases from: 2.x odd-minor => `rust`, otherwise `main`.
branch=$(git branch --show-current)
prev_tag=$(git describe --tags --abbrev=0)
release_branch=$(git branch -r --contains "$prev_tag" \
                   | grep -qE 'origin/(rust)$' && echo rust || echo main)

if [ "$branch" != "$release_branch" ]; then
  echo "ERROR: $release_branch is the release branch for this line (on '$branch')"
  exit 1
fi
```

If the user asks for a release from the other branch, confirm with them before
proceeding — do not silently switch lines.

### 2. Pull latest

```bash
git pull origin "$release_branch"
```

### 3. Pre-release validation

Every feature PR is expected to have passed `make check-all` before merge
(the pre-push gate; see AGENTS.md). If you have any doubt that `main` is
green at this tip, run the full suite once against `main` before tagging:

```bash
make check-all             # lint + typecheck, all languages
make test-ext test-rust runtime-rust-test test-emacs   # the heavier suites
```

Otherwise the release simply reuses the already-passing CI gate and
continues.

### 4. Determine version bump

Check `$ARGUMENTS` for `patch`, `minor`, or `major`. If not provided, ask the
user with `AskUserQuestion`:

> What type of version bump? (patch / minor / major)

Compute the new version from the latest git tag:

```bash
prev_tag=$(git describe --tags --abbrev=0)
prev_version=${prev_tag#v}
```

Split `prev_version` into MAJOR.MINOR.PATCH and apply the bump:

- `patch`: increment PATCH
- `minor`: increment MINOR, reset PATCH to 0
- `major`: increment MAJOR, reset MINOR and PATCH to 0

### 5. Generate changelog

Generate a changelog from the **source diff** between the previous tag and
HEAD, not from the git log:

```bash
git diff "$prev_tag"..HEAD -- '*.py' '*.ts' '*.rs' '*.toml' '*.json' '*.tcl' \
  ':!**/package-lock.json' ':!**/Cargo.lock'
```

Analyse the diff and write a concise `RELEASE_NOTES.md` at the repository root
with these sections (omit empty sections):

```markdown
# vX.Y.Z

## New Features
- ...

## Improvements
- ...

## Bug Fixes
- ...

## Breaking Changes
- ...
```

Focus on user-visible changes. Group related changes. Use UK spelling. Do not
list every file touched; summarise the meaningful changes.

### 6. Land RELEASE_NOTES.md via PR

The release branch is protected, so the release-notes commit lands via a PR
**against that branch** (`--base "$release_branch"` — a notes PR opened against
`main` for a 2.x pre-release would land the notes on the wrong line):

```bash
git checkout -b release/vX.Y.Z
git add RELEASE_NOTES.md
git commit -m "Add release notes for vX.Y.Z"
git push -u origin release/vX.Y.Z
gh pr create --base "$release_branch" --title "Release vX.Y.Z notes" --body "..."
```

Wait for CI on the PR to go green, then ask the user to merge it (squash).
Once merged, switch back and pull:

```bash
git checkout "$release_branch"
git pull origin "$release_branch"
```

### 7. Create and push the tag

`make release-tag` handles validation (clean tree, correct branch, no
existing tag) and pushes only the tag — no source-file edits, no commit on
main:

```bash
make release-tag V=X.Y.Z
```

`tag.sh` derives the channel itself and enforces the branch that goes with it
(pre-release → `rust`, stable → `main`), so it will refuse a tag cut from the
wrong line.

The tag push triggers `.github/workflows/ci.yml` to build artefacts, run
`publish-checksums`, and publish the GitHub release — as a pre-release, with the
matching Marketplace channels, all from `prerelease.sh`. Nothing here takes a
"pre-release" flag; the version *is* the flag.

If the push fails with `Permission denied (publickey)`, the SSH agent has no
identities (a locked 1Password, typically). Rather than reconfiguring git,
override the push URL for that one command with the `gh` token:

```bash
TOKEN=$(gh auth token)
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0=remote.origin.pushurl \
GIT_CONFIG_VALUE_0="https://x-access-token:${TOKEN}@github.com/bitwisecook/tcl-lsp.git" \
  make release-tag V=X.Y.Z
```

(The repo rewrites `https://github.com/` → SSH via `insteadOf`; the
`x-access-token@` form does not match that prefix, so it is not rewritten.)

### 8. Verify published artefacts

After `make release-tag` pushes the tag, CI builds and uploads every
artefact, then the `publish-checksums` job aggregates them into a
`SHA256SUMS` file (cosign-signed when keyless OIDC is enabled). The
installer (`scripts/install/install.sh`) verifies downloads against this file.

**Wait for CI to finish, then verify locally** before publishing to any
editor marketplace. A SUMS mismatch means an artefact was modified
after upload or the build was non-reproducible — either way, **do not
proceed to step 9**.

```bash
tag="vX.Y.Z"
mkdir -p /tmp/release-verify
cd /tmp/release-verify

# Pull SUMS and every artefact
gh release download "$tag" --clobber

# Verify all hashes
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c SHA256SUMS
else
    shasum -a 256 -c SHA256SUMS
fi

# (Optional) verify the cosign keyless signature
if [ -f SHA256SUMS.cosign.bundle ] && command -v cosign >/dev/null 2>&1; then
    cosign verify-blob \
        --bundle SHA256SUMS.cosign.bundle \
        --certificate-identity-regexp "^https://github.com/bitwisecook/tcl-lsp/\.github/workflows/.+@refs/tags/" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        SHA256SUMS
fi

cd - && rm -rf /tmp/release-verify
```

Also smoke-test the installer one-liner from a clean shell and
verify the installed payload — `scripts/release/smoke_installer.sh`
wraps the full check matrix:

1. installer exits 0 — **pinned to the tag** via `TCL_LSP_VERSION`, which the
   script sets for you. Never install unpinned here: the installer's default is
   the latest *stable* release, so an unpinned run silently verifies a different
   version, and for a pre-release (never "latest") it verifies one that has
   nothing to do with the tag you just cut;
2. every installed artefact matches its SHA256SUMS entry;
3. `tcl --version` / `f5 --version` report the released version;
4. `tcl --help` / `f5 --help` exit 0 with non-empty output;
5. the MCP server answers a real `initialize` request and its `serverInfo`
   reports the released version (the native server takes no flags — don't expect
   `--help` or `--version` to print anything);
6. the Claude-skills directory has at least `MIN_SKILLS` entries
   (default 22).

```bash
bash scripts/release/smoke_installer.sh "$tag"
```

Env knobs the script honours: `TCL_LSP_PREFIX` (install destination,
default `/tmp/verify-bin`), `TCL_LSP_OS` (forced detection when
`/etc/os-release` isn't readable in this environment), `MIN_SKILLS`,
`KEEP_PREFIX=1` (skip the cleanup so you can poke at a failure).

The installer aborts by default when `SHA256SUMS` is missing — that's
the safety net for a CI regression that drops the `publish-checksums`
job. The post-install checks above catch the next failure modes after
that: a successfully-completed installer that nevertheless landed
stale or version-skewed binaries (e.g. cached artefact, partial
upload, dev build leaking through). All checks must pass before
step 9.

### 9. Editor publishing

**VS Code and JetBrains are published by CI, not here.** When the tag's CI
run reaches the `publish-vsix-marketplace` and `publish-jetbrains-marketplace`
jobs, each pauses on its protected Environment (`marketplace-vscode` /
`marketplace-jetbrains`) waiting for a reviewer. CI then publishes the
released, checksum-verified `.vsix` / plugin `.zip` using the Environment
secret (`secrets.VSCE_PAT` / `secrets.JETBRAINS_TOKEN`) — no publish token
ever leaves the laptop, and the channel (stable vs pre-release/eap) is derived
from the tag by `scripts/release/prerelease.sh`, not passed by hand.

**Approve both deployments with `gh`, from the release laptop, as part of this
flow** — there is no need to open the Actions UI. Only do this once step 8 has
passed: approving is what actually ships to the marketplaces.

```bash
tag="vX.Y.Z"

# The run for the *tag* — not the branch push that preceded it, which is a
# separate run with the same head commit.
run=$(gh run list --workflow ci.yml --limit 20 \
        --json databaseId,headBranch,event \
        --jq "[.[] | select(.headBranch==\"$tag\" and .event==\"push\")][0].databaseId")

# Both environments wait at once. Check `can_approve` before trying.
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" \
  --jq '.[] | "\(.environment.id)  \(.environment.name)  can_approve=\(.current_user_can_approve)"'

# Approve both in a single call, feeding the ids straight from the query above.
ids=$(gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" \
        --jq '.[].environment.id')
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" \
  -X POST $(for i in $ids; do printf ' -F environment_ids[]=%s' "$i"; done) \
  -f state=approved \
  -f comment="Artefacts verified against SHA256SUMS ($tag)"

# The response is a list of deployments, not an object — a `--jq` expecting an
# object errors *after* the approval has already gone through. Confirm by
# re-reading state rather than trusting the command's output:
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" --jq 'length'  # → 0
gh run watch "$run" --exit-status
```

If `current_user_can_approve` is false, the account `gh` is authenticated as is
not a reviewer on that Environment — report that rather than working around it.

If `current_user_can_approve` is `false`, the account `gh` is authenticated as
is not a reviewer on that Environment — report that rather than trying to work
around it. The laptop targets `make publish-vsix` / `make publish-jetbrains`
remain only as fallbacks if a CI job itself fails.

This step's own remaining work is just **Sublime and Zed**.

Before asking which editors to publish, run a readiness check so any
missing token or unclaimed namespace surfaces *before* the user picks
targets:

```bash
make publish-verify
```

`publish-verify` prints one of `[ok] / [warn] / [fail]` per editor; it
exits non-zero only on `[fail]` (tool missing or remote unreachable).
`[warn]` lines are recoverable — note them back to the user.

Then ask which editors to publish to using `AskUserQuestion`:

> Which editors should be published? (All / None / comma-separated list of: sublime, zed)
> Default: None

(VS Code and JetBrains are excluded — CI publishes both via the approval
gates above. Only add `vscode` / `jetbrains` here if you are deliberately
invoking the laptop fallback because a CI job failed.)

Based on the response:

- **None** (default): Skip publishing entirely.
- **All**: Run `make publish-sublime publish-zed`. Do **not** run
  `make publish-all`: it includes `publish-vsix` and `publish-jetbrains`,
  which would try to re-publish the versions CI already shipped.
- **Specific editors**: Run the corresponding `make publish-<editor>` targets.

Available targets:
- `make publish-vsix` — VS Code Marketplace **laptop fallback only**
  (normally CI publishes VSCE; see the note at the top of this step).
  Keyless: needs an Azure Entra session (`az login --allow-no-subscriptions`)
  and runs `vsce publish --azure-credential`. Set `VSCE_PAT` only to force
  the legacy stored-PAT path (discouraged; Azure DevOps global PATs retire
  2026-12-01).
- `make publish-jetbrains` — JetBrains Marketplace **laptop fallback only**
  (normally CI publishes it). Runs `./gradlew publishPlugin`. Requires
  `JETBRAINS_TOKEN` env var. The first-ever publish must be done
  interactively via the JetBrains web UI; `publishPlugin` only updates an
  already-listed plugin.
- `make publish-sublime` — Sublime Text / Package Control. Pushes the
  built `build/sublime-stage/` tree (the same contents that go into the
  `.sublime-package`) to the dedicated mirror repo
  `bitwisecook/tcl-lsp-sublime-text` at the current tag. Package Control
  scrapes the mirror's tags and serves the package source archive
  directly — no marketplace API call, no per-release channel PR. The
  mirror exists because Package Control needs the package contents at
  the root of a git tag, which our monorepo can't satisfy directly.
  One-time setup: the empty mirror repo must exist on GitHub
  (`gh repo create bitwisecook/tcl-lsp-sublime-text --public ...`).
  Override the mirror destination with `TCL_LSP_SUBLIME_MIRROR_REPO`
  and `TCL_LSP_SUBLIME_MIRROR_DIR`; set `TCL_LSP_SUBLIME_DRY_RUN=1` to
  stage the commit + tag locally without pushing.
- `make publish-zed` — Zed extensions registry. Prepares a local
  checkout of `zed-industries/extensions` with the tcl submodule advanced
  to the new tag and the version bumped in `extensions.toml`, then
  **stops** and prints the suggested commit / push / `gh pr create`
  commands. The script never pushes to a fork or opens a PR — the user
  reviews the diff first and raises the PR themselves.

Neovim (`nvim-lspconfig`) and Helix integration are one-time upstream
PRs that the user raises by hand; there is no per-release publish step
or `make publish-*` target for them.

The make targets in this repository **only push to repositories owned
by the maintainer** (the canonical repo, the
`tcl-lsp-sublime-text` mirror). They never push to or open PRs against
external repositories — any external-repo PR (JetBrains first-time
upload, Package Control channel submission, Zed extensions registry,
nvim-lspconfig, Helix) is raised by the user.

### 10. Summary

Print a summary of what was done:

```
Release vX.Y.Z complete.
  Previous version: <prev>
  New version:      X.Y.Z
  Tag:              vX.Y.Z
  Editors published: <list or "none">
```

$ARGUMENTS
