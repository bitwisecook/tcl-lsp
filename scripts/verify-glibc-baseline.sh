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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later
# Assert that dynamically-linked GNU/Linux release binaries do not reference a
# glibc symbol newer than the stated ABI ceiling.
set -euo pipefail

if [[ $# -lt 2 ]]; then
    printf 'usage: %s MAX_GLIBC BINARY [BINARY ...]\n' "$0" >&2
    exit 2
fi

ceiling="$1"
shift

version_greater_than() {
    local candidate="$1"
    local limit="$2"
    [[ "$candidate" != "$limit" ]] &&
        [[ "$(printf '%s\n%s\n' "$candidate" "$limit" | sort -V | tail -n 1)" == "$candidate" ]]
}

failed=0
for binary in "$@"; do
    if [[ ! -x "$binary" ]]; then
        printf 'FAIL  %s (missing or not executable)\n' "$binary" >&2
        failed=1
        continue
    fi

    if ! readelf -h "$binary" 2>/dev/null | grep -q 'OS/ABI:.*UNIX - System V'; then
        printf 'FAIL  %s (not a System V ELF executable)\n' "$binary" >&2
        failed=1
        continue
    fi

    interpreter="$(
        readelf -l "$binary" 2>/dev/null |
            sed -n 's/.*Requesting program interpreter: \([^]]*\).*/\1/p'
    )"
    if [[ -z "$interpreter" ]]; then
        printf 'FAIL  %s (no dynamic program interpreter)\n' "$binary" >&2
        failed=1
        continue
    fi

    mapfile -t versions < <(
        readelf --version-info "$binary" 2>/dev/null |
            grep -oE 'GLIBC_[0-9]+([.][0-9]+)+' |
            sed 's/^GLIBC_//' |
            sort -Vu
    )
    if [[ ${#versions[@]} -eq 0 ]]; then
        printf 'FAIL  %s (no versioned glibc references)\n' "$binary" >&2
        failed=1
        continue
    fi

    newest="${versions[${#versions[@]} - 1]}"
    if version_greater_than "$newest" "$ceiling"; then
        printf 'FAIL  %s requires GLIBC_%s (ceiling GLIBC_%s)\n' \
            "$binary" "$newest" "$ceiling" >&2
        failed=1
        continue
    fi

    printf 'PASS  %s: interpreter=%s max=GLIBC_%s ceiling=GLIBC_%s\n' \
        "$binary" "$interpreter" "$newest" "$ceiling"
done

exit "$failed"
