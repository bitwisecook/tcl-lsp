#!/usr/bin/env bash
# publish_sublime.sh — verify the Sublime Text package is "published".
#
# Package Control distributes tcl-lsp via Sublime's `wbond/package_control_channel`.
# Once the one-time channel-submission PR has merged, Package Control
# follows GitHub tags automatically and pulls the `Tcl.sublime-package`
# asset from each tagged release. There is no per-release marketplace
# API call to make — the act of attaching the asset to a GitHub release
# *is* the publish.
#
# This script's job is therefore a verification step:
#
#   1. Confirm we are at a tag.
#   2. Confirm the tagged GitHub Release exists.
#   3. Confirm it includes `Tcl.sublime-package` (the asset name Package
#      Control expects per scripts/publish_helpers/sublime_channel_entry.json).
#   4. Print next-step guidance (first-time channel submission, otherwise
#      Package Control polls within ~hour).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="bitwisecook/tcl-lsp"

# 1. Resolve current tag
TAG="$(git -C "$ROOT" describe --tags --exact-match 2>/dev/null || true)"
if [ -z "$TAG" ]; then
    echo "error: HEAD is not a tagged commit — run 'make release-tag V=X.Y.Z' first."
    exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
    echo "error: gh CLI is required (https://cli.github.com)."
    exit 1
fi

echo "==> Verifying GitHub Release $TAG has the Sublime package asset"

# 2. Release exists?
if ! gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
    echo "error: GitHub release $TAG does not exist yet."
    echo "       CI builds and attaches all artefacts on tag push; wait for it"
    echo "       to finish, then re-run this target."
    exit 1
fi

# 3. Look for the unversioned canonical asset name. The release-build job
# uploads tcl-lsp-sublime-<version>.sublime-package; Package Control will
# follow the GitHub release's "browser_download_url" via the channel entry
# under scripts/publish_helpers/sublime_channel_entry.json, which uses the
# versioned name pattern. Accept either canonical asset.
ASSETS="$(gh release view "$TAG" --repo "$REPO" --json assets --jq '.assets[].name')"

found=""
while IFS= read -r name; do
    case "$name" in
        Tcl.sublime-package|tcl-lsp-sublime-*.sublime-package)
            found="$name"
            ;;
    esac
done <<< "$ASSETS"

if [ -z "$found" ]; then
    echo "error: release $TAG does not contain a .sublime-package asset."
    echo "       Re-run 'make sublime' and 'gh release upload $TAG build/Tcl.sublime-package'."
    exit 1
fi

echo "    OK — $TAG carries $found"
echo

# 4. Guidance
echo "==> Package Control distribution"
echo
echo "    Once tcl-lsp is registered in wbond/package_control_channel,"
echo "    Package Control polls the channel and serves the new tag to"
echo "    users within ~1 hour — no per-release marketplace API call."
