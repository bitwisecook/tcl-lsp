#!/usr/bin/env bash
# tclsh_check.sh — evaluate a Tcl snippet against every available reference
# tclsh (8.4, 8.5, 8.6, 9.0) so test expectations can be pinned to C-Tcl ground
# truth. Used when authoring Rust unit tests: derive/verify the expected value
# from real tclsh rather than from assumption.
#
# Usage:
#   scripts/dev/tclsh_check.sh 'expr {9 / 2}'          # run a one-liner in each tclsh
#   scripts/dev/tclsh_check.sh -f path/to/snippet.tcl  # run a whole script file
#   echo 'puts [llength {a b c}]' | scripts/dev/tclsh_check.sh -   # read from stdin
#
# Prints, per version found on PATH, the stdout (or the error message on a
# non-zero Tcl return), so 8.x-vs-9.0 divergences are obvious.
set -u

run_one() {
  local bin="$1" script="$2"
  if ! command -v "$bin" >/dev/null 2>&1; then
    printf '%-10s (not installed)\n' "$bin:"
    return
  fi
  # catch so a Tcl error is reported as text rather than a crash.
  local out
  out=$("$bin" <<TCL 2>&1
set _code [catch {
$script
} _res]
if {\$_code == 0} {
  if {[info exists _res] && \$_res ne ""} { puts \$_res }
} else {
  puts "ERR: \$_res"
}
TCL
)
  printf '%-10s %s\n' "$bin:" "$out"
}

main() {
  local script
  if [[ "${1:-}" == "-f" ]]; then
    script=$(cat "$2")
  elif [[ "${1:-}" == "-" ]]; then
    script=$(cat)
  else
    script="$*"
  fi
  for v in tclsh8.4 tclsh8.5 tclsh8.6 tclsh9.0; do
    run_one "$v" "$script"
  done
}

main "$@"
