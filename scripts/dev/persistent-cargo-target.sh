#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Select and validate the Cargo target retained by one Tank runner
# registration.  The identity is deliberately content-addressed: a runner
# registration, repository, and checkout root can never silently reuse one
# another's artefacts.  Hosted jobs call this helper too in contract tests,
# but the hosted path is an explicit no-op.

set -eu

SELF=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/$(basename -- "$0")
ROOT=${TCL_LSP_TANK_TARGET_ROOT:-/home/runner/.cache/tcl-lsp/cargo-targets}
MIN_FREE_KB=${TCL_LSP_TANK_MIN_FREE_KB:-10485760}
RETENTION_DAYS=${TCL_LSP_TANK_RETENTION_DAYS:-14}
JANITOR_LIMIT=${TCL_LSP_TANK_JANITOR_LIMIT:-8}
MARKER=.tcl-lsp-cargo-target
LOCK=.tcl-lsp-cargo-target.lock

die() {
    echo "persistent-cargo-target: $*" >&2
    exit 1
}

number() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
    esac
}

number "$MIN_FREE_KB" || die "TCL_LSP_TANK_MIN_FREE_KB must be a non-negative integer"
number "$RETENTION_DAYS" || die "TCL_LSP_TANK_RETENTION_DAYS must be a non-negative integer"
number "$JANITOR_LIMIT" || die "TCL_LSP_TANK_JANITOR_LIMIT must be a non-negative integer"

# Reject a symlink at every existing path component.  Checking only the leaf
# permits a hostile runner image to redirect an apparently safe root.
no_symlink_path() {
    local path old_ifs current part
    local -a parts
    path=$1
    case "$path" in
        /*) ;;
        *) die "path must be absolute: $path" ;;
    esac
    IFS=/ read -r -a parts <<< "${path#/}"
    current=
    for part in "${parts[@]}"; do
        [ -n "$part" ] || continue
        current=$current/$part
        [ ! -L "$current" ] || die "symlink path component: $current"
    done
}

canonical_existing() {
    local path canonical
    path=$1
    no_symlink_path "$path"
    [ -e "$path" ] || die "path does not exist: $path"
    canonical=$(readlink -f -- "$path") || die "cannot canonicalise path: $path"
    [ "$canonical" = "$path" ] || die "path is not canonical: $path"
    printf '%s\n' "$canonical"
}

owned_mode() {
    local path expected
    path=$1
    expected=$2
    [ -d "$path" ] || die "not a directory: $path"
    [ ! -L "$path" ] || die "directory is a symlink: $path"
    [ "$(stat -c '%u' -- "$path")" = "$(id -u)" ] || die "wrong owner: $path"
    [ "$(stat -c '%a' -- "$path")" = "$expected" ] || die "unsafe permissions on $path"
}

field() {
    local value
    value=$1
    [ -n "$value" ] || die "identity field is empty"
    [ "${#value}" -le 200 ] || die "identity field is too long"
    case "$value" in
        *$'\t'*|*$'\n'*|*$'\r'*) die "identity field contains control whitespace" ;;
    esac
}

repository_ok() {
    local repository=$1
    case "$repository" in
        */*/*|*/*) ;;
        *) return 1 ;;
    esac
    case "$repository" in */*/*) return 1 ;; esac
    case "$repository" in
        *[!A-Za-z0-9._/-]*) return 1 ;;
    esac
    case "$repository" in
        /*|*/|*//*|*'/.'*|*'/..'*) return 1 ;;
    esac
}

marked_target() {
    local candidate marker
    candidate=$1
    marker=$candidate/$MARKER
    awk -v target="$candidate" '
        NR == 1 && $0 == "version=1" { next }
        NR == 2 && $0 ~ /^registration=[^[:space:]]+$/ { next }
        NR == 3 && $0 ~ /^repository=[^[:space:]]+$/ { next }
        NR == 4 && $0 ~ /^checkout=\/.+$/ { next }
        NR == 5 && $0 == "target=" target { good = 1; next }
        { bad = 1 }
        END { exit !(good && !bad && NR == 5) }
    ' "$marker"
}

