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

# vsce_publish.sh — publish every already-built VSIX to the VS Code
# Marketplace using the local (committed-lockfile) vsce binary.
#
# Invoked by the `publish-vsix-marketplace` job in .github/workflows/ci.yml
# with VSCE_PAT in the environment (an approval-gated marketplace-vscode
# Environment secret).  Runs ONLY the local node_modules vsce binary and
# never fetches npm code, so no freshly-fetched code executes while VSCE_PAT
# is in the environment.  Kept as a committed script (not inline YAML) so the
# workflow stays thin and this credential-adjacent step is reviewable in one
# place.
#
# Usage:  scripts/release/vsce_publish.sh [TAG] [VSIX...]
#         TAG   defaults to the tag in $GITHUB_REF (refs/tags/<TAG>).
#         VSIX  defaults to every *.vsix under dist/ (downloaded and
#               checksum-verified by the workflow before this runs): the
#               untargeted universal package plus six platform-targeted
#               ones.  Each targeted package already carries its target
#               platform in the vsix manifest (baked in by `vsce package
#               --target` at build time), so publish never passes --target
#               again — see "Platform-specific extensions" in the VS Code
#               publishing docs.
#
# Authenticates via VSCE_PAT, which `vsce publish` reads from the environment
# (the keyless Azure/OIDC path was rolled back after it proved unreliable).
#
# Channel (scripts/release/prerelease.sh is the single source of truth): an
# odd-minor 2.x tag (v2.1.x) publishes with --pre-release so it lands on the
# Marketplace pre-release channel; 1.x and even-minor 2.x (v2.2.0) publish
# to the normal channel, keeping 1.x the default install for everyone who
# hasn't opted into pre-releases.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
tag="${1:-${GITHUB_REF#refs/tags/}}"
if [ "$#" -gt 0 ]; then
    shift
fi
if [ "$#" -gt 0 ]; then
    vsixes=("$@")
else
    vsixes=(dist/*.vsix)
fi

prerelease_flag=""
if [ "$(bash "$here/prerelease.sh" "$tag")" = "true" ]; then
    echo "$tag is a pre-release — publishing with --pre-release."
    prerelease_flag="--pre-release"
fi

: "${VSCE_PAT:?VSCE_PAT must be set (marketplace-vscode Environment secret)}"
for vsix in "${vsixes[@]}"; do
    echo "Publishing $vsix via vsce (VSCE_PAT)"
    editors/vscode/node_modules/.bin/vsce publish $prerelease_flag --packagePath "$vsix"
done
