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

# Install the project's git hooks into the repo's hooks directory.
#
# Currently installs:
#   pre-push  — refuses to push unless 'make check-all' (or 'make test-slow',
#               which is a strict superset) has been run against the current
#               worktree.  See scripts/hooks/pre-push.
#
# Resolves the destination via `git rev-parse --git-path hooks` so it works
# correctly for git worktrees (where .git is a file) and for repos that set
# core.hooksPath.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
src_dir="$repo_root/scripts/hooks"
dst_dir=$(cd "$repo_root" && git rev-parse --git-path hooks)
case "$dst_dir" in
    /*) ;;
    *)  dst_dir="$repo_root/$dst_dir" ;;
esac

mkdir -p "$dst_dir"

for hook in pre-push; do
    src="$src_dir/$hook"
    dst="$dst_dir/$hook"
    if [ ! -f "$src" ]; then
        echo "install-hooks: missing source hook $src" >&2
        exit 1
    fi
    install -m 0755 "$src" "$dst"
    echo "==> Installed $hook -> $dst"
done

chmod +x "$repo_root/scripts/worktree-fingerprint.sh"
cat <<EOF
==> Hooks installed.  Before pushing:
==>   make check-all   (full lint + typecheck for every language → tmp/check-all.stamp)
==> Before opening a PR:
==>   make test-slow   (everything; writes tmp/check-all.stamp + tmp/test-slow.stamp)
EOF
