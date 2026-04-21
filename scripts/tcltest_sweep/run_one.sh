#!/bin/bash
# Run a single Tcl 9 tcltest bundle through ``pytest`` with a
# per-file timeout, capturing the outcome as a JSON record.
#
# Usage:
#   bash scripts/tcltest_sweep/run_one.sh <stem> <timeout_secs>
#
# Output goes to stdout as a JSON object (jq-pretty by default;
# compact NDJSON if the ``-c`` flag is added to the jq call).
# The wrapper classifies the pytest outcome into:
#   pass          — test_runs passed; tcltest reports Failed == 0
#   fail          — test_runs finished but Failed > 0
#   trap          — the WASM module raised a runtime trap
#   compile_fail  — the bundle failed to compile
#   no_summary    — ran without trapping but no tcltest summary line
#   timeout       — the pytest invocation exceeded the budget
#   unknown       — pytest exit code didn't match any of the above
#                   (typically "no tests collected" for a misspelt stem).
#
# Intended to be invoked by ``run_all.sh`` or the KCS harness; see
# ``docs/kcs/kcs-how-to-run-tcltest-bundles.md``.

set -u
name=${1:?"usage: run_one.sh <stem> <timeout_secs>"}
tmout=${2:?"usage: run_one.sh <stem> <timeout_secs>"}

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$repo_root"

# Class name mirrors the dynamic creation in run_tcl9_tests.py
# (hyphens and dots become underscores).
classname=$(echo "$name" | tr '.-' '_')

result=$(timeout "$tmout" uv run pytest "tests/external/run_tcl9_tests.py::TestTcl9_${classname}::test_runs" --no-header -q --tb=line 2>&1)
rc=$?

summary=$(echo "$result" | grep -oE 'Total[[:space:]]+[0-9]+[[:space:]]+Passed[[:space:]]+[0-9]*[[:space:]]+Skipped[[:space:]]+[0-9]*[[:space:]]+Failed[[:space:]]+[0-9]*' | tail -1)
outcome="unknown"
if [ "$rc" = "124" ]; then
  outcome="timeout"
elif echo "$result" | grep -q '1 passed'; then
  outcome="pass"
elif echo "$result" | grep -q 'trapped:'; then
  outcome="trap"
elif echo "$result" | grep -q 'bundle failed to compile'; then
  outcome="compile_fail"
elif echo "$result" | grep -q 'No tcltest summary line'; then
  outcome="no_summary"
elif echo "$result" | grep -q '1 failed'; then
  outcome="fail"
fi
trap_line=$(echo "$result" | grep -E 'tcl trap:' | head -1 | head -c 200)
first_failing=$(echo "$result" | grep -oE '==== [^ ]+ FAILED' | head -1)

jq -n --arg name "$name" \
      --arg classname "$classname" \
      --arg outcome "$outcome" \
      --arg summary "$summary" \
      --arg trap "$trap_line" \
      --arg first "$first_failing" \
      --argjson rc "$rc" \
      '{name: $name, classname: $classname, outcome: $outcome, rc: $rc, summary: $summary, trap: $trap, first_failing: $first}'
