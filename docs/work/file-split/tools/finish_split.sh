#!/usr/bin/env bash
# Finish a split of one package after `apply_plans.py` and `cargo fmt`:
# check every target, settle the imports of the new modules
# (settle_imports.py), then lint.
#
#   finish_split.sh PACKAGE CRATE_DIR [LOG_DIR]
#
# Stops at the first step that fails and names its log; fix what the compiler
# reports there and run it again. Every step is safe to repeat.
set -u -o pipefail
package="$1"
crate_dir="$2"
logs="${3:-${TMPDIR:-/tmp}/finish-split-$package}"
root="$(git rev-parse --show-toplevel)"
cd "$root" || exit 1
mkdir -p "$logs"
source scripts/dev/agent-build-env.sh >/dev/null 2>&1 || true
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}" CARGO_PROFILE_TEST_INCREMENTAL=false RUST_BACKTRACE=0
manifest="$crate_dir/Cargo.toml"

step() {
    local name="$1"
    shift
    echo "==> $name"
    if ! "$@" >"$logs/$name.log" 2>&1; then
        echo "FAILED: $name ($(grep -c '^error' "$logs/$name.log") errors) — see $logs/$name.log"
        grep '^error' "$logs/$name.log" | sed 's/`[^`]*`/`_`/g' | sort | uniq -c | sort -rn | head -8
        exit 1
    fi
}

created() {
    git ls-files --others --exclude-standard -- "$crate_dir" | grep '\.rs$'
}

fmt() {
    cargo fmt --manifest-path "$manifest"
}

# With every target compiling, settle the imports: narrow each new module's
# `use super::*;`, point the names at their sources, drop what nothing uses
# and gate what only tests use.
# A package without a library (a tool such as xtask) is checked through its binaries.
unit="--lib"
[ -f "$crate_dir/src/lib.rs" ] || unit="--bins"
step check-lib cargo check --manifest-path "$manifest" -p "$package" "$unit"
step check-all cargo check --manifest-path "$manifest" -p "$package" --all-targets
echo "==> settle-imports"
if ! python3 docs/work/file-split/tools/settle_imports.py "$package" "$crate_dir" --log "$logs/import-edits.log" 2>&1 | tee "$logs/settle-imports.log"; then
    echo "FAILED: settle-imports — see $logs/settle-imports.log"
    exit 1
fi
if grep -q '^settle_imports:' "$logs/settle-imports.log"; then
    echo "FAILED: settle-imports — see $logs/settle-imports.log"
    exit 1
fi
step fmt-1 fmt
# The lints a move brings that clippy fixes itself: a lifetime a smaller impl
# no longer needs, a module doc that wants backticks.
step lint-fixes cargo clippy --fix --allow-dirty --allow-staged --manifest-path "$manifest" -p "$package" --all-targets
step fmt-2 fmt
step clippy-all cargo clippy --manifest-path "$manifest" -p "$package" --all-targets -- -D warnings -A clippy::assert_is_empty
step fmt-check cargo fmt --manifest-path "$manifest" -- --check
echo "OK: $package compiles and lints clean"
