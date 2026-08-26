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

# ovsx_publish.sh — publish every already-built VSIX to Open VSX
# (open-vsx.org) using the local (committed-lockfile) ovsx binary.  Open VSX
# is what code-server / openvscode-server / Gitpod / Theia installs use.
#
# Invoked by the `publish-vsix-openvsx` job in .github/workflows/ci.yml with
# OVSX_PAT in the environment (an approval-gated marketplace-openvsx
# Environment secret).  Runs ONLY the local node_modules ovsx binary and
# never fetches npm code, so no freshly-fetched code executes while OVSX_PAT
# is in the environment.  Kept as a committed script (not inline YAML) so the
# workflow stays thin and this credential-adjacent step is reviewable in one
# place.
#
# Usage:  scripts/release/ovsx_publish.sh [TAG] [VSIX...]
#         TAG   defaults to the tag in $GITHUB_REF (refs/tags/<TAG>).  Kept
#               as an argument for parity with vsce_publish.sh, but this
#               script no longer reads it: the channel isn't decided here
#               (see below).
#         VSIX  defaults to every *.vsix under dist/ (downloaded and
#               checksum-verified by the workflow before this runs): the
#               untargeted universal package plus six platform-targeted
#               ones.  Each targeted package already carries its target
#               platform in the vsix manifest (baked in by `vsce package
#               --target` at build time), so publish never passes --target
#               again — Open VSX accepts the same platform-targeted packages
#               vsce produces, see "Platform-specific extensions" in the VS
#               Code publishing docs.
#
# Authenticates via OVSX_PAT, which `ovsx publish` reads from the
# environment natively (lib/util.js falls back to process.env.OVSX_PAT when
# no --pat is given).
#
# Channel: NOT decided by this script.  `ovsx publish` ignores --pre-release
# for an already-packaged VSIX (node_modules/ovsx/lib/publish.js doPublish()
# warns "Ignoring option '--pre-release' for prepackaged extension." and
# sends no pre-release parameter at all) — every VSIX handed to this script
# is prepackaged, so passing the flag here would be a silent no-op.  The
# channel instead rides inside the VSIX manifest itself
# (Microsoft.VisualStudio.Code.PreRelease), baked in by `vsce package
# $(VSCE_PRERELEASE_FLAG)` at build time (Makefile), which open-vsx.org
# reads directly from the uploaded package.  scripts/release/prerelease.sh
# still governs that manifest flag at package time — just not anything in
# this script.
set -euo pipefail

tag="${1:-${GITHUB_REF#refs/tags/}}"
if [ "$#" -gt 0 ]; then
    shift
fi
if [ "$#" -gt 0 ]; then
    vsixes=("$@")
else
    vsixes=(dist/*.vsix)
fi

: "${OVSX_PAT:?OVSX_PAT must be set (marketplace-openvsx Environment secret)}"
for vsix in "${vsixes[@]}"; do
    echo "Publishing $vsix via ovsx (OVSX_PAT)"
    # --skip-duplicate: a partial failure partway through this loop (e.g.
    # package 5 of 7) must not turn a re-run into a hard failure on
    # packages 1-4, which have already published successfully.
    editors/vscode/node_modules/.bin/ovsx publish --skip-duplicate --packagePath "$vsix"
done
