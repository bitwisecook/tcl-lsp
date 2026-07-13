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

# smoke_installer.sh — post-tag verification that the published
# install.sh actually lands working, version-correct binaries.
#
# Usage:   smoke_installer.sh <tag>            (e.g. v1.10.4)
#          smoke_installer.sh                  (uses `git describe`)
#
# Env overrides:
#   TCL_LSP_PREFIX      install destination (default: /tmp/verify-bin)
#   TCL_LSP_OS          forced detection (e.g. debian) — set if the
#                       installer can't read /etc/os-release in your
#                       environment
#   KEEP_PREFIX=1       skip the final rm of $TCL_LSP_PREFIX (useful
#                       when poking at a failure)
#   MIN_SKILLS          floor on count of Claude skills the installer
#                       should leave behind (default: 22)
#
# Exit non-zero on any failed check. Pairs with the post-tag block in
# .claude/skills/release/SKILL.md.

set -uo pipefail

OWNER_REPO="bitwisecook/tcl-lsp"

tag="${1:-}"
if [ -z "$tag" ]; then
    tag=$(git describe --tags --abbrev=0 2>/dev/null) \
        || { echo "smoke_installer: pass a tag, or run from a repo
                   with at least one tag" >&2; exit 2; }
fi
expected="${tag#v}"

PREFIX="${TCL_LSP_PREFIX:-/tmp/verify-bin}"
MIN_SKILLS="${MIN_SKILLS:-22}"
SUMS_URL="https://github.com/$OWNER_REPO/releases/download/$tag/SHA256SUMS"
INSTALLER_URL="https://github.com/$OWNER_REPO/releases/download/$tag/install.sh"

FAILED=0
note()  { printf '  %s\n'   "$*"; }
pass()  { printf '  [ok]   %s\n' "$*"; }
fail()  { printf '  [fail] %s\n' "$*"; FAILED=1; }
hdr()   { printf '\n== %s ==\n' "$*"; }

cleanup() {
    if [ "${KEEP_PREFIX:-}" != 1 ]; then
        rm -rf "$PREFIX"
    fi
}
trap cleanup EXIT

# ---------------------------------------------------------------- run installer

hdr "Installing $tag from $INSTALLER_URL"
rm -rf "$PREFIX"
installer_env=""
if [ -n "${TCL_LSP_OS:-}" ]; then
    installer_env="TCL_LSP_OS=$TCL_LSP_OS "
fi
# TCL_LSP_VERSION pins the installer to the tag we are verifying. Without it the
# installer resolves its default channel — the latest *stable* release — so the
# whole post-tag check ran against a different version than the one being cut,
# and said nothing about it. That is doubly wrong for a pre-release, which is
# never "latest": v2.1.8's smoke run installed v1.11.4 and failed on a 404.
# shellcheck disable=SC2086
if curl -fsSL "$INSTALLER_URL" \
        | env $installer_env TCL_LSP_VERSION="$tag" TCL_LSP_PREFIX="$PREFIX" \
              TCL_LSP_ASSUME_NO=1 sh \
        > /tmp/smoke-installer.log 2>&1; then
    pass "installer ran cleanly"
else
    fail "installer exited non-zero (see /tmp/smoke-installer.log)"
    tail -10 /tmp/smoke-installer.log | sed 's/^/    /'
    exit 1
fi

[ -d "$PREFIX" ] || { fail "$PREFIX does not exist after install"; exit 1; }

# ---------------------------------------------------------------- 1. hashes

hdr "Hashing installed binaries against $SUMS_URL"
if ! sums=$(curl -fsSL "$SUMS_URL" 2>/dev/null); then
    fail "could not fetch SHA256SUMS"
else
    found=0
    # The installer downloads prebuilt native per-triple binaries
    # (tcl-<triple>, f5-query-<triple>, tcl-mcp-<triple>) and renames them to
    # bare names (tcl, f5, tcl-mcp); on platforms without a native MCP build it
    # falls back to the tcl-lsp-mcp-server.pyz zipapp. Rather than reconstruct
    # the host triple, match each installed file's hash against any entry in
    # SHA256SUMS.
    for f in "$PREFIX/tcl" "$PREFIX/f5" "$PREFIX/tcl-mcp" "$PREFIX/tcl-lsp-mcp-server.pyz"; do
        [ -e "$f" ] || continue
        found=$((found + 1))
        base=$(basename "$f")
        actual_sum=$(sha256sum "$f" | awk '{print $1}')
        if printf '%s\n' "$sums" | awk -v h="$actual_sum" '$1 == h {ok=1} END{exit !ok}'; then
            pass "$base sha256 present in SHA256SUMS"
        else
            fail "$base sha256 $actual_sum not found in SHA256SUMS"
        fi
    done
    [ "$found" -gt 0 ] || fail "no installed binaries found under $PREFIX"
fi

# ---------------------------------------------------------------- 2./3. CLIs

hdr "tcl / f5 binaries"
for bin in tcl f5; do
    if [ ! -x "$PREFIX/$bin" ]; then
        fail "$bin: missing or not executable at $PREFIX/$bin"
        continue
    fi
    if ver=$("$PREFIX/$bin" --version 2>&1); then
        case "$ver" in
            *"$expected"*) pass "$bin --version reports $expected" ;;
            *)             fail "$bin --version did not include $expected
        got: $ver" ;;
        esac
    else
        fail "$bin --version exited non-zero
    out: $ver"
    fi
    if helpout=$("$PREFIX/$bin" --help 2>&1) && [ -n "$helpout" ]; then
        pass "$bin --help prints usage"
    else
        fail "$bin --help failed (exit=$?, stdout-empty=$([ -z "$helpout" ] && echo yes || echo no))"
    fi
