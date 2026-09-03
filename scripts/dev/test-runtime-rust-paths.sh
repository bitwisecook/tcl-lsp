#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract tests for the standalone-runtime CI lane (issue #1768):
#
#   1. `scripts/dev/runtime-rust-path.sh` classifies exactly the runtime's
#      resolved local package closure and its external Tcl 9 smoke corpus as
#      relevant — no less (a new path dependency or oracle edit must trigger
#      the job) and no more (an over-broad entry must not make every PR pay).
#   2. `make runtime-rust-test` really runs the standalone suite locked, with
#      the numeric tower explicitly enabled.
#   3. `.github/workflows/ci.yml` wires the classifier to a step-level,
#      fail-safe skip on a job that always reports.
#
# The closure is derived from `runtime/rust/Cargo.lock` rather than hardcoded:
# a lock entry with no `source` key is a local path package, which is exactly
# the set `cargo metadata` resolves, and using the committed lockfile keeps
# this test fast and offline.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CLASSIFIER=$SCRIPT_DIR/runtime-rust-path.sh
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
LOCKFILE=$REPO_ROOT/runtime/rust/Cargo.lock

expect_relevant() {
    if ! "$CLASSIFIER" "$1"; then
        echo "expected runtime-relevant path: $1" >&2
        exit 1
    fi
}

expect_unrelated() {
    if "$CLASSIFIER" "$1"; then
        echo "expected unrelated path: $1" >&2
        exit 1
    else
        status=$?
        if [ "$status" -ne 1 ]; then
            echo "classifier failed for unrelated path $1 with status $status" >&2
            exit 1
        fi
    fi
}

if [ ! -r "$LOCKFILE" ]; then
    echo "cannot read $LOCKFILE — the locked runtime closure is unknown" >&2
    exit 1
fi

# Local (path) packages: a [[package]] block with a name and no source.
local_packages=$(
    awk '
        /^\[\[package\]\]/ { name = ""; source = "" }
        /^name = / { name = $3; gsub(/"/, "", name) }
        /^source = / { source = $3 }
        /^$/ { if (name != "" && source == "") print name; name = "" }
        END { if (name != "" && source == "") print name }
    ' "$LOCKFILE" | sort -u
)

if [ -z "$local_packages" ]; then
    echo "no local packages found in $LOCKFILE — the closure check would be vacuous" >&2
    exit 1
fi

# Every crate in the closure must be classified relevant, and its directory
# must actually exist (a renamed crate must not silently drop out of the lane).
closure_dirs=''
for package in $local_packages; do
    case "$package" in
        tcl-runtime) directory=runtime/rust ;;
        *) directory=rust/$package ;;
    esac
    if [ ! -f "$REPO_ROOT/$directory/Cargo.toml" ]; then
        echo "closure package $package maps to $directory, which has no Cargo.toml" >&2
        exit 1
    fi
    closure_dirs="$closure_dirs $directory"
    expect_relevant "$directory/src/lib.rs"
    expect_relevant "$directory/Cargo.toml"
done

# Nothing OUTSIDE the closure may be classified relevant. Checking every other
# workspace crate is what stops the case list quietly widening to `rust/*`,
# which would run the standalone suite on every Rust PR in the repository.
for manifest in "$REPO_ROOT"/rust/*/Cargo.toml; do
    directory=rust/$(basename "$(dirname "$manifest")")
    in_closure=no
    for known in $closure_dirs; do
        [ "$known" = "$directory" ] && in_closure=yes
    done
    [ "$in_closure" = yes ] && continue
    expect_unrelated "$directory/src/lib.rs"
done

# Build inputs and the gate's own definition.
expect_relevant runtime/rust/Cargo.lock
expect_relevant samples/tcl9_smoke/eval/01_braced_literal.tcl
expect_relevant samples/tcl9_smoke/eval/01_braced_literal.expected
expect_relevant rust-toolchain.toml
expect_relevant Makefile
expect_relevant .github/workflows/ci.yml
expect_relevant scripts/dev/runtime-rust-path.sh
expect_relevant scripts/dev/test-runtime-rust-paths.sh

expect_unrelated README.md
expect_unrelated samples/hello.tcl
expect_unrelated docs/design/compiler/wasm-native-lowering-plan.md
expect_unrelated editors/vscode/src/extension.ts

# A malformed request must fail closed (status 2), not answer "unrelated":
# ci.yml turns any status other than 0/1 into "everything changed".
if "$CLASSIFIER" '' 2>/dev/null; then
    echo "empty path unexpectedly classified relevant" >&2
    exit 1
else
    status=$?
    if [ "$status" -ne 2 ]; then
        echo "empty path returned $status, expected 2 (fail closed)" >&2
        exit 1
    fi
fi

echo "standalone runtime path classifier tests passed"

# The make target CI invokes must actually be the locked standalone suite with
# the numeric tower enabled. `runtime/rust`'s build script degrades SILENTLY to
# a bignum-less build when it cannot find libtommath — `expr` is then not even
# registered — so a green run without TCL_TOMMATH_DIR would be a much weaker
# gate than it looks (the same trap issue #1542 documents for the real link).
runtime_target=$(awk '
    /^runtime-rust-test:/ { in_target = 1 }
    in_target && /^[A-Za-z0-9_.-]+:/ && $1 != "runtime-rust-test:" { exit }
    in_target { print }
' "$REPO_ROOT/Makefile")

case "$runtime_target" in
    *'--locked'*) ;;
    *)
        echo "runtime-rust-test must run the standalone suite --locked" >&2
        exit 1
        ;;
esac
case "$runtime_target" in
    *'TCL_TOMMATH_DIR'*) ;;
    *)
        echo "runtime-rust-test must pass TCL_TOMMATH_DIR so the numeric tower is not silently disabled" >&2
        exit 1
        ;;
esac

echo "standalone runtime make-target contract tests passed"

# CI wiring. The redundancy contract in AGENTS.md requires the skip to be
# step-level (the job still reports, so required checks and the release
# `needs:` graph stay intact), content-identity based, and fail-safe.
workflow=$REPO_ROOT/.github/workflows/ci.yml

case "$(cat "$workflow")" in
    *'runtime_rust_changed: ${{ steps.paths.outputs.runtime_rust_changed }}'*) ;;
    *)
        echo "ci.yml's channel job must publish a runtime_rust_changed output" >&2
        exit 1
        ;;
esac
case "$(cat "$workflow")" in
    *'scripts/dev/runtime-rust-path.sh'*) ;;
    *)
        echo "ci.yml must classify changed paths with scripts/dev/runtime-rust-path.sh" >&2
        exit 1
        ;;
esac

runtime_job=$(awk '
    /^  runtime-rust-tests:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "runtime-rust-tests:" { exit }
    in_job { print }
' "$workflow")

if [ -z "$runtime_job" ]; then
    echo "ci.yml must define the runtime-rust-tests job" >&2
    exit 1
fi
case "$runtime_job" in
    *'needs: [channel]'*) ;;
    *)
        echo "runtime-rust-tests must take its path facts from the channel job" >&2
        exit 1
        ;;
esac
case "$runtime_job" in
    *"if: needs.channel.outputs.runtime_rust_changed == 'true'"*) ;;
    *)
        echo "runtime-rust-tests must skip at STEP level on runtime_rust_changed, never job level" >&2
        exit 1
        ;;
esac
case "$runtime_job" in
    *'make runtime-rust-test'*) ;;
    *)
        echo "runtime-rust-tests must run the shared make target, so CI and local agree" >&2
        exit 1
        ;;
esac

echo "standalone runtime CI wiring contract tests passed"
