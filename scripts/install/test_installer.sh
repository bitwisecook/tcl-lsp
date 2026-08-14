#!/usr/bin/env bash
# Deterministic unit coverage for install.sh's platform, migration, and UI
# decisions. No network access or package installation is performed.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/tcl-lsp-installer-test.XXXXXX")"
trap 'rm -rf -- "$test_root"' EXIT

export HOME="$test_root/home"
mkdir -p "$HOME" "$test_root/work" "$test_root/path-old" "$test_root/path-new"
export PATH="$test_root/path-new:$test_root/path-old:/usr/bin:/bin"
export TCL_LSP_PREFIX="$test_root/path-new"
export TCL_LSP_INSTALLER_SOURCE_ONLY=1

# shellcheck source=install.sh
source "$repo_root/scripts/install/install.sh"
WORKDIR="$test_root/work"

pass_count=0
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { pass_count=$((pass_count + 1)); printf 'ok %d - %s\n' "$pass_count" "$*"; }
assert_eq() { [ "$1" = "$2" ] || fail "$3 (got '$1', expected '$2')"; }
assert_file() { [ -e "$1" ] || fail "$2 (missing $1)"; }
assert_absent() { [ ! -e "$1" ] || fail "$2 (still present: $1)"; }

make_legacy_zipapp() {
    printf '#!/usr/bin/env python3\nPK\003\004payload shared/_build_info.py PK\005\006\n' > "$1"
    chmod +x "$1"
}

# Published target mapping is kept pure so every supported and unsupported
# branch can be tested on any build host.
assert_eq "$(host_triple_for Darwin arm64)" "aarch64-apple-darwin" "Darwin Arm64 mapping"
assert_eq "$(host_triple_for Linux x86_64)" "x86_64-unknown-linux-gnu" "Linux x86-64 mapping"
assert_eq "$(host_triple_for Linux riscv64)" "riscv64gc-unknown-linux-gnu" "Linux RISC-V mapping"
assert_eq "$(host_triple_for MINGW64_NT x86_64)" "x86_64-pc-windows-msvc" "Windows x86-64 mapping"
if host_triple_for FreeBSD amd64 >/dev/null 2>&1; then fail "FreeBSD unexpectedly has a release mapping"; fi
if host_triple_for Linux ppc64le >/dev/null 2>&1; then fail "Linux ppc64le unexpectedly has a release mapping"; fi
pass "published and unsupported platform mappings"

assert_eq "$(select_release_version '' v2.1.20)" v2.1.20 "stamped pre-release selection"
assert_eq "$(select_release_version v2.1.18 v2.1.20)" v2.1.18 "explicit release override"
assert_eq "$(select_release_version '' dev)" latest "checkout default channel"
pass "release-stamped version selection"

platform_error="$(
    (
        host_triple() { return 1; }
        WANT_TCL=1; WANT_F5=0; WANT_MCP=0; WANT_SKILLS=0
        preflight_native_platform
    ) 2>&1
)" && fail "unsupported native platform preflight unexpectedly succeeded"
case "$platform_error" in
    *"no prebuilt tcl-lsp binaries"*"kcs-howto-build-and-install-on-an-unsupported-platform.md"*) : ;;
    *) fail "unsupported platform error omitted the reason or source-build guide" ;;
esac
pass "unsupported platform fails with source-build guidance"

# Auto UI preference is fzf, then dialog, then whiptail, and it becomes plain
# when no controlling terminal is available.
for tool in fzf dialog whiptail; do
    printf '#!/bin/sh\nexit 0\n' > "$test_root/path-new/$tool"
    chmod +x "$test_root/path-new/$tool"
done
TTY_PROBED=1; TTY_IN=stdin; TTY_OUT=stderr; TCL_LSP_UI=auto
detect_ui_backend
assert_eq "$UI_BACKEND" fzf "auto UI preference"
rm -f "$test_root/path-new/fzf"
detect_ui_backend
assert_eq "$UI_BACKEND" dialog "dialog UI fallback"
TTY_IN=none
detect_ui_backend
assert_eq "$UI_BACKEND" plain "headless UI fallback"
pass "optional installer UI selection"