done

# ---------------------------------------------------------------- 4. MCP

hdr "MCP server"
# The installer prefers the native tcl-mcp binary and falls back to the
# tcl-lsp-mcp-server.pyz zipapp on platforms without a native build.
mcp=""
if [ -x "$PREFIX/tcl-mcp" ]; then
    mcp="$PREFIX/tcl-mcp"
elif [ -f "$PREFIX/tcl-lsp-mcp-server.pyz" ]; then
    mcp="$PREFIX/tcl-lsp-mcp-server.pyz"
fi
if [ -z "$mcp" ]; then
    fail "MCP server missing (looked for $PREFIX/tcl-mcp and $PREFIX/tcl-lsp-mcp-server.pyz)"
else
    # Speak MCP to it rather than asking it for a banner. The native 2.x server
    # takes no flags at all — `--help` just starts the server, which then dies on
    # the closed stdin — so the old banner check could only ever fail once the
    # smoke test was pinned to a 2.x tag. Driving one `initialize` request is a
    # stronger check anyway: it proves the server runs, speaks the protocol, and
    # reports the version we just released, rather than that it can print text.
    init='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}'
    if reply=$(printf '%s\n' "$init" | "$mcp" 2>/dev/null | head -1) && [ -n "$reply" ]; then
        case "$reply" in
            *'"serverInfo"'*"\"version\":\"$expected\""*)
                pass "MCP server answers initialize and reports $expected" ;;
            *'"serverInfo"'*)
                fail "MCP server answered initialize but not with $expected
        got: $(printf '%s' "$reply" | head -c 120)" ;;
            *)
                fail "MCP server did not answer initialize
        got: $(printf '%s' "$reply" | head -c 120)" ;;
        esac
    else
        fail "MCP server did not respond to an initialize request"
    fi
fi

# ---------------------------------------------------------------- 5. Skills

hdr "Claude skills"
skills_dir="$HOME/.claude/skills"
if [ -d "$skills_dir" ]; then
    n=$(find "$skills_dir" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
    if [ "$n" -ge "$MIN_SKILLS" ]; then
        pass "$n skills under $skills_dir (>= $MIN_SKILLS)"
    else
        fail "only $n skills under $skills_dir (expected >= $MIN_SKILLS)"
    fi
else
    note "WARN: $skills_dir does not exist — skipping skill count check
        (set MIN_SKILLS=0 to silence, or pass --no-skills via the installer)"
fi

# ---------------------------------------------------------------- summary

hdr "Summary"
if [ "$FAILED" -eq 0 ]; then
    echo "  Installer smoke test passed for $tag."
    exit 0
else
    echo "  Installer smoke test FAILED for $tag — see [fail] lines above."
    exit 1
fi
