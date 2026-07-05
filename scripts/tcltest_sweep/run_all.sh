#!/bin/bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Drive ``run_one.sh`` across every name in ``names.txt`` (one
# per line), writing JSON records to the given output file.
#
# Usage:
#   bash scripts/tcltest_sweep/run_all.sh <names_file> <out_path> [timeout_secs]
#
# ``names_file`` defaults to ``scripts/tcltest_sweep/names.txt``
# (the full ``_IN_SCOPE`` list mirrored here for reproducibility);
# ``out_path`` defaults to ``/tmp/tcltest-sweep.ndjson``;
# ``timeout_secs`` defaults to 30.
#
# Each bundle is run in a fresh ``uv run pytest`` subprocess so a
# single hung test can't take the whole sweep down with it.  The
# wrapper prints a one-line progress marker to stderr per
# invocation so callers can tail the log.

set -u

script_dir=$(cd "$(dirname "$0")" && pwd)
names_file=${1:-"$script_dir/names.txt"}
out_path=${2:-/tmp/tcltest-sweep.ndjson}
tmout=${3:-30}

if [ ! -f "$names_file" ]; then
  echo "names file not found: $names_file" >&2
  exit 1
fi

> "$out_path"
i=0
total=$(wc -l < "$names_file")
while IFS= read -r name; do
  [ -z "$name" ] && continue
  i=$((i+1))
  echo "[$i/$total] $name" >&2
  bash "$script_dir/run_one.sh" "$name" "$tmout" >> "$out_path"
done < "$names_file"
echo "done — $out_path has $(grep -c '"name"' "$out_path") records" >&2
