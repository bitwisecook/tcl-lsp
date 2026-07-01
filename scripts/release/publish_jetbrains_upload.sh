#!/usr/bin/env bash
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
#   JETBRAINS_CHANNEL    release channel; empty = Stable/default
#
# Exit codes:
#   0  upload accepted (HTTP 2xx)
#   1  bad arguments, missing file, missing token, or upload rejected
set -uo pipefail

ZIP="${1:?usage: $0 <plugin.zip>}"
PLUGIN_ID="${JETBRAINS_PLUGIN_ID:-31801}"
CHANNEL="${JETBRAINS_CHANNEL:-}"
HERE="$(cd "$(dirname "$0")" && pwd)"

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
