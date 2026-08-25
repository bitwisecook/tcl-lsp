#!/usr/bin/env bash
# Deterministic unit coverage for install.sh's platform, migration, and UI
# decisions. No network access or package installation is performed.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/tcl-lsp-installer-test.XXXXXX")"
trap 'rm -rf -- "$test_root"' EXIT

export HOME="$test_root/home"
export XDG_CONFIG_HOME="$HOME/.config"
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

# Migration is the final successful-plan step. If a selected native component
# fails, the working Python-era artefact must remain; a successful plan still
# removes it even when that component was declined.
defer_root="$test_root/deferred-cleanup"
mkdir -p "$defer_root/home" "$defer_root/bin" "$defer_root/work"
make_legacy_zipapp "$defer_root/bin/tcl"
if ! (
    export HOME="$defer_root/home"
    export PATH="$defer_root/bin:/usr/bin:/bin"
    WORKDIR="$defer_root/work"
    ONLY=none; WANT_MCP=1; WANT_SKILLS=0
    needs_prefix() { return 1; }
    install_downloader() { return 0; }
    install_ai_integrations() { return 1; }
    if execute_install_plan; then exit 20; fi
    [ -e "$defer_root/bin/tcl" ]
); then
    fail "failed native plan removed the working legacy CLI"
fi
if ! (
    export HOME="$defer_root/home"
    export PATH="$defer_root/bin:/usr/bin:/bin"
    WORKDIR="$defer_root/work"
    ONLY=none; WANT_MCP=0; WANT_SKILLS=0
    needs_prefix() { return 1; }
    install_downloader() { return 0; }
    install_ai_integrations() { return 0; }
    execute_install_plan
    [ ! -e "$defer_root/bin/tcl" ]
); then
    fail "successful native plan did not clean the legacy CLI"
fi
pass "legacy deletion waits for successful native installation"

# Migration removes the main installer's complete discoverable footprint. It
# must leave unrelated files and the newly installed native binary alone.
make_legacy_zipapp "$test_root/path-old/f5"
make_legacy_zipapp "$test_root/path-old/tcl-explorer"
make_legacy_zipapp "$test_root/path-old/tcl-explorer-gui"
make_legacy_zipapp "$test_root/path-old/tcl-lsp-mcp-server.pyz"
mkdir -p "$test_root/custom-prefix" "$test_root/mcp-custom"
make_legacy_zipapp "$test_root/custom-prefix/tcl-lsp"
make_legacy_zipapp "$test_root/custom-prefix/f5-custom"
make_legacy_zipapp "$test_root/mcp-custom/tcl-lsp-mcp-server.pyz"
cat > "$HOME/.bashrc" <<EOF
# Added by tcl-lsp installer
export PATH="$test_root/custom-prefix:\$PATH"
EOF
cat > "$HOME/.zshrc" <<EOF
# Added by tcl-lsp installer
export PATH="$test_root/path-new:\$PATH"
EOF
mkdir -p "$HOME/.claude"
make_legacy_zipapp "$HOME/.claude/tcl-ai.pyz"
mkdir -p "$HOME/.claude/prompts" "$HOME/.claude/skills/tcl-fix" \
    "$HOME/.claude/skills/f5-query" \
    "$HOME/.claude/skills/unrelated" \
    "$HOME/.local/share/bash-completion/completions"
printf 'Tcl system prompt\n' > "$HOME/.claude/prompts/tcl_system.md"
printf 'python3 .claude/tcl-ai.pyz fix\n' > "$HOME/.claude/skills/tcl-fix/SKILL.md"
printf '%s\n' '---' 'name: f5-query' '---' > "$HOME/.claude/skills/f5-query/SKILL.md"
printf 'keep me\n' > "$HOME/.claude/skills/unrelated/SKILL.md"
printf '_ARGCOMPLETE=1 tcl.pyz\n' > "$HOME/.local/share/bash-completion/completions/tcl"
printf 'native completion\n' > "$HOME/.local/share/bash-completion/completions/f5"
mkdir -p "$HOME/.codex"
cat > "$HOME/.codex/config.toml" <<EOF
model = "example"

[mcp_servers.tcl_lsp]
command = "python3"
args = ["$test_root/mcp-custom/tcl-lsp-mcp-server.pyz"]

[mcp_servers.other]
command = "/keep/me"
EOF
CLAUDE_LOG="$test_root/legacy-claude.log"; export CLAUDE_LOG
cat > "$test_root/path-new/claude" <<EOF
#!/bin/sh
printf '%s\n' "\$*" >> "$CLAUDE_LOG"
if [ "\$1 \$2" = "mcp list" ]; then
    printf 'tcl-lsp: python3 $test_root/mcp-custom/tcl-lsp-mcp-server.pyz\n'
