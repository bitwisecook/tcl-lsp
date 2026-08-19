#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Benchmark every version in MANIFEST.toml, in order, into results/.
#
# Run this rather than looping bench.py by hand: the graphs only mean
# anything if every point came from one machine and one harness revision,
# and the easiest way to produce a misleading chart is to top up a few
# versions later with a changed suite. When the check set or the harness
# changes, re-run the whole sweep — that is what this script is for.
#
# Usage: sweep.sh [--scope small] [--out results] [--only 2.1.5,2.1.6]
#                  [--timeout 600] [--bench-deadline 240]
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

scope=small
out=results
only=""
# External backstop only, now that bench.py enforces its own outer
# `--deadline` (issue #1399, default 240s — see bench.py's DEFAULT_DEADLINE_S
# and BENCH_DEADLINE_S). This external `timeout --foreground` still exists
# for the case bench.py's own watchdog cannot cover — the interpreter itself
# wedged, or a `--deadline` misconfigured below the true need — so it stays
# generous rather than tight: a healthy version takes well under a minute,
# but one whose workspace scan never settles burns 120s on that alone.
per_version_timeout=600
# Forwarded to `bench.py --deadline`. Independent knob from the external
# backstop above: this is what actually produces the phase/binary/last-
# request diagnostic on expiry; the external timeout only produces a bare
# "killed after Ns" with no attribution.
bench_deadline=""
while [ $# -gt 0 ]; do
  case "$1" in
    --scope) scope="$2"; shift 2 ;;
    --out)   out="$2";   shift 2 ;;
    --only)  only="$2";  shift 2 ;;
    --timeout) per_version_timeout="$2"; shift 2 ;;
    --bench-deadline) bench_deadline="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

mapfile -t tags < <(python3 - "$only" <<'PY'
import sys, tomllib, pathlib
only = {t.strip() for t in sys.argv[1].split(",") if t.strip()}
m = tomllib.loads(pathlib.Path("MANIFEST.toml").read_text())
def key(t): return tuple(int(p) for p in t.lstrip("v").split("."))
for tag in sorted(m["versions"]["tags"], key=key):
    if not only or tag.lstrip("v") in only or tag in only:
        print(tag)
PY
)

# Fail fast on a missing corpus rather than discovering it sixteen times.
# bench.py reports it clearly, but a sweep that runs every version to produce
# the same setup error for each buries the one line that matters.
if ! python3 fetch_corpus.py --scope "$scope" --verify-only >/dev/null 2>&1; then
  echo "corpus not present or not at its pins for scope '$scope'." >&2
  echo "run: python3 fetch_corpus.py --scope $scope" >&2
  exit 2
fi

mkdir -p "$out"
ok=0; skipped=0; failed=0; timedout=0
for tag in "${tags[@]}"; do
  version="${tag#v}"
  bin="servers/$tag/tcl-lsp-server"
  if [ ! -x "$bin" ]; then
    echo "== $tag: no binary at $bin — run get_server.py --tag $tag; SKIPPING"
    skipped=$((skipped + 1))
    continue
  fi
  echo
  echo "########## $tag ##########"
  # Hard wall-clock cap per version, as a *backstop* behind bench.py's own
  # `--deadline` (issue #1399): a blocking write to a server that had
  # stopped draining stdin used to have no timeout at all anywhere in the
  # stack — one wedged this sweep for 8h45m, client stuck in write(),
  # server idle in read(). `lsp_client.py`'s transport now bounds that write
  # too, and bench.py's own watchdog reports which phase/binary/request died
  # and exits cleanly well inside this window — so a hit here now means
  # bench.py's own deadline didn't fire (misconfigured, or a fault outside
  # even its coverage), not a silent multi-hour wedge.
  bench_args=(
    python3 bench.py --server "$bin" --version "$version"
    --scope "$scope" --out "$out"
  )
  [ -n "$bench_deadline" ] && bench_args+=(--deadline "$bench_deadline")
  if timeout --foreground "$per_version_timeout" "${bench_args[@]}"; then
    ok=$((ok + 1))
  elif [ $? -eq 124 ]; then
    echo "::: $tag TIMED OUT after ${per_version_timeout}s — see the pipe-deadlock note above :::"
    timedout=$((timedout + 1))
  else
    # A non-zero exit here is a harness failure, not a slow check (the
    # gating flag is off): worth counting, not worth aborting the sweep and
    # losing the versions after it.
    echo "::: $tag FAILED :::"
    failed=$((failed + 1))
  fi
done

echo
echo "sweep complete: $ok measured, $skipped skipped (no binary), $failed failed, $timedout timed out"
echo "now run: python3 report.py --results $out"
[ "$failed" -eq 0 ] && [ "$timedout" -eq 0 ]
