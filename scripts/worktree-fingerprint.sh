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

# Emit a stable fingerprint of the current git worktree (HEAD + index + unstaged
# changes). Used by `make check-all` / `make test-slow` to stamp a successful
# run, and by the pre-push hook to verify the worktree hasn't drifted since
# the stamp was written.
#
# Untracked files are intentionally ignored — they don't affect tests run from
# tracked sources, and including them would invalidate the stamp on every
# stray scratch file.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# sha256 wrapper: GNU coreutils ships sha256sum, macOS / BusyBox ship shasum.
sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | cut -d' ' -f1
    else
        shasum -a 256 | cut -d' ' -f1
    fi
}

head=$(git rev-parse HEAD 2>/dev/null || echo "no-head")
# `git diff --cached` (no HEAD) is robust on empty repos; `git diff` (no HEAD)
# captures only unstaged changes, so we don't double-count the index.
staged=$(git diff --cached --no-color --no-ext-diff 2>/dev/null | sha256)
unstaged=$(git diff --no-color --no-ext-diff 2>/dev/null | sha256)

printf '%s\n%s\n%s\n' "$head" "$staged" "$unstaged" | sha256
