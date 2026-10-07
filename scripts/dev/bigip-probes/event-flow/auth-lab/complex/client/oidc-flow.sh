#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -eu

cookie_jar=/tmp/evtflow-oidc.jar
headers=/tmp/evtflow-oidc-headers.txt
login_html=/tmp/evtflow-oidc-login.html
body=/tmp/evtflow-oidc-body.txt

rm -f "$cookie_jar" "$headers" "$login_html" "$body"
curl -skSL --max-redirs 8 \
    -c "$cookie_jar" -b "$cookie_jar" \
    -H 'X-Evtflow-Request: EFO-oidc-start' \
    https://app.bitwisecook.org:18840/oidc/hello \
    -o "$login_html"

action="$(
    sed -n 's/.*<form id="kc-form-login"[^>]*action="\([^"]*\)".*/\1/p' \
        "$login_html" | sed 's/&amp;/\&/g'
)"
test -n "$action"

curl -skSL --max-redirs 12 \
    -c "$cookie_jar" -b "$cookie_jar" \
    -D "$headers" \
    -H 'X-Evtflow-Request: EFO-oidc-login' \
    --data-urlencode username=evtflow-oidc \
    --data-urlencode 'password=Evtflow-Oidc-User-2026!' \
    --data-urlencode credentialId= \
    "$action" \
    -o "$body"

cat "$headers"
cat "$body"
