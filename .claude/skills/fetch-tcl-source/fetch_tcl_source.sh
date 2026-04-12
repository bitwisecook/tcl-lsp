#!/bin/bash
# fetch_tcl_source.sh — Download Tcl test and library files via sparse checkout.
#
# Usage:
#   ./fetch_tcl_source.sh <version>
#   ./fetch_tcl_source.sh 84        # or 8.4
#   ./fetch_tcl_source.sh 85        # or 8.5
#   ./fetch_tcl_source.sh 86        # or 8.6
#   ./fetch_tcl_source.sh 90        # or 9.0
#   ./fetch_tcl_source.sh all       # all four versions
#   ./fetch_tcl_source.sh status    # show what's present in tmp/
#
# Uses git sparse-checkout to fetch only tests/ and library/ from the
# tcltk/tcl GitHub repository, keeping downloads small (~11 MB per version
# instead of ~100 MB+ for a full clone).
#
# Extracts to tmp/tcl<full_version>/ in the repo root.
# Idempotent — skips download if already present.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TMP_DIR="$REPO_ROOT/tmp"
GITHUB_REPO="https://github.com/tcltk/tcl.git"

# Version → (GitHub tag, local directory name)
# Update these when new patch releases come out.
declare -A LATEST_VERSIONS=(
    [8.4]="8.4.20"
    [8.5]="8.5.19"
    [8.6]="8.6.16"
    [9.0]="9.0.3"
)

declare -A GITHUB_TAGS=(
    [8.4]="core-8-4-20"
    [8.5]="core-8-5-19"
    [8.6]="core-8-6-16"
    [9.0]="core-9-0-3"
)

# Normalise user input
normalise_version() {
    local input="$1"
    case "$input" in
        84|8.4)  echo "8.4" ;;
        85|8.5)  echo "8.5" ;;
        86|8.6)  echo "8.6" ;;
        90|9.0)  echo "9.0" ;;
        *)
            echo "ERROR: Unknown version '$input'" >&2
            echo "Valid versions: 84/8.4, 85/8.5, 86/8.6, 90/9.0, all, status" >&2
            return 1
            ;;
    esac
}

# Show status of what's in tmp/
show_status() {
    echo "Tcl source trees in $TMP_DIR/:"
    echo ""
    local found=0
    for major_minor in 8.4 8.5 8.6 9.0; do
        local full="${LATEST_VERSIONS[$major_minor]}"
        local dir="$TMP_DIR/tcl${full}"
        if [[ -d "$dir/tests" ]]; then
            local test_count
            test_count=$(find "$dir/tests" -name '*.test' 2>/dev/null | wc -l)
            echo "  tcl${full}/  [present]  ${test_count} test files"
            found=$((found + 1))
        else
            echo "  tcl${full}/  [missing]"
        fi
    done
    echo ""
    echo "$found of 4 versions present."
}

# Fetch one version via git sparse-checkout
fetch_version() {
    local major_minor="$1"
    local full="${LATEST_VERSIONS[$major_minor]}"
    local tag="${GITHUB_TAGS[$major_minor]}"
    local target_dir="$TMP_DIR/tcl${full}"

    if [[ -d "$target_dir/tests" ]]; then
        echo "  tcl${full}/ already exists — skipping"
        return 0
    fi

    mkdir -p "$TMP_DIR"

    echo "  Cloning tcl ${full} (sparse: tests/ + library/) ..."
    local attempt
    for attempt in 1 2 3 4; do
        if git clone --depth 1 --filter=blob:none --sparse \
               --branch "$tag" "$GITHUB_REPO" "$target_dir" 2>/dev/null; then
            break
        fi
        if [[ $attempt -lt 4 ]]; then
            local wait=$((2 ** attempt))
            echo "    Retry $attempt (waiting ${wait}s) ..."
            rm -rf "$target_dir"
            sleep "$wait"
        else
            echo "  ERROR: Failed to clone after 4 attempts" >&2
            rm -rf "$target_dir"
            return 1
        fi
    done

    echo "  Setting sparse-checkout to tests/ + library/ ..."
    git -C "$target_dir" sparse-checkout set tests library 2>/dev/null

    if [[ -d "$target_dir/tests" ]]; then
        local test_count
        test_count=$(find "$target_dir/tests" -name '*.test' 2>/dev/null | wc -l)
        echo "  Done: tcl${full}/ (${test_count} test files)"
    else
        echo "  ERROR: tests/ directory not found after sparse checkout" >&2
        return 1
    fi
}

# Main
if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <version|all|status>"
    echo ""
    echo "Versions: 84/8.4, 85/8.5, 86/8.6, 90/9.0"
    echo "  all     — fetch all four versions"
    echo "  status  — show what's already in tmp/"
    exit 1
fi

case "$1" in
    status)
        show_status
        ;;
    all)
        echo "Fetching all Tcl source trees to $TMP_DIR/"
        echo ""
        for v in 8.4 8.5 8.6 9.0; do
            echo "=== Tcl $v ==="
            fetch_version "$v"
            echo ""
        done
        show_status
        ;;
    *)
        major_minor=$(normalise_version "$1")
        echo "=== Tcl $major_minor ==="
        fetch_version "$major_minor"
        ;;
esac
