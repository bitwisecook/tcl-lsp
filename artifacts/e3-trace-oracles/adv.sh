#!/usr/bin/env bash
# Differential probe: tclsh 8.6.16 / tclsh 9.0.4 vs tclvm / runtime-rust.
# usage: adv.sh <script.tcl> [8.6|9.0]
set -u
W=/home/user/tcl-lsp/.claude/worktrees/agent-a8f198553f1c371f0
SCRIPT="$1"
VER="${2:-8.6}"
if [ "$VER" = "8.6" ]; then
    REF=/home/user/tcl-lsp/tmp/tcl8616-install/bin/tclsh8.6
else
    REF=/usr/local/bin/tclsh9.0
fi
"$REF" "$SCRIPT" > /tmp/adv-ref.txt 2>&1
"$W/target/debug/tclvm" --tcl-version "$VER" "$SCRIPT" > /tmp/adv-vm.txt 2>&1
"$W/target/debug/examples/run_script" --quiet --tcl-version "$VER" "$SCRIPT" > /tmp/adv-rt.txt 2>&1
echo "=== tclsh$VER ==="
cat /tmp/adv-ref.txt
echo "=== tclvm diff (ref -> vm) ==="
diff /tmp/adv-ref.txt /tmp/adv-vm.txt && echo "  IDENTICAL"
echo "=== runtime diff (ref -> rt) ==="
diff /tmp/adv-ref.txt /tmp/adv-rt.txt && echo "  IDENTICAL"
