#!/bin/bash
# Off-box: push a directory of evmatrix rules, run them through one tmsh
# process on the appliance, fetch the output over SSH, then clean up.
# Usage: evmatrix-run.sh <rules-dir> <local-output-file>
# Needs BIGIP_SSH (command prefix running a remote command as root).
set -eu
DIR=$1 OUT=$2
R=/var/tmp/tcllsp_oq_evm
COPYFILE_DISABLE=1 tar --no-xattrs -C "$DIR" -cf - . | $BIGIP_SSH "rm -rf $R && mkdir -p $R && tar -C $R -xf -"
$BIGIP_SSH "cd $R && awk -F'\t' '{printf \"run util bash -c \\\"echo === %s %s\\\"\nload sys config merge file $R/%s.conf\ndelete ltm rule tcllsp_oq_%s\n\", \$1, \$2, \$1, \$1}' index.tsv > batch.tmsh && tmsh < batch.tmsh > out.txt 2>&1; echo markers=\$(grep -c '^===' out.txt) left=\$(tmsh list ltm rule 2>/dev/null | grep -c '^ltm rule tcllsp_oq_ev_')" >&2
$BIGIP_SSH "cat $R/out.txt" > "$OUT"
[ -s "$OUT" ] && $BIGIP_SSH "rm -rf $R"
