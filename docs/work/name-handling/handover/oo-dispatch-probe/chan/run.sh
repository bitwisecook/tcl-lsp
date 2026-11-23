#!/bin/bash
# Usage: run.sh FIXTURE_DIR [glob]  -> writes engines.tsv in FIXTURE_DIR. Runs each engine in a fresh temp dir.
set -u
dir=$(cd "$1" && pwd); pat=${2:-*.tcl}
R=/home/user/tcl-lsp
out=$dir/engines.tsv
printf 'fixture\tengine\toutput\n' > "$out"
export LANG=C.UTF-8 LC_ALL=C.UTF-8
run() { # label cmd...
  local t; t=$(mktemp -d)
  local o; o=$(cd "$t" && timeout 20 "$@" </dev/null 2>&1; echo "[rc=$?]")
  rm -rf "$t"
  o=$(printf '%s' "$o" | LC_ALL=C sed 's/\t/\\t/g' | LC_ALL=C tr '\n' '|' | LC_ALL=C tr -d '\r')
  printf '%s\t%s\t%s\n' "$fx" "$label" "$o" >> "$out"
}
for f in $(cd "$dir" && ls $pat); do
  fx=$f; p=$dir/$f
  for v in 8.4 8.5 8.6 9.0 9.1; do label=tclsh$v; run tclsh$v "$p"; done
  label=jim; run $R/tmp/jimtcl-oracle/jimsh "$p"
  label=vm-8.6; TCL_LIBRARY=$R/tmp/tcl8.6.18/library run $R/target/debug/tclvm --tcl-version 8.6 "$p"
  label=vm-9.0; TCL_LIBRARY=$R/tmp/tcl9.0.4/library run $R/target/debug/tclvm --tcl-version 9.0 "$p"
  label=rt-8.6; TCL_LIBRARY=$R/tmp/tcl8.6.18/library run $R/target/debug/examples/run_script --quiet --tcl-version 8.6 "$p"
  label=rt-9.0; TCL_LIBRARY=$R/tmp/tcl9.0.4/library run $R/target/debug/examples/run_script --quiet --tcl-version 9.0 "$p"
done
