#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Hermetic regression for the macOS runner cleanup in issue #1684. The fake
# brew records every call, so the test proves exact-match removal, preservation
# of unrelated taps, and idempotence without mutating a developer machine.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
HELPER=$SCRIPT_DIR/prepare-homebrew-ci.sh
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
fake_bin=$fixture/bin
state=$fixture/taps
log=$fixture/brew.log
mkdir -p "$fake_bin"

{
    printf '%s\n' '#!/bin/sh'
    printf '%s\n' 'set -eu'
    printf '%s\n' 'printf "%s\n" "$*" >> "${BREW_TEST_LOG:?}"'
    printf '%s\n' 'case "${1:-}" in'
    printf '%s\n' '    tap) cat "${BREW_TEST_STATE:?}" ;;'
    printf '%s\n' '    untap)'
    printf '%s\n' '        [ "$#" -eq 2 ] && [ "$2" = aws/tap ] || exit 97'
    printf '%s\n' '        grep -Fvx "$2" "$BREW_TEST_STATE" > "$BREW_TEST_STATE.next"'
    printf '%s\n' '        mv "$BREW_TEST_STATE.next" "$BREW_TEST_STATE" ;;'
    printf '%s\n' '    *) exit 98 ;;'
    printf '%s\n' 'esac'
} > "$fake_bin/brew"
chmod 0755 "$fake_bin/brew"

run_helper() {
    env \
        PATH="$fake_bin:$PATH" \
        BREW_TEST_LOG="$log" \
        BREW_TEST_STATE="$state" \
        "$HELPER"
}

printf '%s\n' aws/tap aws/tap-extra homebrew/core > "$state"
run_helper
run_helper

if grep -Fxq aws/tap "$state"; then
    echo "prepare-homebrew-ci left the exact unused tap installed" >&2
    exit 1
fi
for preserved in aws/tap-extra homebrew/core; do
    if ! grep -Fxq "$preserved" "$state"; then
        echo "prepare-homebrew-ci removed unrelated tap $preserved" >&2
        exit 1
    fi
done

expected_log=$fixture/expected.log
printf '%s\n' tap 'untap aws/tap' tap > "$expected_log"
if ! cmp -s "$expected_log" "$log"; then
    echo "prepare-homebrew-ci was not exact and idempotent" >&2
    diff -u "$expected_log" "$log" >&2 || true
    exit 1
fi

# A lookalike tap must not trigger an untap call.
printf '%s\n' aws/tap-extra homebrew/core > "$state"
: > "$log"
run_helper
if [ "$(cat "$log")" != tap ]; then
    echo "prepare-homebrew-ci matched a non-exact aws/tap name" >&2
    cat "$log" >&2
    exit 1
fi

# The fix must never bypass or broaden Homebrew trust.
if grep -R 'HOMEBREW_NO_REQUIRE_TAP_TRUST' \
    "$HELPER" \
    "$REPO_ROOT/.github/workflows/ci.yml" \
    "$REPO_ROOT/.github/workflows/report-pyz.yml" \
    "$REPO_ROOT/rust/bigip-report-gen/python/deploy/report-pyz.yml" \
    >/dev/null; then
    echo "Homebrew tap-trust bypass is forbidden" >&2
    exit 1
fi

ci_workflow=$(cat "$REPO_ROOT/.github/workflows/ci.yml")
case "$ci_workflow" in
    *'scripts/dev/prepare-homebrew-ci.sh'*) ;;
    *)
        echo "ci.yml does not invoke the Homebrew cleanup" >&2
        exit 1
        ;;
esac
case "$ci_workflow" in
    *'scripts/dev/prepare-homebrew-ci.sh | scripts/dev/test-prepare-homebrew-ci.sh'*) ;;
    *)
        echo "the macOS regression path classifier omits the cleanup scripts" >&2
        exit 1
        ;;
esac

macos_wasm_job=$(awk '
    /^  macos-wasm-check:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "macos-wasm-check:" { exit }
    in_job { print }
' "$REPO_ROOT/.github/workflows/ci.yml")
case "$macos_wasm_job" in
    *'run: scripts/dev/prepare-homebrew-ci.sh'*'setup-rust-toolchain'*) ;;
    *)
        echo "macos-wasm-check must clean the tap before Rust setup" >&2
        exit 1
        ;;
esac

server_matrix_job=$(awk '
    /^  build-server-matrix:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "build-server-matrix:" { exit }
    in_job { print }
' "$REPO_ROOT/.github/workflows/ci.yml")
case "$server_matrix_job" in
    *"if: runner.os == 'macOS'"*'run: scripts/dev/prepare-homebrew-ci.sh'*'setup-rust-toolchain'*) ;;
    *)
        echo "build-server-matrix must clean the tap on macOS before Rust setup" >&2
        exit 1
        ;;
esac

for report_workflow in \
    "$REPO_ROOT/.github/workflows/report-pyz.yml" \
    "$REPO_ROOT/rust/bigip-report-gen/python/deploy/report-pyz.yml"; do
    report_job=$(cat "$report_workflow")
    case "$report_job" in
        *"if: runner.os == 'macOS'"*'run: scripts/dev/prepare-homebrew-ci.sh'*'setup-rust-toolchain'*) ;;
        *)
            echo "$(basename "$report_workflow") must clean the tap on macOS before Rust setup" >&2
            exit 1
            ;;
    esac
done

echo "Homebrew macOS runner cleanup regression: ok"
