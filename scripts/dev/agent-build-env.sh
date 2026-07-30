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

# agent-build-env.sh — per-worktree cargo build isolation for parallel agents.
#
# Sharing one CARGO_TARGET_DIR across concurrently-building git worktrees of
# this workspace serves stale or cross-checkout rlibs: cargo's unit hashing
# does not disambiguate workspace-member crates built from different worktree
# paths of the same workspace (same package name/version/features), so
# concurrent builds race on the same deps/ outputs.  A test run can then pass
# or fail against code that is not in the tree being tested.  See the
# "Parallel worktrees and agent build isolation" section of AGENTS.md and
# issue #1052 for the observed failure modes.
#
# This helper pins the build environment for the current worktree:
#
#   CARGO_TARGET_DIR         -> <worktree-root>/target  (never shared)
#   CARGO_INCREMENTAL=0      -> smaller target dir, no incremental-cache races
#   CARGO_PROFILE_DEV_DEBUG=0 -> keeps a dev target dir ~3-4 GB, not ~15 GB
#
# CARGO_HOME (the registry/git dependency cache) is deliberately left alone:
# sharing it across worktrees is safe — only workspace-member artefacts
# exhibit the collision.
#
# Usage:
#   source scripts/dev/agent-build-env.sh          # export into current shell
#   eval "$(scripts/dev/agent-build-env.sh --print)"  # same, for eval users
#   scripts/dev/agent-build-env.sh --check         # show what would be set

agent_build_env__main() {
    local worktree_root
    if ! worktree_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
        echo "agent-build-env.sh: not inside a git worktree" >&2
        return 1
    fi

    local target_dir="${worktree_root}/target"

    case "${1:-}" in
        --print)
            printf 'export CARGO_TARGET_DIR=%q\n' "${target_dir}"
            printf 'export CARGO_INCREMENTAL=0\n'
            printf 'export CARGO_PROFILE_DEV_DEBUG=0\n'
            ;;
        --check)
            echo "worktree root:            ${worktree_root}"
            echo "CARGO_TARGET_DIR:         ${target_dir}"
            echo "CARGO_INCREMENTAL:        0"
            echo "CARGO_PROFILE_DEV_DEBUG:  0"
            if [ -n "${CARGO_TARGET_DIR:-}" ] \
                && [ "${CARGO_TARGET_DIR}" != "${target_dir}" ]; then
                echo "WARNING: CARGO_TARGET_DIR is currently" \
                    "'${CARGO_TARGET_DIR}' — if another worktree builds into" \
                    "it concurrently, artefacts are untrustworthy." >&2
            fi
            ;;
        '')
            export CARGO_TARGET_DIR="${target_dir}"
            export CARGO_INCREMENTAL=0
            export CARGO_PROFILE_DEV_DEBUG=0
            ;;
        *)
            echo "usage: source agent-build-env.sh | agent-build-env.sh --print | --check" >&2
            return 2
            ;;
    esac
}

# When sourced, never forward "$@": with no explicit source arguments,
# bash leaves the caller's positional parameters in place, and a caller
# argument like "build" would be misread as an unknown mode, erroring out
# with nothing exported.  Sourcing is always the plain export form; the
# --print/--check modes are for direct execution.
if [ "${BASH_SOURCE[0]}" != "$0" ]; then
    agent_build_env__main
else
    agent_build_env__main "$@"
fi
