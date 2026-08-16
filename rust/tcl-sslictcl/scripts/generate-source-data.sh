#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
RAW="$ROOT/rust/tcl-sslictcl/data/raw"
GENERATED="$ROOT/rust/tcl-sslictcl/data/generated"
OUTPUT="$GENERATED/trust-seed.json"
EXCEPTIONS="$GENERATED/trust-material-exceptions.json"
CHROME_OUTPUT="$GENERATED/chrome-root-store.json"
CHECK=0
if [[ "${1:-}" == "--check" ]]; then
    CHECK=1
elif [[ $# -ne 0 ]]; then
    echo "usage: $0 [--check]" >&2
    exit 2
fi

REVISION_FILE="$RAW/observatory-revision.txt"
if [[ ! -s "$REVISION_FILE" ]]; then
    echo "missing pinned Observatory revision: $REVISION_FILE" >&2
    exit 1
fi
REVISION="$(head -n 1 "$REVISION_FILE" | tr -d '[:space:]')"
if [[ ! "$REVISION" =~ ^[0-9a-f]{40}$ ]]; then
    echo "invalid Observatory revision: $REVISION" >&2
    exit 1
fi

TMP="$(mktemp -d /tmp/sslictcl-generate.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
PARTS=()
for spec in apple:apple: google_aosp:android: microsoft_windows:microsoft: mozilla_nss:mozilla:server-auth openjdk:open-jdk: oracle_java:open-jdk:; do
    IFS=: read -r name client purpose <<< "$spec"
    input="$RAW/trust_stores/$name.yaml"
    part="$TMP/$name.json"
    [[ -s "$input" ]] || { echo "missing trust-store snapshot: $input" >&2; exit 1; }
    snapshot_version="$(awk -F': ' '$1 == "version" {v=$2} END {if (v == "" || v == "null") print ""; else print v}' "$input")"
    snapshot_date="$(awk -F': ' '$1 == "date_fetched" {print $2; exit}' "$input")"
    awk -v client="$client" -v purpose="$purpose" -v snapshot_version="$snapshot_version" -v snapshot_date="$snapshot_date" '
        function trim(value) { sub(/^[[:space:]]+/, "", value); sub(/[[:space:]]+$/, "", value); return value }
        /^- subject_name:/ {
            subject = $0; sub(/^[^:]*:[[:space:]]*/, "", subject); subject = trim(subject)
            if (subject ~ /^".*"$/) { sub(/^"/, "", subject); sub(/"$/, "", subject) }
            next
        }
        /^  fingerprint:/ {
            fingerprint = $0; sub(/^[^:]*:[[:space:]]*/, "", fingerprint); fingerprint = trim(fingerprint)
            if (subject != "" && fingerprint ~ /^[0-9a-fA-F]{64}$/) printf "%s\t%s\t%s\t%s\t%s\t%s\n", client, snapshot_version, snapshot_date, purpose, subject, tolower(fingerprint)
            subject = ""
        }
    ' "$input" | jq -R -s 'split("\n") | map(select(length > 0) | split("\t") | {client: .[0], snapshot_version: .[1], snapshot_date: .[2], purpose: .[3], subject: .[4], fingerprint: .[5]})' > "$part"
    PARTS+=("$part")
done

# Parse every certificate in the pinned PEM archive and index by DER SHA-256.
MATERIAL_TSV="$TMP/material.tsv"
: > "$MATERIAL_TSV"
for pem in "$RAW"/pem/*.pem; do
    base="$(basename "$pem" .pem)"
    csplit -s -f "$TMP/$base-" -b '%04d.pem' "$pem" '/-----BEGIN CERTIFICATE-----/' '{*}'
    for cert in "$TMP/$base-"*.pem; do
        grep -q -- '-----BEGIN CERTIFICATE-----' "$cert" || continue
        der="$TMP/der"
        openssl x509 -in "$cert" -outform DER -out "$der"
        fingerprint="$(sha256sum "$der" | awk '{print $1}')"
        der_base64="$(base64 -w 0 "$der")"
        spki_sha256="$(openssl x509 -in "$cert" -pubkey -noout | openssl pkey -pubin -outform DER 2>/dev/null | sha256sum | awk '{print $1}')"
        subject_key_id="$(openssl x509 -in "$cert" -noout -ext subjectKeyIdentifier 2>/dev/null | awk '/Subject Key Identifier/{getline; gsub(/[^0-9A-Fa-f]/, ""); print tolower($0)}')"
        not_before_text="$(openssl x509 -in "$cert" -noout -startdate | sed 's/^notBefore=//')"
        not_after_text="$(openssl x509 -in "$cert" -noout -enddate | sed 's/^notAfter=//')"
        not_before="$(date -u -d "$not_before_text" +%s)"
        not_after="$(date -u -d "$not_after_text" +%s)"
        subject="$(openssl x509 -in "$cert" -noout -subject -nameopt RFC2253 | sed 's/^subject=//')"
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$fingerprint" "$der_base64" "$spki_sha256" "$subject_key_id" "$not_before" "$not_after" "$subject" >> "$MATERIAL_TSV"
    done
done
MATERIAL="$TMP/material.json"
jq -R -s 'split("\n") | map(select(length > 0) | split("\t") | {key: .[0], value: {der_base64: .[1], spki_sha256: .[2], subject_key_id: (if .[3] == "" then null else .[3] end), not_before: (.[4] | tonumber), not_after: (.[5] | tonumber), subject: .[6]}}) | from_entries' "$MATERIAL_TSV" > "$MATERIAL"

CHROME_VERSION="$(awk -F': ' '$1 == "version_major" {print $2; exit}' "$RAW/chrome_root_store/root_store.textproto")"
[[ "$CHROME_VERSION" =~ ^[0-9]+$ ]] || { echo "missing Chrome Root Store version" >&2; exit 1; }
CHROME_PART="$TMP/chrome.json"
awk -F'"' '
    /trust_anchors[[:space:]]*{/ {inside=1; depth=1; next}
    inside && /constraints[[:space:]]*{/ {depth++; next}
    inside && depth == 1 && /sha256_hex:/ {print $2}
    inside && /^[[:space:]]*}/ {depth--; if (depth == 0) inside=0}
' "$RAW/chrome_root_store/root_store.textproto" \
    | jq -R -s --arg version "" --slurpfile material "$MATERIAL" '
        split("\n") | map(select(length > 0) | . as $fingerprint
        | {client: "chrome", snapshot_version: $version, snapshot_date: null, purpose: "server-auth",
           subject: ($material[0][$fingerprint].subject // ""), fingerprint: $fingerprint})
    ' > "$CHROME_PART"
PARTS+=("$CHROME_PART")
CHROME_MISSING="$(jq --slurpfile material "$MATERIAL" '[.[] | select($material[0][.fingerprint] == null)] | length' "$CHROME_PART")"
[[ "$CHROME_MISSING" == 0 ]] || { echo "Chrome Root Store has $CHROME_MISSING anchors without matching PEM DER" >&2; exit 1; }

ANCHORS="$TMP/anchors.json"
jq -s --slurpfile material "$MATERIAL" '
    add | group_by(.fingerprint)
    | map(. as $group | ($material[0][$group[0].fingerprint] // {}) as $cert | {
        fingerprint_sha256: $group[0].fingerprint,
        subject: ((map(.subject) | map(select(length > 0)) | sort | .[0]) // ("SHA256:" + $group[0].fingerprint)),
        der_base64: ($cert.der_base64 // null), spki_sha256: ($cert.spki_sha256 // null),
        subject_key_id: ($cert.subject_key_id // null), not_before: ($cert.not_before // null),
        not_after: ($cert.not_after // null), purposes: (map(.purpose) | map(select(length > 0)) | unique),
        memberships: (map({client: .client, first_version: null, last_version: null,
          snapshot_version: (if (.snapshot_version // "") == "" then null else .snapshot_version end),
          snapshot_date: (if (.snapshot_date // "") == "" then null else .snapshot_date end),
          purposes: (if (.purpose // "") == "" then [] else [.purpose] end),
          policy_complete: (.client != "chrome"),
          trusted: true, distrust_after: null})
          | unique_by([.client, .snapshot_version, .snapshot_date])
          | sort_by([.client, (.snapshot_version // ""), (.snapshot_date // "")]))
      }) | sort_by(.fingerprint_sha256)
' "${PARTS[@]}" > "$ANCHORS"

CHROME_METADATA="$TMP/chrome-root-store.json"
awk '
    /trust_anchors[[:space:]]*{/ {inside=1; depth=1; fingerprint=""; root_id=""; ev=""; constraint=""; next}
    inside && /constraints[[:space:]]*{/ {depth++; next}
    inside && /sha256_hex:/ {value=$0; sub(/^.*sha256_hex:[[:space:]]*"/, "", value); sub(/".*$/, "", value); fingerprint=value}
    inside && /crs_root_id:/ {value=$0; sub(/^.*crs_root_id:[[:space:]]*/, "", value); root_id=value}
    inside && /ev_policy_oids:/ {value=$0; sub(/^.*ev_policy_oids:[[:space:]]*"/, "", value); sub(/".*$/, "", value); ev=value}
    inside && /sct_not_after_sec:/ {value=$0; sub(/^.*sct_not_after_sec:[[:space:]]*/, "", value); constraint=value}
    inside && /^[[:space:]]*}/ {depth--; if (depth == 0) {if (fingerprint != "") printf "%s\t%s\t%s\t%s\n", fingerprint, root_id, ev, constraint; inside=0}}
