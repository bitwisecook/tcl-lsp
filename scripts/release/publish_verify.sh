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

# publish_verify.sh — sanity-check publishing readiness for every editor
# marketplace target without shipping. Intended to be run before a release
# is cut, so any missing token / tool / remote shows up early rather than
# during `make publish-all`.
#
# Each section prints one of:
#
#   [ok]    everything in place
#   [warn]  recoverable (e.g. token not set; will block publish-* but not
#           a no-op gate before tagging)
#   [fail]  unrecoverable (tool missing, repo unreachable) — exit code 1
#
# Sections are independent; we run all of them and only fail at the end.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
EXT_DIR="$ROOT/editors/vscode"
NODE_BIN="$EXT_DIR/node_modules/.bin"
JB_DIR="$ROOT/editors/jetbrains"

VSCE="$NODE_BIN/vsce"

VSCE_PUBLISHER="bitwisecook"
REPO="bitwisecook/tcl-lsp"
ZED_UPSTREAM="zed-industries/extensions"

RC=0
fail() { RC=1; }

hdr()  { printf '\n== %s ==\n' "$1"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
warn() { printf '  [warn] %s\n' "$*"; }
err()  { printf '  [fail] %s\n' "$*"; fail; }

# ---------------------------------------------------------------- VS Code

hdr "VS Code Marketplace (publish-vsix)"

if [ -x "$VSCE" ]; then
    VSCE_VERSION="$("$VSCE" --version 2>/dev/null || echo unknown)"
    ok "vsce installed (version $VSCE_VERSION)"
else
    err "vsce not installed — run 'make npm-env' (or 'cd $EXT_DIR && npm install')"
fi

if [ -n "${VSCE_PAT:-}" ]; then
    ok "VSCE_PAT env var is set (non-interactive publish ready)"
elif [ -x "$VSCE" ]; then
    if "$VSCE" verify-pat "$VSCE_PUBLISHER" >/dev/null 2>&1; then
        ok "vsce has a cached PAT for publisher '$VSCE_PUBLISHER'"
    else
        warn "no cached vsce PAT for '$VSCE_PUBLISHER'. Either:"
        warn "  - set VSCE_PAT in your env (from https://dev.azure.com), or"
        warn "  - run 'cd $EXT_DIR && ./node_modules/.bin/vsce login $VSCE_PUBLISHER'"
    fi
fi

# ---------------------------------------------------------------- JetBrains

hdr "JetBrains Marketplace (publish-jetbrains)"

if [ -x "$JB_DIR/gradlew" ]; then
    ok "gradlew present in $JB_DIR"
else
    err "gradlew missing in $JB_DIR"
fi

JETBRAINS_TOKEN_SRC=""
if [ -n "${JETBRAINS_TOKEN:-}" ]; then
    JETBRAINS_TOKEN_RESOLVED="$JETBRAINS_TOKEN"
    JETBRAINS_TOKEN_SRC="env"
elif JETBRAINS_TOKEN_RESOLVED="$(bash "$ROOT/scripts/release/jetbrains_token.sh" 2>/dev/null)" \
        && [ -n "$JETBRAINS_TOKEN_RESOLVED" ]; then
    case "$(uname -s)" in
        Darwin) JETBRAINS_TOKEN_SRC="macOS Keychain" ;;
        Linux)  JETBRAINS_TOKEN_SRC="libsecret" ;;
        *)      JETBRAINS_TOKEN_SRC="keystore" ;;
    esac
fi

