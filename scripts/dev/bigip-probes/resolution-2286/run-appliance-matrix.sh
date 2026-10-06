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

# Run after fixture loading and owned VIP creation. The traffic-control endpoint
# is a temporary, fixed-VIP driver on the external client/server host.
set -u
run=${1:?run identifier}
evidence=${2:?absolute evidence directory}
fixtures=${3:?absolute canonical fixture directory}
vip="/Common/__tcl_lsp_probe_2286_${run}_vs"
prefix="/Common/__tcl_lsp_probe_2286_${run}"
driver="$evidence/appliance-rules.sh"
trigger_base=http://192.168.9.80:18082/run
mkdir -p "$evidence/attachments" "$evidence/triggers"
port=21000

attach() {
    label=$1
    cmp=$2
    shift 2
    {
        printf 'cmp=%s rules=' "$cmp"
        printf ' %s' "$@"
        printf '\n'
        tmsh modify ltm virtual "$vip" cmp-enabled "$cmp" rules \{ "$@" \}
        printf 'modify_status=%s\n' "$?"
        tmsh list ltm virtual "$vip" all-properties
        tmsh show ltm virtual "$vip" detail
    } > "$evidence/attachments/$label.txt" 2>&1
}

trigger() {
    label=$1
    requests=$2
    expect=$3
    path=${4:-/%3Faction%3Dread}
    url="$trigger_base?label=$label&requests=$requests&source_port=$port&expect=$expect&path=$path"
    curl -fsS "$url" > "$evidence/triggers/$label.json" 2>&1
    printf '%s\t%s\t%s\t%s\n' "$label" "$?" "$port" "$url" >> "$evidence/trigger-status.tsv"
    port=$((port + requests + 4))
}

reload_case() {
    case_name=$1
    tag=$2
    attach "detach-before-$tag" yes "${prefix}_minimal"
    bash "$driver" cleanup "$fixtures" "$evidence/journals/$case_name" > "$evidence/journals/$case_name/reload-cleanup-driver.log" 2>&1
    journal="$evidence/journals/${case_name}_$tag"
    mkdir -m 700 "$journal"
    bash "$driver" load "$fixtures" "$journal" "$case_name" > "$journal/driver.log" 2>&1
}

: > "$evidence/trigger-status.tsv"

# static:: complete mutation matrix under the one-TMM control.
attach static-group-cmp-no no "${prefix}_static_group"
trigger static-group-cmp-no-read-initial 8 0:0
trigger static-group-cmp-no-write 1 0:0 '/%3Faction%3Dwrite%26target%3D0%3A0'
trigger static-group-cmp-no-read-after-write 8 0:0
trigger static-group-cmp-no-unset 1 0:0 '/%3Faction%3Dunset%26target%3D0%3A0'
trigger static-group-cmp-no-read-after-unset 8 0:0
trigger static-group-cmp-no-recreate 1 0:0 '/%3Faction%3Drecreate%26target%3D0%3A0'
trigger static-group-cmp-no-read-after-recreate 8 0:0

# Rule recreation supplies a fresh RULE_INIT before the distributed matrix.
reload_case static_group reinit_cmp_yes
attach static-group-cmp-yes yes "${prefix}_static_group"
trigger static-group-cmp-yes-read-initial 128 0:0,0:1,0:2,0:3
trigger static-group-cmp-yes-write 128 0:0,0:1,0:2,0:3 '/%3Faction%3Dwrite%26target%3D0%3A2'
trigger static-group-cmp-yes-read-after-write 128 0:0,0:1,0:2,0:3
trigger static-group-cmp-yes-unset 128 0:0,0:1,0:2,0:3 '/%3Faction%3Dunset%26target%3D0%3A2'
trigger static-group-cmp-yes-read-after-unset 128 0:0,0:1,0:2,0:3
trigger static-group-cmp-yes-recreate 128 0:0,0:1,0:2,0:3 '/%3Faction%3Drecreate%26target%3D0%3A2'
trigger static-group-cmp-yes-read-after-recreate 128 0:0,0:1,0:2,0:3