free_kb() {
    local value
    value=$(df -Pk -- "$ROOT" | awk 'NR == 2 { print $4 }')
    number "$value" || die "cannot read free space for $ROOT"
    printf '%s\n' "$value"
}

target_size() {
    local value
    # du reports 1K blocks, which is stable across the Ubuntu runner image.
    value=$(du -sk -- "$1" | awk 'NR == 1 { print $1 }')
    number "$value" || die "cannot measure target size: $1"
    printf '%s\n' "$((value * 1024))"
}

marker_contents() {
    local registration=$1 repository=$2 checkout=$3 target=$4
    printf 'version=1\nregistration=%s\nrepository=%s\ncheckout=%s\ntarget=%s\n' \
        "$registration" "$repository" "$checkout" "$target"
}

valid_marker() {
    local candidate expected marker actual
    candidate=$1
    expected=$2
    marker=$candidate/$MARKER
    [ -f "$marker" ] || return 1
    [ ! -L "$marker" ] || return 1
    [ "$(stat -c '%u' -- "$marker")" = "$(id -u)" ] || return 1
    [ "$(stat -c '%a' -- "$marker")" = 600 ] || return 1
    actual=$(cat -- "$marker") || return 1
    [ "$actual" = "$expected" ]
}

janitor() {
    local removed locked inspected candidate marker lock
    removed=0
    locked=0
    inspected=0
    [ -d "$ROOT" ] || {
        printf 'janitor_removed=0 janitor_locked=0 janitor_inspected=0\n'
        return
    }
    owned_mode "$ROOT" 700
    for candidate in "$ROOT"/*; do
        [ -d "$candidate" ] || continue
        inspected=$((inspected + 1))
        [ "$inspected" -le "$JANITOR_LIMIT" ] || break
        [ ! -L "$candidate" ] || continue
        [ "$(readlink -f -- "$candidate")" = "$candidate" ] || continue
        marker=$candidate/$MARKER
        [ -f "$marker" ] || continue
        [ ! -L "$marker" ] || continue
        [ "$(stat -c '%u' -- "$candidate")" = "$(id -u)" ] || continue
        [ "$(stat -c '%a' -- "$candidate")" = 700 ] || continue
        [ "$(stat -c '%u' -- "$marker")" = "$(id -u)" ] || continue
        [ "$(stat -c '%a' -- "$marker")" = 600 ] || continue
        # The marker is the proof that this directory belongs to this helper;
        # an unmarked, malformed, or redirected directory is never a janitor
        # target.
        marked_target "$candidate" || continue
        # Use the marker age, not the directory age: opening the Cargo lock
        # itself changes the directory mtime and must not make an old target
        # look young.
        if find "$marker" -maxdepth 0 -mtime +"$RETENTION_DAYS" -print -quit | grep -q .; then
            lock=$candidate/$LOCK
            # A running Cargo wrapper owns this advisory lock.  Never wait in
            # the janitor: a bounded sweep must preserve the active target.
            if ! exec 9>"$lock"; then
                locked=$((locked + 1))
                continue
            fi
            if ! flock -n 9; then
                locked=$((locked + 1))
                exec 9>&-
                continue
            fi
            rm -rf -- "$candidate"
            exec 9>&-
            removed=$((removed + 1))
        fi
    done
    printf 'janitor_removed=%s janitor_locked=%s janitor_inspected=%s\n' "$removed" "$locked" "$inspected"
}

prepare() {
    local runner repository checkout registration janitor_line free key target state expected size marker_tmp
    local target_locked=false
    runner=$1
    repository=$2
    checkout=$3
    registration=$4
    case "$runner" in
        hosted)
            echo 'persistent-cargo-target state=hosted-noop'
            return 0
            ;;
        tank) ;;
        *) die "runner must be tank or hosted" ;;
    esac
    field "$registration"
    case "$registration" in
        *[!A-Za-z0-9._-]*) die "runner registration contains unsafe characters" ;;
    esac
    field "$repository"
    repository_ok "$repository" || die "repository must be owner/name"
    field "$checkout"
    checkout=$(canonical_existing "$checkout")
    [ -d "$checkout" ] || die "checkout root is not a directory: $checkout"

    no_symlink_path "$ROOT"
    if [ ! -e "$ROOT" ]; then
        (umask 077 && mkdir -p -- "$ROOT") || die "cannot create target root: $ROOT"
    fi
    ROOT=$(canonical_existing "$ROOT")
    owned_mode "$ROOT" 700

    key=$(printf '%s\n%s\n%s\n' "$registration" "$repository" "$checkout" | sha256sum | awk '{print $1}')
    case "$key" in *[!0-9a-f]*|'') die "cannot derive target identity" ;; esac
    target=$ROOT/$key
    state=new
    expected=$(marker_contents "$registration" "$repository" "$checkout" "$target")
    if [ -e "$target" ]; then
        no_symlink_path "$target"
        owned_mode "$target" 700
        valid_marker "$target" "$expected" || die "target identity marker mismatch: $target"
        [ ! -L "$target/$LOCK" ] || die "target lock is a symlink: $target/$LOCK"
        exec 8>"$target/$LOCK"
        flock -n 8 || die "target is already locked: $target"
        touch -- "$target/$MARKER"
        target_locked=true
        state=reused
    fi
    janitor_line=$(janitor)
    free=$(free_kb)
    [ "$free" -ge "$MIN_FREE_KB" ] || die "free space ${free}KiB is below ${MIN_FREE_KB}KiB floor"
    if [ "$state" = new ]; then
        (umask 077 && mkdir -- "$target") || die "cannot create target: $target"
        owned_mode "$target" 700
        marker_tmp=$target/$MARKER.tmp.$$
        (umask 077 && marker_contents "$registration" "$repository" "$checkout" "$target" > "$marker_tmp" && mv -- "$marker_tmp" "$target/$MARKER") || {
            rm -f -- "$marker_tmp"
            die "cannot write target identity marker"
        }
        [ "$(stat -c '%a' -- "$target/$MARKER")" = 600 ] || die "marker permissions are unsafe"
    fi
    size=$(target_size "$target")
    printf 'persistent-cargo-target state=%s target=%s target_bytes=%s free_kb=%s %s\n' \
        "$state" "$target" "$size" "$free" "$janitor_line" >&2
    if [ "$target_locked" = true ]; then
        exec 8>&-
    fi
    printf '%s\n' "$target"
}

with_lock() {
    local target lock
    target=$1
    shift
    [ "$#" -gt 0 ] || die "with-lock requires a command"
        target=$(canonical_existing "$target")
    owned_mode "$target" 700
    lock=$target/$LOCK
    [ ! -L "$lock" ] || die "target lock is a symlink: $lock"
    # Keep the descriptor open for the complete child process.  A janitor can
    # therefore skip this target without guessing whether Cargo is active.
    exec 9>"$lock"
    flock -n 9 || die "target is already locked: $target"
    "$@"
}

[ "$#" -ge 1 ] || die "usage: $SELF prepare|janitor|with-lock ..."
case "$1" in
    prepare)
        [ "$#" -eq 5 ] || die "usage: $SELF prepare RUNNER REPOSITORY CHECKOUT REGISTRATION"
        prepare "$2" "$3" "$4" "$5"
        ;;
    janitor)
        [ "$#" -eq 2 ] || die "usage: $SELF janitor ROOT"
        ROOT=$2
        no_symlink_path "$ROOT"
        ROOT=$(canonical_existing "$ROOT")
        janitor
        ;;
    with-lock)
        [ "$#" -ge 3 ] || die "usage: $SELF with-lock TARGET COMMAND [ARG ...]"
        with_lock "$2" "${@:3}"
        ;;
    *) die "unknown operation: $1" ;;
esac
