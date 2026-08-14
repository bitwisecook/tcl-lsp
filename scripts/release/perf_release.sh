#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# perf_release.sh — measure the release being cut and regenerate the graphs.
#
# Usage:  scripts/release/perf_release.sh X.Y.Z [options]
#         make release-perf V=X.Y.Z
#
# One command, so the release-notes graphs are produced the same way every
# time.  Doing this by hand is how the record drifts: the previous release's
# figures were added to `results/` in a commit of their own, from a machine
# nobody wrote down, with the graphs regenerated separately — and a graph that
# claims to be the release record has to be reproducible from the tree.
#
# What it does, in order:
#
#   1. builds `tcl-lsp-server --release` from the working tree (this is the
#      tree about to be tagged, so it is the right binary to measure);
#   2. reconstitutes the pinned corpus and verifies it against the manifest;
#   3. runs the benchmark suite, writing `scripts/perf/results/X.Y.Z.json`;
#   4. adds `vX.Y.Z` to the manifest's version list, so later sweeps replay it;
#   5. re-renders `scripts/perf/graphs/` with X.Y.Z highlighted as the current
#      release — bright blue, everything before it in ageing grey.
#
# Every one of those is idempotent bar step 3, which refuses to overwrite an
# existing result file without `--force`: a committed measurement is a record,
# and quietly re-measuring it turns a comparison against the past into a
# comparison against this afternoon.
#
# Options:
#   --scope NAME   benchmark scope (default: small — see scripts/perf/README.md)
#   --server PATH  measure an existing binary instead of building one
#   --force        re-measure a version that already has a result file
#   --skip-build   reuse target/release/tcl-lsp-server as-is
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PERF="$ROOT/scripts/perf"

V=""
scope=small
server=""
force=0
skip_build=0

while [ $# -gt 0 ]; do
    case "$1" in
        --scope) scope="$2"; shift 2 ;;
        --server) server="$2"; shift 2 ;;
        --force) force=1; shift ;;
        --skip-build) skip_build=1; shift ;;
        -h|--help)
            cat <<'EOF'
Usage: scripts/release/perf_release.sh X.Y.Z [options]

Build the server from this tree, benchmark it against the pinned corpus, and
re-render scripts/perf/graphs/ with X.Y.Z as the highlighted current release.

  --scope NAME   benchmark scope (default: small — see scripts/perf/README.md)
  --server PATH  measure an existing binary instead of building one
  --force        re-measure a version that already has a committed result
  --skip-build   reuse target/release/tcl-lsp-server as-is
EOF
            exit 0 ;;
        -*) echo "error: unknown option: $1" >&2; exit 2 ;;
        *)
            [ -z "$V" ] || { echo "error: unexpected argument: $1" >&2; exit 2; }
            V="${1#v}"; shift ;;
    esac
done

