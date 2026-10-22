#!/bin/bash
R=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-codegen/run_target.sh
$R word_value "-p tcl-vm --test word_value_resolution_e2e" 2>&1 | tail -6
$R mathfunc "-p tcl-vm --test mathfunc_derivation_e2e" 2>&1 | tail -6
$R debugger "-p tcl-debugger --lib" --test-threads=2 --skip launch_survives_deeply_nested 2>&1 | tail -6
$R engine "-p tcl-engine-tclvm --lib" --test-threads=4 2>&1 | tail -10
$R stamps "-p tcl-spectcl --test codegen_stamps" --test-threads=4 --skip every_shape_of_inlined_body 2>&1 | tail -30
$R spectcl_lib "-p tcl-spectcl --lib" --test-threads=4 2>&1 | tail -10
$R pack_source "-p tcl-spectcl --test pack_source_e2e" --test-threads=2 2>&1 | tail -10
$R spec_corpus "-p tcl-spectcl --test spec_corpus" --test-threads=2 2>&1 | tail -10
$R golden_packs "-p tcl-spectcl --test golden_packs" --test-threads=2 2>&1 | tail -10
echo CHAIN1_DONE
