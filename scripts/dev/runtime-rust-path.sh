#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Decide whether one repository-relative path can change standalone
# `runtime/rust` semantics. Exit 0 for relevant, 1 for unrelated, and 2 when
# the request itself is unusable so CI can fail closed (issue #1768).
#
# `runtime/rust` is its OWN cargo workspace: the root
# `cargo test --workspace --all-features` never runs its unit suite, and
# `wasm-real-link` builds and links it without executing a single one of those
# tests. So a standalone-runtime semantic regression can land with every root
# partition green — which is exactly what #1768 was filed for.
#
# The case list below is the runtime's LOCAL PACKAGE CLOSURE (the crate itself
# plus every path dependency `cargo metadata` resolves for it), its external
# Tcl 9 smoke corpus, and the lane's build inputs — not a guess.
# `scripts/dev/test-runtime-rust-paths.sh` re-derives that closure from cargo
# and fails if this list has drifted either way, so a new path dependency
# cannot silently stop triggering the job, and an over-broad entry cannot
# silently make every PR pay for it.

set -eu

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
    echo "usage: scripts/dev/runtime-rust-path.sh REPOSITORY_PATH" >&2
    exit 2
fi

PATH_TO_CLASSIFY=$1

case "$PATH_TO_CLASSIFY" in
    # The crate under test.
    runtime/rust/*)
        exit 0
        ;;
    # External golden inputs executed by runtime/rust/tests/tcl9_smoke.rs.
    samples/tcl9_smoke/*)
        exit 0
        ;;
    # Its resolved path-dependency closure. Keep in sync with
    # `cargo metadata --manifest-path runtime/rust/Cargo.toml`; the companion
    # test script is the gate that keeps it honest.
    rust/tcl-cmd-core/* | rust/tcl-core-types/* | rust/tcl-dialect/* | \
    rust/tcl-host-native/* | rust/tcl-lexer/* | rust/tcl-platform/* | \
    rust/tcl-regex/* | rust/tcl-registry/* | rust/tcl-runtime-api/* | \
    rust/tcl-syntax/*)
        exit 0
        ;;
    # Build inputs and the gate's own definition: a toolchain bump, a lockfile
    # edit, a change to the make target CI invokes, or a change to this
    # classifier or its test all change what the job would do.
    .cargo/config.toml | rust-toolchain.toml | Makefile | \
    .github/workflows/ci.yml | \
    scripts/dev/runtime-rust-path.sh | scripts/dev/test-runtime-rust-paths.sh)
        exit 0
        ;;
esac

exit 1
