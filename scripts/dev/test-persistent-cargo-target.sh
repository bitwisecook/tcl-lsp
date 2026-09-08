#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Deterministic contract tests for the Tank Cargo-target helper. These tests
# use temporary roots and never inspect or mutate a runner's real Cargo home.
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
HELPER=$SCRIPT_DIR/persistent-cargo-target.sh
ROOT=$(mktemp -d /tmp/tcl-lsp-persistent-target.XXXXXX)
trap 'rm -rf -- "$ROOT"' EXIT HUP INT TERM
mkdir -m 700 "$ROOT/work-a" "$ROOT/work-b"
TARGET_ROOT=$ROOT/targets

fail() { echo "persistent Cargo target contract: $*" >&2; exit 1; }
prepare() {
    local registration=$1 checkout=${2:-$ROOT/work-a} log=$ROOT/prepare.log
    TCL_LSP_TANK_TARGET_ROOT=$TARGET_ROOT \
        TCL_LSP_TANK_MIN_FREE_KB=1 \
        TCL_LSP_TANK_RETENTION_DAYS=14 \
        TCL_LSP_TANK_JANITOR_LIMIT=8 \
        bash "$HELPER" prepare tank owner/repo "$checkout" "$registration" 2>"$log"
}
expect_failure() {
    if "$@" >"$ROOT/unexpected.out" 2>"$ROOT/unexpected.err"; then
        fail "expected failure: $*"
    fi
}

hosted=$(bash "$HELPER" prepare hosted owner/repo "$ROOT/missing" reg-a 2>"$ROOT/hosted.err")
[ "$hosted" = "" ] || [ "$hosted" = "persistent-cargo-target state=hosted-noop" ] || fail "hosted output changed"
[ ! -e "$TARGET_ROOT" ] || fail "hosted mode created a target root"

target_a=$(prepare reg-a)
[ -d "$target_a" ] || fail "new target was not created"
grep -q 'state=new' "$ROOT/prepare.log" || fail "new state was not reported"
grep -q 'target_bytes=' "$ROOT/prepare.log" || fail "target size was not reported"
grep -q 'free_kb=' "$ROOT/prepare.log" || fail "free space was not reported"
grep -q 'janitor_removed=' "$ROOT/prepare.log" || fail "janitor work was not reported"
[ "$(stat -c '%a' "$TARGET_ROOT")" = 700 ] || fail "target root permissions"
[ "$(stat -c '%a' "$target_a")" = 700 ] || fail "target permissions"
[ "$(stat -c '%a' "$target_a/.tcl-lsp-cargo-target")" = 600 ] || fail "marker permissions"

target_a_again=$(prepare reg-a)
[ "$target_a" = "$target_a_again" ] || fail "stable identity did not reuse the target"
grep -q 'state=reused' "$ROOT/prepare.log" || fail "reuse state was not reported"
target_registration=$(prepare reg-b)
target_checkout=$(prepare reg-a "$ROOT/work-b")
[ "$target_a" != "$target_registration" ] || fail "registrations shared a target"
[ "$target_a" != "$target_checkout" ] || fail "checkout roots shared a target"

printf 'version=1\nregistration=wrong\n' > "$target_a/.tcl-lsp-cargo-target"
chmod 600 "$target_a/.tcl-lsp-cargo-target"
expect_failure prepare reg-a
rm -f "$target_a/.tcl-lsp-cargo-target"
ln -s "$ROOT/work-a" "$ROOT/work-link"
expect_failure prepare reg-a "$ROOT/work-link"
ln -s "$TARGET_ROOT" "$ROOT/root-link"
expect_failure env TCL_LSP_TANK_TARGET_ROOT="$ROOT/root-link" TCL_LSP_TANK_MIN_FREE_KB=1 bash "$HELPER" prepare tank owner/repo "$ROOT/work-a" reg-a

# An old marked target is eligible, while a lock held by a running Cargo
# process protects it. The janitor limit bounds one sweep to one candidate.
old=$(prepare old-reg)
touch -d '30 days ago' "$old/.tcl-lsp-cargo-target"
bash "$HELPER" janitor "$TARGET_ROOT" >"$ROOT/janitor.out"
grep -q 'janitor_removed=1' "$ROOT/janitor.out" || fail "old target was not removed"
[ ! -e "$old" ] || fail "old target survived janitor"
locked=$(prepare locked-reg)
touch -d '30 days ago' "$locked/.tcl-lsp-cargo-target"
exec 8>"$locked/.tcl-lsp-cargo-target.lock"
flock -n 8
bash "$HELPER" janitor "$TARGET_ROOT" >"$ROOT/locked.out"
grep -q 'janitor_locked=1' "$ROOT/locked.out" || fail "locked target was not reported"
[ -e "$locked" ] || fail "locked target was removed"
exec 8>&-

expect_failure env TCL_LSP_TANK_MIN_FREE_KB=999999999999 bash "$HELPER" prepare tank owner/repo "$ROOT/work-a" floor-reg

echo 'persistent Cargo target contract passed'
