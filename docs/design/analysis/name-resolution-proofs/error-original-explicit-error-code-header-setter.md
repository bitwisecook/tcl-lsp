# naming.error.original-explicit-error-code-header-setter

Kind: `native-observation`

## Problem statement

An explicit error-code setter receives an original lazy object header. Eager String conversion can run its updater before storage; old-header retirement can observe a release-specific selected header or global write callback. A callback error returned through public Eval and a separately requested String getter reach different API phases.

## Question

For four fresh original-header cases on each selected C release, does public Tcl_SetObjErrorCode retain the same lazy incoming header without a String updater, what storage is visible during old-header retirement/global write tracing, and does a callback/public error return preserve it until a separately requested counted String getter? Which finite original observations remain independent from software Host/refusal controls?

## Conclusion

All five original C releases complete four independently compiled cases each:20 closed cases,70 ROW and20 counted GETTER records. Incoming B retains caller-plus-selected-storage refs2 without a String updater before the separate getter; that getter runs once and yields the same15 bytes containing NUL/non-UTF8. C8.4 old-A free observes already-published B; its case2 global trace observes B before the explicit-code flag is set. C8.5–C9.1 old-A free observes the old private A before B assignment, and these private setters reach no global write trace. Actual callback/public Eval error1 preserves lazy B before later presentation. Original614/614b refusal/compile boundaries remain separate. Jim/BIG-IP are not tested. The two linked Rust controls are unrun; no general callback lifetime, Host/ABA, physical frame, self-alias or implementation-equivalence result follows.

## Scope

Five own native C producers use each actual selected SDK Makefile CC/CC_SWITCHES/LIBS, complete declared header trees, strict reached compiler header/link maps, runtime dependencies and unchanged executable identities. Fresh614c contains16 closed0 processes and20 cases; independent614b contains seven saved processes and four C8.4 cases, C8.5/C8.6 compile failures and C9.0 strict header refusal with C9.1 unentered. Original614 retains compiler-version0 and compile streams/ELF/maps but no saved compile receipt after its header refusal. None of those partial bytes substitute for614c. Cases select absent/old private or C8.4 global header, pure old-free/global-trace observers, original object-command callback and public error return, then separate counted String presentation. The observers perform no interpreter mutation; same-header self-setting and arbitrary reentrant callbacks are not tested. Raw addresses are within-case diagnostics, and allocation/reference roles grant no safe Var/header lifetime or Native Host/frame/VM equality. SDK source/library retention lacks configure/build replay; external system inputs are hash-only replay prerequisites. Software first Host/context-ABA continuations retain the separate embedding question; source inspection and native observations provide no matching Rust assertion or whole-suite/performance outcome.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public Tcl_SetObjErrorCode with original custom lazy headers; legacy object-command callback/public Tcl_EvalObjv error return; separately selected counted Tcl_GetStringFromObj getter. Version: tcl8.4 8.4.20.

Four original614c cases close0 with14 ROW/four GETTER. The same lazy incoming B has refs2 and updater0 before separate presentation; the counted getter reaches B once and returns15 bytes including NUL/non-UTF8. The callback/public Eval error1 preserves B lazy. C8.4 global old-A free sees already-published B; case2 free then global write trace see B with flag0, and setter return observes flag1. Independent614b supplies four separately retained C8.4 cases; original614 header refusal supplies no setter execution. Observer callbacks perform no interpreter mutation and no same-header self-set is tested; addresses/counts confer no general lifetime, Host/frame/VM or Rust parity.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Public Tcl_SetObjErrorCode with original custom lazy headers; legacy object-command callback/public Tcl_EvalObjv error return; separately selected counted Tcl_GetStringFromObj getter. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Four original614c cases close0 with14 ROW/four GETTER. The same lazy incoming B has refs2 and updater0 before separate presentation; the counted getter reaches B once and returns15 bytes including NUL/non-UTF8. The callback/public Eval error1 preserves B lazy. The old private-A free sees A before incoming B assignment; no installed global errorCode write trace is reached. Flag-1 reports that the probe does not inspect a modern explicit-code flag. Independent614b compile fails before this setter purpose. Observer callbacks perform no interpreter mutation and no same-header self-set is tested; addresses/counts confer no general lifetime, Host/frame/VM or Rust parity.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public Tcl_SetObjErrorCode with original custom lazy headers; legacy object-command callback/public Tcl_EvalObjv error return; separately selected counted Tcl_GetStringFromObj getter. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Four original614c cases close0 with14 ROW/four GETTER. The same lazy incoming B has refs2 and updater0 before separate presentation; the counted getter reaches B once and returns15 bytes including NUL/non-UTF8. The callback/public Eval error1 preserves B lazy. The old private-A free sees A before incoming B assignment; no installed global errorCode write trace is reached. Flag-1 reports that the probe does not inspect a modern explicit-code flag. Independent614b C9.0 compile0 is followed by unpinned-header refusal, with C9.1 unentered; neither supplies this executed purpose. Observer callbacks perform no interpreter mutation and no same-header self-set is tested; addresses/counts confer no general lifetime, Host/frame/VM or Rust parity.

