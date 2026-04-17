#!/bin/bash
# capture_reference_test_results.sh — Run Tcl test files through real tclsh
# and capture pass/fail/skip results as reference data.
#
# Usage:
#   ./scripts/capture_reference_test_results.sh              # auto-detect all
#   ./scripts/capture_reference_test_results.sh tclsh9.0     # explicit name
#   ./scripts/capture_reference_test_results.sh /path/tclsh  # explicit path
#
# Each tclsh version uses its OWN test source tree (not a shared one).
# The script searches /usr/src/tcl<version>*/tests/ first, then falls
# back to tmp/tcl<version>*/tests/ in the repo, then ~/src/tcl<version>*/tests/.
#
# Handles Tcl 8.4 through 9.0 — uses reference_test_runner_84.tcl for
# 8.4 (no chan create, dict, ni) and reference_test_runner.tcl for 8.5+.
#
# Writes output to tests/test_reference/<version>/ for each tclsh found.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUNNER_85="$SCRIPT_DIR/reference_test_runner.tcl"
RUNNER_84="$SCRIPT_DIR/reference_test_runner_84.tcl"
OUTPUT_BASE="$REPO_ROOT/tests/test_reference"

# Test files from the VM conformance plan
# Phases 1-12, ordered by phase.  Files that don't exist in a given
# Tcl version are skipped automatically.
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

# Time limit per test file (seconds).
TIMEOUT=120

# Find test source directory for a given Tcl version
# Searches /usr/src/tcl<version>*/tests/ then tmp/tcl<version>*/tests/.
find_tests_dir() {
    local version="$1"  # full version e.g. "9.0.3"
    local major_minor="$2"  # e.g. "9.0"

    # Try /usr/src/tcl<full_version>/tests/ first (exact match)
    if [[ -d "/usr/src/tcl${version}/tests" ]]; then
        echo "/usr/src/tcl${version}/tests"
        return
    fi

    # Try /usr/src/tcl<major.minor>*/tests/ (glob)
    for d in /usr/src/tcl${major_minor}*/tests; do
        if [[ -d "$d" ]]; then
            echo "$d"
            return
        fi
    done

    # Try repo tmp/tcl<major.minor>*/tests/
    for d in "$REPO_ROOT"/tmp/tcl${major_minor}*/tests; do
        if [[ -d "$d" ]]; then
            echo "$d"
            return
        fi
    done

    # Try ~/src/tcl<full_version>/tests/ (exact match)
    if [[ -d "$HOME/src/tcl${version}/tests" ]]; then
        echo "$HOME/src/tcl${version}/tests"
        return
    fi

    # Try ~/src/tcl<major.minor>*/tests/ (glob)
    for d in "$HOME"/src/tcl${major_minor}*/tests; do
        if [[ -d "$d" ]]; then
            echo "$d"
            return
        fi
    done

    return 1
}

# Choose the right Tcl runner for a version
choose_runner() {
    local major_minor="$1"  # e.g. "8.4"
    case "$major_minor" in
        8.4) echo "$RUNNER_84" ;;
        *)   echo "$RUNNER_85" ;;
    esac
}

# Detect tclsh versions
TCLSH_VERSIONS=(tclsh8.4 tclsh8.5 tclsh8.6 tclsh9.0)

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

    # Extract major.minor for directory name (e.g. "9.0" from "9.0.3")
    local VERSION_DIR
    VERSION_DIR=$(echo "$VERSION" | sed 's/^\([0-9]*\.[0-9]*\).*/\1/')

    # Find this version's test source directory
    local TESTS_SRC
    if ! TESTS_SRC=$(find_tests_dir "$VERSION" "$VERSION_DIR"); then
        echo "SKIP: $TCLSH ($VERSION) — no test source directory found"
        echo "  Searched: /usr/src/tcl${VERSION_DIR}*/tests/"
        echo "           $REPO_ROOT/tmp/tcl${VERSION_DIR}*/tests/"
        echo "           $HOME/src/tcl${VERSION_DIR}*/tests/"
        echo ""
        return
    fi

    # Choose the right runner script
    local RUNNER
    RUNNER=$(choose_runner "$VERSION_DIR")

    local OUTPUT_DIR="$OUTPUT_BASE/$VERSION_DIR"
    mkdir -p "$OUTPUT_DIR"

    echo "=== $TCLSH ==="
    echo "  Tcl version:   $VERSION"
    echo "  tclsh path:    $(command -v "$TCLSH")"
    echo "  Tests dir:     $TESTS_SRC"
    echo "  Runner:        $(basename "$RUNNER")"
    echo "  Output dir:    $OUTPUT_DIR"
    echo ""

    local captured=0
    local failed=0
    local skipped=0

    for test_file in "${TEST_FILES[@]}"; do
        local base="${test_file%.test}"
        local outfile="$OUTPUT_DIR/${base}.results"

        # Skip test files that don't exist in this Tcl version
        if [[ ! -f "$TESTS_SRC/$test_file" ]]; then
            echo "  $test_file ... MISSING (skipped)"
            skipped=$((skipped + 1))
            continue
        fi

        echo -n "  $test_file ... "

        # Run with a timeout; capture stdout to the results file,
        # stderr to a temp file for error reporting.
        local errfile
        errfile=$(mktemp)

        if timeout "$TIMEOUT" "$TCLSH" "$RUNNER" "$TESTS_SRC" "$test_file" \
               > "$outfile" 2>"$errfile"; then
            # Extract the summary line for the progress display
            local total passed skip_count fail_count
            total=$(grep  '^TOTAL'   "$outfile" | awk '{print $2}')
            passed=$(grep '^PASSED'  "$outfile" | awk '{print $2}')
            skip_count=$(grep '^SKIPPED' "$outfile" | awk '{print $2}')
            fail_count=$(grep '^FAILED'  "$outfile" | awk '{print $2}')
            echo "ok (${passed:-?}/${total:-?} passed, ${skip_count:-?} skipped, ${fail_count:-?} failed)"
            captured=$((captured + 1))
        else
            local exitcode=$?
            if [[ $exitcode -eq 124 ]]; then
                echo "TIMEOUT (>${TIMEOUT}s)"
            else
                echo "FAILED (exit $exitcode)"
                # Show first few lines of stderr for diagnostics
                head -5 "$errfile" | sed 's/^/    /' >&2
            fi
            # Still keep partial output if any
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
    local summary="$OUTPUT_DIR/SUMMARY.txt"
    {
        echo "# Reference test results for Tcl $VERSION"
        echo "# tclsh: $(command -v "$TCLSH")"
        echo "# tests: $TESTS_SRC"
        echo "# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%Y-%m-%dT%H:%M:%S)"
        echo "#"
        printf "# %-30s %7s %7s %7s %7s\n" "File" "Total" "Passed" "Skipped" "Failed"
        printf "# %-30s %7s %7s %7s %7s\n" "----" "-----" "------" "-------" "------"
        for results in "$OUTPUT_DIR"/*.results; do
            [[ -f "$results" ]] || continue
            local fname
            fname=$(basename "$results" .results)
            local t p s f
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
}

# Main
echo "Output base: $OUTPUT_BASE"
echo ""

if [[ $# -gt 0 ]]; then
    # Explicit tclsh path(s) provided
    for tclsh_arg in "$@"; do
        capture_for_tclsh "$tclsh_arg"
    done
else
    # Auto-detect
    for tclsh in "${TCLSH_VERSIONS[@]}"; do
        capture_for_tclsh "$tclsh"
    done
fi

echo "Done. Reference test results in: $OUTPUT_BASE/"
