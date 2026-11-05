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

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

This fresh stock CLI reports oo::class absent and an explicit unavailable row. No package or extension load was attempted and no constructor lifecycle operation ran; this is not a claim that every embedding of this interpreter family lacks TclOO.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; caught definition and argv acceptance controls, no native getter/header or compiler activation claim. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact31 rows show own empty/empty-malformed constructor removal and acceptance of counts0..3; space/comment a b bodies accept only2; malformed-space definition fails. After own empty removal, the superclass a b constructor still accepts only2. All remaining caught wrong-count operations report TCL WRONGARGS. Outer process code0 and empty stderr are retained separately.

### bigip

Status: `not-tested`.

External hosted TclOO availability and constructor lifecycle require an independently captured actual context. No C/Jim process substitutes for BIG-IP.

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

## Evidence

- [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231)
- [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.4.20)
- [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.5.19)
- [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/8.6.18)
- [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.0.4)
- [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/9.1.0)
- [rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim](../../../../rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231/jim)
