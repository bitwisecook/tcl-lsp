#!/bin/bash
# Drive all repro runs for issue #2021, strictly one at a time (CPU numbers
# must not be polluted by a concurrent server).
set -u
DIR="$(cd "$(dirname "$0")" && pwd)"
PY=python3
JSON="$DIR/runs.jsonl"
LOG="$DIR/runs.log"
: > "$JSON"
: > "$LOG"

run() {
  echo "" | tee -a "$LOG"
  echo "############################################################" | tee -a "$LOG"
  echo "# $*" | tee -a "$LOG"
  echo "############################################################" | tee -a "$LOG"
  "$PY" "$DIR/orphan_repro.py" --json "$JSON" --outdir "$DIR" "$@" 2>&1 | tee -a "$LOG"
  # make sure nothing from this run is still around before the next one
  sleep 2
}

for at in midscan settled; do
  for sc in clean eof shutdown-eof exit-stdout-open; do
    run --scenario "$sc" --at "$at"
  done
done

run --scenario eof --at settled --workspace "$DIR/../../tmp/tcl8.6.18/library" --tag tcl86

echo "" | tee -a "$LOG"
echo "ALL RUNS COMPLETE" | tee -a "$LOG"
pgrep -a tcl-lsp-server | tee -a "$LOG" || echo "no tcl-lsp-server processes left" | tee -a "$LOG"
