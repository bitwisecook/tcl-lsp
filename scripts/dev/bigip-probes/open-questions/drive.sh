#!/bin/bash
# Off-box driver: copies generated cases to the appliance, then for each case
# loads it, sends its traffic from this host, and collects + cleans up.
#
# Usage: drive.sh <cases-dir> [glob]
# Needs BIGIP_SSH (a command prefix that runs a remote command as root).
# BIGIP_REMOTE_DIR overrides the appliance working directory.
set -u
CASES=$1 GLOB=${2:-*}
R=${BIGIP_REMOTE_DIR:-/var/tmp/tcllsp_oq}
HERE=$(cd "$(dirname "$0")" && pwd)
$BIGIP_SSH "rm -rf $R && mkdir -p $R"
COPYFILE_DISABLE=1 tar --no-xattrs -C "$CASES" -cf - . | $BIGIP_SSH "tar -C $R -xf -"
$BIGIP_SSH "cat > $R/run.sh" < "$HERE/run.sh"
for conf in $(cd "$CASES" && ls $GLOB.conf | sort); do
  id=${conf%.conf}
  out=$($BIGIP_SSH "bash $R/run.sh load $R $id" 2>/dev/null)
  echo "$out" | grep -v '^OFFSET'
  off=$(echo "$out" | sed -n 's/^OFFSET~~//p')
  [ -z "$off" ] && continue
  sleep 2
  if [ -f "$CASES/$id.traffic" ]; then
    echo "TRAFFIC~~$id~~$(bash "$CASES/$id.traffic" 2>&1 | tr '\n' ' ' | cut -c1-300)"
    sleep 2
  fi
  $BIGIP_SSH "bash $R/run.sh collect $R $id $off" 2>/dev/null
done
$BIGIP_SSH "rm -rf $R"
