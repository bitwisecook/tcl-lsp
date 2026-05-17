#!/bin/bash
# fetch_tcl_regex.sh — Fetch the Tcl regex engine sources into vendor/.
#
# The WASM runtime links Tcl's own Henry-Spencer regex engine so
# ``regexp``/``regsub`` semantics match tclsh exactly.  The sources
# are NOT vendored in the repo (they'd add ~150 KB of upstream code
# to every checkout) and the build no longer fetches them
# automatically — that was racing under pytest-xdist when several
# ``zig build`` workers shared the same vendor dir.
#
# Instead the SessionStart hook (cloud / Claude Code on the web) runs
# this script once at session boot, and local developers run it once
# after cloning.  ``runtime/zig/build.zig`` then assumes the files are
# already present.  Idempotent: re-runs are no-ops when the stamp file
# matches the pinned version.
#
# Usage: bash scripts/fetch_tcl_regex.sh
#
# Env:
#   TCL_REGEX_VERSION  — override the pinned Tcl version (default: 9.0.3)
#
# Network: works on any runner with curl.  Individual raw files
# total ~150 KB.  Retries with exponential backoff on failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION="${TCL_REGEX_VERSION:-9.0.3}"
# Map the dotted version to the GitHub tag (Tcl uses ``core-X-Y-Z``).
TAG="core-$(echo "$VERSION" | tr '.' '-')"
BASE_URL="https://raw.githubusercontent.com/tcltk/tcl/${TAG}/generic"

TARGET_DIR="$REPO_ROOT/runtime/zig/vendor/tcl-regex"
STAMP_FILE="$TARGET_DIR/.stamp"

# Pinned per-file SHA-256 checksums. The tag pin above is not sufficient
# on its own: a tag-retargeting attack on tcltk/tcl (the same shape as
# the tj-actions/changed-files compromise in March 2025) would silently
# substitute different bytes under the same ``core-9-0-3`` tag. The
# content pin closes that gap. Bump the version line in the checksums
# file when bumping TCL_REGEX_VERSION.
CHECKSUMS_FILE="$SCRIPT_DIR/tcl_regex_sha256sums.txt"

expected_version=$(awk '/^VERSION:/ {print $2; exit}' "$CHECKSUMS_FILE")
if [[ "$expected_version" != "$VERSION" ]]; then
    echo "ERROR: TCL_REGEX_VERSION=$VERSION but $CHECKSUMS_FILE pins $expected_version" >&2
    echo "Bump both in lockstep (regenerate sha256sums for the new version)." >&2
    exit 1
fi

verify_checksums() {
    # Strip comments, blank lines and the VERSION: line, then feed the
    # remainder to ``sha256sum -c``. Run from TARGET_DIR so the bare
    # filenames in the checksums file resolve correctly.
    (
        cd "$TARGET_DIR" && \
        grep -Ev '^[[:space:]]*(#|$)|^VERSION:' "$CHECKSUMS_FILE" \
            | sha256sum --quiet -c -
    )
}

# The regex engine is self-contained within 15 files in ``generic/``.
# ``regcomp.c`` and ``regexec.c`` are the two real compilation units
# — the ``regc_*.c`` and ``rege_*.c`` files are ``#include``d by
# them (the amalgamation pattern).  We deliberately omit upstream
# ``regcustom.h`` — our replacement lives at
# ``runtime/zig/regex_include/regcustom.h`` and is reached via the
# ``-I`` flag in ``build.zig`` (C's ``#include "foo.h"`` searches
# the including file's directory first, so the vendored dir must
# not contain ``regcustom.h`` or it would shadow our shim).
FILES=(
    regcomp.c
    regexec.c
    regfree.c
    regerror.c
    regfronts.c
    regc_color.c
    regc_cvec.c
    regc_lex.c
    regc_locale.c
    regc_nfa.c
    rege_dfa.c
    regerrs.h
    regex.h
    regguts.h
)

stamp_is_complete() {
    [[ -f "$STAMP_FILE" ]] && [[ "$(cat "$STAMP_FILE" 2>/dev/null)" == "$VERSION" ]] || return 1
    for f in "${FILES[@]}"; do
        [[ -f "$TARGET_DIR/$f" ]] || return 1
    done
    # Validate that the files on disk still match the pinned checksums
    # — detects tampering with a previously-fetched vendor dir.
    verify_checksums >/dev/null 2>&1 || return 1
    return 0
}

if stamp_is_complete; then
    exit 0
fi

mkdir -p "$TARGET_DIR"

# Concurrent zig builds (e.g. four pytest-xdist workers each calling
# ``zig build``) used to race here: each instance would curl into
# the same ``$out.tmp`` and ``mv`` it, with the late ``mv`` failing
# because the early one had already moved the file away.  Serialise
# behind a lock; whichever process gets the lock fetches, the rest
# wait and re-check the stamp afterwards.
LOCK_FILE="$TARGET_DIR/.fetch.lock"
exec 9>"$LOCK_FILE"
flock 9

# Re-check after acquiring the lock — another process may have
# completed the fetch while we were waiting.
if stamp_is_complete; then
    exit 0
fi

echo "Fetching Tcl $VERSION regex sources from $TAG ..."
for f in "${FILES[@]}"; do
    url="$BASE_URL/$f"
    out="$TARGET_DIR/$f"
    # Per-file PID-suffixed temp file is belt-and-braces: even if
    # ``flock`` somehow lets two writers in (different filesystems,
    # NFS, ``flock`` missing), they won't collide on the temp name.
    tmp="$out.tmp.$$"
    for attempt in 1 2 3 4; do
        if curl -fsSL --retry 0 -o "$tmp" "$url"; then
            mv "$tmp" "$out"
            break
        fi
        rm -f "$tmp"
        if [[ $attempt -lt 4 ]]; then
            wait=$((2 ** attempt))
            echo "  $f: retry $attempt after ${wait}s" >&2
            sleep "$wait"
        else
            echo "ERROR: failed to fetch $url after 4 attempts" >&2
            exit 1
        fi
    done
done

# Fail closed if the downloaded bytes don't match the pinned checksums.
# Do this BEFORE writing the stamp so a tampered fetch never produces a
# "this dir is good" marker.
if ! verify_checksums; then
    echo "ERROR: SHA-256 mismatch after fetch — refusing to write stamp." >&2
    echo "This means the upstream tag '$TAG' on tcltk/tcl was retargeted," >&2
    echo "or the download was tampered with in transit." >&2
    echo "Investigate before bumping checksums; do not just regenerate them." >&2
    exit 1
fi

echo "$VERSION" > "$STAMP_FILE"
echo "Fetched ${#FILES[@]} files to $TARGET_DIR"
