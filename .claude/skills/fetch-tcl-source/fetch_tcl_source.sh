#!/bin/bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# fetch_tcl_source.sh — Download full Tcl source trees for 8.4, 8.5, 8.6, 9.0,
# 9.1.
#
# Usage:
#   ./fetch_tcl_source.sh <version>
#   ./fetch_tcl_source.sh 84        # or 8.4
#   ./fetch_tcl_source.sh 85        # or 8.5
#   ./fetch_tcl_source.sh 86        # or 8.6
#   ./fetch_tcl_source.sh 90        # or 9.0
#   ./fetch_tcl_source.sh 91        # or 9.1
#   ./fetch_tcl_source.sh all       # all five versions
#   ./fetch_tcl_source.sh status    # show what's present in tmp/
#
# Fetches pre-built release tarballs from GitHub's codeload CDN
# (https://codeload.github.com/tcltk/tcl/tar.gz/refs/tags/<tag>).
# These are CDN-cached by GitHub so the download is easy on the upstream
# Tcl project, and tarballs avoid the disk + CPU overhead of git metadata.
#
# Extracts to tmp/tcl<full_version>/ in the repo root — full source
# (generic/, unix/, win/, tests/, library/, doc/, …), no .git directory.
# Idempotent — skips download if already present.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TMP_DIR="$REPO_ROOT/tmp"
CODELOAD_BASE="https://codeload.github.com/tcltk/tcl/tar.gz/refs/tags"

# Version → (full patch version, GitHub tag)
# Update these when new patch releases come out.
declare -A LATEST_VERSIONS=(
    [8.4]="8.4.20"
    [8.5]="8.5.19"
    [8.6]="8.6.16"
    [9.0]="9.0.4"
    [9.1]="9.1b0"
)

declare -A GITHUB_TAGS=(
    [8.4]="core-8-4-20"
    [8.5]="core-8-5-19"
    [8.6]="core-8-6-16"
    [9.0]="core-9-0-4"
    [9.1]="core-9-1-b0"
)

# Normalise user input
normalise_version() {
    local input="$1"
    case "$input" in
        84|8.4)  echo "8.4" ;;
        85|8.5)  echo "8.5" ;;
        86|8.6)  echo "8.6" ;;
        90|9.0)  echo "9.0" ;;
        91|9.1)  echo "9.1" ;;
        *)
            echo "ERROR: Unknown version '$input'" >&2
            echo "Valid versions: 84/8.4, 85/8.5, 86/8.6, 90/9.0, 91/9.1, all, status" >&2
            return 1
            ;;
    esac
}

# Show status of what's in tmp/
show_status() {
    echo "Tcl source trees in $TMP_DIR/:"
    echo ""
    local found=0
    for major_minor in 8.4 8.5 8.6 9.0 9.1; do
        local full="${LATEST_VERSIONS[$major_minor]}"
        local dir="$TMP_DIR/tcl${full}"
        if [[ -d "$dir/generic" ]] && [[ -d "$dir/tests" ]]; then
            local test_count
            test_count=$(find "$dir/tests" -name '*.test' 2>/dev/null | wc -l)
            echo "  tcl${full}/  [present]  ${test_count} test files"
            found=$((found + 1))
        else
            echo "  tcl${full}/  [missing]"
        fi
    done
    echo ""
    echo "$found of 5 versions present."
}

# Fetch one version by downloading the GitHub codeload tarball.
fetch_version() {
    local major_minor="$1"
    local full="${LATEST_VERSIONS[$major_minor]}"
    local tag="${GITHUB_TAGS[$major_minor]}"
    local target_dir="$TMP_DIR/tcl${full}"
    local url="${CODELOAD_BASE}/${tag}"

    if [[ -d "$target_dir/generic" ]] && [[ -d "$target_dir/tests" ]]; then
        echo "  tcl${full}/ already exists — skipping"
        return 0
    fi

    mkdir -p "$TMP_DIR"
    rm -rf "$target_dir"

    local tmp_tarball
    tmp_tarball="$(mktemp -p "$TMP_DIR" "tcl${full}.XXXXXX.tar.gz")"
    trap "rm -f '$tmp_tarball'" RETURN

    echo "  Downloading tcl ${full} source tarball ..."
    local got_tarball=0
    local attempt
    for attempt in 1 2 3 4; do
        if curl -fsSL --connect-timeout 15 --max-time 600 \
               -o "$tmp_tarball" "$url"; then
            got_tarball=1
            break
        fi
        if [[ $attempt -lt 4 ]]; then
            local wait=$((2 ** attempt))
            echo "    Retry $attempt (waiting ${wait}s) ..."
            sleep "$wait"
        fi
    done

    # Fall back to a shallow tag clone when the codeload CDN is unreachable.
    # Some sandboxed environments route outbound HTTPS through a proxy that
    # serves `github.com` but rejects `codeload.github.com` with a 403, which
    # exhausts every retry above and leaves the session with no Tcl sources —
    # and therefore no `libtommath`, so `runtime/rust` silently builds without
    # the numeric tower.  `git clone` goes through the same proxy happily.
    if [[ $got_tarball -eq 0 ]]; then
        echo "    Tarball unavailable; falling back to a shallow git clone of ${tag} ..."
        if git clone -q --depth 1 --branch "$tag" \
               "https://github.com/tcltk/tcl" "$target_dir" 2>/dev/null; then
            rm -rf "$target_dir/.git"
        else
            echo "  ERROR: Failed to download tcl ${full} (tarball and git clone)" >&2
            rm -rf "$target_dir"
            return 1
        fi
    else
        echo "  Extracting to tcl${full}/ ..."
        mkdir -p "$target_dir"
        tar -xzf "$tmp_tarball" -C "$target_dir" --strip-components=1
    fi

    if [[ -d "$target_dir/generic" ]] && [[ -d "$target_dir/tests" ]]; then
        local test_count
        test_count=$(find "$target_dir/tests" -name '*.test' 2>/dev/null | wc -l)
        local size
        size=$(du -sh "$target_dir" | awk '{print $1}')
        echo "  Done: tcl${full}/ (${test_count} test files, ${size})"
    else
        echo "  ERROR: generic/ or tests/ missing after extract" >&2
        rm -rf "$target_dir"
        return 1
    fi
}

# Main
if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <version|all|status>"
    echo ""
    echo "Versions: 84/8.4, 85/8.5, 86/8.6, 90/9.0, 91/9.1"
    echo "  all     — fetch all five versions"
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
        for v in 8.4 8.5 8.6 9.0 9.1; do
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
