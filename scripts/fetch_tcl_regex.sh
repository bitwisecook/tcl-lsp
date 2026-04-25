#!/bin/bash
# fetch_tcl_regex.sh — Download the regex engine sources from Tcl 9.0.3.
#
# The WASM runtime links Tcl's own Henry-Spencer regex engine so
# ``regexp``/``regsub`` semantics match tclsh exactly.  The sources
# are not vendored in the repo — this script fetches them into
# ``runtime/zig/vendor/tcl-regex/`` on demand.  ``runtime/zig/build.zig``
# invokes this as a pre-compile dependency, and it is idempotent:
# re-runs are no-ops when the stamp file matches the pinned version.
#
# Usage: bash scripts/fetch_tcl_regex.sh
#
# Env:
#   TCL_REGEX_VERSION  — override the pinned Tcl version (default: 9.0.3)
#
# CI/CD: works on any runner with curl.  No git clone is required
# (individual raw files total ~150 KB).  Retry with exponential
# backoff on network failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION="${TCL_REGEX_VERSION:-9.0.3}"
# Map the dotted version to the GitHub tag (Tcl uses ``core-X-Y-Z``).
TAG="core-$(echo "$VERSION" | tr '.' '-')"
BASE_URL="https://raw.githubusercontent.com/tcltk/tcl/${TAG}/generic"

TARGET_DIR="$REPO_ROOT/runtime/zig/vendor/tcl-regex"
STAMP_FILE="$TARGET_DIR/.stamp"

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

if [[ -f "$STAMP_FILE" ]] && [[ "$(cat "$STAMP_FILE")" == "$VERSION" ]]; then
    # Verify all files are present; a stamp with a missing file
    # means the directory was partially wiped — re-fetch.
    all_present=1
    for f in "${FILES[@]}"; do
        if [[ ! -f "$TARGET_DIR/$f" ]]; then
            all_present=0
            break
        fi
    done
    if [[ $all_present -eq 1 ]]; then
        exit 0
    fi
fi

mkdir -p "$TARGET_DIR"

echo "Fetching Tcl $VERSION regex sources from $TAG ..."
for f in "${FILES[@]}"; do
    url="$BASE_URL/$f"
    out="$TARGET_DIR/$f"
    for attempt in 1 2 3 4; do
        if curl -fsSL --retry 0 -o "$out.tmp" "$url"; then
            mv "$out.tmp" "$out"
            break
        fi
        rm -f "$out.tmp"
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

echo "$VERSION" > "$STAMP_FILE"
echo "Fetched ${#FILES[@]} files to $TARGET_DIR"
