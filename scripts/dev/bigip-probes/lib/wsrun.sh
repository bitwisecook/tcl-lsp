#!/bin/bash
for f in /var/tmp/wsc/*.conf; do
  id=$(basename $f .conf)
  OUTP=$(tmsh load sys config merge file $f 2>&1)
  if tmsh list ltm rule probe_ws_$id >/dev/null 2>&1; then
     sleep 0.4
     RES=$(grep "WSRES $id " /var/log/ltm | tail -1 | sed 's/.*WSRES //')
     echo "$id :: ACCEPT :: $RES"
     tmsh delete ltm rule probe_ws_$id >/dev/null 2>&1
  else
     M=$(echo "$OUTP"|grep -o 'error: \[[^]]*\]'|head -2|tr '\n' ' ')
     [ -z "$M" ] && M=$(echo "$OUTP"|grep -v "^Loading\|^  /var"|head -1)
     echo "$id :: REJECT :: $M"
  fi
done
