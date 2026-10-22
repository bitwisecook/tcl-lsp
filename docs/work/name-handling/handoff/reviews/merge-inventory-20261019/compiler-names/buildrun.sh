#!/bin/bash
# usage: buildrun.sh TAG ; builds tcl-compiler lib tests, then runs the probe set immediately
cd /home/user/tcl-lsp
D=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-names
TAG=$1
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh
cargo test --locked -p tcl-compiler --lib --no-run > $D/build-$TAG.full 2>&1 || { echo BUILD_FAILED; grep -E "^error" -A8 $D/build-$TAG.full | head -40; exit 1; }
EXE=$(grep -o 'target/debug/deps/tcl_compiler-[0-9a-f]*' $D/build-$TAG.full | tail -1)
echo built $EXE
"$EXE" completed_world_follows original_readonly_worker_formal crlf_class_method substituted_heads_and_consumed > $D/$TAG-main.log 2>&1
ZZ_PROBE_ENSEMBLE=1 ZZ_PROBE_OPAQUE=1 TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC=1 "$EXE" zz_probe_import_and_crlf --nocapture 2>&1 | head -c 20000000 > $D/$TAG-zz.log
ZZ_PROBE_LOWER=1 "$EXE" w210_pkgindex --nocapture 2>&1 | head -c 20000000 > $D/$TAG-w210.log
"$EXE" conditional_expression_template_retains_grammar_without_actual_entry declaration_layout_keeps_its_original_table_without_a_compiler_visit scoped_nested_normalisation_keeps_original_world_frame_and_source selected_nested_normalisation_projects_actual_operand_and_command_source scoped_lifecycle_coverage_keeps_declared_issuer_frame_and_original_source scoped_procedure_equivalence_keeps_independent_world_and_completion_obligations original_immediate_body_namespace_keeps_actual_native_conditional_and_error_prefix original_namespace_import_keeps_the_source_declaration_allocation > $D/$TAG-regress.log 2>&1
"$EXE" command_binding:: > $D/$TAG-command_binding.log 2>&1
rm -f "$EXE"
grep -E "^test result" $D/$TAG-*.log
