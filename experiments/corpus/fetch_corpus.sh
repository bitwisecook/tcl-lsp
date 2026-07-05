#!/usr/bin/env bash
# tcl-lsp — mro_eval experiment corpus fetcher.
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Reconstitutes the git-ignored external corpus described in MANIFEST.md.
# Idempotent: skips anything already present. Run from anywhere.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

echo "== georgtree TclOO repos (shallow) =="
mkdir -p georgtree
# repo  commit (pin for reproducibility; see MANIFEST.md)
gt_repos=(
  "SpiceGenTcl d9d7187"
  "tclopt     ff9a560"
  "ruff       b63da69"
  "argparse   139d695"
  "tcl_tools  6be5470"
  "tclinterp  ccd894b"
  "tclmeasure 648b0cc"
  "extexpr    1d53317"
)
for entry in "${gt_repos[@]}"; do
  repo="${entry%% *}"
  if [ -d "georgtree/$repo/.git" ]; then
    echo "  $repo present — skipping"
  else
    git clone --depth 1 -q "https://github.com/georgtree/$repo" "georgtree/$repo" \
      && echo "  cloned $repo"
  fi
done

echo "== tcllib metaobject frameworks =="
mkdir -p vendor/tcllib
tcllib="${TCLLIB_DIR:-../../tmp/tcllib-2.0}"
if [ -d "$tcllib" ]; then
  for m in clay oodialect oometa ooutil; do
    cp "$tcllib/modules/$m/"*.tcl vendor/tcllib/ 2>/dev/null || true
  done
  echo "  copied from $tcllib"
else
  echo "  WARNING: $tcllib not found; set TCLLIB_DIR to your tcllib checkout"
fi

echo "== Tcl core oo tests =="
mkdir -p vendor/tcl-oo-tests
for v in tcl8.6.16 tcl9.0.3; do
  tree="../../tmp/$v"
  [ -d "$tree" ] || continue
  for f in oo.test ooNext2.test ooUtil.test; do
    [ -f "$tree/tests/$f" ] && cp "$tree/tests/$f" "vendor/tcl-oo-tests/${v}-${f}"
  done
done

echo "== repo OO fixtures =="
mkdir -p repo-fixtures
for f in oo-shapes.tcl methodParam.tcl; do
  src="../../editors/vscode/testFixture/$f"
  [ -f "$src" ] && cp "$src" repo-fixtures/
done

echo "done. Now run:"
echo "  cargo run --release -p tcl-compiler --example mro_eval -- \\"
echo "    experiments/corpus/georgtree experiments/corpus/adversarial \\"
echo "    experiments/corpus/vendor experiments/corpus/repo-fixtures"
