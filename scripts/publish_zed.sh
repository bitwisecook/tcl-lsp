#!/usr/bin/env bash
# publish_zed.sh — prepare a local checkout of zed-industries/extensions
# with the tcl submodule advanced to the current tag and the
# extensions.toml version bumped, ready for you to review, push, and
# open a PR yourself.
#
# This script never pushes to your fork and never opens a PR — it stops
# at "branch is ready in <path>; review it, then push and open the PR
# manually" so you can sanity-check the change first.
#
# Zed distributes extensions via the central
# https://github.com/zed-industries/extensions repo, where each
# extension is a git submodule pinned to a specific commit. Every
# release of a published extension is its own PR that:
#
#   1. Advances the submodule pointer at extensions/tcl to the new tag.
#   2. Bumps the corresponding `version = "..."` entry in extensions.toml.
#
# Prerequisites
# -------------
#   - You have a local fork of zed-industries/extensions cloned (or you
#     let this script clone one for you on first run).
#   - Optional env vars:
#       ZED_EXTENSIONS_FORK       owner/repo of your fork (used only for
#                                  the suggested `git push` command in
#                                  the final summary)
#       ZED_EXTENSIONS_CHECKOUT   path to your local clone (default:
#                                  $HOME/.cache/tcl-lsp/zed-extensions)
#
# First-time publish
# ------------------
# Before this script can run, the extension must already be REGISTERED
# in zed-industries/extensions via a one-off PR that adds the submodule
# and the `[tcl]` block in extensions.toml. See
# docs/kcs/kcs-howto-publish-editor-extensions.md for the procedure.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM="zed-industries/extensions"
EXT_NAME="tcl"
EXT_PATH="extensions/${EXT_NAME}"

WORK_DIR="${ZED_EXTENSIONS_CHECKOUT:-$HOME/.cache/tcl-lsp/zed-extensions}"

# Resolve the tag we are publishing.
TAG="$(git -C "$ROOT" describe --tags --exact-match 2>/dev/null || true)"
if [ -z "$TAG" ]; then
    echo "error: HEAD is not a tagged commit — run 'make release-tag V=X.Y.Z' first."
    exit 1
fi
VERSION="${TAG#v}"

FORK="${ZED_EXTENSIONS_FORK:-<your-fork>/extensions}"

echo "==> Preparing local PR contents for $EXT_NAME $VERSION → $UPSTREAM"
echo

# 1. Clone the upstream into the checkout (or refresh it). We pull from
# upstream directly — you swap in your fork's remote at push time.
mkdir -p "$(dirname "$WORK_DIR")"
if [ ! -d "$WORK_DIR/.git" ]; then
    echo "==> Cloning $UPSTREAM into $WORK_DIR"
    git clone --recurse-submodules "https://github.com/${UPSTREAM}.git" "$WORK_DIR"
else
    echo "==> Refreshing $WORK_DIR against upstream/main"
    git -C "$WORK_DIR" fetch origin main
    git -C "$WORK_DIR" checkout main
    git -C "$WORK_DIR" reset --hard origin/main
    git -C "$WORK_DIR" submodule update --init --recursive
fi

# 2. Verify the submodule is registered.
if [ ! -d "$WORK_DIR/$EXT_PATH" ]; then
    echo
    echo "error: $UPSTREAM does not yet carry an extensions/$EXT_NAME submodule."
    echo "       The extension has not been registered. See"
    echo "         docs/kcs/kcs-howto-publish-editor-extensions.md"
    echo "       for the first-time submission procedure (which is a PR"
    echo "       you raise yourself)."
    exit 1
fi

# 3. Advance the submodule pointer to our tag.
echo "==> Advancing $EXT_PATH to $TAG"
git -C "$WORK_DIR/$EXT_PATH" fetch origin --tags
git -C "$WORK_DIR/$EXT_PATH" checkout "$TAG"

# 4. Bump the version in extensions.toml. Surgical rewrite of just the
# [tcl] block's version field.
TOML="$WORK_DIR/extensions.toml"
if [ ! -f "$TOML" ]; then
    echo "error: $TOML not found in checkout — repo layout has changed."
    exit 1
fi

python3 - "$TOML" "$EXT_NAME" "$VERSION" <<'PY'
import pathlib, re, sys
path, name, version = sys.argv[1], sys.argv[2], sys.argv[3]
src = pathlib.Path(path).read_text()
header = f"[{name}]"
idx = src.find(header)
if idx < 0:
    sys.exit(f"error: [{name}] block not found in {path}")
next_section = re.search(r"\n\[[^\]]+\]", src[idx + len(header):])
end = idx + len(header) + (next_section.start() if next_section else len(src))
block = src[idx:end]
new_block, count = re.subn(
    r'^(version\s*=\s*")[^"]*(")',
    rf'\g<1>{version}\g<2>',
    block,
    count=1,
    flags=re.MULTILINE,
)
if count != 1:
    sys.exit(f"error: could not rewrite version line in [{name}] block")
pathlib.Path(path).write_text(src[:idx] + new_block + src[end:])
PY

# 5. Stage the changes on a new branch so they are ready to review.
BRANCH="bump-${EXT_NAME}-${VERSION}"
git -C "$WORK_DIR" checkout -B "$BRANCH"
git -C "$WORK_DIR" add "$EXT_PATH" extensions.toml

if git -C "$WORK_DIR" diff --cached --quiet; then
    echo
    echo "    Nothing to commit — $UPSTREAM already references $TAG with version $VERSION."
    echo "    (No PR needed.)"
    exit 0
fi

echo
echo "==> Pending changes:"
git -C "$WORK_DIR" diff --cached --stat
echo

# 6. Print a manual follow-up plan. We deliberately do NOT push or open a
# PR — those steps belong to you.
PR_TITLE="Bump ${EXT_NAME} to v${VERSION}"
PR_BODY_FILE="$WORK_DIR/.tcl-pr-body.md"
cat > "$PR_BODY_FILE" <<EOF
Bumps the \`${EXT_NAME}\` extension to v${VERSION}.

Release notes:
https://github.com/bitwisecook/tcl-lsp/releases/tag/${TAG}

Submodule advanced and \`extensions.toml\` version field updated to
\`${VERSION}\`.
EOF

cat <<EOF
==> Ready to commit + push from your fork.

    Workdir:  $WORK_DIR
    Branch:   $BRANCH
    Title:    $PR_TITLE
    Body:     $PR_BODY_FILE

    Suggested commands (run these yourself after reviewing the diff):

      cd "$WORK_DIR"
      git diff --cached
      git commit -m "$PR_TITLE"
      git remote add fork git@github.com:${FORK}.git    # one-time
      git push -u fork "$BRANCH"
      gh pr create \\
          --repo $UPSTREAM \\
          --head "${FORK%%/*}:$BRANCH" \\
          --base main \\
          --title "$PR_TITLE" \\
          --body-file "$PR_BODY_FILE"

    The script has NOT pushed anything and has NOT opened a PR — those
    steps are yours to run after you have inspected the staged change.
EOF
