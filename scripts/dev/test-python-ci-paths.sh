#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract tests for the Python lane's native-engine dependency closure,
# source-content cache identity, and fail-closed CI wiring.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
CLASSIFIER=$SCRIPT_DIR/python-ci-path.sh
SOURCE_FILES=$SCRIPT_DIR/python-engine-source-files.sh
LOCKFILE=$REPO_ROOT/rust/bigip-report-gen/python/Cargo.lock
WORKFLOW=$REPO_ROOT/.github/workflows/ci.yml

fail() {
    echo "$1" >&2
    exit 1
}

expect_relevant() {
    "$CLASSIFIER" "$1" || fail "expected Python-relevant path: $1"
}

expect_unrelated() {
    if "$CLASSIFIER" "$1"; then
        fail "expected Python-unrelated path: $1"
    else
        status=$?
        [ "$status" -eq 1 ] || fail "classifier failed for $1 with status $status"
    fi
}

[ -r "$LOCKFILE" ] || fail "cannot read $LOCKFILE"

local_packages=$(
    awk '
        /^\[\[package\]\]/ { name = ""; source = "" }
        /^name = / { name = $3; gsub(/"/, "", name) }
        /^source = / { source = $3 }
        /^$/ { if (name != "" && source == "") print name; name = "" }
        END { if (name != "" && source == "") print name }
    ' "$LOCKFILE" | sort -u
)
[ -n "$local_packages" ] || fail "Python engine lockfile has no local packages"

closure_dirs=''
expected_roots=''
for package in $local_packages; do
    case "$package" in
        bigip-report-gen-py)
            directory=rust/bigip-report-gen/python
            package_root=rust/bigip-report-gen
            ;;
        bigip-report-gen-rust)
            directory=rust/bigip-report-gen/rust
            package_root=rust/bigip-report-gen
            ;;
        *)
            directory=rust/$package
            package_root=$directory
            ;;
    esac
    [ -f "$REPO_ROOT/$directory/Cargo.toml" ] \
        || fail "local package $package maps to missing $directory/Cargo.toml"
    closure_dirs="$closure_dirs $directory"
    expected_roots="$expected_roots
$package_root"
    expect_relevant "$directory/Cargo.toml"
    expect_relevant "$directory/src/lib.rs"
done

expected_roots=$(printf '%s\n' "$expected_roots" | sed '/^$/d' | LC_ALL=C sort -u)
manifest_roots=$(sed '/^[[:space:]]*#/d; /^[[:space:]]*$/d' \
    "$SCRIPT_DIR/python-engine-package-paths.txt" | LC_ALL=C sort -u)
if [ "$manifest_roots" != "$expected_roots" ]; then
    echo "Python engine package closure differs from its lockfile:" >&2
    printf 'expected:\n%s\nactual:\n%s\n' "$expected_roots" "$manifest_roots" >&2
    exit 1
fi

for manifest in "$REPO_ROOT"/rust/*/Cargo.toml; do
    directory=rust/$(basename "$(dirname "$manifest")")
    in_closure=no
    for known in $closure_dirs; do
        [ "$known" = "$directory" ] && in_closure=yes
    done
    [ "$in_closure" = yes ] && continue
    expect_unrelated "$directory/src/lib.rs"
done

expect_relevant rust/bigip-report-gen/frontend/dist/report.js
expect_relevant editors/sublime/plugin.py
expect_relevant typings/sublime.pyi
expect_relevant Cargo.toml
expect_relevant rust-toolchain.toml
expect_relevant Makefile
expect_relevant .github/workflows/ci.yml
expect_relevant scripts/dev/python-engine-package-paths.txt
expect_unrelated README.md
expect_unrelated editors/vscode/src/extension.ts

if "$CLASSIFIER" '' 2>/dev/null; then
    fail "empty path unexpectedly classified relevant"
else
    status=$?
    [ "$status" -eq 2 ] || fail "empty path returned $status, expected 2"
fi

listed_file=$(mktemp)
manifest_probe=$(mktemp)
trap 'rm -f "$listed_file" "$manifest_probe"' EXIT HUP INT TERM
$SOURCE_FILES > "$listed_file"
for required in \
    rust/bigip-report-gen/python/src/lib.rs \
    rust/bigip-report-gen/frontend/dist/report.js \
    rust/tcl-bigip-query/src/value.rs \
    Cargo.toml Makefile rust-toolchain.toml \
    scripts/dev/python-engine-package-paths.txt; do
    grep -Fxq "$required" "$listed_file" \
        || fail "native-engine source identity omits $required"
done
if grep -Fxq editors/vscode/src/extension.ts "$listed_file"; then
    fail "native-engine source identity includes unrelated VS Code source"
fi

printf '%s\n' '../outside' > "$manifest_probe"
if TCL_LSP_PYTHON_ENGINE_PACKAGE_MANIFEST=$manifest_probe \
    "$CLASSIFIER" README.md >/dev/null 2>&1; then
    fail "malformed Python path closure unexpectedly classified a path"
else
    status=$?
    [ "$status" -eq 2 ] || fail "malformed path closure returned $status, expected 2"
fi
if TCL_LSP_PYTHON_ENGINE_PACKAGE_MANIFEST=$manifest_probe \
    "$SOURCE_FILES" >/dev/null 2>&1; then
    fail "malformed native-engine source closure unexpectedly succeeded"
else
    status=$?
    [ "$status" -eq 2 ] || fail "malformed source closure returned $status, expected 2"
fi

hash_one=$(make -s -C "$REPO_ROOT" python-engine-source-hash)
hash_two=$(make -s -C "$REPO_ROOT" python-engine-source-hash)
[ "$hash_one" = "$hash_two" ] || fail "native-engine source hash is unstable"
[ "${#hash_one}" -eq 40 ] || fail "native-engine source hash has wrong length: $hash_one"
case "$hash_one" in *[!0-9a-f]*) fail "native-engine source hash is malformed: $hash_one" ;; esac

grep -Fq 'scripts/dev/python-ci-path.sh' "$WORKFLOW" \
    || fail "CI does not use the Python path classifier"
grep -Fq 'TCL_LSP_PYTHON_ENGINE_PACKAGE_MANIFEST="$trusted/python-engine-package-paths.txt"' "$WORKFLOW" \
    || fail "trusted PR classification does not use the base package closure"
grep -Fq "engine=\$(make --no-print-directory python-engine-source-hash)" "$WORKFLOW" \
    || fail "CI cache key does not use the native-engine source identity"
grep -Fq 'hash=$$($(MAKE) --no-print-directory python-engine-source-hash)' "$REPO_ROOT/Makefile" \
    || fail "local venv stamp does not use the native-engine source identity"
grep -Fq 'run: make test-py-engine' "$WORKFLOW" \
    || fail "Python CI does not run the native-engine binding tests"

count=$(printf '%s\n' "$local_packages" | wc -l | tr -d ' ')
echo "Python CI path and cache contracts passed ($count local packages)"
