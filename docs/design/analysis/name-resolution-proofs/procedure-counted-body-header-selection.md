# naming.procedure.counted-body-header-selection

Kind: `native-observation`

## Problem statement

A no-op procedure compiler header may classify a body through a CString prefix or its counted bytes, and continuation/control whitespace can differ by release. Inferring body emptiness from a presentation string would collapse these measured boundaries.

## Question

Which compiler header pointer remains after defining args procedures with counted raw-zero, FF, continuation and whitespace body bytes?

## Conclusion

For cases0,3,4,5,6,7, C8.4 header bits are1,1,0,0,1,1; C8.5–9.1 bits are1,0,0,1,1,0. Every definition returns code0. Thus the counted raw-zero and backslash-newline controls differ across the observed release boundary; this establishes only actual header presence, not procedure argument acceptance or entered body effects.

## Scope

Counted original Tcl_NewStringObj parameter/body objects passed to proc via public Tcl_EvalObjv with TCL_EVAL_DIRECT, followed by private Command.compileProc inspection. Eight definitions redefine p in one native interpreter; all observed code0. Full original source/five outputs and linked-library digests are retained; runtime version queries, compiler/executable/header digests and full stderr are not in this receipt. Jim/BIG-IP are not tested. This compiler-header purpose remains independent of declaration/name/procedure-body ownership.

## Provider answers

### tcl8.4

Status: `observed`. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Version: tcl8.4 8.4.20 (receipt release association.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[3,0,1],[4,0,0],[5,0,0],[6,0,1],[7,0,1]]. Recorded process status0.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Version: tcl8.5 8.5.19 (receipt release association; tcl8.6 8.6.18 (receipt release association; tcl9.0 9.0.4 (receipt release association; tcl9.1 9.1.0 (receipt release association.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[3,0,0],[4,0,0],[5,0,1],[6,0,1],[7,0,0]]. Recorded process status0.

### jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `procedure_header_compilation_bytes`: Owns the selected header byte-purpose independently of formal storage and actual entered procedure execution.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::tests::byte_header_selection_matches_original_native_storage` (linked): The specific counted parameter/body controls compare NoOp/Absent header selection against actual native pointer observations, without proving allocation or call acceptance.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `tests::physical_procedure_capture_and_header_remain_separate_from_authored_names` (linked): Checks raw-zero body header absence under actual C9 despite an authored C8.4 name policy, retained/copy body ownership separately, and missing native provider refusal.
- [rust/tcl-vm/src/interp/original_name_residual_tests.rs](../../../../rust/tcl-vm/src/interp/original_name_residual_tests.rs), `interp::original_name_residual_tests::original_body_capture_observes_actual_reference_role_before_working_copy` (linked): VM body capture observes the true original reference role before selected working copying. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [rust/tcl-vm/src/interp/original_name_residual_tests.rs](../../../../rust/tcl-vm/src/interp/original_name_residual_tests.rs), `interp::original_name_residual_tests::original_shared_body_string_worker_keeps_exact_native_reference_count` (linked): The shared original body String formatter observes the genuine borrowed reference count before working-copy selection. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [rust/tcl-vm/src/interp/original_name_residual_tests.rs](../../../../rust/tcl-vm/src/interp/original_name_residual_tests.rs), `interp::original_name_residual_tests::original_formal_list_capture_order_is_selected_before_body_reference_acquisition` (linked): Selected C capture skips formal parsing here, while Jim reaches its original formal List first; genuine body acquisition order stays independent. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.

## Evidence

- [rust/tcl-syntax/tests/data/native_procedure_headers](../../../../rust/tcl-syntax/tests/data/native_procedure_headers)
- [rust/tcl-vm/src/interp/original_name_residual_tests.rs](../../../../rust/tcl-vm/src/interp/original_name_residual_tests.rs)
