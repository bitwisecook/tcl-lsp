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

set -u

if [ "$#" -ne 2 ]; then
    echo "usage: $0 FIXTURE_DIR EVIDENCE_DIR" >&2
    exit 2
fi

fixture_dir=$1
evidence_dir=$2
results=$evidence_dir/event-load-results.tsv
details=$evidence_dir/event-load-details

if ! cd "$fixture_dir"; then
    exit 2
fi
if ! sha256sum -c SHA256SUMS > "$evidence_dir/event-load-precheck.txt" 2>&1; then
    echo "fixture hash verification failed" >&2
    exit 1
fi

mkdir -p "$details"
printf 'event\tsha256\trc\taccepted\n' > "$results"

for path in event-load/*.conf; do
    event=${path#event-load/}
    event=${event%.conf}
    sha256=$(sha256sum "$path" | cut -d ' ' -f 1)
    output=$details/$event.txt
    tmsh load sys config merge file "$fixture_dir/$path" > "$output" 2>&1
    rc=$?
    if [ "$rc" -eq 0 ]; then
        accepted=yes
    else
        accepted=no
    fi
    printf '%s\t%s\t%s\t%s\n' "$event" "$sha256" "$rc" "$accepted" >> "$results"
done

printf 'accepted\t'
awk -F '\t' 'NR > 1 && $4 == "yes" { count++ } END { print count + 0 }' "$results"
printf 'rejected\t'
awk -F '\t' 'NR > 1 && $4 == "no" { count++ } END { print count + 0 }' "$results"
