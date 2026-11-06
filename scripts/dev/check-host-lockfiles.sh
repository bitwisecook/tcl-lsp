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

# check-host-lockfiles.sh — every Cargo.lock outside the root workspace (the
# wasm hosts, runtime/rust, editors/zed, …) must still resolve its manifest.
# `cargo check --workspace` never reads these, so a dependency added to a
# shared crate goes stale in them silently (#2301).

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

failed=0
while IFS= read -r lock; do
    dir=$(dirname "$lock")
    [ "$dir" = "." ] && continue
    if ! err=$(cd "$dir" && cargo metadata --locked --format-version 1 2>&1 >/dev/null); then
        echo "$lock: stale — run \`cargo update --workspace\` in $dir" >&2
        printf '%s\n' "$err" | sed 's/^/    /' >&2
        failed=1
    fi
done < <(git ls-files -- '*Cargo.lock')

if [ "$failed" -ne 0 ]; then
    exit 1
fi
echo "host Cargo.lock files resolve their manifests"