# Ordinary global is kept separate because it may change CMP eligibility.
attach global-cmp-no no "${prefix}_global"
trigger global-cmp-no-read-initial 8 0
trigger global-cmp-no-write 1 0 '/%3Faction%3Dwrite%26target%3D0'
trigger global-cmp-no-read-after-write 8 0
trigger global-cmp-no-unset 1 0 '/%3Faction%3Dunset%26target%3D0'
trigger global-cmp-no-read-after-unset 8 0
trigger global-cmp-no-recreate 1 0 '/%3Faction%3Drecreate%26target%3D0'
trigger global-cmp-no-read-after-recreate 8 0
reload_case global reinit_cmp_yes
attach global-cmp-yes yes "${prefix}_global"
trigger global-cmp-yes-read-initial 128 0,1,2,3
trigger global-cmp-yes-write 128 0,1,2,3 '/%3Faction%3Dwrite%26target%3D0'
trigger global-cmp-yes-read-after-write 128 0,1,2,3
trigger global-cmp-yes-unset 128 0,1,2,3 '/%3Faction%3Dunset%26target%3D0'
trigger global-cmp-yes-read-after-unset 128 0,1,2,3
trigger global-cmp-yes-recreate 128 0,1,2,3 '/%3Faction%3Drecreate%26target%3D0'
trigger global-cmp-yes-read-after-recreate 128 0,1,2,3

# Cross-rule static collision, fixed creation order A then B, both attachment orders.
attach collision-a-b yes "${prefix}_collision_a" "${prefix}_collision_b" "${prefix}_collision_observer"
trigger collision-a-b-read 128 0,1,2,3
trigger collision-a-b-write-a 1 '' '/%3Faction%3Dwrite_a'
trigger collision-a-b-read-after-a 128 0,1,2,3
trigger collision-a-b-write-b 1 '' '/%3Faction%3Dwrite_b'
trigger collision-a-b-read-after-b 128 0,1,2,3
attach collision-b-a yes "${prefix}_collision_b" "${prefix}_collision_a" "${prefix}_collision_observer"
trigger collision-b-a-read 128 0,1,2,3

# F5 procedure ownership and attachment-order control.
attach procedure-a-b yes "${prefix}_procedure_a" "${prefix}_procedure_b" "${prefix}_procedure_caller"
trigger procedure-a-b 128 0,1,2,3
attach procedure-b-a yes "${prefix}_procedure_b" "${prefix}_procedure_a" "${prefix}_procedure_caller"
trigger procedure-b-a 128 0,1,2,3

# Each dynamic case is run first/repeated on one TMM, cleaned there, then run
# across the complete roster and cleaned on every reached group/unit.
dynamic_cases='dynamic_namespace dynamic_path_shadow dynamic_path_provider dynamic_global_alias dynamic_upvar dynamic_upvar_unset dynamic_uplevel dynamic_upvar_absolute dynamic_upvar_zero dynamic_uplevel_absolute dynamic_static_global_link dynamic_alias dynamic_alias_rename dynamic_rename_epoch dynamic_namespace_import dynamic_namespace_delete dynamic_trace_scalar dynamic_trace_upvar_array dynamic_trace_static dynamic_package dynamic_package_resolution dynamic_colon_names dynamic_read_failure dynamic_exists dynamic_expr dynamic_word_bytes'
for case_name in $dynamic_cases; do
    attach "$case_name-cmp-no" no "${prefix}_$case_name"
    trigger "$case_name-cmp-no" 2 0
    attach "$case_name-clean-cmp-no" no "${prefix}_runtime_cleanup_ascii"
    trigger "$case_name-clean-cmp-no" 1 0:0
    attach "$case_name-cmp-yes" yes "${prefix}_$case_name"
    trigger "$case_name-cmp-yes" 128 0,1,2,3
    attach "$case_name-clean-cmp-yes" yes "${prefix}_runtime_cleanup_ascii"
    trigger "$case_name-clean-cmp-yes" 128 0:0,0:1,0:2,0:3
done

# Forwarded-flow event frame control (the response is supplied by the backend).
attach event-frames-cmp-no no "${prefix}_event_frames"
trigger event-frames-cmp-no 8 ''
attach event-frames-cmp-yes yes "${prefix}_event_frames"
trigger event-frames-cmp-yes 128 ''

# Leave the exact cleanup responder attached for final runtime cleanup.
attach final-runtime-cleanup yes "${prefix}_runtime_cleanup_ascii"
trigger final-runtime-cleanup 128 0:0,0:1,0:2,0:3
