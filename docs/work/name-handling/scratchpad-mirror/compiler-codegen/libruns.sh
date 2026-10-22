#!/bin/bash
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-codegen
E=/home/user/tcl-lsp/target/debug/deps/tcl_compiler-ef22ef87d0bd65a7
C=$S/tcl_compiler_lib_copy
until [ -x $E ] && [ $(( $(date +%s) - $(stat -c %Y $E) )) -gt 60 ]; do sleep 30; done
cp $E $C
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
cd /home/user/tcl-lsp/rust/tcl-compiler
timeout 900 $C zz_ccg --nocapture 2>&1 | grep -aE "O129PROBE|RELPROBE|test result" > $S/res/probes.txt
F="compile_service:: codegen:: command_binding::compiled_preflight command_binding::runtime_entry inlining:: native_compilation_admission site_claims optimiser:: interprocedural:: compilation_unit::"
for mode in base after; do
  if [ $mode = base ]; then export ZZ_CCG_BASELINE=1; else unset ZZ_CCG_BASELINE; fi
  timeout 7000 $C $F --test-threads=3 --skip memoized_ --skip zz_ccg --skip depth_far_past_cap --skip ordinary_large_body --skip many_small_bodies 2>&1 | grep -aE "^test .*\.\.\. |^test result|panicked at" > $S/res/lib-$mode.txt
done
rm -f $C
grep -a "^test .* ok$" $S/res/lib-base.txt | sed 's/ \.\.\. ok//' | sort > $S/res/lib-base-ok.txt
grep -a "^test .* ok$" $S/res/lib-after.txt | sed 's/ \.\.\. ok//' | sort > $S/res/lib-after-ok.txt
{ echo "lib base: $(grep -a 'test result' $S/res/lib-base.txt)"; echo "lib after: $(grep -a 'test result' $S/res/lib-after.txt)"; echo "pass->fail:"; comm -23 $S/res/lib-base-ok.txt $S/res/lib-after-ok.txt; echo "fail->pass: $(comm -13 $S/res/lib-base-ok.txt $S/res/lib-after-ok.txt | wc -l)"; echo LIBRUNS_DONE; } > $S/res/lib-summary.txt
