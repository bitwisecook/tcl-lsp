#!/bin/bash
# usage: build.sh <log> <cargo test args...>   ; retries while tcl-vm (Sol-46) is mid-edit
log=$1; shift
cd /home/user/tcl-lsp/rust
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
for i in $(seq 1 40); do
  cargo test "$@" --no-run --message-format=short > $log 2>&1 && { grep -o "(/home/user/tcl-lsp/target/debug/deps/[^)]*)" $log | tr -d '()' ; exit 0; }
  if grep -qE "could not compile \`(tcl-vm|tcl-cmd-core|tcl-engine-tclvm)" $log && ! grep -qE "could not compile \`(tcl-compiler|tcl-registry|tcl-lsp|tcl-syntax)" $log; then
    echo "retry $i: foreign crate mid-edit" >&2; sleep 45; continue
  fi
  echo "BUILD FAILED" >&2; grep -E "^(error|warning)" $log | head -20 >&2; exit 1
done
exit 2
