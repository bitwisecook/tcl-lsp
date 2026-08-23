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

# Offline regression for smoke_installer.sh's affirmative, isolated release
# verification path. The fixture installer refuses to run unless the harness
# selected MCP/skills without exposing the caller's real HOME or Claude CLI.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/tcl-lsp-smoke-installer-test.XXXXXX")"
trap 'rm -rf -- "$test_root"' EXIT

installer="$test_root/install.sh"
sums="$test_root/SHA256SUMS"

cat > "$installer" <<'FIXTURE'
#!/bin/sh
set -eu

[ "${TCL_LSP_ASSUME_YES:-}" = 1 ] || {
    echo "fixture: smoke harness did not select affirmative choices" >&2
    exit 31
}
[ "${TCL_LSP_ASSUME_NO:-0}" != 1 ] || {
    echo "fixture: smoke harness still opts out" >&2
    exit 32
}
case "$HOME" in
    "$SMOKE_FIXTURE_PARENT"/tcl-lsp-release-smoke.*/home) ;;
    *) echo "fixture: HOME is not isolated: $HOME" >&2; exit 33 ;;
esac
case "$XDG_CONFIG_HOME:$ZDOTDIR" in
    "$HOME/.config:$HOME") ;;
    *) echo "fixture: config and shell homes are not isolated" >&2; exit 35 ;;
esac
case "$(command -v claude)" in
    "$SMOKE_FIXTURE_PARENT"/tcl-lsp-release-smoke.*/bin/claude) ;;
    *) echo "fixture: real Claude CLI is visible" >&2; exit 34 ;;
esac

prefix="$TCL_LSP_PREFIX"
version="${TCL_LSP_VERSION#v}"
mkdir -p "$prefix" "$HOME/.claude/skills/tcl-fixture-one" \
    "$HOME/.claude/skills/tcl-fixture-two"

for name in tcl f5; do
    cat > "$prefix/$name" <<CLI
#!/bin/sh
case "\${1:-}" in
    --version) echo "$name $version" ;;
    --help) echo "usage: $name" ;;
    *) exit 2 ;;
esac
CLI
    chmod +x "$prefix/$name"
done

cat > "$prefix/tcl-mcp" <<MCP
#!/bin/sh
read -r request
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"fixture","version":"$version+fixture"}}}'
MCP
chmod +x "$prefix/tcl-mcp"

: > "$SMOKE_FIXTURE_SUMS"
for name in tcl f5 tcl-mcp; do
    if command -v sha256sum >/dev/null 2>&1; then
        digest="$(sha256sum "$prefix/$name" | awk '{print $1}')"
    else
        digest="$(shasum -a 256 "$prefix/$name" | awk '{print $1}')"
    fi
    printf '%s  %s-fixture\n' "$digest" "$name" >> "$SMOKE_FIXTURE_SUMS"
done
FIXTURE
chmod +x "$installer"

output="$({
    TMPDIR="$test_root" \
    SMOKE_FIXTURE_PARENT="$test_root" \
    SMOKE_FIXTURE_SUMS="$sums" \
    TCL_LSP_INSTALLER_URL="file://$installer" \
    TCL_LSP_SUMS_URL="file://$sums" \
    MIN_SKILLS=2 \
        bash "$repo_root/scripts/release/smoke_installer.sh" v9.9.9
} 2>&1)"

case "$output" in
    *"[ok]   MCP server answers initialize and reports 9.9.9"*) ;;
    *) printf '%s\n' "$output" >&2; echo "missing MCP protocol/version success" >&2; exit 1 ;;
esac
case "$output" in
    *"[ok]   2 skills under"*) ;;
    *) printf '%s\n' "$output" >&2; echo "missing isolated skills success" >&2; exit 1 ;;
esac
case "$output" in
    *"Installer smoke test passed for v9.9.9."*) ;;
    *) printf '%s\n' "$output" >&2; echo "fixture smoke did not pass" >&2; exit 1 ;;
esac

if find "$test_root" -mindepth 1 -maxdepth 1 -name 'tcl-lsp-release-smoke.*' | grep -q .; then
    echo "smoke harness left its isolated root behind" >&2
    exit 1
fi

echo "ok - release installer smoke selects MCP/skills in an isolated environment"
