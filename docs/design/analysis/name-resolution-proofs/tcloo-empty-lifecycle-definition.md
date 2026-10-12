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

Status: `unsupported`. Version: 8.4.20. Build: Selected executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

This fresh stock CLI reports oo::class absent and an explicit unavailable row. No package or extension load was attempted and no constructor lifecycle operation ran; this is not a claim that every embedding of this interpreter family lacks TclOO.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Selected executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

This fresh stock CLI reports oo::class absent and an explicit unavailable row. No package or extension load was attempted and no constructor lifecycle operation ran; this is not a claim that every embedding of this interpreter family lacks TclOO.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

The exact31 rows show own empty/empty-malformed constructor removal and acceptance of counts0..3; space/comment a b bodies accept only2; malformed-space definition fails. After own empty removal, the superclass a b constructor still accepts only2. All remaining caught wrong-count operations report TCL WRONGARGS. Outer process code0 and empty stderr are retained separately.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

The exact31 rows show own empty/empty-malformed constructor removal and acceptance of counts0..3; space/comment a b bodies accept only2; malformed-space definition fails. After own empty removal, the superclass a b constructor still accepts only2. All remaining caught wrong-count operations report TCL WRONGARGS. Outer process code0 and empty stderr are retained separately.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: C Tcl.

The exact31 rows show own empty/empty-malformed constructor removal and acceptance of counts0..3; space/comment a b bodies accept only2; malformed-space definition fails. After own empty removal, the superclass a b constructor still accepts only2. All remaining caught wrong-count operations report TCL WRONGARGS. Outer process code0 and empty stderr are retained separately.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Selected executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806. Exact required header/library/Makefile/source hashes and environment are retained in its receipt; no additional configuration interpretation is inferred.. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim.. Dialect: Jim Tcl.

This fresh stock CLI reports oo::class absent and an explicit unavailable row. No package or extension load was attempted and no constructor lifecycle operation ran; this is not a claim that every embedding of this interpreter family lacks TclOO.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

External hosted TclOO availability and constructor lifecycle require an independently captured actual context. No C/Jim process substitutes for BIG-IP.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/probe.tcl](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/probe.tcl). SHA-256 `34a28b9cdc4d5501378a76bbdbbe0db1d7886ed62e4a146ff37e3e756fb92139`. Exact original ASCII LF lifecycle source program; includes empty/space/comment/malformed and inherited controls.
- `request` (input): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/request.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/request.json). SHA-256 `0392d7d7435269e31c084c92649007f754ce5be41c9a8c20d6147ed5423e376b`. Retained finite question, channel and provider request; no constructor or external hosted result is inferred from this request.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/receipt.json). SHA-256 `3717714170767da55e75196119c67133cd39521daaa915f9c976882a5ecde4b6`. Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stdout). SHA-256 `345f6dd5236a9936fbfcf7e08af419ee6490927bd96801a6a993bb988c7e6f98`. Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/receipt.json). SHA-256 `2b23b898a8a84a53ba7da9e3687ab9d1a29a75b87d91a5fecaafae710f319c44`. Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stdout). SHA-256 `13d3575a3c17cce80731ae0e5c4f1cd60354661fba8c96a62cb795b1e5eb39da`. Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/receipt.json). SHA-256 `2c6c5b7fa10eaa69c65fa7b40786bcffc0a8f807172408be40c2d49902dd6210`. Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stdout). SHA-256 `fc2a19783266fa9519f06bbacb2ace4d125e342ae361c9c44b3a5686f300228b`. Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/receipt.json). SHA-256 `a562fd937d73e60cc9044b1200b4580d7709ddda724e9ffcd97653f4eb477a2c`. Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stdout). SHA-256 `397b7c61f88621890bc0a7c78a26a498ab5415198aa21afc206dbec8ee67133f`. Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/receipt.json). SHA-256 `7607abf485d0ad2d2fda32c34c66e43aae7d82390c994a68150780602070dbc3`. Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stdout). SHA-256 `c8d83a2775addfe0df770ec98d756b6676eab3b038d17a33a0ac95e0599520a2`. Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/receipt.json). SHA-256 `74a06840c0c5a3b071b26d0ca0db0253dc22f482ce28d3617f72457a4152794e`. Original selected executable/version, full argv/environment and exact required build/source and stream hashes.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stdout](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stdout). SHA-256 `dd85ff426b11610af5c4a1eb2f7067c0610c6a6cfd753d87aa93d8c2bf6997aa`. Exact31 reflected definition/call rows for available TclOO, or exact2 version/unavailable rows in this fresh stock process.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stderr](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr; guest caught error outcomes remain independently visible in stdout.
- `tcl8.6-constructor-source` (source-anchor): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/8.6.18/tclOODefineCmds.c](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/8.6.18/tclOODefineCmds.c). SHA-256 `56fb36d62f1ec51cca4cc5177bb90dc57986021206ea9700454ff67db7905e81`. Lines 1233–1265. Exact retained release constructor-definition source branch, independently inspected; this window is not a native object/frame observation.
- `tcl9.0-constructor-source` (source-anchor): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.0.4/tclOODefineCmds.c](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.0.4/tclOODefineCmds.c). SHA-256 `1eb881c851f09719a9cb932c5ceb36034b96569affda2b2f870652a0bf29426d`. Lines 1858–1894. Exact retained release constructor-definition source branch, independently inspected; this window is not a native object/frame observation.
- `tcl9.1-constructor-source` (source-anchor): [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.1.0/tclOODefineCmds.c](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/sources/9.1.0/tclOODefineCmds.c). SHA-256 `bd254581a59362769c56269dfb009dd9ed5e5b9bcf00ac67b5738e97e50b5106`. Lines 1832–1868. Exact retained release constructor-definition source branch, independently inspected; this window is not a native object/frame observation.

## Source inspection

tcl8.6 8.6.18, revision `Original retained release source checkout 8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOODefineConstructorObjCmd`, lines 1233–1265. Full-source SHA-256 `56fb36d62f1ec51cca4cc5177bb90dc57986021206ea9700454ff67db7905e81`; snippet SHA-256 `d82bc1fc1bb3f00a4cf7e4bfbbcb4c3d54a6c82d95a3824f3e5a9b45557e143f`; retained evidence `tcl8.6-constructor-source`.

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

tcl9.0 9.0.4, revision `Original retained release source checkout 9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOODefineConstructorObjCmd`, lines 1858–1894. Full-source SHA-256 `1eb881c851f09719a9cb932c5ceb36034b96569affda2b2f870652a0bf29426d`; snippet SHA-256 `0f31dbfdce05dc9a3151af324f98d2674f5ff5c5d71aabfd0f3363d9aff61b1a`; retained evidence `tcl9.0-constructor-source`.

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

tcl9.1 9.1.0, revision `Original retained release source checkout 9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineConstructorObjCmd`, lines 1832–1868. Full-source SHA-256 `bd254581a59362769c56269dfb009dd9ed5e5b9bcf00ac67b5738e97e50b5106`; snippet SHA-256 `0f31dbfdce05dc9a3151af324f98d2674f5ff5c5d71aabfd0f3363d9aff61b1a`; retained evidence `tcl9.1-constructor-source`.

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