### jim

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: jim No Jim Tcl API process selected.

The selected worker is the original public Tcl_SetObjErrorCode C API. No Jim Tcl API process is selected, compiled or executed; this is not a measured Jim Guest rejection, unavailable-worker trace or equivalence to Jim error-state APIs.

### bigip

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: bigip not tested for this source/API question.

No original bigip process tests this explicit original C error-code header setter contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained source bytes and independently derived physical-LF function extent; static inspection, independently joined to the finite original process records`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclResult.c`, function `Tcl_SetObjErrorCode`, lines 952–961. Retained reference `setter-source-window-8.4.20`.

```text
Tcl_SetObjErrorCode(interp, errorObjPtr)
    Tcl_Interp *interp;
    Tcl_Obj *errorObjPtr;
{
    Interp *iPtr;
    
    iPtr = (Interp *) interp;
    Tcl_SetVar2Ex(interp, "errorCode", NULL, errorObjPtr, TCL_GLOBAL_ONLY);
    iPtr->flags |= ERROR_CODE_SET;
}

```

tcl8.5 8.5.19, revision `Exact retained source bytes and independently derived physical-LF function extent; static inspection, independently joined to the finite original process records`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclResult.c`, function `Tcl_SetObjErrorCode`, lines 1070–1081. Retained reference `setter-source-window-8.5.19`.

```text
Tcl_SetObjErrorCode(
    Tcl_Interp *interp,
    Tcl_Obj *errorObjPtr)
{
    Interp *iPtr = (Interp *) interp;

    if (iPtr->errorCode) {
	Tcl_DecrRefCount(iPtr->errorCode);
    }
    iPtr->errorCode = errorObjPtr;
    Tcl_IncrRefCount(iPtr->errorCode);
}

```

tcl8.6 8.6.18, revision `Exact retained source bytes and independently derived physical-LF function extent; static inspection, independently joined to the finite original process records`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclResult.c`, function `Tcl_SetObjErrorCode`, lines 1103–1114. Retained reference `setter-source-window-8.6.18`.

```text
Tcl_SetObjErrorCode(
    Tcl_Interp *interp,
    Tcl_Obj *errorObjPtr)
{
    Interp *iPtr = (Interp *) interp;

    if (iPtr->errorCode) {
	Tcl_DecrRefCount(iPtr->errorCode);
    }
    iPtr->errorCode = errorObjPtr;
    Tcl_IncrRefCount(iPtr->errorCode);
}

```

tcl9.0 9.0.4, revision `Exact retained source bytes and independently derived physical-LF function extent; static inspection, independently joined to the finite original process records`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclResult.c`, function `Tcl_SetObjErrorCode`, lines 535–546. Retained reference `setter-source-window-9.0.4`.

```text
Tcl_SetObjErrorCode(
    Tcl_Interp *interp,
    Tcl_Obj *errorObjPtr)
{
    Interp *iPtr = (Interp *) interp;

    if (iPtr->errorCode) {
	Tcl_DecrRefCount(iPtr->errorCode);
    }
    iPtr->errorCode = errorObjPtr;
    Tcl_IncrRefCount(iPtr->errorCode);
}

```

tcl9.1 9.1.0, revision `Exact retained source bytes and independently derived physical-LF function extent; static inspection, independently joined to the finite original process records`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclResult.c`, function `Tcl_SetObjErrorCode`, lines 536–547. Retained reference `setter-source-window-9.1.0`.

```text
Tcl_SetObjErrorCode(
    Tcl_Interp *interp,
    Tcl_Obj *errorObjPtr)
{
    Interp *iPtr = (Interp *) interp;

    if (iPtr->errorCode) {
	Tcl_DecrRefCount(iPtr->errorCode);
    }
    iPtr->errorCode = errorObjPtr;
    Tcl_IncrRefCount(iPtr->errorCode);
}

```

## Consumer bindings

