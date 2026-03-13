#!/bin/bash
# capture_reference_bytecode_84.sh — Capture reference disassembly from Tcl 8.4.
#
# Tcl 8.4 lacks tcl::unsupported::disassemble, so we use tcl_traceCompile 2
# which dumps bytecode to stderr during compilation.
#
# Usage:
#   ./scripts/capture_reference_bytecode_84.sh [TCLSH_PATH]
#
# If TCLSH_PATH is not provided, uses TCLSH84 environment variable, or
# tries to find tclsh8.4 in PATH.
#
# For macOS custom builds, pass the path to the tclsh binary:
#   ./scripts/capture_reference_bytecode_84.sh ~/src/tcl8.4.20/unix/tclsh
#
# The script will automatically set DYLD_LIBRARY_PATH from the tclsh
# directory for the duration of the run.
#
# Writes output to tests/bytecode_reference/8.4/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SNIPPETS_DIR="$REPO_ROOT/tests/bytecode_snippets"

# Save original DYLD_LIBRARY_PATH so we can restore on exit
_ORIG_DYLD_LIBRARY_PATH="${DYLD_LIBRARY_PATH:-}"
cleanup() {
    if [[ -n "$_ORIG_DYLD_LIBRARY_PATH" ]]; then
        export DYLD_LIBRARY_PATH="$_ORIG_DYLD_LIBRARY_PATH"
    else
        unset DYLD_LIBRARY_PATH 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Determine tclsh path
if [[ $# -ge 1 ]]; then
    TCLSH="$1"
elif [[ -n "${TCLSH84:-}" ]]; then
    TCLSH="$TCLSH84"
elif [[ -f "$HOME/src/tcl8.4.20/unix/tclsh" ]]; then
    TCLSH="$HOME/src/tcl8.4.20/unix/tclsh"
elif command -v tclsh8.4 &>/dev/null; then
    TCLSH="tclsh8.4"
else
    echo "ERROR: No tclsh 8.4 found." >&2
    echo "Usage: $0 [TCLSH_PATH]" >&2
    echo "  or set TCLSH84 env var" >&2
    echo "  or ensure tclsh8.4 is in PATH" >&2
    echo "  or place a build in ~/src/tcl8.4.20/unix/tclsh" >&2
    exit 1
fi

# Resolve to absolute path and set DYLD_LIBRARY_PATH from the tclsh directory
TCLSH="$(cd "$(dirname "$TCLSH")" && pwd)/$(basename "$TCLSH")"
TCLSH_DIR="$(dirname "$TCLSH")"
export DYLD_LIBRARY_PATH="$TCLSH_DIR${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"

# Verify tclsh works
if ! "$TCLSH" <<'EOF' &>/dev/null
puts [info patchlevel]
EOF
then
    echo "ERROR: Cannot run $TCLSH" >&2
    echo "Make sure the Tcl shared library (e.g. libtcl8.4g.dylib) is in: $TCLSH_DIR" >&2
    exit 1
fi

VERSION=$("$TCLSH" <<'EOF'
puts [info patchlevel]
EOF
)

VERSION_DIR=$(echo "$VERSION" | sed 's/^\([0-9]*\.[0-9]*\).*/\1/')
OUTPUT_DIR="$REPO_ROOT/tests/bytecode_reference/$VERSION_DIR"
mkdir -p "$OUTPUT_DIR"

echo "=== Tcl 8.4 bytecode capture ==="
echo "  Tcl version:   $VERSION"
echo "  tclsh path:    $TCLSH"
echo "  Output dir:    $OUTPUT_DIR"
echo ""

count=0
failed=0
skipped=0
trace_tmp=$(mktemp)
trap 'rm -f "$trace_tmp"; cleanup' EXIT

for snippet in "$SNIPPETS_DIR"/*.tcl; do
    base="$(basename "$snippet" .tcl)"
    outfile="$OUTPUT_DIR/${base}.disasm"

    echo -n "  $base ... "

    # Create a wrapper that enables tcl_traceCompile 2, wraps the snippet
    # in a proc, and calls it to trigger lazy bytecode compilation.
    # In Tcl 8.4, tcl_traceCompile output goes to stdout.
    wrapper=$(cat <<WRAPPER
set tcl_traceCompile 2
set fd [open {$snippet} r]
set source [read \$fd]
close \$fd
proc _snippet_ {} \$source
catch {_snippet_}
WRAPPER
    )

    # Run and capture stdout (bytecode trace), discard stderr
    "$TCLSH" <<< "$wrapper" 2>/dev/null >"$trace_tmp" || true

    if [[ -s "$trace_tmp" ]]; then
        # Strip wrapper noise: only keep from 'Compiling body of proc "_snippet_"'
        # onward — that's the actual snippet bytecode, not the wrapper's own
        # compilation (set fd, open, close, proc, catch, etc.).
        if grep -qn 'Compiling body of proc "_snippet_"' "$trace_tmp"; then
            sed -n '/^Compiling body of proc "_snippet_"/,$p' "$trace_tmp" > "$outfile"
            echo "ok"
            count=$((count + 1))
        else
            echo "no snippet body (skipped)"
            rm -f "$outfile"
            skipped=$((skipped + 1))
        fi
    else
        echo "no output (skipped)"
        rm -f "$outfile"
        skipped=$((skipped + 1))
    fi
done
rm -f "$trace_tmp"

echo ""
echo "  $count captured, $skipped skipped, $failed failed"
echo ""
echo "Done. Reference bytecode in: $OUTPUT_DIR/"
