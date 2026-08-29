#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# release.sh — create and push the annotated release tag.
#
# Usage:  scripts/release/tag.sh X.Y.Z
#         make release-tag V=X.Y.Z
#
# All version literals in the repo derive from the latest annotated tag
# (via hatch-vcs for the Python wheel, via the Makefile + ``git describe``
# for every editor build). A release therefore needs no source-file edits
# and no commit on ``rust``; this script just validates state, tags HEAD,
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

# Channel for this version (odd/even-minor convention — see prerelease.sh).
# Every active release is cut from ``rust``; parity controls only the GitHub
# Release and Marketplace channel. The locked ``legacy-py`` branch is an
# archive and is never a release source.
prerelease="$(bash "$(dirname "$0")/prerelease.sh" "$V")"
if [[ "$prerelease" == "true" ]]; then
    channel="pre-release"
else
    channel="stable"
fi
expected_branch="rust"
echo "==> v$V is a $channel release (expected branch: $expected_branch)"

# Sanity-check the branch. Override by setting ALLOW_NON_RUST_RELEASE=1 if
# you need to tag from elsewhere (RC branches, hot-fix branches, etc.).
branch="$(git branch --show-current)"
if [[ -n "$branch" && "$branch" != "$expected_branch" && "${ALLOW_NON_RUST_RELEASE:-0}" != "1" ]]; then
    echo "error: refusing to tag $channel release v$V from branch '$branch' (expected '$expected_branch')."
    echo "       set ALLOW_NON_RUST_RELEASE=1 to override."
    exit 1
fi

echo "==> Creating annotated tag v$V"
git tag -a "v$V" -m "Release $V"

echo "==> Pushing tag v$V"
git push origin "v$V"

echo
echo "Tagged v$V — CI will now build and publish the release artefacts."
echo "Track the run at: https://github.com/bitwisecook/tcl-lsp/actions"
