#!/bin/bash
# usage: cycle.sh <tag> <filters...> : build tcl-compiler lib tests, run filters, delete the exe — one command
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol49
tag=$1; shift
cd /home/user/tcl-lsp
for i in $(seq 1 6); do
  exe=$($S/build.sh $S/$tag-build.log -p tcl-compiler --lib) && break
  grep -q "SIGKILL\|could not compile .tcl-lsp-core" $S/$tag-build.log || { echo BUILD-ERROR; grep -E "^(error|warning)" -A12 $S/$tag-build.log | head -60; exit 1; }
  sleep 120
done
[ -n "$exe" ] || { echo BUILD-GAVE-UP; exit 1; }
echo "BUILT $exe"
THREADS=${THREADS:-3} $S/batch.sh $exe $tag "$@"
rm -f "$exe"
echo "DONE (exe deleted)"
