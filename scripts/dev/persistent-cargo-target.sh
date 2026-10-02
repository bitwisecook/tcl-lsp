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
MIN_FREE_KB=${TCL_LSP_TANK_MIN_FREE_KB:-20971520}
RETENTION_DAYS=${TCL_LSP_TANK_RETENTION_DAYS:-14}
JANITOR_LIMIT=${TCL_LSP_TANK_JANITOR_LIMIT:-8}
# Reclaim regardless of marker age. Only an operator sweep sets this: the
# retention window is what keeps a *warm* target warm, and a target removed
# here is rebuilt from scratch on its next job. Every other guard still
# applies — the target must be marked, owned, safe-moded, and its Cargo lock
# free, so a target a running job holds is still skipped.
SWEEP_ALL=${TCL_LSP_TANK_SWEEP_ALL:-0}
MARKER=.tcl-lsp-cargo-target
LOCK=.tcl-lsp-cargo-target.lock
ROOT_LOCK=.tcl-lsp-cargo-target.root.lock
TEMP_DIR=

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
case "$SWEEP_ALL" in
    0|1) ;;
    *) die "TCL_LSP_TANK_SWEEP_ALL must be 0 or 1" ;;
esac

cleanup() {
    local status=$?
    trap - EXIT
    if [ -n "${TEMP_DIR:-}" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf -- "$TEMP_DIR"
    fi
    exit "$status"
}

trap cleanup EXIT

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

valid_lock() {
    local lock=$1
    [ -f "$lock" ] || return 1
    [ ! -L "$lock" ] || return 1
    [ "$(stat -c '%u' -- "$lock")" = "$(id -u)" ] || return 1
    [ "$(stat -c '%a' -- "$lock")" = 600 ] || return 1
    [ "$(stat -c '%h' -- "$lock")" = 1 ]
}

opened_lock_ok() {
    local fd=$1 lock=$2 opened
    opened=$(readlink -f -- "/proc/${BASHPID}/fd/$fd") || return 1
    [ "$opened" = "$lock" ] || return 1
    valid_lock "$opened"
}

janitor() {
    local removed locked unsafe inspected candidate marker lock
    removed=0
    locked=0
    unsafe=0
    inspected=0
    [ -d "$ROOT" ] || {
        printf 'janitor_removed=0 janitor_locked=0 janitor_unsafe=0 janitor_inspected=0\n'
        return
    }
    owned_mode "$ROOT" 700
    for candidate in "$ROOT"/*; do
        [ -d "$candidate" ] || continue
        inspected=$((inspected + 1))
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
        if [ "$SWEEP_ALL" = 1 ] ||
            find "$marker" -maxdepth 0 -mtime +"$RETENTION_DAYS" -print -quit | grep -q .; then
            # Scan every direct child, but cap destructive work per invocation.
            # Fresh/unmarked entries must not starve old targets.
            [ "$removed" -lt "$JANITOR_LIMIT" ] || continue
            lock=$candidate/$LOCK
            # A running Cargo wrapper owns this advisory lock.  Never wait in
            # the janitor: a bounded sweep must preserve the active target.
            if ! valid_lock "$lock"; then
                unsafe=$((unsafe + 1))
                continue
            fi
            if ! exec 9<>"$lock"; then
                unsafe=$((unsafe + 1))
                continue
            fi
            if ! opened_lock_ok 9 "$lock"; then
                unsafe=$((unsafe + 1))
                exec 9>&-
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
    printf 'janitor_removed=%s janitor_locked=%s janitor_unsafe=%s janitor_inspected=%s\n' "$removed" "$locked" "$unsafe" "$inspected"
}

root_lock() {
    local root_lock_path=$ROOT/$ROOT_LOCK
    if [ ! -e "$root_lock_path" ]; then
        # noclobber makes creation recoverable when two helpers initialise a
        # fresh root concurrently; the loser validates the winner's file.
        (umask 077 && set -C && : > "$root_lock_path") || :
    fi
    valid_lock "$root_lock_path" || die "root lock is missing or unsafe: $root_lock_path"
    exec 7<>"$root_lock_path"
    opened_lock_ok 7 "$root_lock_path" || {
        exec 7>&-
        die "root lock changed while opening: $root_lock_path"
    }
    flock 7 || {
        exec 7>&-
        die "root lock is already held: $root_lock_path"
    }
}

prepare() {
    local runner repository checkout registration janitor_line free key target state expected size marker_tmp lock_tmp
    local target_locked=false
    runner=$1
    repository=$2
    checkout=$3
    registration=${4:-${TCL_LSP_TANK_REGISTRATION_ID:-${RUNNER_NAME:-}}}
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
    root_lock

    key=$(printf '%s\n%s\n%s\n' "$registration" "$repository" "$checkout" | sha256sum | awk '{print $1}')
    case "$key" in *[!0-9a-f]*|'') die "cannot derive target identity" ;; esac
    target=$ROOT/$key
    state=new
    expected=$(marker_contents "$registration" "$repository" "$checkout" "$target")
    if [ -e "$target" ]; then
        no_symlink_path "$target"
        owned_mode "$target" 700
        valid_marker "$target" "$expected" || die "target identity marker mismatch: $target"
        valid_lock "$target/$LOCK" || die "target lock is missing or unsafe: $target/$LOCK"
        exec 8<>"$target/$LOCK"
        opened_lock_ok 8 "$target/$LOCK" || {
            exec 8>&-
            die "target lock changed while opening: $target/$LOCK"
        }
        flock -n 8 || die "target is already locked: $target"
        touch -- "$target/$MARKER"
        target_locked=true
        state=reused
    fi
    janitor_line=$(janitor)
    free=$(free_kb)
    [ "$free" -ge "$MIN_FREE_KB" ] || die "free space ${free}KiB is below ${MIN_FREE_KB}KiB floor"
    if [ "$state" = new ]; then
        TEMP_DIR=$(mktemp -d -- "$ROOT/.${key}.tmp.XXXXXX") || die "cannot create private target staging directory"
        no_symlink_path "$TEMP_DIR"
        owned_mode "$TEMP_DIR" 700
        lock_tmp=$TEMP_DIR/$LOCK
        (umask 077 && : > "$lock_tmp") || die "cannot create staged target lock"
        valid_lock "$lock_tmp" || die "staged target lock is unsafe"
        marker_tmp=$TEMP_DIR/$MARKER
        (umask 077 && marker_contents "$registration" "$repository" "$checkout" "$target" > "$marker_tmp") || die "cannot write staged target marker"
        [ "$(stat -c '%a' -- "$marker_tmp")" = 600 ] || die "staged marker permissions are unsafe"
        mv -T -- "$TEMP_DIR" "$target" || die "target appeared during atomic creation: $target"
        TEMP_DIR=
        no_symlink_path "$target"
        owned_mode "$target" 700
        valid_marker "$target" "$expected" || die "atomically created target marker mismatch: $target"
        valid_lock "$target/$LOCK" || die "atomically created target lock is unsafe: $target/$LOCK"
    fi
    size=$(target_size "$target")
    printf 'persistent-cargo-target state=%s target=%s target_bytes=%s free_kb=%s %s\n' \
        "$state" "$target" "$size" "$free" "$janitor_line" >&2
    if [ "$target_locked" = true ]; then
        exec 8>&-
    fi
    exec 7>&-
    printf '%s\n' "$target"
}

report() {
    local target size free
    target=$(canonical_existing "$1")
    no_symlink_path "$ROOT"
    ROOT=$(canonical_existing "$ROOT")
    case "$target" in
        "$ROOT"/*) ;;
        *) die "target is outside the dedicated root: $target" ;;
    esac
    owned_mode "$target" 700
    marked_target "$target" || die "target marker is invalid: $target"
    valid_lock "$target/$LOCK" || die "target lock is missing or unsafe: $target/$LOCK"
    size=$(target_size "$target")
    free=$(free_kb)
    printf 'persistent-cargo-target report target=%s target_bytes=%s free_kb=%s\n' "$target" "$size" "$free"
}

with_lock() {
    local target lock
    target=$1
    shift
    [ "$#" -gt 0 ] || die "with-lock requires a command"
    target=$(canonical_existing "$target")
    owned_mode "$target" 700
    lock=$target/$LOCK
    valid_lock "$lock" || die "target lock is missing or unsafe: $lock"
    # Keep the descriptor open in this wrapper for the complete command.  The
    # command itself must not inherit it: compiler-cache daemons can outlive
    # Cargo and would otherwise retain the lock into the following CI step.
    exec 9<>"$lock"
    opened_lock_ok 9 "$lock" || {
        exec 9>&-
        die "target lock changed while opening: $lock"
    }
    flock -n 9 || die "target is already locked: $target"
    "$@" 9>&-
}

[ "$#" -ge 1 ] || die "usage: $SELF prepare|janitor|with-lock ..."
case "$1" in
    prepare)
        { [ "$#" -eq 4 ] || [ "$#" -eq 5 ]; } || die "usage: $SELF prepare RUNNER REPOSITORY CHECKOUT [REGISTRATION]"
        prepare "$2" "$3" "$4" "${5:-}"
        ;;
    janitor)
        [ "$#" -eq 2 ] || die "usage: $SELF janitor ROOT"
        ROOT=$2
        no_symlink_path "$ROOT"
        ROOT=$(canonical_existing "$ROOT")
        root_lock
        janitor
        exec 7>&-
        ;;
    with-lock)
        [ "$#" -ge 3 ] || die "usage: $SELF with-lock TARGET COMMAND [ARG ...]"
        with_lock "$2" "${@:3}"
        ;;
    report)
        [ "$#" -eq 2 ] || die "usage: $SELF report TARGET"
        report "$2"
        ;;
    *) die "unknown operation: $1" ;;
esac
