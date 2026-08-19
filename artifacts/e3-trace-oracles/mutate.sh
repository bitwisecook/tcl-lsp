#!/usr/bin/env bash
# Mutation harness: apply one mutation, run its test target, expect FAILURE.
# A mutant that still passes is a SURVIVOR = unpinned behaviour.
# usage: mutate.sh <id> <file> <from-file> <to-file> <scope>
set -u
W=/home/user/tcl-lsp/.claude/worktrees/agent-a8f198553f1c371f0
export CARGO_TARGET_DIR=$W/target
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=2
export TCL_LSP_TCLSH86=/home/user/tcl-lsp/tmp/tcl8616-install/bin/tclsh8.6

ID="$1"; FILE="$2"; FROMF="$3"; TOF="$4"; SCOPE="$5"

avail=$(df --output=avail / | tail -1)
if [ "$avail" -lt 4194304 ]; then echo "$ID ABORT-DISK"; exit 9; fi

cd "$W"
# Apply. Abort loudly if the anchor is absent or ambiguous — a silently
# unapplied mutation would run the pristine tree and be scored a survivor.
if ! python3 - "$FILE" "$FROMF" "$TOF" <<'PY'
import sys
path, fromf, tof = sys.argv[1:4]
src = open(path).read()
frm = open(fromf).read()
to = open(tof).read()
n = src.count(frm)
if n != 1:
    sys.stderr.write(f"anchor count = {n}, want exactly 1\n")
    sys.exit(1)
open(path, "w").write(src.replace(frm, to, 1))
PY
then
    echo "$ID ANCHOR-ERROR (mutation NOT applied - result would be meaningless)"
    git checkout -- "$FILE"
    exit 8
fi

if [ "$SCOPE" = "runtime" ]; then
    ( cd "$W/runtime/rust" && cargo test --no-fail-fast ) > "/tmp/mut-$ID.log" 2>&1
else
    cargo test $SCOPE --no-fail-fast > "/tmp/mut-$ID.log" 2>&1
fi
rc=$?
git checkout -- "$FILE"

if [ $rc -ne 0 ]; then
    n=$(grep -cE "^test .* FAILED$" "/tmp/mut-$ID.log")
    if [ "$n" -eq 0 ]; then
        # Non-zero without a failing test = the mutant did not compile.
        echo "$ID BUILD-FAIL (mutant invalid, rewrite it)"
    else
        first=$(grep -E "^test .* FAILED$" "/tmp/mut-$ID.log" | head -1 | sed 's/^test //; s/ \.\.\. FAILED$//')
        echo "$ID KILLED by $n test(s), first: $first"
    fi
else
    echo "$ID *** SURVIVOR ***"
fi
