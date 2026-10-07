#!/bin/bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "usage: $0 FIXTURE_DIR EVIDENCE_DIR RUN" >&2
    exit 2
fi

fixture_dir=$1
evidence_dir=$2
run=$3
cert_name="/Common/__tcl_lsp_evtflow_${run,,}_complex_keycloak_ca"

mkdir -p "$evidence_dir"
cd "$fixture_dir"
sha256sum -c SHA256SUMS >"$evidence_dir/hash-precheck.txt" 2>&1
if tmsh -q list sys file ssl-cert "$cert_name" >/dev/null 2>&1; then
    echo "existing owned certificate retained" >"$evidence_dir/cert-install.txt"
else
    tmsh install sys crypto cert "$cert_name" from-local-file "$fixture_dir/keycloak-ca.crt" \
        >"$evidence_dir/cert-install.txt" 2>&1
fi
tmsh load sys config merge file "$fixture_dir/rule.conf" \
    >"$evidence_dir/rule-load.txt" 2>&1
tmsh load sys config merge file "$fixture_dir/config.conf" \
    >"$evidence_dir/config-load.txt" 2>&1
tmsh save sys config >"$evidence_dir/save.txt" 2>&1
