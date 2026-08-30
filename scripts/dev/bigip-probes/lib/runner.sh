#!/bin/bash
PB=/var/tmp/pb; rm -rf $PB; mkdir -p $PB
awk -v dir="$PB" '
/^@@ /{ line=substr($0,4); split(line,a,/\|/);
        id=a[1]; gsub(/^ +| +$/,"",id); mode=a[2]; gsub(/^ +| +$/,"",mode); desc=a[3]; gsub(/^ +| +$/,"",desc);
        print id"~~"mode"~~"desc >> (dir"/index.txt"); cur=dir"/"id".body"; next }
{ if(cur!="") print $0 >> cur }
' /var/tmp/probes.txt
: > /var/tmp/warnings.txt
while read -r line; do
  id=${line%%~~*}; rest=${line#*~~}; mode=${rest%%~~*}; desc=${rest#*~~}
  B=$PB/$id.body; C=$PB/$id.conf
  if [ "$mode" = "RAW" ]; then { echo "ltm rule probe_$id {"; cat $B; echo "}"; } > $C
  else { echo "ltm rule probe_$id {"; echo "when HTTP_REQUEST {"; cat $B; echo "}"; echo "}"; } > $C; fi
  OUTP=$(tmsh load sys config merge file $C 2>&1)
  if tmsh list ltm rule probe_$id >/dev/null 2>&1; then CREATED=yes; else CREATED=no; fi
  MSG=$(echo "$OUTP"|grep -v "^Loading configuration\|^  /var/tmp"|tr '\n' ' ')
  if [ "$CREATED" = "yes" ] && [ -z "$MSG" ]; then VERD=ACCEPT
  elif [ "$CREATED" = "yes" ]; then VERD=WARN
  else VERD=REJECT; fi
  [ -n "$MSG" ] && echo "### $id ($VERD): $MSG" >> /var/tmp/warnings.txt
  tmsh delete ltm rule probe_$id >/dev/null 2>&1
  T84=$(timeout 5 tclsh8.4 /var/tmp/tclcheck.tcl $B 2>&1|head -1); T84M=${T84#*~~}; T84=${T84%%~~*}
  echo "$id~~$mode~~$VERD~~$T84~~$T84M~~$desc"
done < $PB/index.txt
