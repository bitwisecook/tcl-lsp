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

# Build the deliberately pinned current-upstream Jim oracle. Reference
# metadata and selection belong to tcl-test-support, not this shell helper.
set -euo pipefail
oracle_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
oracle_manifest="${oracle_repo_root}/rust/tcl-test-support/jim-reference.txt"
read_manifest() {
    sed -n "s/^${1}=//p" "$oracle_manifest"
}
oracle_jim_root="${TCL_LSP_JIM_ROOT:-${oracle_repo_root}/tmp/jimtcl-oracle}"
oracle_jim_repository="$(read_manifest repository)"
oracle_jim_revision="$(read_manifest revision)"
oracle_jim_patchlevel="$(read_manifest patchlevel)"
if [[ ! -d "$oracle_jim_root/.git" ]]; then
    mkdir -p "$(dirname "$oracle_jim_root")"
    git clone "$oracle_jim_repository" "$oracle_jim_root"
fi
if ! git -C "$oracle_jim_root" diff --quiet HEAD --; then
    echo "Jim source has local modifications: $oracle_jim_root" >&2
    exit 1
fi
if ! git -C "$oracle_jim_root" rev-parse --verify "$oracle_jim_revision^{commit}" >/dev/null 2>&1; then
    git -C "$oracle_jim_root" fetch --tags origin
fi
git -C "$oracle_jim_root" checkout --detach "$oracle_jim_revision"
(
    cd "$oracle_jim_root"
    ./configure --prefix="$oracle_jim_root/install"
    make -j"${JOBS:-4}"
)
oracle_jim_reported="$("$oracle_jim_root/jimsh" -e 'puts [info patchlevel]')"
if [[ "$oracle_jim_reported" != "$oracle_jim_patchlevel" ]]; then
    echo "Jim reports $oracle_jim_reported, expected $oracle_jim_patchlevel" >&2
    exit 1
fi
printf 'TCL_LSP_JIMSH=%s\n' "$oracle_jim_root/jimsh"
