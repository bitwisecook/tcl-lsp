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

# Run on the isolated appliance after continuous /var/log/ltm capture starts.
set -uo pipefail
umask 077
run=${1:?run identifier}
evidence=${2:?absolute appliance evidence directory}
fixtures=${3:?absolute fixture directory}
lab_config=${4:?absolute pool and virtual configuration}
trigger_base=${5:?dev traffic-control URL}
[[ $run =~ ^[A-Za-z][A-Za-z0-9_]{0,15}$ ]] || exit 2
[[ -f $fixtures/rules.tsv && -f $fixtures/manifest.json && -f $fixtures/objects.json ]] || exit 2
[[ -f $lab_config ]] || exit 2
mkdir -p "$evidence/load" "$evidence/attachments" "$evidence/triggers"

prefix="__tcl_lsp_probe_2286_${run}"
partition="R2286_${run}"
common_a="/Common/${prefix}_fa"
common_b="/Common/${prefix}_fb"
part_root="/${partition}"
part_a="${part_root}/${prefix}_fa"
part_b="${part_root}/${prefix}_fb"
part_nested="${part_a}/${prefix}_nested"
vip="/Common/${prefix}_vs"
pool="/Common/${prefix}_pool"
port=30000

status_row() {
    printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$evidence/status.tsv"
}

exists_rule() {
    tmsh list ltm rule "$1" >/dev/null 2>&1
}

load_case() {
    case_name=$1
    file="$fixtures/$case_name.conf"
    object=$(awk -F '\t' -v file="$case_name.conf" '$1 == file {print $2}' "$fixtures/rules.tsv")
    tmsh load sys config merge file "$file" > "$evidence/load/$case_name.log" 2>&1
    rc=$?
    present=no
    if exists_rule "$object"; then
        present=yes
        tmsh list ltm rule "$object" all-properties > "$evidence/load/$case_name.created.conf" 2>&1
    fi
    status_row load "$case_name" "rc=$rc,present=$present"
    return "$rc"
}

delete_case() {
    case_name=$1
    object=$(awk -F '\t' -v file="$case_name.conf" '$1 == file {print $2}' "$fixtures/rules.tsv")
    if exists_rule "$object"; then
        tmsh delete ltm rule "$object" > "$evidence/load/$case_name.delete.log" 2>&1
        status_row delete "$case_name" "rc=$?"
    else
        status_row delete "$case_name" absent
    fi
}

attach() {
    label=$1
    rule=$2
    {
        tmsh modify ltm virtual "$vip" cmp-enabled yes rules \{ "$rule" \}
        modify_rc=$?
        printf 'modify_status=%s\n' "$modify_rc"
        tmsh list ltm virtual "$vip" all-properties
        tmsh show ltm virtual "$vip" detail
    } > "$evidence/attachments/$label.txt" 2>&1
    rc=$modify_rc
    status_row attach "$label" "rc=$rc,rule=$rule"
    return "$rc"
}

trigger() {
    label=$1
    requests=${2:-128}
    url="${trigger_base}?label=${label}&requests=${requests}&source_port=${port}&expect=0,1,2,3&path=/%3Faction%3Dread"
    curl -fsS "$url" > "$evidence/triggers/$label.json" 2>&1
    rc=$?
    status_row trigger "$label" "rc=$rc,source_port=$port"
    port=$((port + requests + 4))
    return "$rc"
}

: > "$evidence/status.tsv"
{
    tmsh show sys version
    tmsh show sys failover
    tmsh show sys tmm-info
    tmsh list auth partition
    tmsh list sys folder recursive
} > "$evidence/inventory-before.txt" 2>&1

{
    for object in "$vip" "$pool" "$common_a" "$common_b" "$part_root"; do
        printf 'OBJECT %s\n' "$object"
        tmsh list ltm virtual "$object" 2>&1 || true
        tmsh list ltm pool "$object" 2>&1 || true
        tmsh list sys folder "$object" 2>&1 || true
        tmsh list auth partition "${object#/}" 2>&1 || true
    done
    while IFS=$'\t' read -r _ object; do
        printf 'RULE %s\n' "$object"
        tmsh list ltm rule "$object" 2>&1 || true
    done < "$fixtures/rules.tsv"
    printf 'NODE /Common/192.168.9.80\n'
    tmsh list ltm node /Common/192.168.9.80 2>&1 || true
} > "$evidence/precreate-absence.txt"

if tmsh list auth partition "$partition" >/dev/null 2>&1; then
    echo "owned partition already exists" >&2
    exit 3
fi
if tmsh list ltm virtual "$vip" >/dev/null 2>&1 || \
    tmsh list ltm pool "$pool" >/dev/null 2>&1 || \
    tmsh list ltm node /Common/192.168.9.80 >/dev/null 2>&1 || \
    tmsh list sys folder "$common_a" >/dev/null 2>&1 || \
    tmsh list sys folder "$common_b" >/dev/null 2>&1; then
    echo "owned lab object already exists" >&2
    exit 3