fi
EOF
chmod +x "$test_root/path-new/claude"
printf '#!/bin/sh\n# Unified Tcl toolchain CLI\n' > "$test_root/path-new/tcl"
chmod +x "$test_root/path-new/tcl"
PROJECT_ROOT="$test_root/project"
mkdir -p "$PROJECT_ROOT"
WANT_TCL=0; WANT_F5=0; WANT_MCP=0; WANT_SKILLS=0
MCP_PREFIX_OVERRIDE="$test_root/path-new"
cleanup_legacy_python_installs
cleanup_legacy_claude_bundle
cleanup_legacy_path_entries
for old in tcl f5 tcl-explorer tcl-explorer-gui tcl-lsp-mcp-server.pyz; do
    assert_absent "$test_root/path-old/$old" "legacy $old cleanup"
done
assert_absent "$test_root/custom-prefix/tcl-lsp" "suffixed legacy tcl cleanup"
assert_absent "$test_root/custom-prefix/f5-custom" "suffixed legacy f5 cleanup"
assert_absent "$test_root/mcp-custom/tcl-lsp-mcp-server.pyz" "registered MCP path cleanup"
assert_absent "$HOME/.claude/tcl-ai.pyz" "legacy Claude AI zipapp cleanup"
assert_absent "$HOME/.claude/prompts/tcl_system.md" "legacy Claude prompt cleanup"
assert_absent "$HOME/.claude/skills/tcl-fix" "legacy Claude skill cleanup"
assert_absent "$HOME/.claude/skills/f5-query" "standalone legacy Claude skill cleanup"
assert_file "$HOME/.claude/skills/unrelated/SKILL.md" "unrelated Claude skill preservation"
if ! compgen -G "$HOME/.claude/.tcl-lsp-python-backup-*/tcl-ai.pyz" >/dev/null; then
    fail "legacy Claude bundle backup was not created"
fi
assert_absent "$HOME/.local/share/bash-completion/completions/tcl" \
    "legacy argcomplete cleanup"
assert_file "$HOME/.local/share/bash-completion/completions/f5" \
    "native completion preservation"
grep -qF 'mcp remove -s local tcl-lsp' "$CLAUDE_LOG" \
    || fail "legacy Claude MCP registration was not removed"
if grep -qF '[mcp_servers.tcl_lsp]' "$HOME/.codex/config.toml"; then
    fail "legacy Codex MCP registration survived"
fi
grep -qF 'command = "/keep/me"' "$HOME/.codex/config.toml" \
    || fail "unrelated Codex MCP registration was not preserved"
if grep -qF '# Added by tcl-lsp installer' "$HOME/.bashrc"; then
    fail "stale installer PATH entry survived"
fi
if ! compgen -G "$HOME/.bashrc.bak.*" >/dev/null; then
    fail "shell startup backup was not created"
fi
grep -qF '# Added by tcl-lsp installer' "$HOME/.zshrc" \
    || fail "active native installer PATH entry was removed"
assert_file "$test_root/path-new/tcl" "native tcl preservation"
assert_file "$test_root/path-old/unrelated" "unrelated Python file preservation"
pass "complete safe main-installer migration cleanup"

# A colliding prompt filename is not sufficient evidence that the main-branch
# installer owns it. Preserve it when no legacy zipapp or skill marker exists.
mkdir -p "$HOME/.claude/prompts"
printf 'independent prompt manifest\n' > "$HOME/.claude/prompts/manifest.json"
cleanup_legacy_claude_bundle
assert_file "$HOME/.claude/prompts/manifest.json" \
    "unowned Claude prompt preservation"
rm -f "$HOME/.claude/prompts/manifest.json"
pass "Claude prompt cleanup requires a legacy bundle marker"

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

# Detection recognises CLI/config footprints for every supported harness.
PROJECT_ROOT="$test_root/project"
mkdir -p "$PROJECT_ROOT/.bobbit" "$HOME/.gemini" "$HOME/.copilot" \
    "$HOME/.config/opencode" "$HOME/.hermes" "$HOME/.config/goose"
AI_DETECTED=0
detect_ai_clients
assert_eq "$HAS_CLAUDE:$HAS_CODEX:$HAS_GEMINI:$HAS_COPILOT" "1:1:1:1" \
    "primary harness detection"
assert_eq "$HAS_OPENCODE:$HAS_HERMES:$HAS_GOOSE:$HAS_BOBBIT" "1:1:1:1" \
    "config-driven harness detection"
pass "supported harness detection"

# Harness selection is independent. A harness with project-owned files defaults
# to project scope; a detected harness without them gets user scope. Bobbit is
# project-only because it deliberately discovers a project-root .mcp.json.
mkdir -p "$PROJECT_ROOT/.claude" "$PROJECT_ROOT/.bobbit"
HAS_CLAUDE=1; HAS_CODEX=1; HAS_GEMINI=0; HAS_COPILOT=0
HAS_OPENCODE=0; HAS_HERMES=0; HAS_GOOSE=0; HAS_BOBBIT=1
TCL_LSP_ASSUME_YES=1
choose_ai_components
assert_eq "$INSTALL_MCP_CLAUDE:$MCP_SCOPE_CLAUDE" "1:project" "Claude project scope selection"
assert_eq "$INSTALL_MCP_CODEX:$MCP_SCOPE_CODEX" "1:user" "Codex user scope selection"
assert_eq "$INSTALL_MCP_BOBBIT:$MCP_SCOPE_BOBBIT" "1:project" "Bobbit project scope selection"
assert_eq "$WANT_MCP:$WANT_SKILLS" "1:1" "per-harness MCP and Claude skills plan"
unset TCL_LSP_ASSUME_YES
pass "per-harness registration and project-aware scope planning"

