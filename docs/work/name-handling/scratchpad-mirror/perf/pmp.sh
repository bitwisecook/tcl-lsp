#!/bin/bash
# poor man's profiler: pmp.sh <outfile> <samples> <interval> -- cmd...
out=$1; n=$2; iv=$3; shift 4
"$@" > $out.stdout 2>$out.stderr &
pid=$!
sleep 1
: > $out
for i in $(seq 1 $n); do
  kill -0 $pid 2>/dev/null || break
  gdb -p $pid -batch -nx -ex "set pagination off" -ex "bt 200" 2>/dev/null | grep -E '^#' >> $out
  echo "----" >> $out
  sleep $iv
done
kill $pid 2>/dev/null
wait $pid 2>/dev/null
