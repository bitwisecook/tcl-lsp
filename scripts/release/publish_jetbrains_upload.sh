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

# publish_jetbrains_upload.sh — upload an already-built JetBrains plugin
# .zip to the JetBrains Marketplace via the REST upload API.
#
# Unlike `./gradlew publishPlugin` (which rebuilds the plugin from source),
# this uploads a specific, already-produced artefact — so CI can publish the
# exact .zip that was attached to the GitHub Release and checksum-verified,
# with no Java/Gradle toolchain in the publish job.
#
# Usage:  scripts/release/publish_jetbrains_upload.sh <plugin.zip>
#
# The token is resolved by scripts/release/jetbrains_token.sh (env var
# $JETBRAINS_TOKEN first, then the OS keystore), so this works both from CI
# (token in the environment) and the laptop (token in the Keychain).
#
# Env knobs:
#   JETBRAINS_PLUGIN_ID  numeric Marketplace plugin id (default: 31801)
#   JETBRAINS_CHANNEL    release channel.  If set (even to empty) it wins —
#                        empty = the default Stable channel.  If UNSET, the
#                        channel is derived from the version in the zip name
#                        using the same odd/even-minor convention as the VS
#                        Code pre-release track (scripts/release/prerelease.sh):
#                        a pre-release (odd-minor 2.x) build -> "eap", a
#                        stable build -> the default Stable channel.
#
# Exit codes:
#   0  upload accepted (HTTP 2xx)
#   1  bad arguments, missing file, missing token, or upload rejected
set -uo pipefail

ZIP="${1:?usage: $0 <plugin.zip>}"
PLUGIN_ID="${JETBRAINS_PLUGIN_ID:-31801}"
HERE="$(cd "$(dirname "$0")" && pwd)"

# Release channel.  An explicitly-set JETBRAINS_CHANNEL wins (even when empty
# — empty means the default Stable channel).  When UNSET, derive it from the
# version in the zip name (tcl-lsp-jetbrains-X.Y.Z.zip) via the shared
# odd/even-minor convention: a pre-release build -> "eap", stable -> Stable.
if [ -n "${JETBRAINS_CHANNEL+set}" ]; then
    CHANNEL="$JETBRAINS_CHANNEL"
else
    CHANNEL=""
    ver="$(basename "$ZIP" .zip)"; ver="${ver#tcl-lsp-jetbrains-}"
    if [ "$(bash "$HERE/prerelease.sh" "$ver" 2>/dev/null)" = "true" ]; then
        CHANNEL="eap"
    fi
fi

[ -f "$ZIP" ] || { echo "error: plugin zip not found: $ZIP" >&2; exit 1; }

TOKEN="$(bash "$HERE/jetbrains_token.sh")" || exit 1
[ -n "$TOKEN" ] || { echo "error: empty JetBrains token" >&2; exit 1; }

echo "==> Uploading $(basename "$ZIP") to JetBrains Marketplace (pluginId=$PLUGIN_ID${CHANNEL:+, channel=$CHANNEL})" >&2

# The documented bearer-token upload endpoint
# (https://plugins.jetbrains.com/docs/marketplace/plugin-upload.html):
# POST multipart to /api/updates/upload with `pluginId` + `file`, and an
# optional `channel` (omit entirely for the default Stable channel — an
# empty `channel=` form field is not the same as "unset").
#
# The endpoint returns 201/302 on success and 4xx with a JSON error body
# otherwise.  Capture the HTTP status separately from the body so a failed
# upload is a hard error rather than a silently-ignored 4xx.
body="$(mktemp)"
trap 'rm -f "$body"' EXIT
channel_field=()
[ -n "$CHANNEL" ] && channel_field=(-F "channel=${CHANNEL}")
status="$(curl -sS -o "$body" -w '%{http_code}' \
    --retry 3 --retry-delay 5 --retry-all-errors \
    -X POST \
    -H "Authorization: Bearer $TOKEN" \
    -F "pluginId=${PLUGIN_ID}" \
    "${channel_field[@]}" \
    -F "file=@${ZIP}" \
    https://plugins.jetbrains.com/api/updates/upload)"

if [ "$status" -ge 200 ] && [ "$status" -lt 400 ]; then
    echo "==> JetBrains Marketplace accepted the upload (HTTP $status)." >&2
    exit 0
fi

echo "error: JetBrains Marketplace rejected the upload (HTTP $status):" >&2
cat "$body" >&2
echo >&2
exit 1
