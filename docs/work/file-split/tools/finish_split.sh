#!/usr/bin/env bash
# Finish a split of one package after `apply_plans.py` and `cargo fmt`:
# narrow the wildcard imports, point each new module's imports at their real
# sources, drop what became unused, then check and lint every target.
#
#   finish_split.sh PACKAGE CRATE_DIR [LOG_DIR]
#
# Stops at the first step that fails and names its log; fix what the compiler
# reports there and run it again. Every step is safe to repeat.
set -u
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

redirect_imports() {
    if [ -n "$(created)" ]; then
        # shellcheck disable=SC2046
        python3 docs/work/file-split/tools/direct_imports.py $(created) | awk '{n += $1} END {print "    " n + 0 " imports now name their source"}'
    fi
}

# Production code first: with the library compiling, narrow each new module's
# `use super::*;`, point the names at their sources and drop what is unused.
step check-lib cargo check --manifest-path "$manifest" -p "$package" --lib
step narrow-wildcards cargo clippy --fix --allow-dirty --allow-staged --manifest-path "$manifest" -p "$package" --lib
redirect_imports
step fmt-1 fmt
step drop-unused cargo clippy --fix --allow-dirty --allow-staged --manifest-path "$manifest" -p "$package" --lib
step fmt-2 fmt
# Then every target: the same for the new test modules, plus the lints a move
# brings that clippy can fix itself (a lifetime a smaller impl no longer needs,
# a module doc that wants backticks).
step check-all cargo check --manifest-path "$manifest" -p "$package" --all-targets
step lint-fixes cargo clippy --fix --allow-dirty --allow-staged --manifest-path "$manifest" -p "$package" --all-targets
redirect_imports
step fmt-3 fmt
step drop-unused-tests cargo clippy --fix --allow-dirty --allow-staged --manifest-path "$manifest" -p "$package" --all-targets
step fmt-4 fmt
step clippy-all cargo clippy --manifest-path "$manifest" -p "$package" --all-targets -- -D warnings -A clippy::assert_is_empty
step fmt-check cargo fmt --manifest-path "$manifest" -- --check
echo "OK: $package compiles and lints clean"
