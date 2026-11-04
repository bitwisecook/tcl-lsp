# naming.tcloo.empty-lifecycle-definition

Kind: `native-observation`

## Problem statement

Treating an empty TclOO constructor body as an installed zero-command method can validate unused formals or reject arguments that the selected runtime accepts. Removing an own constructor also need not remove an inherited constructor.

## Question

Does an empty TclOO constructor definition remove its own constructor before formal validation, while space/comment bodies retain arity and an inherited constructor remains selected after own removal?

## Conclusion

In the retained C8.6.18, C9.0.4 and C9.1.0 original ASCII source-file processes, empty constructor bodies leave info class constructor empty and accept all four tested argument counts 0..3. The malformed three-field formal control also succeeds when its body is empty. A one-space or comment body retains the a b constructor and accepts only count2; other tested counts report TCL WRONGARGS. A malformed formal with a one-space body fails definition. An own empty malformed definition on a child leaves its own constructor empty while the superclass a b constructor still accepts only count2. Exact source inspection shows the positive body-length branch constructs a procedure method and the empty branch installs NULL instead. The stock C8.4.20, C8.5.19 and Jim processes report no original oo::class command, so no lifecycle branch is exercised. The linked source implementation projects formal-validation applicability from the genuine selected member/factory grammar and static counted original body: Required, SkippedRemovedLifecycle or unavailable. Ordinary/other-family members retain their own validation; missing or dynamic lifecycle bodies cannot be promoted to empty. The unit counted-NUL control is an authored count invariant, not another native source-process observation. These source bindings establish neither an installed method nor an entered frame. Source constructor arity advice uses each inherited provider's own original factory grammar, naming policy and formal dialect. A static empty own lifecycle body proceeds to inherited/default source signature; missing or dynamic body remains unavailable. These linked source controls do not extend the fixed native process matrix.

## Scope

Six fresh selected executable processes, fixed ASCII LF source-file input, caught definition outcomes, own-constructor reflection and new-call acceptance for four exact argument counts. No package loading, external BIG-IP, destructor control, original native object/header/cache getter, opaque name, frame recipe, compiler admission, arbitrary callback effects or general Normal proof is measured. Generated object names in the retained stdout are results of these exact runs, not portable identity assertions.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Selected executable. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

This fresh stock CLI reports oo::class absent and an explicit unavailable row. No package or extension load was attempted and no constructor lifecycle operation ran; this is not a claim that every embedding of this interpreter family lacks TclOO.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Selected executable. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

This fresh stock CLI reports oo::class absent and an explicit unavailable row. No package or extension load was attempted and no constructor lifecycle operation ran; this is not a claim that every embedding of this interpreter family lacks TclOO.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected executable. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

The exact31 rows show own empty/empty-malformed constructor removal and acceptance of counts0..3; space/comment a b bodies accept only2; malformed-space definition fails. After own empty removal, the superclass a b constructor still accepts only2. All remaining caught wrong-count operations report TCL WRONGARGS. Outer process code0 and empty stderr are retained separately.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected executable. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

The exact31 rows show own empty/empty-malformed constructor removal and acceptance of counts0..3; space/comment a b bodies accept only2; malformed-space definition fails. After own empty removal, the superclass a b constructor still accepts only2. All remaining caught wrong-count operations report TCL WRONGARGS. Outer process code0 and empty stderr are retained separately.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected executable. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

The exact31 rows show own empty/empty-malformed constructor removal and acceptance of counts0..3; space/comment a b bodies accept only2; malformed-space definition fails. After own empty removal, the superclass a b constructor still accepts only2. All remaining caught wrong-count operations report TCL WRONGARGS. Outer process code0 and empty stderr are retained separately.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Selected executable. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: Jim Tcl.

This fresh stock CLI reports oo::class absent and an explicit unavailable row. No package or extension load was attempted and no constructor lifecycle operation ran; this is not a claim that every embedding of this interpreter family lacks TclOO.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

External hosted TclOO availability and constructor lifecycle require an independently captured actual context. No C/Jim process substitutes for BIG-IP.

## Historical references

