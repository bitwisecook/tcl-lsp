---
name: release
description: >
  Run the full release workflow: validate the active `rust` release branch,
  write the changelog, bump the Zed manifest, land the notes PR, tag, then let
  CI build and publish — approving the marketplace Environments with `gh` from
  the release laptop. Stable (even-minor) cuts go through `tag.sh`; the
  odd-minor pre-release line adds a benchmark and regenerated performance
  graphs via `scripts/release/rust_release.sh`. Asks for patch/minor/major if
  not specified.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion
---

# Release

Internal to the project. Follow the steps **in order**; stop and report on
any failure.

The release is **tag-only**: every version literal derives from the latest
annotated tag (Makefile + `git describe`). Cutting a release is pushing a
`vX.Y.Z` tag — no source bump, no commit on the release branch. The two
exceptions land via a PR before the tag: `RELEASE_NOTES.md`, and
`editors/zed/extension.toml` (Zed's registry builds the extension directory
at the tagged commit, so the version must already be in the tree — `tag.sh`
refuses to tag until it is).

## One branch, two lines — and which one you are on

Every release is cut from `rust`; the tag selects the channel.
`scripts/release/prerelease.sh` is the single decider — pre-release when
major ≥ 2 and minor is odd (`v2.1.8` pre-release, `v2.2.0` stable) — and CI
reads it for the GitHub release flag and the marketplace channel; nothing is
passed by hand.

| Line | Channel | GitHub release | Notes carry |
|---|---|---|---|
| **even minor (stable)** | VS Code / Open VSX / JetBrains stable | latest | prose only |
| odd minor (pre-release) | VS Code / Open VSX pre-release, JetBrains eap | pre-release, never latest | prose + benchmark + graphs |

**The project is on the stable line.** `v2.2.x` is current, so a `patch` bump
is a stable cut and the steps below are written for it. The odd-minor
pre-release path still exists in the tooling and is called out inline wherever
it differs; resuming it means a `minor` bump onto `v2.3.x`.

Check rather than assume — the bump the user picks decides it:

```bash
scripts/release/prerelease.sh X.Y.Z    # "true" = pre-release line, "false" = stable
```

`legacy-py` (the Python 1.x line) is a locked archive: never branch, merge,
or tag from it.

## The drivers

- **Stable (even minor).** `scripts/release/tag.sh`, via
  `make release-tag V=X.Y.Z`. There is no scripted prepare step: the notes are
  hand-written prose and the Zed bump is one make target. No benchmark, no
  graphs, no `## Performance` section — `scripts/perf/results/` legitimately
  has no entry for a stable release.
- **Pre-release (odd minor).** `scripts/release/rust_release.sh` implements
  everything except the prose changelog, and adds the benchmark and graph
  regeneration on top of the same tagging primitive.

```bash
scripts/release/rust_release.sh next patch      # -> the next version (either line)
scripts/release/rust_release.sh preflight X.Y.Z # pre-release line only
scripts/release/rust_release.sh prepare X.Y.Z   # pre-release line only: + bench + graphs + notes
scripts/release/rust_release.sh tag X.Y.Z       # pre-release line only: re-verifies, then tag.sh
```

Only `next` accepts a stable version. `preflight`, `prepare` and `tag` refuse
one outright —

```
error: v2.2.5 is a stable release (cut from rust via tag.sh). This script drives the
       rust pre-release line only — see scripts/release/tag.sh.
```

— on purpose: running the pre-release path against a stable version would put
pre-release graphs in stable notes. Do not work around it.

## Workflow

### 1. Guard and pull

```bash
[ "$(git branch --show-current)" = rust ] || { echo "ERROR: release from rust"; exit 1; }
git pull origin rust
git fetch --tags origin
```

### 2. Validate

Every PR passed `make prep-pr` before merge and CI ran the deep suites on the
push. If you doubt the tip is green, run once before tagging:

```bash
make check-all
make test-ext test-rust runtime-rust-test test-emacs
```

Also confirm the workflows that are not part of the PR gate are green on the
tip — the **Pages** deploy in particular runs only on a push to `rust`, so a
PR can merge green and still leave it broken:

```bash
gh run list --workflow github-pages.yml --limit 3 \
  --json status,conclusion,headSha --jq '.[] | "\(.status) \(.conclusion) \(.headSha[0:9])"'
```

### 3. Version

Take `patch` / `minor` / `major` from `$ARGUMENTS`, else ask with
`AskUserQuestion`. Say which channel each option lands on — an odd minor
publishes as a pre-release, and that is rarely what someone means by "cut a
release" while the project is on `v2.2.x`.

```bash
scripts/release/rust_release.sh next <bump>       # prints X.Y.Z
scripts/release/prerelease.sh X.Y.Z               # which line: "true" | "false"
```

**Stable** — there is no scripted preflight, so check the same invariants by
hand:

