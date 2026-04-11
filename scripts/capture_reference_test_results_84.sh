#!/bin/bash
# capture_reference_test_results_84.sh — Capture test results from Tcl 8.4.
#
# Tcl 8.4 has tcltest (2.2) but lacks reflected channels (chan create),
# dict, ni, and {*}, so it needs a dedicated script
# (like capture_reference_bytecode_84.sh).
#
# Usage:
#   ./scripts/capture_reference_test_results_84.sh [TCLSH_PATH] [TESTS_DIR]
#
# If TCLSH_PATH is not provided, uses TCLSH84 environment variable, or
# tries to find tclsh8.4 in PATH, or looks in ~/src/tcl8.4.20/unix/tclsh.
#
# If TESTS_DIR is not provided, auto-detects from the tclsh path:
#   <tclsh_dir>/../tests/
#
# For macOS custom builds:
#   ./scripts/capture_reference_test_results_84.sh ~/src/tcl8.4.20/unix/tclsh
#
# The script will automatically set DYLD_LIBRARY_PATH from the tclsh
# directory for the duration of the run.
#
# Writes output to tests/test_reference/8.4/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUNNER="$SCRIPT_DIR/reference_test_runner_84.tcl"
OUTPUT_BASE="$REPO_ROOT/tests/test_reference"

# Test files from the VM conformance plan
# Same list as the main script, but many won't exist in 8.4.
# The script skips missing files gracefully.
TEST_FILES=(
    # Phase 1 — Parser
    parse.test
    parseOld.test
    # Phase 2 — Expressions
    compExpr-old.test
    compExpr.test
    expr-old.test
    expr.test
    mathop.test
    # Phase 3 — Variables & Scoping
    incr-old.test
    incr.test
    set-old.test
    var.test
    upvar.test
    uplevel.test
    # Phase 4 — Strings
    split.test
    format.test
    scan.test
    append.test
    subst.test
    string.test
    # Phase 5 — Lists (Core)
    list.test
    llength.test
    concat.test
    join.test
    lindex.test
    lrange.test
    linsert.test
    lreplace.test
    lrepeat.test
    # Phase 6 — Lists (Advanced)
    lsearch.test
    cmdIL.test
    lmap.test
    lset.test
    lpop.test
    lseq.test
    # Phase 7 — Control Flow
    if-old.test
    if.test
    while-old.test
    while.test
    for-old.test
    for.test
    foreach.test
    switch.test
    # Phase 8 — Procedures
    proc-old.test
    proc.test
    apply.test
    rename.test
    unknown.test
    # Phase 9 — Dicts & Arrays
    dict.test
    cmdAH.test
    # Phase 10 — Namespaces
    namespace-old.test
    namespace.test
    # Phase 11 — Error Handling & Eval
    eval.test
    error.test
    result.test
    # Phase 12 — Info & Introspection
    info.test
)

TIMEOUT=120

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
elif command -v tclsh8.4 &>/dev/null; then
    TCLSH="$(command -v tclsh8.4)"
elif [[ -f "$HOME/src/tcl8.4.20/unix/tclsh" ]]; then
    TCLSH="$HOME/src/tcl8.4.20/unix/tclsh"
else
    echo "ERROR: No tclsh 8.4 found." >&2
    echo "Usage: $0 [TCLSH_PATH] [TESTS_DIR]" >&2
    echo "  or set TCLSH84 env var" >&2
    echo "  or ensure tclsh8.4 is in PATH" >&2
    exit 1
fi

# Resolve to absolute path and set DYLD_LIBRARY_PATH
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

