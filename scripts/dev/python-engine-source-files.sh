#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Print the tracked compilation inputs that determine the native f5report
# wheel. This is shared by the local venv stamp and GitHub Actions cache key.
#
# The Python job deliberately classifies whole dependency packages as relevant:
# a Rust test change still needs the binding tests to run. The wheel cache stays
# broad by default, because library sources may deliberately include fixture
# data under `tests/`. It omits only exact Cargo target roots that are solely
# integration tests, examples, or benches, plus the Python binding's own pytest
# tree; neither is compiled by maturin into the extension.

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
target_roots=$(mktemp)
filtered=$(mktemp)
trap 'rm -f "$paths" "$target_roots" "$filtered"' EXIT HUP INT TERM

if ! cargo metadata --locked --format-version 1 \
    --manifest-path "$REPO_ROOT/rust/bigip-report-gen/python/Cargo.toml" | \
    python3 -c '
import json
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
metadata = json.load(sys.stdin)
kinds_by_source = {}
for package in metadata["packages"]:
    for target in package["targets"]:
        source = Path(target["src_path"]).resolve()
        kinds_by_source.setdefault(source, set()).update(target["kind"])

for source, kinds in kinds_by_source.items():
    if not kinds or not kinds <= {"test", "bench", "example"}:
        continue
    try:
        print(source.relative_to(root).as_posix())
    except ValueError:
        pass
    ' "$REPO_ROOT" > "$target_roots"; then
    echo "cannot derive Python engine non-compilation target roots" >&2
    exit 2
fi

while IFS= read -r package_root || [ -n "$package_root" ]; do
    case "$package_root" in '' | \#*) continue ;; esac
    if [ ! -d "$REPO_ROOT/$package_root" ]; then
        echo "missing Python engine package root: $package_root" >&2
        exit 2
    fi
    git -C "$REPO_ROOT" ls-files -- "$package_root" >> "$paths"
done < "$PACKAGE_MANIFEST"

awk '
    NR == FNR { omit[$0] = 1; next }
    $0 ~ /^rust\/bigip-report-gen\/python\/tests\// { next }
    !($0 in omit) { print }
' "$target_roots" "$paths" > "$filtered"

git -C "$REPO_ROOT" ls-files -- \
    .cargo/config.toml Cargo.toml Makefile rust-toolchain.toml \
    scripts/dev/python-ci-path.sh \
    scripts/dev/python-engine-package-paths.txt \
    scripts/dev/python-engine-source-files.sh >> "$filtered"

LC_ALL=C sort -u "$filtered"