```bash
[ "$(git branch --show-current)" = rust ]                       # right branch
git status --short                                              # clean tree
git rev-parse -q --verify "refs/tags/vX.Y.Z" && echo "TAG EXISTS"   # must print nothing
git ls-remote --tags origin "vX.Y.Z"                            # must be empty
```

**Pre-release** — `scripts/release/rust_release.sh preflight X.Y.Z` does all
of that plus the ordering check.

### 4. Changelog (prose)

Generate from the **source diff** since the previous tag, not the git log —
though on a range of a few hundred commits the conventional-commit subjects
are the practical index into it:

```bash
prev_tag=$(git describe --tags --abbrev=0)
git diff "$prev_tag"..HEAD -- '*.rs' '*.ts' '*.kt' '*.toml' '*.json' '*.tcl' '*.tclspec' \
  ':!**/package-lock.json' ':!**/Cargo.lock'
git log "$prev_tag"..HEAD --no-merges --format='%s' | grep -E '^(feat|fix|perf)' | sort -u
```

Prepend to `RELEASE_NOTES.md` at the root — newest section first, since
`github_release.sh` publishes the top section verbatim as the GitHub release
body. Open with `# vX.Y.Z` and a short paragraph saying what the release is
about, then `## New features`, `## Improvements`, `## Bug fixes`,
`## Breaking changes` (sentence case, matching the file; omit empty
sections). User-visible changes, grouped, UK spelling, no file lists.

**The `## Performance` section belongs to the pre-release line only.** On that
line, never hand-write it — step 5 generates it, and a hand-edited one is how
a release ships the previous release's graphs. On a stable cut there is no
such section at all.

### 5. Prepare the notes commit

**Stable.** Bump the Zed manifest and commit both files on a
`release/vX.Y.Z` branch:

```bash
make release-zed-version V=X.Y.Z
bash scripts/release/zed_version.sh check X.Y.Z    # the invariant tag.sh enforces
git checkout -b release/vX.Y.Z
git add RELEASE_NOTES.md editors/zed/extension.toml
git commit    # "Release vX.Y.Z notes"
make xtask-check
```

**Pre-release.** `scripts/release/rust_release.sh prepare X.Y.Z` does the Zed
bump for you, plus a release build of `tcl-lsp-server`, the pinned-corpus
benchmark into `scripts/perf/results/X.Y.Z.json`, the `MANIFEST.toml` entry, a
re-render of `scripts/perf/graphs/` with X.Y.Z highlighted, the
`## Performance` section with its four release-asset URLs, `verify` (re-render
and diff against the committed graphs), then the local commit. It stops there
and prints the push / PR commands — read the diff first. Report rather than
paper over:

- **Measurement host.** A warning that this release was measured on a
  different machine than the last means wall time and CPU are not
  comparable; the notes say so instead of quoting a delta
  (`scripts/perf/README.md`).
- **`--force`.** An existing `results/X.Y.Z.json` is kept; re-measuring needs
  `scripts/release/perf_release.sh X.Y.Z --force` and a reason.

### 6. Land the notes PR

`rust` is ruleset-protected, so the notes commit lands via a PR **against
`rust`** (a PR against another branch lands the notes on the wrong line):

```bash
git push -u origin release/vX.Y.Z
gh pr create --base rust --title "Release vX.Y.Z notes" --body "..."
gh pr merge <n> --auto --squash      # both notes PRs to date landed squashed
```

A stable PR carries `RELEASE_NOTES.md` and `editors/zed/extension.toml`. A
pre-release PR carries those plus `scripts/perf/results/X.Y.Z.json`, the
regenerated `scripts/perf/graphs/`, and the `MANIFEST.toml` entry — the graphs
only mean something beside the result they were rendered from. Wait for green
CI and the merge, then:

```bash
git checkout rust && git pull origin rust
```

### 7. Tag

```bash
make release-tag V=X.Y.Z                        # stable — tag.sh directly
scripts/release/rust_release.sh tag X.Y.Z       # pre-release — re-verifies, then tag.sh
```

`tag.sh` checks the Zed manifest invariant, refuses a dirty tree, refuses an
existing tag, then pushes only the tag. The push runs `ci.yml`: artefacts,
`publish-checksums`, the GitHub release with the channel from `prerelease.sh`.

A maintainer's tag push reports a **bypassed ruleset** —

```
remote: Bypassed rule violations for refs/tags/vX.Y.Z:
remote: - Cannot create ref due to creations being restricted.
```

— and succeeds. That is the ruleset working as configured for an account that
can bypass it, not an error; say so in the report rather than treating the tag
as suspect.

`Permission denied (publickey)` means the SSH agent has no identities
(a locked 1Password). Override the push URL for that one command instead of
reconfiguring git — the `x-access-token@` form escapes the repo's
`insteadOf` rewrite:

```bash
TOKEN=$(gh auth token)
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0=remote.origin.pushurl \
GIT_CONFIG_VALUE_0="https://x-access-token:${TOKEN}@github.com/bitwisecook/tcl-lsp.git" \
  make release-tag V=X.Y.Z
```

