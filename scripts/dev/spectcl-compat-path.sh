#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Decide whether one repository-relative path can change the SpecTcl 1.x/2.0
# execution contract. Exit 0 for relevant, 1 for unrelated, and 2 when the
# central shipped-pack directory manifest is unusable so CI can fail closed.

set -eu

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
    echo "usage: scripts/dev/spectcl-compat-path.sh REPOSITORY_PATH" >&2
    exit 2
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
MANIFEST=${TCL_LSP_SPECTCL_PACK_DIRS_MANIFEST:-$REPO_ROOT/rust/tcl-spectcl/data/shipped-pack-dirs.txt}
PATH_TO_CLASSIFY=$1

if [ ! -r "$MANIFEST" ]; then
    echo "spectcl-compat-path: cannot read shipped-pack directory manifest $MANIFEST" >&2
    exit 2
fi

if ! awk '
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    $0 ~ /^[[:space:]]/ || $0 ~ /[[:space:]]$/ || $0 ~ /^\// || $0 ~ /\/$/ || $0 ~ /[*?]/ || index($0, "[") || seen[$0]++ {
        printf "spectcl-compat-path: invalid shipped-pack directory on row %d in %s: %s\n", NR, FILENAME, $0 > "/dev/stderr"
        failed = 1
    }
    { count++ }
    END {
        if (count == 0) {
            printf "spectcl-compat-path: no shipped-pack directories in %s\n", FILENAME > "/dev/stderr"
            failed = 1
        }
        exit failed
    }
' "$MANIFEST"; then
    exit 2
fi

# Source/runtime/compiler closure. The shipped pack directories themselves
# come from the manifest below; do not duplicate those paths here.
case "$PATH_TO_CLASSIFY" in
    runtime/rust/* | \
    rust/tcl-bytecode/* | rust/tcl-cmd-core/* | rust/tcl-compiler/* | \
    rust/tcl-core-types/* | rust/tcl-dialect/* | rust/tcl-engine-api/* | \
    rust/tcl-engine-tclvm/* | rust/tcl-host-native/* | rust/tcl-lexer/* | \
    rust/tcl-lsp-core/* | rust/tcl-platform/* | rust/tcl-regex/* | \
    rust/tcl-registry/* | rust/tcl-runtime-api/* | rust/tcl-spec-hooks/* | \
    rust/tcl-spectcl/* | rust/tcl-syntax/* | rust/tcl-test-support/* | \
    rust/tcl-userdirs/* | rust/tcl-version/* | rust/tcl-vm/* | \
    .claude/hooks/session-start.sh | .claude/skills/fetch-tcl-source/* | \
    scripts/dev/ensure-test-deps.sh | scripts/dev/tcl-reference-toolchains.sh | \
    scripts/dev/test-reference-tcl-toolchains.sh | \
    scripts/dev/spectcl-compat-path.sh | scripts/dev/test-spectcl-compat-paths.sh | \
    .cargo/config.toml | Cargo.toml | Cargo.lock | rust-toolchain.toml | \
    Makefile | .github/workflows/ci.yml)
        exit 0
        ;;
esac

while IFS= read -r directory; do
    case "$directory" in
        '' | \#*) continue ;;
    esac
    case "$PATH_TO_CLASSIFY" in
        "$directory"/*) exit 0 ;;
    esac
done < "$MANIFEST"

exit 1