' "$RAW/chrome_root_store/root_store.textproto" | jq -R -s --arg revision "$(cat "$RAW/chrome_root_store/revision.txt" 2>/dev/null || echo unknown)" --arg version "$CHROME_VERSION" '
    {schema: 1, client: "chrome", version: $version, revision: $revision,
     anchors: (split("\n") | map(select(length > 0) | split("\t") | {
       fingerprint_sha256: .[0], crs_root_id: (.[1] | tonumber),
       ev_policy_oids: (if .[2] == "" then [] else [.[2]] end),
       sct_not_after_sec: (if .[3] == "" then null else (.[3] | tonumber) end)
     }))}
' > "$CHROME_METADATA"

MATERIAL_EXCEPTIONS="$TMP/material-exceptions.json"
jq -n --slurpfile anchors "$ANCHORS" --slurpfile material "$MATERIAL" '
    {schema: 1,
     notice: "The Observatory YAML lists this root, but the pinned PEM archive does not contain matching complete DER. Trust membership is retained for audit; chain completion and certificate metadata remain Unknown.",
     exceptions: [$anchors[0][] as $anchor | select(($material[0][$anchor.fingerprint_sha256] // null) == null) | {fingerprint_sha256: $anchor.fingerprint_sha256, subject: $anchor.subject, reason: "no matching certificate in the pinned PEM archive"}]}
' > "$MATERIAL_EXCEPTIONS"

LATEST_DATE="$(awk -F': ' '/^date_fetched:/ {print $2}' "$RAW"/trust_stores/*.yaml | sort | tail -n 1)"
[[ "$LATEST_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || { echo "could not determine a stable source date from Observatory YAML" >&2; exit 1; }
CHROMIUM_REVISION="$(head -n 1 "$RAW/chrome_root_store/revision.txt" | tr -d '[:space:]')"
[[ "$CHROMIUM_REVISION" =~ ^[0-9a-f]{40}$ ]] || { echo "invalid Chromium revision" >&2; exit 1; }
mkdir -p "$GENERATED"
TEMP_OUTPUT="$TMP/trust-seed.json"
jq -n --arg revision "$REVISION" --arg chromium_revision "$CHROMIUM_REVISION" --arg generated_at "$LATEST_DATE" --slurpfile anchors "$ANCHORS" '
    {trust: {provenance: {
        schema: 1, generated_at: ($generated_at + "T00:00:00Z"), bootstrap_seed: false,
        notice: "Generated from a pinned Trust Stores Observatory snapshot. Membership is presence in the represented root-program snapshot; absence is Unknown, not proof of distrust. Certificate material gaps are explicit in trust-material-exceptions.json.",
        sources: [
          {name: "Trust Stores Observatory / Apple", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/apple.yaml", revision: $revision, license: "MIT"},
          {name: "Trust Stores Observatory / Android AOSP", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/google_aosp.yaml", revision: $revision, license: "MIT"},
          {name: "Trust Stores Observatory / Microsoft Windows", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/microsoft_windows.yaml", revision: $revision, license: "MIT"},
          {name: "Trust Stores Observatory / Mozilla NSS", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/mozilla_nss.yaml", revision: $revision, license: "MIT"},
          {name: "Trust Stores Observatory / OpenJDK", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/openjdk.yaml", revision: $revision, license: "MIT"},
          {name: "Trust Stores Observatory / Oracle Java", url: "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/\($revision)/trust_stores/oracle_java.yaml", revision: $revision, license: "MIT"},
          {name: "Chromium Chrome Root Store", url: "https://chromium.googlesource.com/chromium/src/+/\($chromium_revision)/net/data/ssl/chrome_root_store/root_store.textproto", revision: $chromium_revision, license: "BSD-3-Clause"}
        ]}, anchors: $anchors[0]}}
' > "$TEMP_OUTPUT"

if (( CHECK )); then
    [[ -f "$OUTPUT" ]] && cmp -s "$TEMP_OUTPUT" "$OUTPUT" || { echo "generated trust data is stale; run make update-source-data" >&2; exit 1; }
    [[ -f "$EXCEPTIONS" ]] && cmp -s "$MATERIAL_EXCEPTIONS" "$EXCEPTIONS" || { echo "generated trust-material exceptions are stale; run make update-source-data" >&2; exit 1; }
    [[ -f "$CHROME_OUTPUT" ]] && cmp -s "$CHROME_METADATA" "$CHROME_OUTPUT" || { echo "generated Chrome Root Store metadata is stale; run make update-source-data" >&2; exit 1; }
else
    install -m 0644 "$TEMP_OUTPUT" "$OUTPUT"
    install -m 0644 "$MATERIAL_EXCEPTIONS" "$EXCEPTIONS"
    install -m 0644 "$CHROME_METADATA" "$CHROME_OUTPUT"
fi
echo "SslicTcl trust data: $(jq '.trust.anchors | length' "$TEMP_OUTPUT") anchors; $(jq '.exceptions | length' "$MATERIAL_EXCEPTIONS") material exceptions from Observatory $REVISION"