### 8. Verify the published artefacts

CI aggregates every artefact into `SHA256SUMS` (cosign-signed when keyless
OIDC is on); the installer verifies against it. **Wait for CI, then verify
locally before anything is published.** A mismatch means a modified or
non-reproducible artefact — **do not proceed to step 9**.

```bash
tag="vX.Y.Z"
mkdir -p /tmp/release-verify && cd /tmp/release-verify
gh release download "$tag" --clobber
if command -v sha256sum >/dev/null 2>&1; then sha256sum -c SHA256SUMS; else shasum -a 256 -c SHA256SUMS; fi
if [ -f SHA256SUMS.cosign.bundle ] && command -v cosign >/dev/null 2>&1; then
    cosign verify-blob --bundle SHA256SUMS.cosign.bundle \
        --certificate-identity-regexp "^https://github.com/bitwisecook/tcl-lsp/\.github/workflows/.+@refs/tags/" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        SHA256SUMS
fi
cd - && rm -rf /tmp/release-verify
```

Then smoke the installer from a clean shell:

```bash
bash scripts/release/smoke_installer.sh "$tag"
```

It pins `TCL_LSP_VERSION` to the tag (an unpinned run installs the latest
*stable* and verifies the wrong version), checks every installed artefact
against `SHA256SUMS`, `tcl --version` / `f5 --version` report the release,
`--help` exits 0, the MCP server answers `initialize` with the released
`serverInfo` (it takes no flags), and at least `MIN_SKILLS` (default 22)
Claude skills installed. Knobs: `TCL_LSP_PREFIX`, `TCL_LSP_OS`, `MIN_SKILLS`,
`KEEP_PREFIX=1`. All checks pass before step 9.

### 9. Publish

**VS Code, Open VSX, and JetBrains publish from CI.** The tag run pauses at
`publish-vsix-marketplace`, `publish-vsix-openvsx`, and
`publish-jetbrains-marketplace` on their protected Environments
(`marketplace-vscode` / `marketplace-openvsx` / `marketplace-jetbrains`);
approving is what ships, so it is the user's call — surface the pending
approvals and wait rather than approving unprompted. Once step 8 passed and
the user has said to go:

```bash
tag="vX.Y.Z"
# The run for the *tag*, not the branch push with the same head commit.
run=$(gh run list --workflow ci.yml --limit 20 --json databaseId,headBranch,event \
        --jq "[.[] | select(.headBranch==\"$tag\" and .event==\"push\")][0].databaseId")
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" \
  --jq '.[] | "\(.environment.id)  \(.environment.name)  can_approve=\(.current_user_can_approve)"'
ids=$(gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" --jq '.[].environment.id')
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" \
  -X POST $(for i in $ids; do printf ' -F environment_ids[]=%s' "$i"; done) \
  -f state=approved -f comment="Artefacts verified against SHA256SUMS ($tag)"
# The POST returns a list; confirm by re-reading rather than parsing it.
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" --jq 'length'  # -> 0
gh run watch "$run" --exit-status
```

`current_user_can_approve=false` means this `gh` account is not a reviewer
on that Environment — report it, do not work around it. The laptop targets
`make publish-vsix` / `publish-openvsx` / `publish-jetbrains` are fallbacks
only when a CI job itself failed (keyless `az login` for vsce; `OVSX_PAT`;
`JETBRAINS_TOKEN`, first-ever upload is manual in the web UI).

Sublime needs no step: Package Control resolves the `LSP-Tcl.sublime-package`
asset on the release (`make publish-verify` checks it is there; registering
the channel entry is a one-time PR the user raises —
`editors/sublime-text/SUBMITTING.md`). Neovim and Helix are one-time
upstream PRs by the user.

What remains is **Zed**. Run `make publish-verify` (`[ok]` / `[warn]` /
`[fail]` per editor; non-zero only on `[fail]`), then ask with
`AskUserQuestion`:

> Which editors should be published? (All / None / zed) — default None

- **None** — skip. **All** or **zed** — `make publish-zed`: prepares a local
  checkout of `zed-industries/extensions` with the submodule advanced to the
  tag and the version bumped, then **stops** and prints the commit / push /
  `gh pr create` commands; the user reviews and raises the PR. Never run
  `make publish-all` — it re-publishes what CI already shipped.

The make targets push only to repositories the maintainer owns; every
external-repo PR is raised by the user.

### 10. Summary

```
Release vX.Y.Z complete.
  Previous version: <prev>
  New version:      X.Y.Z
  Tag:              vX.Y.Z          (stable | pre-release)
  Benchmarked on:   <host from results/X.Y.Z.json, or "n/a — stable line">
  Editors published: <list or "none">
```

On the pre-release line, say if the benchmark host differed from the previous
release's — it is why the notes quote no delta. On a stable cut say the line
carries no benchmark, so nobody goes looking for the missing graphs.

Issues never auto-close on a release; sweep them by hand after the cut.

$ARGUMENTS
