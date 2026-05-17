#!/usr/bin/env bash
# publish_sublime.sh — push the just-built Sublime package tree into the
# bitwisecook/tcl-lsp-sublime-text mirror repo at the current tag, so
# Package Control (which fetches the git source tarball of each tag) sees
# the new version.
#
# Why a mirror repo?
# ------------------
# Package Control's release auto-discovery (`tags: true`) downloads the
# *source* archive of each git tag and unpacks it as the package
# contents. Our package is a monorepo subdirectory plus a bundled
# server, so the main repo's tarball is not a valid Sublime package
# layout. The mirror at bitwisecook/tcl-lsp-sublime-text exists solely
# to *be* that valid layout, with its contents replaced from
# build/sublime-stage on every release.
#
# Flow per release
# ----------------
#   1. Resolve the current tag (HEAD must be at v<X.Y.Z>).
#   2. Confirm `make sublime` ran (build/sublime-stage exists).
#   3. (Best-effort) confirm the GitHub Release on the main repo carries
#      the .sublime-package asset.
#   4. Clone or refresh the mirror at $HOME/.cache/tcl-lsp/sublime-mirror.
#   5. Replace the mirror's working tree with build/sublime-stage.
#   6. Commit "Release v<X.Y.Z>" and create an annotated tag v<X.Y.Z>.
#   7. Push main + tag to the mirror's origin.
#
# Dry-run mode
# ------------
# Set TCL_LSP_SUBLIME_DRY_RUN=1 to perform steps 1–6 but skip the push,
# leaving the staged commit + tag in the local mirror clone for review.
#
# One-time setup
# --------------
# Before the first run, the empty mirror repo must exist on GitHub:
#
#   gh repo create bitwisecook/tcl-lsp-sublime-text --public \
#       --description "Sublime Text mirror of tcl-lsp for Package Control"
#
# This script never creates the repo for you.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="bitwisecook/tcl-lsp"
MIRROR_REPO="${TCL_LSP_SUBLIME_MIRROR_REPO:-bitwisecook/tcl-lsp-sublime-text}"
STAGE_DIR="$ROOT/build/sublime-stage"
MIRROR_DIR="${TCL_LSP_SUBLIME_MIRROR_DIR:-$HOME/.cache/tcl-lsp/sublime-mirror}"
DRY_RUN="${TCL_LSP_SUBLIME_DRY_RUN:-0}"

# 1. Resolve current tag.
TAG="$(git -C "$ROOT" describe --tags --exact-match 2>/dev/null || true)"
if [ -z "$TAG" ]; then
    echo "error: HEAD is not a tagged commit — run 'make release-tag V=X.Y.Z' first."
    exit 1
fi
VERSION="${TAG#v}"

# 2. Verify make sublime has run.
if [ ! -d "$STAGE_DIR" ]; then
    echo "error: $STAGE_DIR not found — run 'make sublime' first."
    exit 1
fi

echo "==> Mirroring Sublime package $VERSION → $MIRROR_REPO"
echo

# 3. Best-effort: confirm the .sublime-package asset is on the main
# repo's release. The mirror push doesn't strictly need this, but it's
# the canonical sanity check that CI has finished its tagged build.
if command -v gh >/dev/null 2>&1 \
    && gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
    ASSETS="$(gh release view "$TAG" --repo "$REPO" --json assets --jq '.assets[].name')"
    found=""
    while IFS= read -r name; do
        case "$name" in
            tcl-lsp-sublime-*.sublime-package|Tcl.sublime-package)
                found="$name"
                ;;
        esac
    done <<< "$ASSETS"
    if [ -n "$found" ]; then
        echo "    Main repo release $TAG carries $found (canonical for manual installs)."
    else
        echo "    warning: $REPO release $TAG does not yet carry a .sublime-package asset."
        echo "             Package Control will read the mirror, not the asset, but you'll"
        echo "             want CI's release upload to finish before announcing the release."
    fi
else
    echo "    (gh not installed or release $TAG missing on $REPO — skipping asset check.)"
fi
echo

# 4. Clone or refresh the mirror.
mkdir -p "$(dirname "$MIRROR_DIR")"
if [ ! -d "$MIRROR_DIR/.git" ]; then
    echo "==> Cloning $MIRROR_REPO into $MIRROR_DIR"
    if ! git clone "https://github.com/${MIRROR_REPO}.git" "$MIRROR_DIR" 2>/dev/null; then
        cat <<EOF
