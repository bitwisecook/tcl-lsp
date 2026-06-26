#!/usr/bin/env bash
# github_release.sh — create the GitHub Release for the current tag.
#
# Invoked by the `create-release` job in .github/workflows/ci.yml.  Kept as
# a committed script rather than inline YAML so the workflow stays thin and
# the release logic is reviewable in one place — per the
# release-and-publish contract, CI *invokes* scripts/release/*.sh rather
# than embedding their bodies.
#
# Usage:  scripts/release/github_release.sh [TAG]
#         TAG defaults to the tag in $GITHUB_REF (refs/tags/<TAG>).
#
# Requires GH_TOKEN in the environment (set by the workflow).  Idempotent:
# exits 0 without changes if the release already exists, so re-runs of a
# tag's CI are safe.
#
# Channel (the VS Code odd/even-minor convention — scripts/release/prerelease.sh
# is the single source of truth): an odd-minor 2.x tag (v2.1.x) is published
# as a GitHub *pre-release* so it never becomes "latest" and the 1.x line
# stays the default download for everyone who hasn't opted in.  Even-minor
# 2.x (v2.2.0) and 1.x publish as normal stable releases.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
tag="${1:-${GITHUB_REF#refs/tags/}}"

if gh release view "$tag" >/dev/null 2>&1; then
    echo "Release $tag already exists — nothing to do."
    exit 0
fi

# Require curated release notes.  Falling back to `gh release create
# --generate-notes` would build the changelog from commit messages and PR
# titles, both attacker-influenced — anyone whose PR merged since the last
# release could land text inside our official release notes.  Fail closed
# so the maintainer must commit RELEASE_NOTES.md before tagging.
if [ ! -f RELEASE_NOTES.md ]; then
    echo "ERROR: RELEASE_NOTES.md is missing for tag $tag." >&2
    echo "Commit curated notes to RELEASE_NOTES.md before pushing the tag." >&2
    exit 1
fi

prerelease_flag=""
if [ "$(bash "$here/prerelease.sh" "$tag")" = "true" ]; then
    echo "==> $tag is a pre-release (odd minor) — marking the GitHub Release accordingly."
    prerelease_flag="--prerelease"
fi

gh release create "$tag" \
    --title "$tag" \
    --notes-file RELEASE_NOTES.md \
    $prerelease_flag