- `probe` (input): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/probe.tcl](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/probe.tcl). Exact original ASCII LF lifecycle source program; includes empty/space/comment/malformed and inherited controls.
- `request` (input): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/request.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/request.json). Retained finite question, channel and provider request; no constructor or external hosted result is inferred from this request.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/receipt.json). Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stdout). Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stderr). Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/receipt.json). Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stdout). Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stderr). Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/receipt.json). Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stdout). Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stderr). Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/receipt.json). Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stdout). Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stderr). Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/receipt.json). Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stdout). Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stderr). Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/receipt.json). Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stdout). Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stderr). Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl8.6-constructor-source` (source-anchor): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/8.6.18/tclOODefineCmds.c](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/8.6.18/tclOODefineCmds.c). Lines 1233–1265. Exact retained release constructor-definition source branch, independently inspected; this window is not a native object/frame observation.
- `tcl9.0-constructor-source` (source-anchor): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.0.4/tclOODefineCmds.c](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.0.4/tclOODefineCmds.c). Lines 1858–1894. Exact retained release constructor-definition source branch, independently inspected; this window is not a native object/frame observation.
- `tcl9.1-constructor-source` (source-anchor): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.1.0/tclOODefineCmds.c](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.1.0/tclOODefineCmds.c). Lines 1832–1868. Exact retained release constructor-definition source branch, independently inspected; this window is not a native object/frame observation.

## Source inspection

tcl8.6 8.6.18, revision `Original retained release source checkout 8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOODefineConstructorObjCmd`, lines 1233–1265. Retained reference `tcl8.6-constructor-source`.

```text
    Tcl_GetStringFromObj(objv[2], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, objv[1], objv[2], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the constructor method record and set the field in the
	 * class record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the constructor as this might be being done during
     * execution of the constructor itself.
     */

    Tcl_ClassSetConstructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *

```

tcl9.0 9.0.4, revision `Original retained release source checkout 9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOODefineConstructorObjCmd`, lines 1858–1894. Retained reference `tcl9.0-constructor-source`.

```text
    (void) TclGetStringFromObj(objv[2], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, objv[1], objv[2], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the constructor method record and set the field in the
	 * class record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the constructor as this might be being done during
     * execution of the constructor itself.
     */

    Tcl_ClassSetConstructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDefnNsObjCmd --
 *
 *	Implementation of the "definitionnamespace" subcommand of the
 *	"oo::define" command.

```

tcl9.1 9.1.0, revision `Original retained release source checkout 9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineConstructorObjCmd`, lines 1832–1868. Retained reference `tcl9.1-constructor-source`.

```text
    (void) TclGetStringFromObj(objv[2], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, objv[1], objv[2], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the constructor method record and set the field in the
	 * class record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the constructor as this might be being done during
     * execution of the constructor itself.
     */

    Tcl_ClassSetConstructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDefnNsObjCmd --
 *
 *	Implementation of the "definitionnamespace" subcommand of the
 *	"oo::define" command.

```


## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `DefinitionBodyGrammar::source_member_formal_validation_applicability`: Require authentic selected member/factory grammar and static counted body; delegate empty removal to the shared native lifecycle disposition. Missing lifecycle body stays unknown; ordinary and other-family members retain formal validation.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `DefinitionBodyGrammar::source_special_member_formal_validation_applicability`: Require authentic selected member/factory grammar and static counted body; delegate empty removal to the shared native lifecycle disposition. Missing lifecycle body stays unknown; ordinary and other-family members retain formal validation.
- [rust/tcl-compiler/src/analyser/oo.rs](../../../../rust/tcl-compiler/src/analyser/oo.rs), `Analyser::member_formal_parameter_indices`: All five E006 member consumers obtain selected source applicability and formal ordinals centrally. Original special-member metadata owns complete body geometry; NativeCompilerWords forms its static literal under the retained source protocol, without runtime entry.
- [rust/tcl-compiler/src/analyser/types/original_constructor_call.rs](../../../../rust/tcl-compiler/src/analyser/types/original_constructor_call.rs), `OriginalConstructorArityCall::source_arity`: Use each actual inherited provider's own original factory grammar, naming policy and formal dialect; static empty lifecycle removal proceeds to inherited/default source signature, while missing/dynamic body remains unavailable.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::lifecycle_formal_applicability_keeps_empty_unknown_and_foreign_members_separate` (linked): Selected TclOO empty body suppresses its parser, whitespace/comment/NUL byte values retain it, missing lifecycle body and foreign member decline, and ordinary/Snit members retain their own required parser. The NUL unit value is an authored count check, not a new source-script observation.
- [rust/tcl-compiler/src/analyser/oo.rs](../../../../rust/tcl-compiler/src/analyser/oo.rs), `analyser::oo::lifecycle_formal_tests::original_tcloo_formals_follow_counted_lifecycle_body_applicability` (linked): Original C86/C90/C91 source E006 mirrors measured empty versus whitespace/comment removal/validation, including malformed list, inline setter, inherited empty removal and ordinary method controls. No actual constructor frame is asserted.
- [rust/tcl-compiler/src/analyser/oo.rs](../../../../rust/tcl-compiler/src/analyser/oo.rs), `analyser::oo::lifecycle_formal_tests::original_tcloo_formal_applicability_preserves_dynamic_body_uncertainty` (linked): Unresolved variable/bracket and inline dynamic bodies cannot be promoted to an empty or retained static lifecycle value; this is an implementation abstention control, not a newly measured native outcome.
- [rust/tcl-compiler/src/analyser/types/original_constructor_call.rs](../../../../rust/tcl-compiler/src/analyser/types/original_constructor_call.rs), `analyser::types::original_constructor_call::tests::original_constructor_signature_respects_exact_lifecycle_body_values` (linked): Conditional source signature advice distinguishes empty and malformed-empty default arity from whitespace/comment exact-two arity; malformed-space refuses, and own empty removal retains the inherited exact-two signature. No native header or activation is observed.

A named test is a coverage binding, not a claim that it executed.

## Replay

Replay the exact retained probe with each receipt's independently verified executable and required header/library/build/source hashes and environment. Preserve stdout/stderr separately; the outer process code0 does not make every caught guest operation succeed. Native source-file observations do not establish a runtime/VM implementation result, selected source carrier, object/cache/frame proof or changed-source gate pass. BIG-IP requires a separately measured external hosted context.
