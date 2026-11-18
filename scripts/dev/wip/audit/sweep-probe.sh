#!/bin/bash
# usage: sweep.sh <outname>
cd /home/user/tcl-lsp
source scripts/dev/agent-build-env.sh
export CARGO_INCREMENTAL=0
S=/tmp/claude-0/-home-user-tcl-lsp/95d74e4c-e1a2-5809-ae6c-93553558eeec/scratchpad
ARGS=()
for c in $(cat $S/codes.txt); do ARGS+=(--code "$c"); done
for d in /tmp/claude-0/-home-user-tcl-lsp/95d74e4c-e1a2-5809-ae6c-93553558eeec/scratchpad/probe; do ARGS+=(--corpus "$d"); done
cargo xtask fp-sweep "${ARGS[@]}" --examples 1000000 > $S/sweep-$1.txt 2>$S/sweep-$1.err
echo "exit $?" >> $S/sweep-$1.err
