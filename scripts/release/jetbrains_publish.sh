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

# jetbrains_publish.sh — publish an already-built plugin .zip to the JetBrains
# Marketplace via the Marketplace upload REST API.
#
# Invoked by the `publish-jetbrains-marketplace` job in
# .github/workflows/ci.yml with JETBRAINS_TOKEN in the environment (an
# approval-gated marketplace-jetbrains Environment secret).  We upload the
# exact plugin .zip that was attached to the GitHub Release and verified
# against the cosign-signed SHA256SUMS — the same "publish the persisted,
# checksum-verified asset, never a mutable workflow artifact" property the
# VS Code job relies on.  Uploading via the Marketplace API (rather than
# `./gradlew publishPlugin`) keeps this to a single curl: it publishes those
# exact bytes and avoids re-resolving the whole IntelliJ Platform dependency
# just to push a file.  Kept as a committed script (not inline YAML) so the
# workflow stays thin and this credential-adjacent step is reviewable in one
# place.
#
# Usage:  scripts/release/jetbrains_publish.sh [TAG] [ZIP]
#         TAG defaults to the tag in $GITHUB_REF (refs/tags/<TAG>).
#         ZIP defaults to the single tcl-lsp-jetbrains-*.zip under dist/
#             (downloaded and checksum-verified by the workflow before this
#             runs).
#
# Channel (scripts/release/prerelease.sh is the single source of truth): an
# odd-minor 2.x tag (v2.1.x) uploads to the "eap" channel, only visible to
# users who add the custom repository URL
# https://plugins.jetbrains.com/plugins/eap/list; 1.x and even-minor 2.x
# (v2.2.0) upload with an empty channel, which the Marketplace treats as the
# public Stable channel.  This mirrors the JETBRAINS_CHANNEL mapping in the
# Makefile and the `channels` block in editors/jetbrains/build.gradle.kts.
set -euo pipefail

# The plugin's <id> from
# editors/jetbrains/src/main/resources/META-INF/plugin.xml — the stable
# identifier the Marketplace keys the listing on (plugin 31801,
# "Tcl Language Support").
XML_ID="com.tcllsp.jetbrains"
UPLOAD_URL="https://plugins.jetbrains.com/api/updates/upload"

here="$(cd "$(dirname "$0")" && pwd)"
tag="${1:-${GITHUB_REF#refs/tags/}}"
zip="${2:-$(ls dist/tcl-lsp-jetbrains-*.zip)}"

channel=""
if [ "$(bash "$here/prerelease.sh" "$tag")" = "true" ]; then
    echo "$tag is a pre-release — uploading to the 'eap' channel."
    channel="eap"
fi

: "${JETBRAINS_TOKEN:?JETBRAINS_TOKEN must be set (marketplace-jetbrains Environment secret)}"
[ -f "$zip" ] || { echo "error: plugin zip not found: $zip" >&2; exit 1; }
echo "Uploading $zip to the JetBrains Marketplace (xmlId=$XML_ID${channel:+, channel=$channel})"

# The status check is ours rather than curl's --fail-with-body, which exits
# non-zero on a 4xx: inside the http_code="$(curl …)" assignment under `set -e`
# that aborts the script *before* the body is printed, so the Marketplace's
# stated reason is lost exactly when it is needed.  (That is how a v2.1.16 eap
# upload failed with a bare `curl: (22) … error: 400` and nothing else.)  So
# curl only transports here: its exit status is captured without killing the
# shell, the body is printed unconditionally, and the HTTP code is judged
# afterwards.  channel is passed only when non-empty so a stable release
# genuinely uses the Marketplace default.
response_body="$(mktemp)"
trap 'rm -f "$response_body"' EXIT

set +e
http_code="$(
    curl -sS -o "$response_body" -w '%{http_code}' \
        --header "Authorization: Bearer ${JETBRAINS_TOKEN}" \
        -F "xmlId=${XML_ID}" \
        ${channel:+-F "channel=${channel}"} \
        -F "file=@${zip}" \
        "$UPLOAD_URL"
)"
curl_status=$?
set -e

# A transport-level failure (DNS, TLS, connection reset) means there may be no
# HTTP status at all, so report curl's own error rather than an empty code.
if [ "$curl_status" -ne 0 ]; then
    echo "error: curl failed with exit status $curl_status before a complete HTTP response" >&2
    [ -s "$response_body" ] && cat "$response_body" >&2
    exit 1
fi

echo "Marketplace responded HTTP $http_code"
cat "$response_body"
echo

case "$http_code" in
    2??) ;;
    *)
        echo "error: JetBrains Marketplace upload failed with HTTP $http_code (see the response body above)" >&2
        exit 1
        ;;
esac