# Claude migration must remove the historical implicit local registration and
# replace the selected scope explicitly. `mcp list` prints names with a colon,
# so migration must not depend on parsing that display format.
CLAUDE_LOG="$test_root/claude.log"; export CLAUDE_LOG
cat > "$test_root/path-new/claude" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >> "$CLAUDE_LOG"
if [ "$1 $2" = "mcp list" ]; then
    printf 'tcl-lsp: python3 /old/tcl-lsp-mcp-server.pyz\n'
fi
EOF
chmod +x "$test_root/path-new/claude"
MCP_SCOPE_CLAUDE=user
register_mcp_claude
grep -qF 'mcp remove -s local tcl-lsp' "$CLAUDE_LOG" \
    || fail "Claude stale local registration was not removed"
grep -qF 'mcp remove -s user tcl-lsp' "$CLAUDE_LOG" \
    || fail "Claude selected user registration was not replaced"
grep -qF "mcp add -s user tcl-lsp -- $MCP_PATH" "$CLAUDE_LOG" \
    || fail "Claude native user registration was not added explicitly"
pass "Claude delete/add migration uses an explicit scope"

# Detection through ~/.claude is enough to offer integration, but the CLI can
# still be absent. Keep the manual-registration warning nonfatal so remaining
# harnesses and cleanup continue.
mv "$test_root/path-new/claude" "$test_root/path-new/claude.disabled"
claude_config_only_output="$(register_mcp_claude 2>&1)" \
    || fail "config-only Claude registration was fatal"
case "$claude_config_only_output" in
    *"CLI is not on PATH"*"Register manually"*) : ;;
    *) fail "config-only Claude warning omitted manual registration guidance" ;;
esac
mv "$test_root/path-new/claude.disabled" "$test_root/path-new/claude"
pass "config-only Claude registration is nonfatal"

# Gemini has native scope-aware commands too; use those instead of editing its
# settings format when the CLI is present.
GEMINI_LOG="$test_root/gemini.log"; export GEMINI_LOG
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >> "$GEMINI_LOG"\n' > "$test_root/path-new/gemini"
chmod +x "$test_root/path-new/gemini"
MCP_SCOPE_GEMINI=project
register_mcp_gemini
grep -qF 'mcp remove -s project tcl-lsp' "$GEMINI_LOG" \
    || fail "Gemini project registration was not removed before replacement"
grep -qF "mcp add -s project tcl-lsp $MCP_PATH" "$GEMINI_LOG" \
    || fail "Gemini native project registration was not added"
pass "Gemini delete/add migration uses an explicit scope"

# Config-only harnesses can be bootstrapped without Python, Node.js, jq, or yq
# when their config file does not exist yet.
json_cfg="$test_root/config/new/.mcp.json"
yaml_cfg="$test_root/config/hermes/config.yaml"
write_json_mcp_config "$json_cfg" standard
write_yaml_mcp_config "$yaml_cfg" hermes
grep -qF '"tcl-lsp"' "$json_cfg" || fail "new standard MCP JSON omitted tcl-lsp"
grep -qF "\"command\": \"$MCP_PATH\"" "$json_cfg" || fail "new standard MCP JSON omitted native command"
grep -qF '  tcl-lsp:' "$yaml_cfg" || fail "new Hermes YAML omitted tcl-lsp"
grep -qF "    command: \"$MCP_PATH\"" "$yaml_cfg" || fail "new Hermes YAML omitted native command"
pass "native-only JSON and YAML MCP config bootstrap"

# The no-dependency YAML updater replaces only our child map and preserves
# unrelated harness configuration.
cat > "$yaml_cfg" <<'EOF'
model: example
mcp_servers:
  tcl-lsp:
    command: python3
    args: [/old/tcl-lsp-mcp-server.pyz]
  other:
    command: /keep/me
theme: dark
EOF
write_yaml_mcp_config "$yaml_cfg" hermes
grep -qF "    command: \"$MCP_PATH\"" "$yaml_cfg" \
    || fail "Hermes native MCP command was not replaced"
grep -qF '    command: /keep/me' "$yaml_cfg" \
    || fail "unrelated Hermes MCP entry was not preserved"
grep -qF 'theme: dark' "$yaml_cfg" || fail "unrelated Hermes root setting was not preserved"
if grep -qF '.pyz' "$yaml_cfg"; then fail "old Hermes zipapp command survived"; fi
pass "Hermes YAML migration preserves unrelated configuration"

if grep -qE 'plan_python_if_needed|install_python|install_mcp_zipapp|MCP_NATIVE' \
    "$repo_root/scripts/install/install.sh"; then
    fail "retired Python installer path remains"
fi
pass "native-only installer path"

printf '1..%d\n' "$pass_count"
