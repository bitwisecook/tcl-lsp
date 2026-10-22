#!/bin/bash
# usage: cycle2.sh <tag> <filters...> : build, run probes (nocapture) then filters, delete exe — one command
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol49
tag=$1; shift
cd /home/user/tcl-lsp
for i in $(seq 1 6); do
  exe=$($S/build.sh $S/$tag-build.log -p tcl-compiler --lib) && break
  grep -q "SIGKILL\|extern location\|\.rlib" $S/$tag-build.log || { echo BUILD-ERROR; grep -E "^(error|warning)" -A12 $S/$tag-build.log | head -60; exit 1; }
  sleep 60
done
[ -n "$exe" ] || { echo BUILD-GAVE-UP; exit 1; }
echo "BUILT $exe"
(cd rust/tcl-compiler && ZZ_SOL49_ANY=1 TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS=1 TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh timeout 600 $exe zz_sol49 --nocapture --test-threads 1 > $S/$tag-probe.log 2>&1)
echo "PROBE done: $(wc -l < $S/$tag-probe.log) lines"
THREADS=${THREADS:-3} $S/batch.sh $exe $tag "$@"
rm -f "$exe"
echo "DONE (exe deleted)"
