#!/usr/bin/env bash
# Expand a .probes suite into one iRule .conf per probe, exactly as lib/runner.sh
# does on the appliance. Usage: materialise.sh <suite.probes> <outdir>
set -euo pipefail
SUITE="$1"; OUT="$2"; mkdir -p "$OUT"; rm -f "$OUT"/*.conf "$OUT"/index.txt
awk -v dir="$OUT" '
/^@@ /{ line=substr($0,4); split(line,a,/\|/);
        id=a[1]; gsub(/^ +| +$/,"",id); mode=a[2]; gsub(/^ +| +$/,"",mode); desc=a[3]; gsub(/^ +| +$/,"",desc);
        print id"~~"mode"~~"desc >> (dir"/index.txt"); cur=dir"/"id".body"; next }
{ if(cur!="") print $0 >> cur }
' "$SUITE"
while read -r line; do
  id=${line%%~~*}; rest=${line#*~~}; mode=${rest%%~~*}
  B="$OUT/$id.body"; C="$OUT/$id.conf"
  if [ "$mode" = "RAW" ]; then { echo "ltm rule probe_$id {"; cat "$B"; echo "}"; } > "$C"
  else { echo "ltm rule probe_$id {"; echo "when HTTP_REQUEST {"; cat "$B"; echo "}"; echo "}"; } > "$C"; fi
  rm -f "$B"
done < "$OUT/index.txt"
echo "$(ls "$OUT"/*.conf | wc -l) iRules -> $OUT"
