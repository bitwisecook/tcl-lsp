#!/bin/bash
# Appliance-side steps for the open-question probes. drive.sh calls these
# over SSH and sends any traffic from off-box between them: on a single-NIC
# VE, traffic the host itself sends to the self address never reaches TMM.
#
#   run.sh load    <dir> <id>   prove absence, merge <id>.conf, classify, print log offset
#   run.sh collect <dir> <id> <offset>
#                               print tagged log lines since offset, delete, prove deletion
#
# Every object is listed in <id>.delete as `tmsh delete <path>` lines.
# Never runs `save sys config`.
set -u
cmd=$1 DIR=$2 id=$3
LOG=/var/log/ltm
del="$DIR/$id.delete"
paths() { sed -n 's/^tmsh delete //p' "$del"; }

case $cmd in
load)
  while read -r path; do
    if tmsh list $path >/dev/null 2>&1; then echo "RESULT~~$id~~SKIP~~$path already exists"; exit 3; fi
  done < <(paths)
  off=$(stat -c %s $LOG)
  out=$(tmsh load sys config merge file "$DIR/$id.conf" 2>&1)
  if tmsh list ltm rule tcllsp_oq_$id >/dev/null 2>&1; then created=yes; else created=no; fi
  msg=$(echo "$out" | grep -v '^Loading configuration\|^  /' | tr '\n' ' ' | sed 's/ *$//')
  if [ $created = yes ] && [ -z "$msg" ]; then v=ACCEPT
  elif [ $created = yes ]; then v=WARN
  else v=REJECT; fi
  echo "RESULT~~$id~~$v~~$msg"
  echo "OFFSET~~$off"
  ;;
collect)
  off=$4
  tail -c +$((off + 1)) $LOG | grep -a -F "TCLLSP_OQ|$id|" | sed 's/^.*TCLLSP_OQ|/LOG~~/' | sort | uniq -c | sed 's/^ *//'
  tail -c +$((off + 1)) $LOG | grep -a -F "tcllsp_oq_$id" | grep -a -v 'TCLLSP_OQ|' | sed "s/^/OTHER~~$id~~/" | head -5
  bash "$del" >/dev/null 2>&1
  while read -r path; do
    if tmsh list $path >/dev/null 2>&1; then echo "LEFTOVER~~$id~~$path"; fi
  done < <(paths)
  ;;
esac
