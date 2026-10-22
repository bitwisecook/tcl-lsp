#!/bin/bash
# Resume batch for the review triage: restores the probe, builds+runs+deletes in one
# command per step, and writes the log into this evidence directory.
# Usage (from the repository root): bash docs/work/name-handling/reviews/triage-20261022/run5.sh
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
log=$E/run5.log
mkdir -p $E/out
cp $E/zz_review_probe.rs.txt rust/tcl-spectcl/tests/zz_review_probe.rs
avail=$(df --output=avail -B1M / | tail -1)
echo "start $(date -u) avail=${avail}M" > $log
if [ "$avail" -lt 2400 ]; then echo "disk too low" >> $log; exit 1; fi
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh ZZ_SCRATCH=$PWD/$E/out
# 1. pending: seams (refactor disabled reasons, reworked nested-passthrough probe), N17 control,
#    rename re-check for R1 (N04-N07), then the 4.1 scaling rows (P = 25..200).
timeout 3600 cargo test -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 \
  seam_ n17_control_mylib n17_live_hook rename_with_registry >> $log 2>&1
echo "exit-main=$?" >> $log
timeout 3600 cargo test -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 scaling_rows >> $log 2>&1
echo "exit-scale=$?" >> $log
rm -f target/debug/deps/zz_review_probe-*[0-9a-f]
# 2. existing live-hook fold test (does any const_fold hook fold through optimise_raw?)
timeout 2400 cargo test -p tcl-spectcl --test pack_source_e2e -- --nocapture --test-threads=1 \
  a_pack_file_folds_a_call_site_in_the_optimiser >> $log 2>&1
echo "exit-e2e=$?" >> $log
rm -f target/debug/deps/pack_source_e2e-*[0-9a-f]
rm -f rust/tcl-spectcl/tests/zz_review_probe.rs
echo "cleaned $(date -u)" >> $log
# Then: apply saved edits with  bash $E/oracle.sh $PWD/$E/out/<file>.tcl  (oracle.sh runs tclsh8.4..9.1 + Jim)
