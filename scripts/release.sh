#!/usr/bin/env bash
# release.sh — create and push the annotated release tag.
#
# Usage:  scripts/release.sh X.Y.Z
#         make release-tag V=X.Y.Z
#
# All version literals in the repo derive from the latest annotated tag
# (via hatch-vcs for the Python wheel, via the Makefile + ``git describe``
# for every editor build). A release therefore needs no source-file edits
# and no commit on ``main``; this script just validates state, tags HEAD,
# and pushes the tag. The push triggers ``.github/workflows/ci.yml``,
# which builds + publishes the release artefacts.
set -euo pipefail

V="${1:?Usage: scripts/release.sh X.Y.Z}"

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

# Sanity-check the branch — releases are tagged off ``main``. Override by
# setting ALLOW_NON_MAIN_RELEASE=1 if you need to tag from elsewhere
# (RC branches, hot-fix branches, etc.).
branch="$(git branch --show-current)"
if [[ -n "$branch" && "$branch" != "main" && "${ALLOW_NON_MAIN_RELEASE:-0}" != "1" ]]; then
    echo "error: refusing to tag from branch '$branch' (expected 'main')."
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
