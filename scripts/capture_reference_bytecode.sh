#!/bin/bash
# capture_reference_bytecode.sh — Capture reference disassembly from tclsh.
#
# Usage:
#   ./scripts/capture_reference_bytecode.sh
#
# Automatically runs against tclsh8.5, tclsh8.6, and tclsh9.0,
# skipping any that are not found in PATH.
#
# For Tcl 8.4, use the dedicated script:
#   ./scripts/capture_reference_bytecode_84.sh [TCLSH_PATH]
# (Tcl 8.4 uses tcl_traceCompile 2 instead of tcl::unsupported::disassemble)
#
# Writes output to tests/bytecode_reference/<version>/ for each
# available tclsh.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SNIPPETS_DIR="$REPO_ROOT/tests/bytecode_snippets"
DISASM_SCRIPT="$SCRIPT_DIR/reference_disasm.tcl"

TCLSH_VERSIONS=(tclsh8.5 tclsh8.6 tclsh9.0)

capture_for_tclsh() {
    local TCLSH="$1"

    if ! command -v "$TCLSH" &>/dev/null; then
        echo "SKIP: $TCLSH not found in PATH"
        echo ""
        return
    fi

    # Detect Tcl version
    local VERSION
    VERSION=$("$TCLSH" <<'EOF'
puts [info patchlevel]
EOF
    )

    # Extract major.minor for directory name (e.g. "9.0" from "9.0.1")
    local VERSION_DIR
    VERSION_DIR=$(echo "$VERSION" | sed 's/^\([0-9]*\.[0-9]*\).*/\1/')

    local OUTPUT_DIR="$REPO_ROOT/tests/bytecode_reference/$VERSION_DIR"
    mkdir -p "$OUTPUT_DIR"

    echo "=== $TCLSH ==="
    echo "  Tcl version:   $VERSION"
    echo "  tclsh path:    $(command -v "$TCLSH")"
    echo "  Output dir:    $OUTPUT_DIR"

    local count=0
    local failed=0

    # Process each snippet individually
    for snippet in "$SNIPPETS_DIR"/*.tcl; do
        local base
        base="$(basename "$snippet" .tcl)"
        local outfile="$OUTPUT_DIR/${base}.disasm"

        echo -n "  $base ... "

        if "$TCLSH" "$DISASM_SCRIPT" "$SNIPPETS_DIR" 2>/dev/null | \
           sed -n "/^=== ${base}\.tcl ===/,/^=== END ===/p" > "$outfile.tmp" 2>/dev/null; then
            # Strip the delimiter lines
            sed '1d;$d' "$outfile.tmp" > "$outfile"
            rm -f "$outfile.tmp"
            echo "ok"
            count=$((count + 1))
        else
            echo "FAILED"
            rm -f "$outfile.tmp" "$outfile"
            failed=$((failed + 1))
        fi
    done

    echo "  $count captured, $failed failed"
    echo ""
}

echo "Snippets dir: $SNIPPETS_DIR"
echo ""

for tclsh in "${TCLSH_VERSIONS[@]}"; do
    capture_for_tclsh "$tclsh"
done

echo "Done. Reference bytecode in: $REPO_ROOT/tests/bytecode_reference/"
