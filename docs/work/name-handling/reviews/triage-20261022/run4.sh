#!/bin/bash
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/triage
log=$S/run4.log
cd /home/user/tcl-lsp
avail=$(df --output=avail -B1M / | tail -1)
echo "start $(date) avail=${avail}M" > $log
if [ "$avail" -lt 2400 ]; then echo "disk too low" >> $log; exit 1; fi
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh ZZ_SCRATCH=$S/out
timeout 3600 cargo test -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 seam_ >> $log 2>&1
echo "exit-seam=$?" >> $log
timeout 2400 cargo test -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 scaling_rows >> $log 2>&1
echo "exit-scale=$?" >> $log
rm -f target/debug/deps/zz_review_probe-*[0-9a-f]
timeout 2400 cargo test -p tcl-spectcl --test pack_source_e2e -- --nocapture --test-threads=1 a_pack_file_folds_a_call_site_in_the_optimiser without_a_host >> $log 2>&1
echo "exit-e2e=$?" >> $log
rm -f target/debug/deps/pack_source_e2e-*[0-9a-f]
ls target/debug/deps | grep -E 'zz_review_probe|pack_source_e2e' >> $log
echo "cleaned $(date)" >> $log
