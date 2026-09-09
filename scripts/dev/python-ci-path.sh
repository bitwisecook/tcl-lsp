#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Exit 0 when a repository-relative path can change Python lint/typecheck or
# the native f5report engine it compiles, 1 when it cannot, and 2 when the
# request or committed dependency closure is unusable. CI fails closed on 2.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PACKAGE_MANIFEST=${TCL_LSP_PYTHON_ENGINE_PACKAGE_MANIFEST:-$REPO_ROOT/scripts/dev/python-engine-package-paths.txt}

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
    echo "usage: scripts/dev/python-ci-path.sh REPOSITORY_PATH" >&2
    exit 2
fi
if [ ! -r "$PACKAGE_MANIFEST" ]; then
    echo "cannot read Python engine package closure: $PACKAGE_MANIFEST" >&2
    exit 2
fi
awk '
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    $0 ~ /^[[:space:]]/ || $0 ~ /[[:space:]]$/ || $0 ~ /^\// || $0 ~ /\/$/ || $0 ~ /[*?]/ || index($0, "[") || index($0, "\\") || index($0, "//") || $0 ~ /(^|\/)(\.\.?)(\/|$)/ || seen[$0]++ {
        printf "python-ci-path: invalid row %d in %s: %s\n", NR, FILENAME, $0 > "/dev/stderr"; bad=1
    }
    { count++ }
    END { if (count == 0) { print "python-ci-path: empty manifest " FILENAME > "/dev/stderr"; bad=1 } exit bad }
' "$PACKAGE_MANIFEST" || exit 2

PATH_TO_CLASSIFY=$1

case "$PATH_TO_CLASSIFY" in
    *.py | *.pyi | typings/* | ruff.toml | pyrightconfig.json)
        exit 0
        ;;
    .cargo/config.toml | Cargo.toml | rust-toolchain.toml | Makefile | \
    .github/workflows/ci.yml | scripts/dev/python-*)
        exit 0
        ;;
esac

while IFS= read -r package_root || [ -n "$package_root" ]; do
    case "$package_root" in '' | \#*) continue ;; esac
    case "$PATH_TO_CLASSIFY" in
        "$package_root" | "$package_root"/*) exit 0 ;;
    esac
done < "$PACKAGE_MANIFEST"

exit 1
