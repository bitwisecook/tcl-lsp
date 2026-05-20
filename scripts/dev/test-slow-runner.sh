#!/usr/bin/env bash
# Orchestrate the test-slow gate so that:
#   * every phase runs even when an earlier one fails (so the summary
#     lists ALL failures, not just the first),
#   * full per-phase output is preserved on stdout (redirect to a file
#     for later analysis),
#   * a consolidated PASS/FAIL summary is printed at the very END so a
#     `| tail` can't hide failures, and
#   * the script exits non-zero if any phase failed.
#
# Usage:
#   test-slow-runner.sh --serial "t1 t2" --parallel "t3 t4 t5"
#
# Serial targets run first, one at a time, in order.  Parallel targets
# then run concurrently (up to $NPROC), each captured to its own log and
# replayed to stdout in declaration order so the combined output stays
# readable.  Environment (e.g. SKIP_TEST_VM) is inherited by each target.

set -u -o pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MAKE="${MAKE:-make}"
NPROC="${NPROC:-$(nproc 2>/dev/null || echo 4)}"
LOGDIR="$ROOT/tmp/test-slow-logs"

SERIAL=""
PARALLEL=""
while [ $# -gt 0 ]; do
	case "$1" in
		--serial) SERIAL="$2"; shift 2 ;;
		--parallel) PARALLEL="$2"; shift 2 ;;
		*) echo "test-slow-runner: unknown arg: $1" >&2; exit 2 ;;
	esac
done

rm -rf "$LOGDIR"
mkdir -p "$LOGDIR"

PASSED=""
FAILED=""

_record() {  # _record <target> <rc>
	if [ "$2" = "0" ]; then
		PASSED="$PASSED $1"
	else
		FAILED="$FAILED $1"
	fi
}

# Serial phases: stream live to stdout (via tee) and capture rc honestly
# through pipefail.
for tgt in $SERIAL; do
	echo "==================== [test-slow] BEGIN $tgt ===================="
	if "$MAKE" -C "$ROOT" "$tgt" 2>&1 | tee "$LOGDIR/$tgt.log"; then
		_record "$tgt" 0
	else
		_record "$tgt" 1
	fi
	echo "==================== [test-slow] END $tgt ===================="
done

# Parallel batch: each target to its own log + rc file, throttled to
# $NPROC concurrent jobs, then replayed in declaration order.
if [ -n "${PARALLEL// /}" ]; then
	echo "==================== [test-slow] parallel batch ($PARALLEL) ===================="
	running=0
	for tgt in $PARALLEL; do
		(
			"$MAKE" -C "$ROOT" "$tgt" >"$LOGDIR/$tgt.log" 2>&1
			echo $? >"$LOGDIR/$tgt.rc"
		) &
		running=$((running + 1))
		if [ "$running" -ge "$NPROC" ]; then
			wait -n 2>/dev/null || wait
			running=$((running - 1))
		fi
	done
	wait
	for tgt in $PARALLEL; do
		echo "==================== [test-slow] $tgt ===================="
		cat "$LOGDIR/$tgt.log" 2>/dev/null || true
		rc="$(cat "$LOGDIR/$tgt.rc" 2>/dev/null || echo 1)"
		_record "$tgt" "$rc"
	done
fi

echo ""
echo "============================ TEST-SLOW SUMMARY ============================"
for tgt in $PASSED; do
	echo "  PASS  $tgt"
done
for tgt in $FAILED; do
	echo "  FAIL  $tgt"
done
echo "=========================================================================="
if [ -n "${FAILED// /}" ]; then
	echo "test-slow FAILED:${FAILED}"
	echo "  per-target logs under: $LOGDIR/"
	exit 1
fi
echo "test-slow: all phases passed"
