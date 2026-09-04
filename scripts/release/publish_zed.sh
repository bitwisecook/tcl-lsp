#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later

# Prepare, but do not push, the Zed registry PR for the current tcl-lsp tag.
# Handles both the first registration and later version bumps.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
UPSTREAM="zed-industries/extensions"
EXT_NAME="tcl-lsp"
EXT_PATH="extensions/${EXT_NAME}"
EXT_SOURCE_PATH="editors/zed"
REPO_URL="https://github.com/bitwisecook/tcl-lsp.git"
WORK_DIR="${ZED_EXTENSIONS_CHECKOUT:-${TMPDIR:-/tmp}/tcl-lsp-zed-extensions}"
FORK="${ZED_EXTENSIONS_FORK:-<your-fork>/extensions}"

TAG="$(git -C "$ROOT" describe --tags --exact-match 2>/dev/null || true)"
if [ -z "$TAG" ]; then
    echo "error: HEAD is not a tag; publish Zed after the vX.Y.Z release tag exists."
    exit 1
fi
VERSION="${TAG#v}"

MANIFEST_VERSION="$({
    sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/$EXT_SOURCE_PATH/extension.toml"
} | head -n 1)"
if [ "$MANIFEST_VERSION" != "$VERSION" ]; then
    echo "error: $EXT_SOURCE_PATH/extension.toml is $MANIFEST_VERSION, but HEAD is $TAG."
    echo "       Set the extension version before creating the release tag."
    exit 1
fi

if [ ! -d "$WORK_DIR/.git" ]; then
    mkdir -p "$(dirname "$WORK_DIR")"
    git clone "https://github.com/${UPSTREAM}.git" "$WORK_DIR"
else
    if [ -n "$(git -C "$WORK_DIR" status --porcelain)" ]; then
        echo "error: $WORK_DIR has uncommitted changes; choose a clean checkout."
        exit 1
    fi
    git -C "$WORK_DIR" fetch origin main
    git -C "$WORK_DIR" switch --detach origin/main
fi

BRANCH="${EXT_NAME}-${VERSION}"
git -C "$WORK_DIR" switch -c "$BRANCH"

if git -C "$WORK_DIR" config --file .gitmodules --get-regexp \
    "submodule\.${EXT_PATH}\.path" >/dev/null 2>&1; then
    ACTION="Update"
    git -C "$WORK_DIR" submodule update --init "$EXT_PATH"
    git -C "$WORK_DIR/$EXT_PATH" fetch origin tag "$TAG"
    git -C "$WORK_DIR/$EXT_PATH" checkout "$TAG"
else
    ACTION="Add"
    git -C "$WORK_DIR" submodule add "$REPO_URL" "$EXT_PATH"
    git -C "$WORK_DIR/$EXT_PATH" checkout "$TAG"
fi

TOML="$WORK_DIR/extensions.toml"
python3 - "$TOML" "$EXT_NAME" "$EXT_PATH" "$EXT_SOURCE_PATH" "$VERSION" <<'PY'
import pathlib
import re
import sys

path, name, submodule, source_path, version = sys.argv[1:]
toml = pathlib.Path(path)
text = toml.read_text()
block = (
    f"[{name}]\n"
    f'submodule = "{submodule}"\n'
    f'path = "{source_path}"\n'
    f'version = "{version}"\n'
)
pattern = re.compile(rf"(?ms)^\[{re.escape(name)}\]\n.*?(?=^\[|\Z)")
if pattern.search(text):
    text = pattern.sub(block + "\n", text, count=1)
else:
    text = text.rstrip() + "\n\n" + block
toml.write_text(text)
PY

if command -v pnpm >/dev/null 2>&1; then
    pnpm --dir "$WORK_DIR" install --frozen-lockfile
    pnpm --dir "$WORK_DIR" sort-extensions
elif command -v corepack >/dev/null 2>&1; then
    corepack pnpm --dir "$WORK_DIR" install --frozen-lockfile
    corepack pnpm --dir "$WORK_DIR" sort-extensions
else
    echo "error: pnpm (or corepack) is required by zed-industries/extensions."
    exit 1
fi

git -C "$WORK_DIR" add .gitmodules "$EXT_PATH" extensions.toml

TITLE="$ACTION $EXT_NAME extension v$VERSION"
BODY_FILE="$WORK_DIR/.tcl-lsp-pr-body.md"
printf '%s\n' \
    "$ACTION the Tcl LSP extension at v$VERSION." \
    "" \
    "Release: https://github.com/bitwisecook/tcl-lsp/releases/tag/$TAG" \
    >"$BODY_FILE"

echo
echo "Prepared the Zed registry change; nothing was pushed."
echo "  checkout: $WORK_DIR"
echo "  branch:   $BRANCH"
echo "  title:    $TITLE"
echo "  body:     $BODY_FILE"
echo
echo "Review with: git -C \"$WORK_DIR\" diff --cached"
echo "Then commit, push to $FORK, and open the PR against $UPSTREAM:main."
