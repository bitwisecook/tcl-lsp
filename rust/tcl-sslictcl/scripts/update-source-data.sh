#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later

# Network-capable source refresh.  Normal builds and checks never invoke this
# script; use make update-source-data deliberately when refreshing the pinned
# snapshot.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
RAW="$ROOT/rust/tcl-sslictcl/data/raw"
STORE_DIR="$RAW/trust_stores"
PEM_DIR="$RAW/pem"
OBSERVATORY="https://github.com/nabla-c0d3/trust_stores_observatory.git"
OBSERVATORY_RAW="https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory"
REVISION="${SSLICTCL_OBSERVATORY_REVISION:-}"
CHROMIUM="https://chromium.googlesource.com/chromium/src"
CHROMIUM_RAW="https://chromium.googlesource.com/chromium/src/+"
CHROMIUM_REVISION="${SSLICTCL_CHROMIUM_REVISION:-}"

command -v curl >/dev/null || { echo "update-source-data requires curl" >&2; exit 1; }
command -v git >/dev/null || { echo "update-source-data requires git" >&2; exit 1; }
command -v tar >/dev/null || { echo "update-source-data requires tar" >&2; exit 1; }
command -v base64 >/dev/null || { echo "update-source-data requires base64" >&2; exit 1; }

sha256_digest() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$@" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$@" | awk '{print $1}'
    else
        echo "update-source-data requires sha256sum or shasum" >&2
        return 1
    fi
}

decode_base64() {
    if base64 --decode </dev/null >/dev/null 2>&1; then
        base64 --decode
    else
        base64 -D
    fi
}

if [[ -z "$REVISION" ]]; then
    REVISION="$(git ls-remote "$OBSERVATORY" HEAD | awk 'NR == 1 {print $1}')"
fi
if [[ ! "$REVISION" =~ ^[0-9a-f]{40}$ ]]; then
    echo "invalid Trust Stores Observatory revision: $REVISION" >&2
    exit 1
fi

mkdir -p "$STORE_DIR" "$PEM_DIR"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/sslictcl-update.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/trust_stores" "$STAGE/pem" "$STAGE/chrome_root_store" "$STAGE/licenses"
declare -a STORE_NAMES=(apple google_aosp microsoft_windows mozilla_nss openjdk oracle_java)
for name in "${STORE_NAMES[@]}"; do
    url="$OBSERVATORY_RAW/$REVISION/trust_stores/$name.yaml"
    echo "fetching $url"
    curl --fail --location --retry 3 --silent --show-error "$url" \
        -o "$STAGE/trust_stores/$name.yaml"
done
printf '%s\n' "$REVISION" > "$STAGE/observatory-revision.txt"
curl --fail --location --retry 3 --silent --show-error \
    "$OBSERVATORY_RAW/$REVISION/LICENSE" -o "$STAGE/licenses/trust-stores-observatory-LICENSE"

# Keep the upstream PEM bundles as auditable raw inputs too.  The generated
# report data uses fingerprints and subjects, while these bundles permit later
# chain/certificate tooling to reproduce the exact source snapshot offline.
ARCHIVE="$STAGE/trust_stores_as_pem.tar.gz"
curl --fail --location --retry 3 --silent --show-error \
    "$OBSERVATORY_RAW/$REVISION/docs/trust_stores_as_pem.tar.gz" -o "$ARCHIVE"
tar -xzf "$ARCHIVE" -C "$STAGE/pem" \
    apple.pem google_aosp.pem microsoft_windows.pem mozilla_nss.pem openjdk.pem oracle_java.pem

if [[ -z "$CHROMIUM_REVISION" ]]; then
    CHROMIUM_REVISION="$(git ls-remote "$CHROMIUM" refs/heads/main | awk 'NR == 1 {print $1}')"
fi
if [[ ! "$CHROMIUM_REVISION" =~ ^[0-9a-f]{40}$ ]]; then
    echo "invalid Chromium revision: $CHROMIUM_REVISION" >&2
    exit 1
fi
CHROME_DIR="$STAGE/chrome_root_store"
echo "fetching Chromium Chrome Root Store at $CHROMIUM_REVISION"
curl --fail --location --retry 3 --silent --show-error \
    "$CHROMIUM_RAW/$CHROMIUM_REVISION/net/data/ssl/chrome_root_store/root_store.certs?format=TEXT" \
    | decode_base64 > "$CHROME_DIR/root_store.certs"
curl --fail --location --retry 3 --silent --show-error \
    "$CHROMIUM_RAW/$CHROMIUM_REVISION/net/data/ssl/chrome_root_store/root_store.textproto?format=TEXT" \
    | decode_base64 > "$CHROME_DIR/root_store.textproto"
printf '%s\n' "$CHROMIUM_REVISION" > "$CHROME_DIR/revision.txt"
cp "$CHROME_DIR/root_store.certs" "$STAGE/pem/chrome.pem"

curl --fail --location --retry 3 --silent --show-error \
    "$CHROMIUM_RAW/$CHROMIUM_REVISION/LICENSE?format=TEXT" \
    | decode_base64 > "$STAGE/licenses/chromium-LICENSE"

