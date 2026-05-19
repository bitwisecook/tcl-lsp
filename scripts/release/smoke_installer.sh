#!/usr/bin/env bash
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
# shellcheck disable=SC2086
if curl -fsSL "$INSTALLER_URL" \
        | env $installer_env TCL_LSP_PREFIX="$PREFIX" TCL_LSP_ASSUME_NO=1 sh \
        > /tmp/smoke-installer.log 2>&1; then
    pass "installer ran cleanly"
else
    fail "installer exited non-zero (see /tmp/smoke-installer.log)"
    tail -10 /tmp/smoke-installer.log | sed 's/^/    /'
    exit 1
fi

[ -d "$PREFIX" ] || { fail "$PREFIX does not exist after install"; exit 1; }

# ---------------------------------------------------------------- 1. hashes

hdr "Hashing installed pyz files against $SUMS_URL"
if ! sums=$(curl -fsSL "$SUMS_URL" 2>/dev/null); then
    fail "could not fetch SHA256SUMS"
else
    found=0
    for f in "$PREFIX"/*.pyz; do
        [ -e "$f" ] || continue
        found=$((found + 1))
        base=$(basename "$f")
        # The installer renames release artefacts to bare names
        # (tcl-lsp-mcp-server-X.Y.Z.pyz -> tcl-lsp-mcp-server.pyz).
        # Match against the versioned SHA256SUMS entry that shares
        # the bare prefix.
        stem="${base%.pyz}"
        expected_sum=$(printf '%s\n' "$sums" \
            | awk -v stem="$stem" -v ver="$expected" \
                '$2 == stem "-" ver ".pyz" {print $1; found=1}
                 END { if (!found) exit 1 }') || true
        if [ -z "$expected_sum" ]; then
            note "WARN: $base — no SHA256SUMS entry for ${stem}-${expected}.pyz"
            continue
        fi
        actual_sum=$(sha256sum "$f" | awk '{print $1}')
        if [ "$expected_sum" = "$actual_sum" ]; then
            pass "$base sha256 matches SHA256SUMS"
        else
            fail "$base sha mismatch:
        expected $expected_sum
        actual   $actual_sum"
        fi
    done
    [ "$found" -gt 0 ] || fail "no .pyz files found under $PREFIX"
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

hdr "MCP server zipapp"
mcp="$PREFIX/tcl-lsp-mcp-server.pyz"
if [ ! -f "$mcp" ]; then
    fail "MCP pyz missing at $mcp"
else
    # The MCP CLI's --version output is empty on purpose; the
    # version banner shows on --help instead.
    if helpout=$("$mcp" --help 2>&1) && [ -n "$helpout" ]; then
        case "$helpout" in
            *"$expected"*) pass "MCP --help banner mentions $expected" ;;
            *)             fail "MCP --help did not include $expected
        got: $(printf '%s' "$helpout" | head -1)" ;;
        esac
    else
        fail "MCP --help failed"
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
