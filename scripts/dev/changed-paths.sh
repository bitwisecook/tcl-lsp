#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Print the changed files for a supported CI event. Exit 0 with a complete
# list, or 2 when the API/diff cannot prove the list is complete. Callers must
# treat every non-zero result as "everything changed".
set -eu

if [ "$#" -ne 6 ]; then
    echo "usage: scripts/dev/changed-paths.sh EVENT REPOSITORY PR_NUMBER BEFORE SHA REF" >&2
    exit 2
fi

EVENT=$1
REPOSITORY=$2
PR_NUMBER=$3
BEFORE=$4
SHA=$5
REF=$6
GH_BIN=${GH_BIN:-gh}
ZERO_SHA=0000000000000000000000000000000000000000

canonical_sha() {
    value=$1
    [ "${#value}" -eq 40 ] || return 1
    case "$value" in *[!0-9a-f]*|'') return 1 ;; esac
    [ "$value" != "$ZERO_SHA" ]
}

# Validate the raw API response and print one or two repository paths per file
# object.  The API's file-list limits apply to objects, not to emitted paths:
# a renamed file contributes two paths but still counts as one object.  Keep
# this parser here instead of trusting `--jq '.[].filename'`: a schema drift
# that turns a missing field into `null` must fail closed.
validate_files() {
    cap=$1
    shape=$2
    python3 -c '
import json
import sys

cap = int(sys.argv[1])
shape = sys.argv[2]

def fail():
    raise SystemExit(2)

try:
    payload = json.load(sys.stdin)
except (json.JSONDecodeError, TypeError, ValueError):
    fail()

if shape == "pull_request":
    # --paginate --slurp returns one array per API page.
    if not isinstance(payload, list) or not payload or not all(isinstance(page, list) for page in payload):
        fail()
    rows = [row for page in payload for row in page]
else:
    if not isinstance(payload, dict) or not isinstance(payload.get("files"), list):
        fail()
    rows = payload["files"]

# GitHub truncates exactly at these boundaries.  Treat a full response as
# ambiguous, and reject an empty response as an unproven changed-file list.
if not rows or len(rows) >= cap:
    fail()

statuses = {"added", "modified", "deleted", "renamed", "copied", "changed", "unchanged"}

def path(value):
    # The downstream shell transport is line-oriented.  Refuse control-line
    # characters rather than allowing one API field to masquerade as several.
    if not isinstance(value, str) or not value or "\x00" in value or "\n" in value or "\r" in value:
        fail()
    if value.startswith("/") or any(part in ("", ".", "..") for part in value.split("/")):
        fail()
    return value

for row in rows:
    if not isinstance(row, dict):
        fail()
    status = row.get("status")
    if not isinstance(status, str) or status not in statuses:
        fail()
    filename = path(row.get("filename"))
    previous_present = "previous_filename" in row
    previous = row.get("previous_filename")
    if previous_present:
        if status != "renamed":
            fail()
        previous = path(previous)
    elif status == "renamed":
        fail()
    print(filename)
    if previous_present:
        print(previous)
' "$cap" "$shape"
}

case "$EVENT" in
    pull_request)
        case "$PR_NUMBER" in
            '' | *[!0-9]*) exit 2 ;;
        esac
        raw=$("$GH_BIN" api "repos/$REPOSITORY/pulls/$PR_NUMBER/files?per_page=100" \
            --paginate --slurp 2>/dev/null) || exit 2
        files=$(printf '%s' "$raw" | validate_files 3000 pull_request) || exit 2
        ;;
    push)
        case "$REF" in refs/heads/rust) ;; *) exit 2 ;; esac
        canonical_sha "$BEFORE" || exit 2
        canonical_sha "$SHA" || exit 2
        raw=$("$GH_BIN" api "repos/$REPOSITORY/compare/$BEFORE...$SHA" \
            2>/dev/null) || exit 2
        files=$(printf '%s' "$raw" | validate_files 300 push) || exit 2
        ;;
    *)
        exit 2
        ;;
esac

[ -n "$files" ] || exit 2
printf '%s\n' "$files"