# Validate every staged certificate before replacing the committed snapshot.
for pem in "$STAGE"/pem/*.pem "$CHROME_DIR/root_store.certs"; do
    grep -q -- '-----BEGIN CERTIFICATE-----' "$pem" || { echo "no certificates in $pem" >&2; exit 1; }
done

# All network and structural checks succeeded. Install the complete snapshot
# only now, so an interrupted fetch cannot leave mixed upstream revisions.
mkdir -p "$RAW/licenses" "$RAW/chrome_root_store"
for name in "${STORE_NAMES[@]}"; do install -m 0644 "$STAGE/trust_stores/$name.yaml" "$STORE_DIR/$name.yaml"; done
for pem in "$STAGE"/pem/*.pem; do install -m 0644 "$pem" "$PEM_DIR/$(basename "$pem")"; done
install -m 0644 "$ARCHIVE" "$RAW/trust_stores_as_pem.tar.gz"
install -m 0644 "$STAGE/observatory-revision.txt" "$RAW/observatory-revision.txt"
install -m 0644 "$CHROME_DIR/root_store.certs" "$RAW/chrome_root_store/root_store.certs"
install -m 0644 "$CHROME_DIR/root_store.textproto" "$RAW/chrome_root_store/root_store.textproto"
install -m 0644 "$CHROME_DIR/revision.txt" "$RAW/chrome_root_store/revision.txt"
install -m 0644 "$STAGE/licenses/trust-stores-observatory-LICENSE" "$RAW/licenses/trust-stores-observatory-LICENSE"
install -m 0644 "$STAGE/licenses/chromium-LICENSE" "$RAW/licenses/chromium-LICENSE"

"$ROOT/rust/tcl-sslictcl/scripts/generate-source-data.sh"

PROVENANCE="$ROOT/rust/tcl-sslictcl/data/provenance.json"
RETRIEVED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
FILES_JSON="$(mktemp "${TMPDIR:-/tmp}/sslictcl-files.XXXXXX")"
trap 'rm -rf "$STAGE"; rm -f "$FILES_JSON"' EXIT
{
    find "$RAW" -type f -print0
    find "$ROOT/rust/tcl-sslictcl/data/generated" -type f -print0
} | while IFS= read -r -d '' file; do
    relative="${file#"$ROOT/"}"
    digest="$(sha256_digest "$file")"
    if [[ "$relative" == rust/tcl-sslictcl/data/raw/* ]]; then
        kind=raw
    else
        kind=generated
    fi
    jq -n --arg path "$relative" --arg kind "$kind" --arg sha256 "$digest" \
        '{path: $path, kind: $kind, sha256: $sha256}'
done | jq -s 'sort_by(.path)' > "$FILES_JSON"

jq -n \
    --arg retrieved_at "$RETRIEVED_AT" \
    --arg revision "$REVISION" \
    --arg chromium_revision "$CHROMIUM_REVISION" \
    --slurpfile files "$FILES_JSON" '
    {
      schema_version: 1,
      sources: [
        {id: "trust-stores-observatory/apple", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/apple.yaml", revision: $revision, retrieved_at: $retrieved_at, license: "MIT"},
        {id: "trust-stores-observatory/android-aosp", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/google_aosp.yaml", revision: $revision, retrieved_at: $retrieved_at, license: "MIT"},
        {id: "trust-stores-observatory/microsoft-windows", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/microsoft_windows.yaml", revision: $revision, retrieved_at: $retrieved_at, license: "MIT"},
        {id: "trust-stores-observatory/mozilla-nss", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/mozilla_nss.yaml", revision: $revision, retrieved_at: $retrieved_at, license: "MIT"},
        {id: "trust-stores-observatory/openjdk", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/openjdk.yaml", revision: $revision, retrieved_at: $retrieved_at, license: "MIT"},
        {id: "trust-stores-observatory/oracle-java", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/oracle_java.yaml", revision: $revision, retrieved_at: $retrieved_at, license: "MIT"},
        {id: "trust-stores-observatory/pem-bundle", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/docs/trust_stores_as_pem.tar.gz", revision: $revision, retrieved_at: $retrieved_at, license: "MIT"},
        {id: "chromium/chrome-root-store-certs", url: "https://chromium.googlesource.com/chromium/src/+/\($chromium_revision)/net/data/ssl/chrome_root_store/root_store.certs", revision: $chromium_revision, retrieved_at: $retrieved_at, license: "BSD-3-Clause"},
        {id: "chromium/chrome-root-store-textproto", url: "https://chromium.googlesource.com/chromium/src/+/\($chromium_revision)/net/data/ssl/chrome_root_store/root_store.textproto", revision: $chromium_revision, retrieved_at: $retrieved_at, license: "BSD-3-Clause"}
      ],
      files: $files[0]
    }
    ' > "$PROVENANCE"

echo "SslicTcl source data refreshed at $RETRIEVED_AT (revision $REVISION)"
