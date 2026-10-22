#!/usr/bin/env bash
# runbin.sh <binary> <label> [filter...]  -- run a test binary, keep only result lines
set -u
bin=$1; label=$2; shift 2
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/luna41
cd /home/user/tcl-lsp/rust/${CRATE_DIR:-tcl-lsp-core}
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh RUST_BACKTRACE=0
timeout ${TMO:-3000} "$bin" --test-threads ${THREADS:-3} "$@" 2>&1 \
  | awk '/panicked at/{n=6} /^test |^test result|^---- /{print; next} n>0{print; n--}' | cut -c1-600 | head -c 20000000 > "$S/run-$label.log"
grep -aE "^test result" "$S/run-$label.log"
grep -aE "^test .* (FAILED|ignored)$" "$S/run-$label.log" | sed -E 's/^test (.*) \.\.\. (FAILED|ignored).*/\2 \1/' | sort > "$S/fail-$label.txt"
echo "fail/ignored count: $(wc -l < "$S/fail-$label.txt")"
