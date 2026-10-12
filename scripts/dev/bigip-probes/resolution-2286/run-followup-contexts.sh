#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Run on BIG-IP after continuous /var/log/ltm capture has started.
set -uo pipefail
umask 077
run=${1:?run identifier}
fixtures=${2:?fixture directory}
evidence=${3:?evidence directory}
[[ $run =~ ^[A-Za-z][A-Za-z0-9]{0,15}$ ]] || exit 2
mkdir -p "$evidence"

prefix="__tcl_lsp_2286_${run}_contexts"
cli="/Common/${prefix}_cli"
iapp="/Common/${prefix}_iapp"
service="/Common/${prefix}_iapp_service"
icall="/Common/${prefix}_icall"
handler="/Common/${prefix}_icall_handler"
event="${run^^}_FOLLOWUP"

cleanup() {
    if [[ -n ${scriptd_tail_pid:-} ]]; then kill "$scriptd_tail_pid" >/dev/null 2>&1 || true; fi
    tmsh delete sys icall handler triggered "$handler" >/dev/null 2>&1 || true
    tmsh delete sys icall script "$icall" >/dev/null 2>&1 || true
    tmsh delete sys application service "$service" >/dev/null 2>&1 || true
    tmsh delete sys application template "$iapp" >/dev/null 2>&1 || true
    tmsh delete cli script "$cli" >/dev/null 2>&1 || true
}
trap cleanup EXIT

{
    tmsh list cli script "$cli" 2>&1 || true
    tmsh list sys application template "$iapp" 2>&1 || true
    tmsh list sys application service "$service" 2>&1 || true
    tmsh list sys icall script "$icall" 2>&1 || true
    tmsh list sys icall handler triggered "$handler" 2>&1 || true
} > "$evidence/precreate-absence.txt"
if tmsh list cli script "$cli" >/dev/null 2>&1 || \
   tmsh list sys application template "$iapp" >/dev/null 2>&1 || \
   tmsh list sys application service "$service" >/dev/null 2>&1 || \
   tmsh list sys icall script "$icall" >/dev/null 2>&1 || \
   tmsh list sys icall handler triggered "$handler" >/dev/null 2>&1; then
    echo collision >&2
    exit 3
fi

tmsh load sys config merge file "$fixtures/cli.conf" > "$evidence/cli-load.txt" 2>&1
tmsh run cli script "$cli" > "$evidence/cli-run.txt" 2>&1
tmsh list cli script "$cli" all-properties > "$evidence/cli-config.txt" 2>&1
tmsh delete cli script "$cli" > "$evidence/cli-delete.txt" 2>&1

tail -n 0 -F /var/tmp/scriptd.out > "$evidence/scriptd-raw.txt" 2>&1 &
scriptd_tail_pid=$!
tmsh load sys config merge file "$fixtures/iapp.conf" > "$evidence/iapp-template-load.txt" 2>&1
tmsh load sys config merge file "$fixtures/iapp-service.conf" > "$evidence/iapp-service-load.txt" 2>&1
tmsh modify sys application service "$service" execute-action definition > "$evidence/iapp-run.txt" 2>&1
sleep 2
tmsh list sys application template "$iapp" all-properties > "$evidence/iapp-template-config.txt" 2>&1
tmsh list sys application service "$service" all-properties > "$evidence/iapp-service-config.txt" 2>&1
tmsh delete sys application service "$service" > "$evidence/iapp-service-delete.txt" 2>&1
tmsh delete sys application template "$iapp" > "$evidence/iapp-template-delete.txt" 2>&1

tmsh load sys config merge file "$fixtures/icall.conf" > "$evidence/icall-load.txt" 2>&1
tmsh list sys icall script "$icall" all-properties > "$evidence/icall-script-config.txt" 2>&1
tmsh list sys icall handler triggered "$handler" all-properties > "$evidence/icall-handler-config.txt" 2>&1
tmsh generate sys icall event name "$event" > "$evidence/icall-generate.txt" 2>&1
sleep 4
tmsh delete sys icall handler triggered "$handler" > "$evidence/icall-handler-delete.txt" 2>&1
tmsh delete sys icall script "$icall" > "$evidence/icall-script-delete.txt" 2>&1
sleep 1
kill "$scriptd_tail_pid" >/dev/null 2>&1 || true
wait "$scriptd_tail_pid" 2>/dev/null || true
unset scriptd_tail_pid
grep "R2286FOLLOWUP|${run}|" "$evidence/scriptd-raw.txt" > "$evidence/scriptd-results.txt" || true

{
    tmsh list cli script "$cli" 2>&1 || true
    tmsh list sys application template "$iapp" 2>&1 || true
    tmsh list sys application service "$service" 2>&1 || true
    tmsh list sys icall script "$icall" 2>&1 || true
    tmsh list sys icall handler triggered "$handler" 2>&1 || true
} > "$evidence/cleanup-verification.txt"
trap - EXIT
