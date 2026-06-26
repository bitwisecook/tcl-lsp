#!/usr/bin/env bash
# release.sh — create and push the annotated release tag.
#
# Usage:  scripts/release/tag.sh X.Y.Z
#         make release-tag V=X.Y.Z
#
# All version literals in the repo derive from the latest annotated tag
# (via hatch-vcs for the Python wheel, via the Makefile + ``git describe``
# for every editor build). A release therefore needs no source-file edits
# and no commit on ``main``; this script just validates state, tags HEAD,
# and pushes the tag. The push triggers ``.github/workflows/ci.yml``,
# which builds + publishes the release artefacts.
set -euo pipefail

V="${1:?Usage: scripts/release/tag.sh X.Y.Z}"

# Validate the version literal.
[[ "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
    echo "error: '$V' is not a valid version (expected X.Y.Z)"
    exit 1
}

# Refuse to tag a dirty tree — hatch-vcs would otherwise embed a ``+dev``
# suffix in the wheel built from this commit later.
git diff --quiet && git diff --cached --quiet || {
    echo "error: worktree is dirty — commit or stash first"
    exit 1
}

# Refuse to tag if the version already exists.
if git rev-parse "v$V" >/dev/null 2>&1; then
    echo "error: tag v$V already exists"
    exit 1
fi

# Channel for this version (odd/even-minor convention — see prerelease.sh):
# pre-release (2.1.x) is cut from ``rust``; stable (1.x, 2.2.0, …) is cut
# from ``main``.  CI marks the GitHub Release and the VS Code publish
# accordingly off the very same parity, so the tag alone fully determines
# the channel.
prerelease="$(bash "$(dirname "$0")/prerelease.sh" "$V")"
if [[ "$prerelease" == "true" ]]; then
    channel="pre-release"
    expected_branch="rust"
else
    channel="stable"
    expected_branch="main"
fi
echo "==> v$V is a $channel release (expected branch: $expected_branch)"

# Sanity-check the branch. Override by setting ALLOW_NON_MAIN_RELEASE=1 if
# you need to tag from elsewhere (RC branches, hot-fix branches, etc.).
branch="$(git branch --show-current)"
if [[ -n "$branch" && "$branch" != "$expected_branch" && "${ALLOW_NON_MAIN_RELEASE:-0}" != "1" ]]; then
    echo "error: refusing to tag $channel release v$V from branch '$branch' (expected '$expected_branch')."
    echo "       set ALLOW_NON_MAIN_RELEASE=1 to override."
    exit 1
fi

echo "==> Creating annotated tag v$V"
git tag -a "v$V" -m "Release $V"

echo "==> Pushing tag v$V"
git push origin "v$V"

echo
echo "Tagged v$V — CI will now build and publish the release artefacts."
echo "Track the run at: https://github.com/bitwisecook/tcl-lsp/actions"
