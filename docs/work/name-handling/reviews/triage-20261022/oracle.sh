#!/bin/bash
# Usage: oracle.sh /absolute/path/to/file.tcl
set -u
f="$(realpath "$1")"
d="$(dirname "$f")"
jim="${TCL_LSP_JIMSH:-$(git rev-parse --show-toplevel)/tmp/jimtcl-oracle/jimsh}"
for t in "${TCL_LSP_TCLSH84:-tclsh8.4}" "${TCL_LSP_TCLSH85:-tclsh8.5}" "${TCL_LSP_TCLSH86:-tclsh8.6}" "${TCL_LSP_TCLSH90:-tclsh9.0}" "${TCL_LSP_TCLSH91:-tclsh9.1}" "$jim"; do
  capture="$(mktemp)"
  (cd "$d" && timeout 10 "$t" "$f") > "$capture" 2>&1
  status=$?
  printf '  %s exit=%s: ' "$(basename "$t")" "$status"
  tr '\n' '|' < "$capture"
  printf '\n'
  rm -f "$capture"
done
