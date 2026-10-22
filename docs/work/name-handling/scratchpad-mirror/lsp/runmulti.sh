#!/bin/bash
# usage: runmulti.sh <pkg> <tag> <target>...   (integration test targets)
set -u
cd /home/user/tcl-lsp
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/lsp
pkg=$1; tag=$2; shift 2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh
avail=$(df -BM --output=avail / | tail -1 | tr -dc 0-9)
echo "disk avail ${avail}M"
if [ "$avail" -lt 1900 ]; then echo "DISK TOO LOW"; exit 3; fi
sel=""; for t in "$@"; do sel="$sel --test $t"; done
cargo test -p $pkg $sel --no-run --message-format=json 2>$S/build-$tag.err > $S/build-$tag.json
rc=$?
python3 -c 'import json,sys
for l in open(sys.argv[1]):
    try: m=json.loads(l)
    except Exception: continue
    if m.get("reason")=="compiler-message" and m["message"].get("level") in ("error","warning"): print(m["message"]["rendered"])' $S/build-$tag.json > $S/diag-$tag.txt
grep -o '"executable":"[^"]*"' $S/build-$tag.json | grep -v null | cut -d'"' -f4 > $S/exes-$tag.txt
rm -f $S/build-$tag.json
if [ $rc -ne 0 ]; then echo "BUILD FAILED rc=$rc"; head -80 $S/diag-$tag.txt; xargs -r rm -f < $S/exes-$tag.txt; exit 2; fi
cd /home/user/tcl-lsp/rust/$pkg
while read exe; do
  name=$(basename $exe | sed 's/-[0-9a-f]*$//')
  extra=""; case $name in zz_*) extra="--nocapture";; esac
  timeout ${TIMEOUT:-3000} $exe --test-threads=${THREADS:-2} $extra > $S/out-$tag-$name.log 2>&1
  echo "$name exit $?"
  rm -f "$exe"
  grep -E "^test .* \.\.\. (ok|FAILED|ignored)" $S/out-$tag-$name.log | sed -E 's/^test (.*) \.\.\. (.*)$/\2 \1/' | sort > $S/status-$tag-$name.txt
  echo "$name ok=$(grep -c '^ok ' $S/status-$tag-$name.txt) fail=$(grep -c '^FAILED ' $S/status-$tag-$name.txt)"
done < $S/exes-$tag.txt
