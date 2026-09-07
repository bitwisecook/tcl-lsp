#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Exit 0 for a path in the root Rust-test closure (nextest plus the full
# workspace doctest command), 1 for unrelated, and 2 for an unusable closure.
# CI turns all errors into a broad test run.
set -eu

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
    echo "usage: scripts/dev/rust-tests-path.sh REPOSITORY_PATH" >&2
    exit 2
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PACKAGE_MANIFEST=${TCL_LSP_RUST_TESTS_PACKAGE_MANIFEST:-$REPO_ROOT/scripts/dev/rust-tests-package-paths.txt}
INPUT_MANIFEST=${TCL_LSP_RUST_TESTS_INPUT_MANIFEST:-$REPO_ROOT/scripts/dev/rust-tests-input-paths.txt}
PATH_TO_CLASSIFY=$1

for manifest in "$PACKAGE_MANIFEST" "$INPUT_MANIFEST"; do
    [ -r "$manifest" ] || { echo "rust-tests-path: cannot read $manifest" >&2; exit 2; }
    awk '
        /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
        $0 ~ /^[[:space:]]/ || $0 ~ /[[:space:]]$/ || $0 ~ /^\// || $0 ~ /\/$/ || $0 ~ /[*?]/ || index($0, "[") || seen[$0]++ {
            printf "rust-tests-path: invalid row %d in %s: %s\n", NR, FILENAME, $0 > "/dev/stderr"; bad=1
        }
        { count++ }
        END { if (count == 0) { print "rust-tests-path: empty manifest " FILENAME > "/dev/stderr"; bad=1 } exit bad }
    ' "$manifest" || exit 2
done

while IFS= read -r directory; do
    case "$directory" in ''|\#*) continue ;; esac
    case "$PATH_TO_CLASSIFY" in "$directory"|"$directory"/*) exit 0 ;; esac
done < "$PACKAGE_MANIFEST"

while IFS= read -r path; do
    case "$path" in ''|\#*) continue ;; esac
    case "$PATH_TO_CLASSIFY" in "$path"|"$path"/*) exit 0 ;; esac
done < "$INPUT_MANIFEST"

exit 1
