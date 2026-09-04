#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Cargo fallback for the local smoke tier. `cargo test --workspace smoke`
# selects the right test names, but first builds and launches every workspace
# test target. This manifest-driven runner invokes only the target shapes that
# contain smoke tests, while retaining Cargo's complete workspace feature
# graph. The full deep suite still runs in CI.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
MANIFEST=$SCRIPT_DIR/smoke-targets.tsv
LIST_ONLY=false

case "${1:-}" in
    '') ;;
    --list) LIST_ONLY=true ;;
    *)
        echo "usage: $0 [--list]" >&2
        exit 2
        ;;
esac

cd "$REPO_ROOT"

# Do not consume a stale manifest when this runner is invoked directly from
# `make smoke` / `make prep-pr`. The rust-check gate also runs this contract,
# but the fallback must be safe on its own.
sh "$SCRIPT_DIR/test-smoke-targets.sh"
if [ "$LIST_ONLY" = true ]; then
    exec python3 "$SCRIPT_DIR/check_smoke_targets.py" --list-manifest "$MANIFEST"
fi
exec python3 "$SCRIPT_DIR/check_smoke_targets.py" --run-manifest "$MANIFEST"
