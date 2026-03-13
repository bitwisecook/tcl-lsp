#!/usr/bin/env bash
# release.sh — bump version across all source files, commit, tag, and push.
#
# Usage:  scripts/release.sh X.Y.Z
#         make release-tag V=X.Y.Z
set -euo pipefail

V="${1:?Usage: scripts/release.sh X.Y.Z}"

# Validate
[[ "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "error: '$V' is not a valid version (expected X.Y.Z)"; exit 1; }
git diff --quiet && git diff --cached --quiet || { echo "error: worktree is dirty — commit or stash first"; exit 1; }
if git rev-parse "v$V" >/dev/null 2>&1; then
    echo "error: tag v$V already exists"
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Bump version in source files
echo "==> Bumping version to $V"

# pyproject.toml
sed -i '' "s/^version = \".*\"/version = \"$V\"/" "$ROOT/pyproject.toml"

# editors/vscode/package.json (first "version" key only)
sed -i '' "s/\"version\": \".*\"/\"version\": \"$V\"/" "$ROOT/editors/vscode/package.json"

# explorer/static/worker.js (wheel filename)
sed -i '' "s/tcl_lsp-.*-py3-none-any\.whl/tcl_lsp-$V-py3-none-any.whl/" "$ROOT/explorer/static/worker.js"

# Note: server/server.py version comes from git describe at build time
# via server/_build_info.py — no source file update needed.

# Commit, tag, push
echo "==> Committing version bump"
git add \
    "$ROOT/pyproject.toml" \
    "$ROOT/editors/vscode/package.json" \
    "$ROOT/explorer/static/worker.js"

git commit -m "Bump version to $V"

echo "==> Creating annotated tag v$V"
git tag -a "v$V" -m "Release $V"

echo "==> Pushing with tags"
git push --follow-tags

echo ""
echo "Released v$V"
