#!/bin/bash
R=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-codegen/run_target.sh
L=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-codegen/res/chain2.log
for spec in \
 'registry_string|-p tcl-registry --lib|native_string_operand --test-threads=4' \
 'cmd_string|-p tcl-vm --test cmd_string_e2e|--test-threads=4' \
 'mathfunc|-p tcl-vm --test mathfunc_derivation_e2e|' \
 'word_value|-p tcl-vm --test word_value_resolution_e2e|' \
 'debugger|-p tcl-debugger --lib|--test-threads=2 --skip launch_survives_deeply_nested' \
 'engine|-p tcl-engine-tclvm --lib|--test-threads=4' \
 'spectcl_lib|-p tcl-spectcl --lib|--test-threads=4' \
 'stamps|-p tcl-spectcl --test codegen_stamps|--test-threads=4 --skip every_shape_of_inlined_body' \
 'pack_source|-p tcl-spectcl --test pack_source_e2e|--test-threads=2' \
 'spec_corpus|-p tcl-spectcl --test spec_corpus|--test-threads=2' \
 'golden_packs|-p tcl-spectcl --test golden_packs|--test-threads=2' ; do
  IFS='|' read -r name cargs targs <<< "$spec"
  if [ "$name" = compiler_lib ]; then
    KEEP=1 $R "$name" "$cargs" $targs >> $L 2>&1
    EXE=$(cat /tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-codegen/res/compiler_lib-exe.txt)
    (cd /home/user/tcl-lsp/rust/tcl-compiler && timeout 900 /home/user/tcl-lsp/$EXE zz_ccg --nocapture 2>&1 | grep -aE "O129PROBE|RELPROBE|test result" > /tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-codegen/res/probes.txt)
    rm -f /home/user/tcl-lsp/$EXE
  else
    $R "$name" "$cargs" $targs >> $L 2>&1
  fi
  echo "== $name finished $(date +%H:%M)" >> $L
done
echo CHAIN2_DONE >> $L
