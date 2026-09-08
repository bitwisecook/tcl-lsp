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
prepare_repository() {
    local repository=$1 registration=$2 log=$ROOT/prepare.log
    TCL_LSP_TANK_TARGET_ROOT=$TARGET_ROOT \
        TCL_LSP_TANK_MIN_FREE_KB=1 \
        bash "$HELPER" prepare tank "$repository" "$ROOT/work-a" "$registration" 2>"$log"
}
expect_failure() {
    if "$@" >"$ROOT/unexpected.out" 2>"$ROOT/unexpected.err"; then
        fail "expected failure: $*"
    fi
}

hosted=$(bash "$HELPER" prepare hosted owner/repo "$ROOT/missing" reg-a 2>"$ROOT/hosted.err")
[ "$hosted" = "" ] || [ "$hosted" = "persistent-cargo-target state=hosted-noop" ] || fail "hosted output changed"
[ ! -e "$TARGET_ROOT" ] || fail "hosted mode created a target root"
expect_failure env -u TCL_LSP_TANK_REGISTRATION_ID -u RUNNER_NAME bash "$HELPER" prepare tank owner/repo "$ROOT/work-a"
fallback=$(TCL_LSP_TANK_TARGET_ROOT="$TARGET_ROOT" TCL_LSP_TANK_MIN_FREE_KB=1 RUNNER_NAME=stable-runner bash "$HELPER" prepare tank owner/repo "$ROOT/work-a")
fallback_again=$(TCL_LSP_TANK_TARGET_ROOT="$TARGET_ROOT" TCL_LSP_TANK_MIN_FREE_KB=1 RUNNER_NAME=stable-runner bash "$HELPER" prepare tank owner/repo "$ROOT/work-a")
[ "$fallback" = "$fallback_again" ] || fail "stable runner-name fallback did not reuse the target"

target_a=$(prepare reg-a)
[ -d "$target_a" ] || fail "new target was not created"
grep -q 'state=new' "$ROOT/prepare.log" || fail "new state was not reported"
grep -q 'target_bytes=' "$ROOT/prepare.log" || fail "target size was not reported"
grep -q 'free_kb=' "$ROOT/prepare.log" || fail "free space was not reported"
grep -q 'janitor_removed=' "$ROOT/prepare.log" || fail "janitor work was not reported"
[ "$(stat -c '%a' "$TARGET_ROOT")" = 700 ] || fail "target root permissions"
[ "$(stat -c '%a' "$target_a")" = 700 ] || fail "target permissions"
[ "$(stat -c '%a' "$target_a/.tcl-lsp-cargo-target")" = 600 ] || fail "marker permissions"
[ -f "$target_a/.tcl-lsp-cargo-target.lock" ] || fail "target lock was not created"
[ ! -L "$target_a/.tcl-lsp-cargo-target.lock" ] || fail "target lock is a symlink"
[ "$(stat -c '%u' "$target_a/.tcl-lsp-cargo-target.lock")" = "$(id -u)" ] || fail "target lock owner"
[ "$(stat -c '%a' "$target_a/.tcl-lsp-cargo-target.lock")" = 600 ] || fail "target lock permissions"

report=$(TCL_LSP_TANK_TARGET_ROOT="$TARGET_ROOT" bash "$HELPER" report "$target_a")
grep -q 'target_bytes=' <<< "$report" || fail "final target size was not reported"
grep -q 'free_kb=' <<< "$report" || fail "final free space was not reported"
bash "$HELPER" with-lock "$target_a" true
chmod 644 "$target_a/.tcl-lsp-cargo-target.lock"
expect_failure bash "$HELPER" with-lock "$target_a" true
chmod 600 "$target_a/.tcl-lsp-cargo-target.lock"
rm -f "$target_a/.tcl-lsp-cargo-target.lock"
ln -s "$ROOT/work-a" "$target_a/.tcl-lsp-cargo-target.lock"
expect_failure bash "$HELPER" with-lock "$target_a" true
rm -f "$target_a/.tcl-lsp-cargo-target.lock"
mkdir -m 700 "$target_a/.tcl-lsp-cargo-target.lock"
expect_failure bash "$HELPER" with-lock "$target_a" true
rmdir "$target_a/.tcl-lsp-cargo-target.lock"
(umask 077 && : > "$target_a/.tcl-lsp-cargo-target.lock")
ln "$target_a/.tcl-lsp-cargo-target.lock" "$ROOT/hard-linked-lock"
expect_failure bash "$HELPER" with-lock "$target_a" true
rm -f "$ROOT/hard-linked-lock"