[ -n "$V" ] || { echo "Usage: scripts/release/perf_release.sh X.Y.Z [options]" >&2; exit 2; }
[[ "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
    echo "error: '$V' is not a valid version (expected X.Y.Z)" >&2
    exit 1
}

result="$PERF/results/$V.json"

# tomllib. The harness needs it and the failure without it is an ImportError
# from deep inside fetch_corpus.py, which reads as a corpus problem.
python3 -c 'import sys; assert sys.version_info >= (3, 11), sys.version' 2>/dev/null || {
    echo "error: python3 >= 3.11 required (got $(python3 --version 2>&1))" >&2
    exit 1
}

if [ -f "$result" ] && [ "$force" -ne 1 ]; then
    echo "error: $result already exists."
    echo "       That file is the committed record for $V. Re-measuring it makes"
    echo "       every graph that quotes it a comparison against a different run."
    echo "       Pass --force if you really do mean to replace it."
    exit 1
fi

# --- 1. the binary under test ------------------------------------------------

# Every step below runs `bench.py` from `scripts/perf`, so a relative
# `--server` given at the repo root would be re-resolved against that directory
# — `target/release/tcl-lsp-server` becoming `scripts/perf/target/…`, which
# does not exist. Anchor it to the caller's cwd once, here, before anything
# changes directory.
case "$server" in
    ""|/*) ;;
    *) server="$PWD/$server" ;;
esac

if [ -z "$server" ]; then
    server="$ROOT/target/release/tcl-lsp-server"
    if [ "$skip_build" -eq 1 ]; then
        [ -x "$server" ] || { echo "error: --skip-build but no binary at $server" >&2; exit 1; }
        echo "==> Reusing $server (--skip-build)"
    else
        echo "==> Building tcl-lsp-server (release)"
        (cd "$ROOT" && cargo build -p tcl-lsp-server --release)
    fi
fi
[ -x "$server" ] || { echo "error: no executable server at $server" >&2; exit 1; }

# --- 2. the corpus -----------------------------------------------------------

echo "==> Reconstituting the pinned corpus (scope: $scope)"
(cd "$PERF" && python3 fetch_corpus.py --scope "$scope")
# Separate verify pass: fetch_corpus is happy to leave a checkout it already
# had, and a corpus that has drifted off its pins measures different bytes than
# every stored result it will be graphed against.
(cd "$PERF" && python3 fetch_corpus.py --scope "$scope" --verify-only)

# --- 3. the measurement ------------------------------------------------------

echo "==> Benchmarking $V"
(cd "$PERF" && python3 bench.py --server "$server" --version "$V" --scope "$scope" --out results)

# --- 4. the manifest ---------------------------------------------------------

python3 - "$PERF/MANIFEST.toml" "$V" <<'PY'
"""Append vX.Y.Z to [versions] tags if it is not already listed.

Textual rather than a TOML round-trip: the manifest is a hand-maintained,
heavily-commented file and rewriting it through a serialiser would strip every
comment that explains why a pin is what it is.
"""
import pathlib
import re
import sys

path, version = pathlib.Path(sys.argv[1]), sys.argv[2]
tag = f"v{version}"
text = path.read_text(encoding="utf-8")

block = re.search(r"(?m)^tags = \[\n(.*?)^\]\n", text, re.S)
if block is None:
    sys.exit(f"error: no [versions] tags array in {path}")
if f'"{tag}"' in block.group(1):
    print(f"    manifest already lists {tag}")
    sys.exit(0)

body = block.group(1)
# Append to the final populated line while it is short enough to stay inside
# the file's ~76-column wrap, else start a fresh line at the same indent.
lines = body.rstrip("\n").split("\n")
indent = re.match(r"\s*", lines[-1]).group(0)
if len(lines[-1]) + len(tag) + 4 <= 76:
    lines[-1] = f'{lines[-1]} "{tag}",'
else:
    lines.append(f'{indent}"{tag}",')
text = text.replace(block.group(0), "tags = [\n" + "\n".join(lines) + "\n]\n", 1)
path.write_text(text, encoding="utf-8", newline="\n")
print(f"    added {tag} to {path.name}")
PY

# --- 5. the graphs -----------------------------------------------------------

echo "==> Rendering graphs with $V highlighted"
(cd "$PERF" && python3 report.py --results results --out graphs --highlight "$V")

# --- Host check --------------------------------------------------------------
#
# Wall time and CPU are properties of the machine as much as the build. The
# report already refuses to quote a release-over-release delta across a host
# change, but the person cutting the release should hear about it here, while
# they can still choose to re-run somewhere comparable.
python3 - "$PERF/results" "$V" <<'PY'
import json
import pathlib
import sys

results, version = pathlib.Path(sys.argv[1]), sys.argv[2]


def key(name: str) -> tuple[int, ...]:
    return tuple(int("".join(c for c in p if c.isdigit()) or 0) for p in name.split("."))


runs = sorted((p.stem for p in results.glob("*.json")), key=key)
if version not in runs:
    sys.exit(0)
i = runs.index(version)
if i == 0:
    sys.exit(0)


def host(name: str) -> tuple[str, str]:
    d = json.loads((results / f"{name}.json").read_text(encoding="utf-8"))
    h = d.get("host") or {}
    return str(d.get("platform", "?")), str(h.get("cpu") or "?")


now, before = host(version), host(runs[i - 1])
if now != before:
    print()
    print(f"warning: {version} was measured on a different host than {runs[i - 1]}:")
    print(f"           {runs[i - 1]}: {before[1]} ({before[0]})")
    print(f"           {version}: {now[1]} ({now[0]})")
    print("         Wall time and CPU are not comparable across that boundary, so")
    print("         the notes will not quote a release-over-release delta. Re-run on")
    print("         the previous host if you want one.")
PY

echo
echo "Benchmarked $V and regenerated the graphs. Changed:"
git -C "$ROOT" status --short -- scripts/perf | sed 's/^/  /'
echo
echo "Next: scripts/release/rust_release.sh notes $V"