# Recognise only the retired project zipapp shape, not an arbitrary Python
# script or ZIP file.
make_legacy_zipapp "$test_root/path-old/tcl"
printf '#!/usr/bin/env python3\nprint("mine")\n' > "$test_root/path-old/unrelated"
chmod +x "$test_root/path-old/unrelated"
looks_like_legacy_zipapp "$test_root/path-old/tcl" || fail "legacy zipapp was not recognised"
if looks_like_legacy_zipapp "$test_root/path-old/unrelated"; then fail "unrelated Python file was claimed"; fi
pass "legacy ownership recognition"

# Migration removes only positively identified historical names. It must leave
# unrelated files and the newly installed native binary alone.
make_legacy_zipapp "$test_root/path-old/f5"
make_legacy_zipapp "$test_root/path-old/tcl-explorer"
make_legacy_zipapp "$test_root/path-old/tcl-explorer-gui"
make_legacy_zipapp "$test_root/path-old/tcl-lsp-mcp-server.pyz"
mkdir -p "$HOME/.claude"
make_legacy_zipapp "$HOME/.claude/tcl-ai.pyz"
printf '#!/bin/sh\n# Unified Tcl toolchain CLI\n' > "$test_root/path-new/tcl"
chmod +x "$test_root/path-new/tcl"
WANT_TCL=1; WANT_F5=1; WANT_MCP=1; WANT_SKILLS=1
MCP_PREFIX_OVERRIDE="$test_root/path-new"
cleanup_legacy_python_installs
for old in tcl f5 tcl-explorer tcl-explorer-gui tcl-lsp-mcp-server.pyz; do
    assert_absent "$test_root/path-old/$old" "legacy $old cleanup"
done
assert_absent "$HOME/.claude/tcl-ai.pyz" "legacy Claude AI zipapp cleanup"
assert_file "$test_root/path-new/tcl" "native tcl preservation"
assert_file "$test_root/path-old/unrelated" "unrelated Python file preservation"
pass "safe Python-era migration cleanup"

# When Codex is detected by its config directory but its CLI is unavailable,
# replace exactly the old MCP table and preserve unrelated TOML settings.
mkdir -p "$HOME/.codex"
cat > "$HOME/.codex/config.toml" <<'EOF'
model = "example"

[mcp_servers.tcl_lsp]
command = "python3"
args = ["/old/tcl-lsp-mcp-server.pyz"]

[mcp_servers.other]
command = "/keep/me"
EOF
MCP_PATH="$test_root/path-new/tcl-mcp"
printf '#!/bin/sh\n' > "$MCP_PATH"
chmod +x "$MCP_PATH"
register_mcp_codex
grep -qF "command = \"$MCP_PATH\"" "$HOME/.codex/config.toml" \
    || fail "Codex native MCP command was not written"
grep -qF 'command = "/keep/me"' "$HOME/.codex/config.toml" \
    || fail "unrelated Codex MCP table was not preserved"
if grep -qF '.pyz' "$HOME/.codex/config.toml"; then fail "old Codex zipapp command survived"; fi
pass "Codex MCP migration without CLI"

# With the Codex CLI present, use its supported remove/add interface so config
# format changes remain Codex's responsibility.
CODEX_LOG="$test_root/codex.log"; export CODEX_LOG
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >> "$CODEX_LOG"\n' > "$test_root/path-new/codex"
chmod +x "$test_root/path-new/codex"
register_mcp_codex
grep -qF 'mcp remove tcl_lsp' "$CODEX_LOG" || fail "Codex old registration was not removed"
grep -qF "mcp add tcl_lsp -- $MCP_PATH" "$CODEX_LOG" || fail "Codex native registration was not added"
pass "Codex MCP migration through CLI"

if grep -qE 'plan_python_if_needed|install_python|install_mcp_zipapp|MCP_NATIVE' \
    "$repo_root/scripts/install/install.sh"; then
    fail "retired Python installer path remains"
fi
pass "native-only installer path"

printf '1..%d\n' "$pass_count"
