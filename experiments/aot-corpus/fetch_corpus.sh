#!/usr/bin/env bash
# tcl-lsp — AOT WASM command-priority corpus fetcher.
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Reconstitutes the git-ignored real-world corpus named in issue #1181 (the
# "which Tcl commands matter most for the AOT WASM compiler" study). Shallow
# (--depth 1) clones. Idempotent: skips anything already present. Records the
# resolved HEAD commit of every repo to provenance.txt (git-ignored — the
# design doc that consumes this corpus quotes the commits it needs directly,
# so the study stays reproducible without committing the file).
#
# Usage: experiments/aot-corpus/fetch_corpus.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

provenance="provenance.txt"
: > "$provenance"
failed=()

clone_into() {
  local group="$1" owner="$2" repo="$3"
  local dir="$group/$repo"
  mkdir -p "$group"
  if [ -d "$dir/.git" ]; then
    echo "  $repo already present"
  else
    if git clone -q --depth 1 "https://github.com/$owner/$repo" "$dir" 2>"/tmp/clone-$repo.err"; then
      echo "  cloned $owner/$repo"
    else
      echo "  clone $owner/$repo FAILED:"
      sed 's/^/    /' "/tmp/clone-$repo.err"
      failed+=("$owner/$repo")
      return
    fi
  fi
  local sha size
  sha="$(git -C "$dir" rev-parse --short=12 HEAD 2>/dev/null || echo unknown)"
  size="$(du -sh "$dir" 2>/dev/null | cut -f1)"
  echo "$owner/$repo	$sha	$size" >>"$provenance"
  # Guard against an absurdly large clone eating the shared disk — flag and
  # move on rather than deleting (the caller decides whether to keep it).
  local size_mb
  size_mb="$(du -sm "$dir" 2>/dev/null | cut -f1)"
  if [ "${size_mb:-0}" -gt 2048 ]; then
    echo "  WARNING: $owner/$repo is ${size}B (> 2 GiB) — consider excluding it from the count"
  fi
}

echo "== georgtree (8 repos) =="
for repo in SpiceGenTcl tclopt ruff argparse tcl_tools tclinterp tclmeasure extexpr; do
  clone_into georgtree georgtree "$repo"
done

echo "== nico-robert (6 repos) =="
for repo in ticklecharts tomato pix zesty haru implottk; do
  clone_into nico-robert nico-robert "$repo"
done

echo "== tcltk core (3 repos) =="
for repo in tcllib tklib tk; do
  clone_into tcltk tcltk "$repo"
done

echo "== aplsimple (2 repos) =="
for repo in pave alited; do
  clone_into aplsimple aplsimple "$repo"
done

echo "== EDA tooling (2 repos) =="
clone_into eda Xilinx XilinxTclStore
clone_into eda OSVVM OSVVM-Scripts

echo
df -h "$here" | tail -1
echo
if [ "${#failed[@]}" -gt 0 ]; then
  echo "FAILED clones (network or repo-name issue — recorded, not fatal):"
  printf '  %s\n' "${failed[@]}"
else
  echo "all clones present."
fi
echo
echo "provenance recorded in $here/$provenance (git-ignored)."