error: failed to clone https://github.com/${MIRROR_REPO}.git

       If the repo does not exist yet, create it as a one-time step:

         gh repo create $MIRROR_REPO --public \\
             --description "Sublime Text mirror of tcl-lsp for Package Control"

       Then re-run 'make publish-sublime'.
EOF
        exit 1
    fi
fi

# Sync to the latest origin/main, or initialise a fresh main if the repo
# has never received a push.
if git -C "$MIRROR_DIR" ls-remote --exit-code origin main >/dev/null 2>&1; then
    git -C "$MIRROR_DIR" fetch origin main
    git -C "$MIRROR_DIR" checkout -B main origin/main
else
    echo "==> Mirror has no main branch yet — bootstrapping"
    git -C "$MIRROR_DIR" checkout -B main
fi

# Refuse to clobber an existing tag on the mirror.
if git -C "$MIRROR_DIR" ls-remote --exit-code --tags origin "$TAG" >/dev/null 2>&1; then
    echo
    echo "    Tag $TAG already exists on $MIRROR_REPO — nothing to do."
    echo "    (Package Control already sees this release.)"
    exit 0
fi

# A previous TCL_LSP_SUBLIME_DRY_RUN=1 invocation may have created the
# tag locally but skipped the push. The remote check above confirmed the
# tag is unpublished, so the stale local pointer should be regenerated
# against the freshly-staged commit rather than blocking the real publish
# with `git tag: tag '...' already exists`.
if git -C "$MIRROR_DIR" rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null; then
    echo "==> Clearing stale local tag $TAG left over from a previous dry-run"
    git -C "$MIRROR_DIR" tag -d "$TAG" >/dev/null
fi

# 5. Replace mirror tree with build/sublime-stage. Preserve .git only.
echo "==> Refreshing mirror tree from $STAGE_DIR"
find "$MIRROR_DIR" -mindepth 1 -maxdepth 1 ! -name '.git' -exec rm -rf {} +
cp -a "$STAGE_DIR/." "$MIRROR_DIR/"

# Add a short README so the mirror page on GitHub points at the canonical
# project rather than looking like an orphaned dump of files.
cat > "$MIRROR_DIR/README.md" <<EOF
# tcl-lsp (Sublime Text mirror)

This repository is the **Sublime Text Package Control mirror** of
[tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

It exists so Package Control can fetch the package contents via
\`tags: true\` against a flat repo. Every tag here is built from
\`editors/sublime-text/\` in the canonical monorepo plus a bundled
language server.

**Do not file issues, send PRs, or develop here.** Use
<https://github.com/bitwisecook/tcl-lsp> for everything.

This release: \`$TAG\` — see
<https://github.com/bitwisecook/tcl-lsp/releases/tag/$TAG>.

## Licence

AGPL-3.0-or-later, mirrored from the canonical repo.
EOF

# 6. Commit + tag.
git -C "$MIRROR_DIR" add -A

# Default to the canonical repo's most-recent author for both commit
# and annotated-tag creation when the local mirror clone has no
# identity configured. GIT_*_NAME/EMAIL win over git config, so this
# is safe even on a fully-configured machine — it just keeps the
# attribution consistent with the source repo.
if [ -z "$(git -C "$MIRROR_DIR" config user.email 2>/dev/null)" ]; then
    export GIT_AUTHOR_NAME="$(git -C "$ROOT" log -1 --format='%an')"
    export GIT_AUTHOR_EMAIL="$(git -C "$ROOT" log -1 --format='%ae')"
    export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
    export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"
fi

if git -C "$MIRROR_DIR" diff --cached --quiet; then
    echo "    No content change between $MIRROR_REPO@main and the new build."
    echo "    Tagging the existing tip as $TAG anyway so Package Control sees it."
else
    git -C "$MIRROR_DIR" commit -m "Release $TAG"
fi

git -C "$MIRROR_DIR" tag -a "$TAG" -m "Release $TAG"

# 7. Push (or stop on dry-run).
echo
echo "==> Mirror state ready in $MIRROR_DIR"
git -C "$MIRROR_DIR" --no-pager log -1 --stat | sed 's/^/    /'
echo

if [ "$DRY_RUN" = "1" ]; then
    echo "    TCL_LSP_SUBLIME_DRY_RUN=1 — skipping push."
    echo "    Push manually with:"
    echo "      git -C $MIRROR_DIR push origin main --follow-tags"
    exit 0
fi

echo "==> Pushing main + $TAG to $MIRROR_REPO"
git -C "$MIRROR_DIR" push origin main --follow-tags

echo
echo "    Done. Package Control's scraper will pick up $TAG within ~1 hour."