target_a_again=$(prepare reg-a)
[ "$target_a" = "$target_a_again" ] || fail "stable identity did not reuse the target"
grep -q 'state=reused' "$ROOT/prepare.log" || fail "reuse state was not reported"
target_registration=$(prepare reg-b)
target_checkout=$(prepare reg-a "$ROOT/work-b")
[ "$target_a" != "$target_registration" ] || fail "registrations shared a target"
[ "$target_a" != "$target_checkout" ] || fail "checkout roots shared a target"

target_repository=$(prepare_repository owner/other-repo repo-a)
[ "$target_a" != "$target_repository" ] || fail "repositories shared a target"
expect_failure env TCL_LSP_TANK_TARGET_ROOT="$TARGET_ROOT" TCL_LSP_TANK_MIN_FREE_KB=1 bash "$HELPER" prepare tank owner/../repo "$ROOT/work-a" reg-a
expect_failure env TCL_LSP_TANK_TARGET_ROOT="$TARGET_ROOT" TCL_LSP_TANK_MIN_FREE_KB=1 bash "$HELPER" prepare tank ../owner/repo "$ROOT/work-a" reg-a
expect_failure env TCL_LSP_TANK_TARGET_ROOT="$TARGET_ROOT" TCL_LSP_TANK_MIN_FREE_KB=1 bash "$HELPER" prepare tank 'owner/repo name' "$ROOT/work-a" reg-a

lock="$target_a/.tcl-lsp-cargo-target.lock"
rm -f "$lock"
ln -s "$ROOT/work-a" "$lock"
expect_failure prepare reg-a
rm -f "$lock"
(umask 077 && : > "$lock")

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
fresh=$(prepare fresh-reg)
mkdir -m 700 "$TARGET_ROOT/unmarked"
mkdir -m 700 "$TARGET_ROOT/malformed"
printf 'not-a-marker\n' > "$TARGET_ROOT/malformed/.tcl-lsp-cargo-target"
chmod 600 "$TARGET_ROOT/malformed/.tcl-lsp-cargo-target"
symlink_target="$TARGET_ROOT/symlinked"
ln -s "$ROOT/work-a" "$symlink_target"
touch -d '30 days ago' "$TARGET_ROOT/unmarked" "$TARGET_ROOT/malformed/.tcl-lsp-cargo-target"
bash "$HELPER" janitor "$TARGET_ROOT" >"$ROOT/preserved.out"
[ -e "$fresh" ] || fail "fresh target was removed"
[ -e "$TARGET_ROOT/unmarked" ] || fail "unmarked target was removed"
[ -e "$TARGET_ROOT/malformed" ] || fail "malformed target was removed"
[ -L "$symlink_target" ] || fail "symlinked target was removed"
locked=$(prepare locked-reg)
touch -d '30 days ago' "$locked/.tcl-lsp-cargo-target"
exec 8>"$locked/.tcl-lsp-cargo-target.lock"
flock -n 8
bash "$HELPER" janitor "$TARGET_ROOT" >"$ROOT/locked.out"
grep -q 'janitor_locked=1' "$ROOT/locked.out" || fail "locked target was not reported"
[ -e "$locked" ] || fail "locked target was removed"
exec 8>&-

unsafe=$(prepare unsafe-reg)
touch -d '30 days ago' "$unsafe/.tcl-lsp-cargo-target"
rm -f "$unsafe/.tcl-lsp-cargo-target.lock"
ln -s "$ROOT/work-a" "$unsafe/.tcl-lsp-cargo-target.lock"
bash "$HELPER" janitor "$TARGET_ROOT" >"$ROOT/unsafe.out"
grep -q 'janitor_unsafe=1' "$ROOT/unsafe.out" || fail "unsafe lock was not reported"
[ -e "$unsafe" ] || fail "unsafe-lock target was removed"

expect_failure env TCL_LSP_TANK_MIN_FREE_KB=999999999999 bash "$HELPER" prepare tank owner/repo "$ROOT/work-a" floor-reg

echo 'persistent Cargo target contract passed'
