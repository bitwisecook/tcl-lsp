#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Exit 0 for a path in the native lsp-e2e archive closure, 1 for an
# unrelated path, and 2 for an unusable closure. CI turns all errors into a
# full archive and all three partitions.
set -eu

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
    echo "usage: scripts/dev/lsp-e2e-path.sh REPOSITORY_PATH" >&2
    exit 2
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PACKAGE_MANIFEST=${TCL_LSP_E2E_PACKAGE_MANIFEST:-$REPO_ROOT/scripts/dev/lsp-e2e-package-paths.txt}
INPUT_MANIFEST=${TCL_LSP_E2E_INPUT_MANIFEST:-$REPO_ROOT/scripts/dev/lsp-e2e-input-paths.txt}
PATH_TO_CLASSIFY=$1

for manifest in "$PACKAGE_MANIFEST" "$INPUT_MANIFEST"; do
    [ -r "$manifest" ] || { echo "lsp-e2e-path: cannot read $manifest" >&2; exit 2; }
    awk '
        /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
        $0 ~ /^[[:space:]]/ || $0 ~ /[[:space:]]$/ || $0 ~ /^\// || $0 ~ /\/$/ || $0 ~ /[*?]/ || index($0, "[") || index($0, "\\") || index($0, "//") || $0 ~ /(^|\/)(\.\.?)(\/|$)/ || seen[$0]++ {
            printf "lsp-e2e-path: invalid row %d in %s: %s\n", NR, FILENAME, $0 > "/dev/stderr"; bad=1
        }
        { count++ }
        END { if (count == 0) { print "lsp-e2e-path: empty manifest " FILENAME > "/dev/stderr"; bad=1 } exit bad }
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