# Determine tests directory
if [[ $# -ge 2 ]]; then
    TESTS_SRC="$2"
else
    # Auto-detect: try /usr/src/tcl<version>*/tests/ first, then
    # <tclsh_dir>/../tests/, then ~/src/tcl8.4*/tests/
    TESTS_SRC=""
    for d in /usr/src/tcl${VERSION}*/tests /usr/src/tcl${VERSION_DIR}*/tests; do
        if [[ -d "$d" ]]; then
            TESTS_SRC="$d"
            break
        fi
    done
    if [[ -z "$TESTS_SRC" ]] && [[ -d "$TCLSH_DIR/../tests" ]]; then
        TESTS_SRC="$(cd "$TCLSH_DIR/.." && pwd)/tests"
    fi
    if [[ -z "$TESTS_SRC" ]]; then
        for d in "$HOME"/src/tcl${VERSION_DIR}*/tests; do
            if [[ -d "$d" ]]; then
                TESTS_SRC="$d"
                break
            fi
        done
    fi
    # Try repo tmp/ (used by fetch-tcl-source skill in cloud environments)
    if [[ -z "$TESTS_SRC" ]]; then
        for d in "$REPO_ROOT"/tmp/tcl${VERSION_DIR}*/tests; do
            if [[ -d "$d" ]]; then
                TESTS_SRC="$d"
                break
            fi
        done
    fi
fi

if [[ -z "$TESTS_SRC" ]] || [[ ! -d "$TESTS_SRC" ]]; then
    echo "ERROR: Tests directory not found." >&2
    echo "Searched: /usr/src/tcl${VERSION_DIR}*/tests/" >&2
    echo "         $TCLSH_DIR/../tests/" >&2
    echo "Pass TESTS_DIR as second argument:" >&2
    echo "  $0 $TCLSH /path/to/tcl8.4.20/tests" >&2
    exit 1
fi

OUTPUT_DIR="$OUTPUT_BASE/$VERSION_DIR"
mkdir -p "$OUTPUT_DIR"

echo "=== Tcl 8.4 test result capture ==="
echo "  Tcl version:   $VERSION"
echo "  tclsh path:    $TCLSH"
echo "  Tests dir:     $TESTS_SRC"
echo "  Output dir:    $OUTPUT_DIR"
echo ""

captured=0
failed=0
skipped=0

for test_file in "${TEST_FILES[@]}"; do
    base="${test_file%.test}"
    outfile="$OUTPUT_DIR/${base}.results"

    # Skip test files that don't exist in this Tcl version
    if [[ ! -f "$TESTS_SRC/$test_file" ]]; then
        echo "  $test_file ... MISSING (skipped)"
        skipped=$((skipped + 1))
        continue
    fi

    echo -n "  $test_file ... "

    errfile=$(mktemp)

    if timeout "$TIMEOUT" "$TCLSH" "$RUNNER" "$TESTS_SRC" "$test_file" \
           > "$outfile" 2>"$errfile"; then
        # Extract summary for progress display
        total=$(grep  '^TOTAL'   "$outfile" | awk '{print $2}')
        passed=$(grep '^PASSED'  "$outfile" | awk '{print $2}')
        skip_count=$(grep '^SKIPPED' "$outfile" | awk '{print $2}')
        fail_count=$(grep '^FAILED'  "$outfile" | awk '{print $2}')
        echo "ok (${passed:-?}/${total:-?} passed, ${skip_count:-?} skipped, ${fail_count:-?} failed)"
        captured=$((captured + 1))
    else
        exitcode=$?
        if [[ $exitcode -eq 124 ]]; then
            echo "TIMEOUT (>${TIMEOUT}s)"
        else
            echo "FAILED (exit $exitcode)"
            head -5 "$errfile" | sed 's/^/    /' >&2
        fi
        if [[ -s "$outfile" ]]; then
            captured=$((captured + 1))
        else
            rm -f "$outfile"
            failed=$((failed + 1))
        fi
    fi
    rm -f "$errfile"
done

echo ""
echo "  $captured captured, $failed failed, $skipped missing"

# Generate combined summary
summary="$OUTPUT_DIR/SUMMARY.txt"
{
    echo "# Reference test results for Tcl $VERSION"
    echo "# tclsh: $TCLSH"
    echo "# tests: $TESTS_SRC"
    echo "# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%Y-%m-%dT%H:%M:%S)"
    echo "#"
    printf "# %-30s %7s %7s %7s %7s\n" "File" "Total" "Passed" "Skipped" "Failed"
    printf "# %-30s %7s %7s %7s %7s\n" "----" "-----" "------" "-------" "------"
    for results in "$OUTPUT_DIR"/*.results; do
        [[ -f "$results" ]] || continue
        fname=$(basename "$results" .results)
        t=$(grep  '^TOTAL'   "$results" | awk '{print $2}')
        p=$(grep  '^PASSED'  "$results" | awk '{print $2}')
        s=$(grep  '^SKIPPED' "$results" | awk '{print $2}')
        f=$(grep  '^FAILED'  "$results" | awk '{print $2}')
        printf "  %-30s %7s %7s %7s %7s\n" "${fname}.test" "${t:-?}" "${p:-?}" "${s:-?}" "${f:-?}"
    done
} > "$summary"

echo ""
echo "  Summary written to: $summary"
cat "$summary"
echo ""
echo "Done. Reference test results in: $OUTPUT_DIR/"