fi
while IFS=$'\t' read -r _ object; do
    if exists_rule "$object"; then
        echo "owned rule already exists: $object" >&2
        exit 3
    fi
done < "$fixtures/rules.tsv"
tmsh create auth partition "$partition" default-route-domain 0 > "$evidence/create-partition.log" 2>&1
for folder in "$common_a" "$common_b" "$part_a" "$part_b" "$part_nested"; do
    tmsh create sys folder "$folder" >> "$evidence/create-folders.log" 2>&1
done

# Callers load before providers to prove that dynamic call targets can be late-bound.
for case_name in caller_common_root caller_common_a caller_partition_root caller_partition_a caller_partition_nested unicode_dynamic; do
    load_case "$case_name" || exit 4
done
for case_name in unicode_literal_precomposed unicode_literal_decomposed \
    unicode_literal_emoji_grinning unicode_literal_emoji_text_vs \
    unicode_literal_emoji_skin_tone unicode_literal_emoji_zwj \
    unicode_literal_emoji_family unicode_literal_emoji_flag; do
    load_case "$case_name" || true
done

tmsh load sys config merge file "$lab_config" > "$evidence/create-lab.log" 2>&1 || exit 5
tmsh list ltm virtual "$vip" all-properties > "$evidence/lab-created-config.txt" 2>&1
tmsh list ltm pool "$pool" all-properties >> "$evidence/lab-created-config.txt" 2>&1
attach pre-provider-common-root "/Common/${prefix}_caller" && trigger pre-provider-common-root

provider_order='provider_common_root provider_common_a provider_common_b provider_partition_root provider_partition_a provider_partition_b provider_partition_nested'
for case_name in $provider_order; do
    load_case "$case_name" || exit 6
done

run_callers() {
    suffix=$1
    while IFS='|' read -r label rule; do
        if attach "${label}-${suffix}" "$rule"; then
            trigger "${label}-${suffix}"
        fi
    done <<EOF
common-root|/Common/${prefix}_caller
common-a|${common_a}/${prefix}_caller
partition-root|${part_root}/${prefix}_caller
partition-a|${part_a}/${prefix}_caller
partition-nested|${part_nested}/${prefix}_caller
unicode-dynamic|/Common/${prefix}_unicode_dynamic
EOF
}
run_callers providers-forward

# Remove and recreate one provider while the caller remains unchanged.
attach common-root-before-provider-delete "/Common/${prefix}_caller" && trigger common-root-before-provider-delete
delete_case provider_common_root
trigger common-root-provider-deleted
load_case provider_common_root || exit 7
trigger common-root-provider-recreated

# Recreate every provider in reverse order, preserving the compiled callers.
for case_name in $provider_order; do delete_case "$case_name"; done
for case_name in provider_partition_nested provider_partition_b provider_partition_a \
    provider_partition_root provider_common_b provider_common_a provider_common_root; do
    load_case "$case_name" || exit 8
done
run_callers providers-reverse

{
    tmsh list auth partition "$partition"
    tmsh list sys folder "$common_a" "$common_b" "$part_a" "$part_b" "$part_nested"
    tmsh list ltm virtual "$vip" all-properties
    tmsh list ltm pool "$pool" all-properties
    while IFS=$'\t' read -r _ object; do
        tmsh list ltm rule "$object" all-properties 2>&1 || true
    done < "$fixtures/rules.tsv"
} > "$evidence/config-before-cleanup.txt" 2>&1

# Remove only the exact owned objects and verify every absence.
tmsh delete ltm virtual "$vip" > "$evidence/delete-virtual.log" 2>&1
tmsh delete ltm pool "$pool" > "$evidence/delete-pool.log" 2>&1
while IFS=$'\t' read -r file _; do delete_case "${file%.conf}"; done < "$fixtures/rules.tsv"
if tmsh list ltm node /Common/192.168.9.80 >/dev/null 2>&1; then
    tmsh delete ltm node /Common/192.168.9.80 > "$evidence/delete-node.log" 2>&1
fi
for folder in "$part_nested" "$part_b" "$part_a" "$common_b" "$common_a"; do
    tmsh delete sys folder "$folder" >> "$evidence/delete-folders.log" 2>&1
done
tmsh delete auth partition "$partition" > "$evidence/delete-partition.log" 2>&1

{
    for object in "$vip" "$pool" "$common_a" "$common_b" "$part_root"; do
        printf 'OBJECT %s\n' "$object"
        tmsh list ltm virtual "$object" 2>&1 || true
        tmsh list ltm pool "$object" 2>&1 || true
        tmsh list sys folder "$object" 2>&1 || true
        tmsh list auth partition "${object#/}" 2>&1 || true
    done
    while IFS=$'\t' read -r _ object; do
        printf 'RULE %s\n' "$object"
        tmsh list ltm rule "$object" 2>&1 || true
    done < "$fixtures/rules.tsv"
    printf 'NODE /Common/192.168.9.80\n'
    tmsh list ltm node /Common/192.168.9.80 2>&1 || true
} > "$evidence/cleanup-verification.txt"