if [ -n "$JETBRAINS_TOKEN_SRC" ]; then
    ok "JetBrains token resolved from $JETBRAINS_TOKEN_SRC"
    # The marketplace REST API exposes /api/auth/me when called with the
    # uploader token. A 200 confirms the token is live.
    if command -v curl >/dev/null 2>&1; then
        # Drop -f so non-2xx still emits %{http_code} instead of curl
        # itself failing and producing a concatenated "404000" garbage
        # status. /api/auth/me is the documented liveness endpoint for
        # the uploader-token scope.
        HTTP="$(curl -sS -o /dev/null -w '%{http_code}' \
            -H "Authorization: Bearer $JETBRAINS_TOKEN_RESOLVED" \
            "https://plugins.jetbrains.com/api/auth/me" 2>/dev/null)"
        HTTP="${HTTP:-000}"
        if [ "$HTTP" = "200" ]; then
            ok "JetBrains token is accepted by plugins.jetbrains.com"
        elif [ "$HTTP" = "404" ]; then
            warn "JetBrains /api/auth/me returned 404. The endpoint may have
        moved; the token is still passed verbatim to gradle publishPlugin,
        which is the source of truth."
        else
            warn "JetBrains token did not return 200 from /api/auth/me (got $HTTP).
        Token may be expired or wrong scope (need 'Plugin uploader')."
        fi
    fi
else
    warn "JetBrains token not found. Either set \$JETBRAINS_TOKEN, or store it"
    warn "  in the OS keystore (see scripts/release/jetbrains_token.sh header for the"
    warn "  exact 'security add-generic-password' / 'secret-tool store' incantation)."
fi
unset JETBRAINS_TOKEN_RESOLVED JETBRAINS_TOKEN_SRC

ok "plugin page: https://plugins.jetbrains.com/plugin/31801-tcl-language-support"

# ---------------------------------------------------------------- Sublime

hdr "Sublime Text (Package Control)"

# Nothing to publish by hand: Package Control's channel entry points at the
# `TclLsp.sublime-package` asset on each GitHub Release, so a tagged CI run
# is the whole publish step.  What is worth checking is that the asset the
# channel resolves actually exists on the most recent stable release.
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    LATEST_STABLE="$(gh release list --repo "$REPO" --exclude-drafts --limit 20 \
        --json tagName,isPrerelease \
        --jq 'map(select(.isPrerelease | not)) | .[0].tagName' 2>/dev/null || true)"
    if [ -n "$LATEST_STABLE" ] && [ "$LATEST_STABLE" != "null" ]; then
        if gh release view "$LATEST_STABLE" --repo "$REPO" --json assets \
            --jq '.assets[].name' 2>/dev/null | grep -qx "TclLsp.sublime-package"; then
            ok "release $LATEST_STABLE carries TclLsp.sublime-package (what Package Control fetches)"
        else
            warn "release $LATEST_STABLE has no TclLsp.sublime-package asset."
            warn "  Package Control serves the newest stable release that has one;"
            warn "  check the build-sublime job on that tag."
        fi
    else
        warn "could not determine the latest stable release from $REPO."
    fi
else
    warn "gh CLI missing or unauthenticated — skipping the Package Control asset check."
fi

ok "channel entry: editors/sublime-text/SUBMITTING.md"

# ----------------------------------------------------------------- Zed

hdr "Zed extensions registry (publish-zed)"

if command -v gh >/dev/null 2>&1; then
    if gh repo view "$ZED_UPSTREAM" >/dev/null 2>&1; then
        ok "$ZED_UPSTREAM is reachable"
    else
        err "$ZED_UPSTREAM is not reachable from this machine."
    fi

    FORK="${ZED_EXTENSIONS_FORK:-}"
    if [ -z "$FORK" ] && gh auth status >/dev/null 2>&1; then
        FORK="$(gh api user --jq .login)/extensions"
    fi
    if [ -n "$FORK" ]; then
        if gh repo view "$FORK" >/dev/null 2>&1; then
            ok "your fork $FORK is reachable"
        else
            warn "fork $FORK not found. Create one once:"
            warn "  gh repo fork $ZED_UPSTREAM --remote=false"
        fi
    fi
fi

ok "publish_zed.sh prepares the bump locally and stops before push (no PR opened)"

# ---------------------------------------------------------------- Summary

hdr "Summary"
if [ "$RC" -eq 0 ]; then
    echo "  Publishing readiness: PASS"
else
    echo "  Publishing readiness: FAIL — see [fail] lines above"
fi
exit "$RC"
