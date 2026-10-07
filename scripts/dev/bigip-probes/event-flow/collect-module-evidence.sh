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

if [[ $# -ne 1 ]]; then
    echo "usage: $0 OUTPUT_DIRECTORY" >&2
    exit 2
fi

out=$1
mkdir -p "$out"

tmsh show sys version > "$out/sys-version.txt"
tmsh show sys hardware > "$out/sys-hardware.txt"
tmsh show sys tmm-info > "$out/sys-tmm-info.txt"
tmsh show sys failover > "$out/sys-failover.txt"
tmsh list sys provision > "$out/sys-provision.txt"
tmsh list sys global-settings hostname > "$out/sys-hostname.txt"
tmsh list sys software update > "$out/sys-software-update.txt"

{
    printf '#TMSH-VERSION: 21.1.0.1\n\n'
    tmsh -q list ltm rule '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm virtual '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm pool '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm profile client-ssl '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm profile server-ssl '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm message-routing generic protocol '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm message-routing generic transport-config '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm message-routing generic peer '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm message-routing generic route '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list ltm message-routing generic router '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm aaa ldap '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy agent ending-allow '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy agent ending-deny '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy agent irule-event '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy agent logon-page '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy agent aaa-ldap '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy policy-item '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy access-policy '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm profile access '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list apm policy customization-group '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list asm policy '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list sys file ssl-cert '/Common/__tcl_lsp_evtflow_*'
    tmsh -q list sys file ssl-key '/Common/__tcl_lsp_evtflow_*'
} > "$out/probe-only.scf"

grep 'EVTFLOW[234]|' /var/log/ltm > "$out/ltm-events.log" || true
grep -E '__tcl_lsp_evtflow_|LDAP Module:' /var/log/apm \
    > "$out/apm-events.log" || true

find "$out" -type f -print0 | sort -z | xargs -0 sha256sum \
    > "$out/SHA256SUMS"
