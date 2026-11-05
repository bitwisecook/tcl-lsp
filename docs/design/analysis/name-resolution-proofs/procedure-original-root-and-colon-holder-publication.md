# naming.procedure.original-root-and-colon-holder-publication

Kind: `native-observation`

## Problem statement

A source model can collapse a literal root Procedure name beginning with one colon, or treat Procedure holder publication and a lambda namespace spelling as the same name operation. The actual release-specific public results and independently inspected producer recipes must remain distinct.

## Question

In the exact safe-child source, where is a root :source procedure visible, where does holder :ns publish its source procedure, and which namespace does a literal lambda with explicit :ns or omitted namespace select?

## Conclusion

All five C releases keep the root :source procedure distinct from the hidden plain source command. C8.4/8.5 publish a Procedure declared under :ns into ns; C8.6/9.0/9.1 retain it under :ns. Tcl8.4 cannot execute Apply. Tcl8.5 and later select ns for a lambda explicitly naming :ns, while an omitted lambda namespace selects root. Jim rejects the child surface and supplies no colon-holder or lambda namespace answer.

## Scope

Six fresh CLI processes retain six actual version rows and 78 sequential caught observations. The thirteen controls within each process share creation and procedure state; independent catch completion does not mean independent interpreter state. Results measure finite public names, inventories, caught arity/missing-command outcomes and errorCode values only. Two exact C8.4/C8.5 publisher windows and four C8.5–C9.1 lambda windows are independent source-inspection evidence. Original requested source-anchor metadata is retained unchanged; four Lambda ranges are replaced only in the separate LF-normalized joins. Neither source inspection nor CLI results measure input objects, native tokens, pointer/epoch correspondence, physical namespace/cell allocation, arbitrary constructor Normal, reached runtime frame, compiler admission or BIG-IP behavior. Linked Rust API coverage separates the counted lambda namespace object prefix from subsequent CString lookup extent. A conditional child-source fixture keeps C8.5 non-global procedure holder reparsing distinct from C8.6+ literal holder publication; it supplies no child frame/cell, selected callable or runtime execution observation.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row. Version: tcl8.4 8.4.20.

Root :source is callable as :source while the hidden plain source remains unavailable. The procedure declared under holder :ns is visible and callable under ns, and absent from :ns. Apply is unavailable, so the two lambda controls provide no lambda namespace answer. Missing source/apply commands retain errorCode NONE.

### tcl8.5

Status: `observed`. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row. Version: tcl8.5 8.5.19.

Root :source remains distinct from hidden plain source. Holder :ns publishes its source procedure under ns. The explicit lambda namespace :ns selects that ns procedure and reaches its arity failure; the omitted lambda namespace selects root and cannot call hidden source. These failures retain errorCode NONE.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Root :source remains distinct from hidden plain source. Holder :ns retains its source procedure under :ns, and ns has none. Calling it with a.tcl reaches TCL WRONGARGS. The explicit lambda namespace :ns selects ns and cannot find source; the omitted namespace selects root and likewise cannot find the hidden command, with TCL LOOKUP COMMAND source.

### jim

Status: `unsupported`. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row. Version: jim 0.84-9-g5bac7c9.

Every tested child-interpreter form fails with the actual interp arity message. These failures supply no Jim colon-holder, Procedure publication or lambda namespace answer; they do not establish that the bare interp command is absent.

### bigip

Status: `not-tested`.

No appliance observation of these exact root and colon-holder publication controls.

## Source inspection

tcl8.4 8.4.20, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.4.20/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 126–143. Retained reference `colon-source-window-0`.

```text
     * Now create a command for the procedure. This will initially be in
     * the current namespace unless the procedure's name included namespace
     * qualifiers. To create the new command in the right namespace, we
     * generate a fully qualified name for it.
     */

    Tcl_DStringInit(&ds);
    if (nsPtr != iPtr->globalNsPtr) {
	Tcl_DStringAppend(&ds, nsPtr->fullName, -1);
	Tcl_DStringAppend(&ds, "::", 2);
    }
    Tcl_DStringAppend(&ds, procName, -1);

    Tcl_CreateCommand(interp, Tcl_DStringValue(&ds), TclProcInterpProc,
	    (ClientData) procPtr, TclProcDeleteProc);
    cmd = Tcl_CreateObjCommand(interp, Tcl_DStringValue(&ds),
	    TclObjInterpProc, (ClientData) procPtr, TclProcDeleteProc);

```

tcl8.5 8.5.19, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.5.19/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 173–187. Retained reference `colon-source-window-1`.

```text
     * Now create a command for the procedure. This will initially be in the
     * current namespace unless the procedure's name included namespace
     * qualifiers. To create the new command in the right namespace, we
     * generate a fully qualified name for it.
     */

    Tcl_DStringInit(&ds);
    if (nsPtr != iPtr->globalNsPtr) {
	Tcl_DStringAppend(&ds, nsPtr->fullName, -1);
	Tcl_DStringAppend(&ds, "::", 2);
    }
    Tcl_DStringAppend(&ds, procName, -1);

    cmd = Tcl_CreateObjCommand(interp, Tcl_DStringValue(&ds),
	    TclObjInterpProc, (ClientData) procPtr, TclProcDeleteProc);

```

tcl8.5 8.5.19, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.5.19/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2591–2602. Retained reference `colon-source-window-2`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```

tcl8.6 8.6.18, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.6.18/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2569–2580. Retained reference `colon-source-window-3`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```

tcl9.0 9.0.4, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl9.0.4/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2603–2614. Retained reference `colon-source-window-4`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((nsName[0] != ':') || (nsName[1] != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```

tcl9.1 9.1.0, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl9.1.0/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2503–2514. Retained reference `colon-source-window-5`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::command_publication_slot`: Pure release-specific Procedure publication projection, including literal root colon handling; native CLI observations remain an independent finite question.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNamePurpose`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_input`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_path`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `literal_lambda_body`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::original_root_procedure_publication_preserves_a_literal_colon` (linked): Two original root spellings across five C policies produce their exact pure command slots. These Rust slot assertions do not replay the safe-child source or establish native object/holder correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::original_lambda_namespace_preserves_counted_object_and_lookup_purposes` (linked): Pure C8.5–C9.1 original getter-byte prefix and subsequent independent CString lookup projection retain count/zero/colon/Unicode boundaries. Unsupported C8.4/Jim C-specific purpose refuses. This is Rust API correspondence only, with no actual namespace existence, object/header identity, entered frame or native CLI replay.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `analyser::interp_visibility::tests::original_child_lambda_visibility_uses_its_own_literal_namespace` (linked): An authentic original literal lambda retains its independent C85+ namespace object prefix before selecting source lookup geometry. In the measured fixed :ns child script C85 procedure publication reparses its non-global holder into ns, so only the default-root source b.tcl has a conditional hidden visibility warning. C86+ keeps procedure publication in :ns while Apply selects ns, so both source a.tcl and default-root source b.tcl have conditional visibility hypotheses. Exact original head anchors remain asserted; successful operations, source-body entry and absence of observer mutation remain explicit assumptions. The procedure-publication CLI proof is independent of source-owner assertions and supplies no actual child frame/cell or selected callable.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18)
- [rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0)
- [rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20)
- [rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4)
- [rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19)
- [rust/tcl-registry/tests/data/native_child_colon_publication/jim](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/jim)
