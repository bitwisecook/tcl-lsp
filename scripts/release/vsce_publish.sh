#!/usr/bin/env bash
# vsce_publish.sh — publish an already-built VSIX to the VS Code Marketplace
# using the local (committed-lockfile) vsce binary.
#
# Invoked by the `publish-vsix-marketplace` job in .github/workflows/ci.yml
# AFTER azure/login has minted a short-lived Entra credential (keyless — no
# PAT at rest).  Runs ONLY the local node_modules vsce binary and never
# fetches npm code, preserving the job's hardening invariant that no
# freshly-fetched code executes while the marketplace-capable Azure session
# is live.  Kept as a committed script (not inline YAML) so the workflow
# stays thin and this credential-adjacent step is reviewable in one place.
#
# Usage:  scripts/release/vsce_publish.sh [TAG] [VSIX]
#         TAG  defaults to the tag in $GITHUB_REF (refs/tags/<TAG>).
#         VSIX defaults to the single *.vsix under dist/ (downloaded and
#              checksum-verified by the workflow before this runs).
#
# Authenticates via vsce --azure-credential, which reads the ambient
# DefaultAzureCredential established by azure/login — no token argument.
#
# Channel (scripts/release/prerelease.sh is the single source of truth): an
# odd-minor 2.x tag (v2.1.x) publishes with --pre-release so it lands on the
# Marketplace pre-release channel; 1.x and even-minor 2.x (v2.2.0) publish
# to the normal channel, keeping 1.x the default install for everyone who
# hasn't opted into pre-releases.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
tag="${1:-${GITHUB_REF#refs/tags/}}"
vsix="${2:-$(ls dist/*.vsix)}"

prerelease_flag=""
if [ "$(bash "$here/prerelease.sh" "$tag")" = "true" ]; then
    echo "$tag is a pre-release — publishing with --pre-release."
    prerelease_flag="--pre-release"
fi

echo "Publishing $vsix via vsce --azure-credential (no PAT)"
editors/vscode/node_modules/.bin/vsce publish $prerelease_flag --azure-credential --packagePath "$vsix"
