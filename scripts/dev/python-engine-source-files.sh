#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Print the tracked source/build inputs that determine the native f5report
# wheel. This is shared by the local venv stamp and GitHub Actions cache key.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PACKAGE_MANIFEST=${TCL_LSP_PYTHON_ENGINE_PACKAGE_MANIFEST:-$REPO_ROOT/scripts/dev/python-engine-package-paths.txt}

if [ ! -r "$PACKAGE_MANIFEST" ]; then
    echo "cannot read Python engine package closure: $PACKAGE_MANIFEST" >&2
    exit 2
fi
awk '
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    $0 ~ /^[[:space:]]/ || $0 ~ /[[:space:]]$/ || $0 ~ /^\// || $0 ~ /\/$/ || $0 ~ /[*?]/ || index($0, "[") || index($0, "\\") || index($0, "//") || $0 ~ /(^|\/)(\.\.?)(\/|$)/ || seen[$0]++ {
        printf "python-engine-source-files: invalid row %d in %s: %s\n", NR, FILENAME, $0 > "/dev/stderr"; bad=1
    }
    { count++ }
    END { if (count == 0) { print "python-engine-source-files: empty manifest " FILENAME > "/dev/stderr"; bad=1 } exit bad }
' "$PACKAGE_MANIFEST" || exit 2

paths=$(mktemp)
trap 'rm -f "$paths"' EXIT HUP INT TERM

while IFS= read -r package_root || [ -n "$package_root" ]; do
    case "$package_root" in '' | \#*) continue ;; esac
    if [ ! -d "$REPO_ROOT/$package_root" ]; then
        echo "missing Python engine package root: $package_root" >&2
        exit 2
    fi
    git -C "$REPO_ROOT" ls-files -- "$package_root" >> "$paths"
done < "$PACKAGE_MANIFEST"
git -C "$REPO_ROOT" ls-files -- \
    .cargo/config.toml Cargo.toml Makefile rust-toolchain.toml \
    scripts/dev/python-ci-path.sh \
    scripts/dev/python-engine-package-paths.txt \
    scripts/dev/python-engine-source-files.sh >> "$paths"

LC_ALL=C sort -u "$paths"