- [runtime/rust/src/capi.rs](../../../../runtime/rust/src/capi.rs), `Tcl_SetObjErrorCode`: Public borrowed-header C API entry delegates to the authentic entered operation without first converting the header to bytes.
- [runtime/rust/src/capi/collections.rs](../../../../runtime/rust/src/capi/collections.rs), `set_error_code`: Retain the entered interpreter and original header lease, route the shared setter, and preserve the first full typed refusal with unconditional currency checks.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `set_original_c_error_code`: Selected C8.4 global or C8.5–C9.1 private original-header ownership, checked liveness/current operation and exact reached retirement/adoption stages; no implicit String getter or spelling reconstruction.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `publish_original_c84_error_code`: Selected software global-cell store, write callback and explicit-code flag stages. Actual callbacks and first-cause transport do not establish original C worker callback parity.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `error_code_bytes_checked`: A separately selected checked String getter reads the actual C8.4 global or modern private header and returns its exact counted bytes, preserving typed Host refusal. This does not grant an unavailable C8.4 return-options API.
- [runtime/rust/src/interp/original_error_code_setter_tests.rs](../../../../runtime/rust/src/interp/original_error_code_setter_tests.rs), `interp::original_error_code_setter_tests::original_capi_error_code_setter_retains_lazy_header_without_a_string_getter` (linked): The genuine five-C-profile software setter retains the original lazy header with zero reached updater/free calls, preserves the prior result, and keeps exact caller-plus-selected-storage references. A subsequent explicitly selected checked String getter reaches the updater once and returns the counted NUL/non-UTF8 bytes from the same header. These counts and pointers are software controls, not original-provider refcount or updater observations. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/interp/original_error_code_setter_tests.rs](../../../../runtime/rust/src/interp/original_error_code_setter_tests.rs), `interp::original_error_code_setter_tests::original_c84_explicit_callback_code_survives_actual_command_logging` (linked): An actual installed software C8.4 object-command callback sets its explicit original header, returns an error, and keeps the callback code through command logging with the global explicit-code flag and no Host refusal. This supplies no C8.4 Tcl_GetReturnOptions availability, original C callback/logging outcome or unrelated error-worker chronology. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/interp/original_error_code_setter_tests.rs](../../../../runtime/rust/src/interp/original_error_code_setter_tests.rs), `interp::original_error_code_setter_tests::original_error_code_decrement_keeps_the_selected_raw_header_role_without_a_native_hold` (linked): Error code decrement keeps the selected raw header role without a native hold. This is a source-defined software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native chronology, physical reference-count/frame identity or execution permission.
- [runtime/rust/src/interp/native_body_artifact/original_return_parent_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/original_return_parent_tests.rs), `interp::native_body_artifact::original_return_parent_tests::original_compiled_errorcode_uses_selected_old_role_before_incoming_hold_without_capi_counter` (linked): Compiled original -errorcode delegates the same selected setter: genuine modern old-free observes old raw role and pre-incoming count, original frame/currency checked before incoming hold, no String getter/public CAPI counter. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [runtime/rust/src/interp/native_body_artifact/original_return_parent_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/original_return_parent_tests.rs), `interp::native_body_artifact::original_return_parent_tests::original_generic_field_publication_remains_distinct_from_selected_setter_retirement` (linked): Actual generic451c publication exposes its incoming role at the old free independently of selected setter old-role order; no Native614c semantic getter or broad role timing is inferred. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.

## Evidence

- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/tests/data/original_explicit_error_code_header_setter_source/8.4.20](../../../../runtime/rust/tests/data/original_explicit_error_code_header_setter_source/8.4.20)
- [runtime/rust/tests/data/original_explicit_error_code_header_setter_source/8.5.19](../../../../runtime/rust/tests/data/original_explicit_error_code_header_setter_source/8.5.19)
- [runtime/rust/tests/data/original_explicit_error_code_header_setter_source/8.6.18](../../../../runtime/rust/tests/data/original_explicit_error_code_header_setter_source/8.6.18)
- [runtime/rust/tests/data/original_explicit_error_code_header_setter_source/9.0.4](../../../../runtime/rust/tests/data/original_explicit_error_code_header_setter_source/9.0.4)
- [runtime/rust/tests/data/original_explicit_error_code_header_setter_source/9.1.0](../../../../runtime/rust/tests/data/original_explicit_error_code_header_setter_source/9.1.0)
- [runtime/rust/src](../../../../runtime/rust/src)
- [runtime/rust/src/capi](../../../../runtime/rust/src/capi)
- [runtime/rust/src/interp](../../../../runtime/rust/src/interp)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614/providers/8.4.20](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614/providers/8.4.20)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614)
- [rust/tcl-registry/tests/data/native_capi_scalar_publication_original](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original)
- [rust/tcl-registry/tests/data/native_command_recreation_original](../../../../rust/tcl-registry/tests/data/native_command_recreation_original)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20)
- [rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original](../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original)
- [rust/tcl-registry/tests/data/native_deep_import_chain_original](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original)
- [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original)
- [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers/8.5.19](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers/8.5.19)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers/8.6.18](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers/8.6.18)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers/9.0.4](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b/providers/9.0.4)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614b)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/independent-interpretation](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/independent-interpretation)
- [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/receiver-crossreview](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/receiver-crossreview)
- [runtime/rust/src/interp/native_body_artifact](../../../../runtime/rust/src/interp/native_body_artifact)
