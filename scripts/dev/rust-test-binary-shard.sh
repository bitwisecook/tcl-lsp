#!/usr/bin/env bash
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

set -euo pipefail

if [ "$#" -lt 2 ]; then
    echo "usage: scripts/dev/rust-test-binary-shard.sh {run|list} INDEX/COUNT [nextest arguments...]" >&2
    exit 2
fi

operation=$1
partition=$2
shift 2

case "$operation" in
    run | list) ;;
    *)
        echo "rust-test-binary-shard: operation must be run or list" >&2
        exit 2
        ;;
esac

case "$partition" in
    [1-9]*/[1-9]*) ;;
    *)
        echo "rust-test-binary-shard: partition must be INDEX/COUNT" >&2
        exit 2
        ;;
esac

index=${partition%/*}
count=${partition#*/}
case "$index:$count" in
    *[!0-9:]* | :* | *:)
        echo "rust-test-binary-shard: partition must contain positive integers" >&2
        exit 2
        ;;
esac
if [ "$index" -lt 1 ] || [ "$index" -gt "$count" ]; then
    echo "rust-test-binary-shard: partition index is outside its count" >&2
    exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
manifest=${TCL_LSP_RUST_TEST_BINARY_SHARDS:-$script_dir/rust-test-binary-shards.tsv}
if [ ! -r "$manifest" ]; then
    echo "rust-test-binary-shard: cannot read $manifest" >&2
    exit 2
fi

declared_count=
declare -a exclusions=()
declare -a targets=(--lib --bins)
declare -A seen_test_targets=()
filter=
rows=0
line_number=0
while IFS=$'\t' read -r field1 field2 field3 field4 field5 extra || [ -n "${field1:-}" ]; do
    line_number=$((line_number + 1))
    case "${field1:-}" in
        '' | \#*) continue ;;
        @partitions)
            if [ -n "$declared_count" ] || [ -z "${field2:-}" ] || [ -n "${field3:-}" ]; then
                echo "rust-test-binary-shard: malformed partitions directive at $manifest:$line_number" >&2
                exit 2
            fi
            declared_count=$field2
            continue
            ;;
        @exclude)
            if [[ ! ${field2:-} =~ ^[A-Za-z0-9_-]+$ ]] || [ -n "${field3:-}" ]; then
                echo "rust-test-binary-shard: malformed exclusion at $manifest:$line_number" >&2
                exit 2
            fi
            exclusions+=(--exclude "$field2")
            continue
            ;;
    esac

    if [ -z "${field5:-}" ] || [ -n "${extra:-}" ]; then
        echo "rust-test-binary-shard: malformed row at $manifest:$line_number" >&2
        exit 2
    fi
    shard=$field1
    binary_id=$field2
    kind=$field3
    package=$field4
    target=$field5
    if [[ ! $shard =~ ^[1-9][0-9]*$ ]] \
        || [[ ! $binary_id =~ ^[A-Za-z0-9_:/.-]+$ ]] \
        || [[ ! $package =~ ^[A-Za-z0-9_-]+$ ]] \
        || [[ ! $target =~ ^[A-Za-z0-9_-]+$ ]]; then
        echo "rust-test-binary-shard: invalid field at $manifest:$line_number" >&2
        exit 2
    fi
    case "$kind" in
        lib | bin | test) ;;
        *)
            echo "rust-test-binary-shard: invalid target kind at $manifest:$line_number" >&2
            exit 2
            ;;
    esac
    [ "$shard" = "$index" ] || continue

    if { [ "$package" = tcl-lsp-core ] && [ "$kind" = lib ]; } \
        || { [ "$package" = tcl-lsp-db ] && [ "$target" = compiler_check_corpus ]; }; then
        needs_fixed_tcllib=1
    fi
    rows=$((rows + 1))
    if [ "$kind" = test ] && [ -z "${seen_test_targets[$target]:-}" ]; then
        targets+=(--test "$target")
        seen_test_targets[$target]=1
    fi
    clause="(package(=$package) & binary(=$target))"
    if [ -z "$filter" ]; then
        filter=$clause
    else
        filter="$filter | $clause"
    fi
done < "$manifest"

if [ -z "$declared_count" ] || [ "$declared_count" != "$count" ]; then
    echo "rust-test-binary-shard: $manifest does not declare $count partitions" >&2
    exit 2
fi
if [ "$rows" -eq 0 ]; then
    echo "rust-test-binary-shard: partition $partition contains no binaries" >&2
    exit 2
fi

# The light CI tier leaves out every test listed as heavy and runs under the
# time-boxed ci-light profile; the heavy tier (and every local use) runs the
# complete assignment. Listing and running both pass through here, so the
# recorded selection is exactly the executed one.
tier=${TCL_LSP_RUST_TEST_TIER:-heavy}
declare -a profile=()
case "$tier" in
    heavy) ;;
    light)
        heavy_list=${TCL_LSP_RUST_HEAVY_TESTS:-$script_dir/rust-heavy-tests.tsv}
        if [ ! -r "$heavy_list" ]; then
            echo "rust-test-binary-shard: cannot read $heavy_list" >&2
            exit 2
        fi
        heavy=
        heavy_line=0
        while IFS=$'\t' read -r heavy_binary heavy_test heavy_extra || [ -n "${heavy_binary:-}" ]; do
            heavy_line=$((heavy_line + 1))
            case "${heavy_binary:-}" in
                '' | \#*) continue ;;
            esac
            if [ -z "${heavy_test:-}" ] || [ -n "${heavy_extra:-}" ] \
                || [[ ! $heavy_binary =~ ^[A-Za-z0-9_:/.-]+$ ]] \
                || [[ ! $heavy_test =~ ^([A-Za-z0-9_:]+|\*)$ ]]; then
                echo "rust-test-binary-shard: malformed heavy row at $heavy_list:$heavy_line" >&2
                exit 2
            fi
            if [ "$heavy_test" = '*' ]; then
                clause="binary_id(=$heavy_binary)"
            else
                clause="(binary_id(=$heavy_binary) & test(=$heavy_test))"
            fi
            heavy=${heavy:+$heavy | }$clause
        done < "$heavy_list"
        if [ -n "$heavy" ]; then
            filter="($filter) & not ($heavy)"
        fi
        profile=(--profile ci-light)
        ;;
    *)
        echo "rust-test-binary-shard: TCL_LSP_RUST_TEST_TIER must be light or heavy" >&2
        exit 2
        ;;
esac

# Core and the DB compiler-check differential consume fixed upstream Tcllib
# fixtures. Listing/build-only jobs do not; execution uses Make's bootstrap.
if [ "$operation" = run ] && [ "${needs_fixed_tcllib:-0}" = 1 ]; then
    # stdout is the reporter stream; keep the bootstrap log off it.
    bash "$script_dir/ensure-test-deps.sh" --tcllib-only >&2
fi

exec cargo nextest "$operation" \
    "${profile[@]}" \
    --workspace \
    "${exclusions[@]}" \
    --all-features \
    "${targets[@]}" \
    -E "$filter" \
    "$@"
