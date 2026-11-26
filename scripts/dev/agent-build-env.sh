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
# or fail against code that is not in the tree being tested.  See
# docs/kcs/kcs-issue-parallel-worktree-builds-serve-stale-artefacts.md for
# the observed failure modes.
#
# This helper pins the build environment for the current worktree:
#
#   CARGO_TARGET_DIR         -> <worktree-root>/target  (never shared)
#   CARGO_PROFILE_TEST_INCREMENTAL=true
#                            -> test builds are incremental: an edit recompiles
#                               the changed part of a crate, not the whole
#                               crate — about 1 minute from an edit in
#                               tcl-compiler to its tests, against 11 without.
#                               Costs 5-10 GB of target dir; the per-worktree
#                               target above keeps the cache private, so there
#                               is no cross-worktree race.
#   CARGO_PROFILE_DEV_INCREMENTAL=false
#                            -> `cargo xtask`, `cargo clippy` and `cargo check`
#                               stay non-incremental: they run once before a
#                               push, and a second and third incremental cache
#                               of the whole workspace does not fit the disk.
#                               Set TCL_LSP_AGENT_INCREMENTAL=0 to turn the
#                               test cache off too (CI sets CARGO_INCREMENTAL=0
#                               itself, which overrides both).
#   CARGO_PROFILE_DEV_DEBUG=0 -> keeps a dev target dir ~3-4 GB, not ~15 GB
#   CARGO_PROFILE_TEST_DEBUG=0 -> drops the test profile's line tables too, so a
#                               full local test build fits on a laptop
#   CARGO_PROFILE_TEST_OPT_LEVEL=1
#                            -> matches CI's test shards: tests run ~4x faster
#                               (tcl-compiler unit harness 1,902 s -> 439 s)
#                               and the stack-budget tests see CI's frame sizes;
#                               at opt-level 0 depth_guard's budget test
#                               overflows and aborts its whole harness.
#                               TCL_LSP_AGENT_TEST_OPT_LEVEL overrides it.
#   CARGO_ALIAS_XTASK="run --profile test --package xtask --"
#   CARGO_ALIAS_XTASK_BACKING="run --profile test --package xtask-backing --"
#                            -> `cargo xtask` and `cargo xtask-backing` build
#                               under the test profile, so the gates reuse the
#                               dependency units `cargo test` already built
#                               instead of a third, dev-build copy of the
#                               workspace (plan item 297). Check and clippy
#                               stay on dev: check mode never shares units with
#                               a build, whatever the profile. CI keeps the
#                               checked-in aliases. Run binaries the same way:
#                               `cargo run --profile test -p tcl`.
#
#   RUSTC_WRAPPER=sccache, SCCACHE_DIR=$HOME/.cache/sccache, SCCACHE_CACHE_SIZE=20G
#                            -> only when `sccache --version` succeeds: one
#                               compiler cache per machine, shared by every
#                               worktree (plan item 260). With sccache v0.18.0
#                               it shares third-party libraries only, since
#                               workspace crates are incremental (test) or
#                               keyed by their absolute CARGO_MANIFEST_DIR
#                               (dev); the units cargo builds do not change.
#
# Stale unit and incremental directories are removed by
# scripts/dev/prune-targets.py (`--dry-run` lists them).
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
    local test_incremental=true
    local test_opt_level="${TCL_LSP_AGENT_TEST_OPT_LEVEL:-1}"
    local sccache_dir=""
    if command -v sccache >/dev/null 2>&1 && sccache --version >/dev/null 2>&1; then
        sccache_dir="${HOME}/.cache/sccache"
    fi
    local xtask_alias="run --profile test --package xtask --"
    local xtask_backing_alias="run --profile test --package xtask-backing --"
    if [ "${TCL_LSP_AGENT_INCREMENTAL:-1}" = 0 ]; then
        test_incremental=false
    fi

    case "${1:-}" in
        --print)
            printf 'export CARGO_TARGET_DIR=%q\n' "${target_dir}"
            printf 'unset CARGO_INCREMENTAL\n'
            printf 'export CARGO_PROFILE_TEST_INCREMENTAL=%q\n' "${test_incremental}"
            printf 'export CARGO_PROFILE_DEV_INCREMENTAL=false\n'
            printf 'export CARGO_PROFILE_DEV_DEBUG=0\n'
            printf 'export CARGO_PROFILE_TEST_DEBUG=0\n'
            printf 'export CARGO_PROFILE_TEST_OPT_LEVEL=%q\n' "${test_opt_level}"
            printf 'export CARGO_ALIAS_XTASK=%q\n' "${xtask_alias}"
            printf 'export CARGO_ALIAS_XTASK_BACKING=%q\n' "${xtask_backing_alias}"
            if [ -n "${sccache_dir}" ]; then
                printf 'export RUSTC_WRAPPER=sccache\n'
                printf 'export SCCACHE_DIR=%q\n' "${sccache_dir}"
                printf 'export SCCACHE_CACHE_SIZE=20G\n'
            fi
            ;;
        --check)
            echo "worktree root:            ${worktree_root}"
            echo "CARGO_TARGET_DIR:         ${target_dir}"
            echo "CARGO_PROFILE_TEST_INCREMENTAL: ${test_incremental}"
            echo "CARGO_PROFILE_DEV_INCREMENTAL:  false"
            echo "CARGO_PROFILE_DEV_DEBUG:  0"
            echo "CARGO_PROFILE_TEST_DEBUG: 0"
            echo "CARGO_PROFILE_TEST_OPT_LEVEL: ${test_opt_level}"
            echo "CARGO_ALIAS_XTASK:        ${xtask_alias}"
            echo "CARGO_ALIAS_XTASK_BACKING: ${xtask_backing_alias}"
            if [ -n "${sccache_dir}" ]; then
                echo "RUSTC_WRAPPER:            sccache"
                echo "SCCACHE_DIR:              ${sccache_dir}"
                echo "SCCACHE_CACHE_SIZE:       20G"
            else
                echo "RUSTC_WRAPPER:            (unset: sccache is not installed)"
            fi
            if [ -n "${CARGO_TARGET_DIR:-}" ] \
                && [ "${CARGO_TARGET_DIR}" != "${target_dir}" ]; then
                echo "WARNING: CARGO_TARGET_DIR is currently" \
                    "'${CARGO_TARGET_DIR}' — if another worktree builds into" \
                    "it concurrently, artefacts are untrustworthy." >&2
            fi
            ;;
        '')
            export CARGO_TARGET_DIR="${target_dir}"
            unset CARGO_INCREMENTAL
            export CARGO_PROFILE_TEST_INCREMENTAL="${test_incremental}"
            export CARGO_PROFILE_DEV_INCREMENTAL=false
            export CARGO_PROFILE_DEV_DEBUG=0
            export CARGO_PROFILE_TEST_DEBUG=0
            export CARGO_PROFILE_TEST_OPT_LEVEL="${test_opt_level}"
            export CARGO_ALIAS_XTASK="${xtask_alias}"
            export CARGO_ALIAS_XTASK_BACKING="${xtask_backing_alias}"
            if [ -n "${sccache_dir}" ]; then
                export RUSTC_WRAPPER=sccache
                export SCCACHE_DIR="${sccache_dir}"
                export SCCACHE_CACHE_SIZE=20G
            fi
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
