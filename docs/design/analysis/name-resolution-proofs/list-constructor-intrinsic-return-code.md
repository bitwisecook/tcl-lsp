# naming.list.constructor-intrinsic-return-code

Kind: `source-anchor`

## Problem statement

A generic error alternative for object-vector list construction loses retained cells even though the selected intrinsic handler has a fixed return expression; callback effects remain independent.

## Question

Does the selected original List handler derive a guest error code from its children or result publication?

## Conclusion

Pinned C5 Tcl_ListObjCmd and Jim_ListCoreCommand finish with unconditional TCL_OK/JIM_OK. The original argv pointers are passed to native list construction without per-child string getters. This closes only the intrinsic return-code axis for a selected valid handler invocation. It does not close argv evaluation, command lookup, execution traces, previous result release/free callbacks, later rendering, allocation failure, or general Normal completion.

## Scope

Inspected exact C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim5bac7 handler functions. No guest/native/Rust launch; no Native object/frame/cache or Normal capability.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Source inspection only; no executed build association.. Channel: Inspected original handler function body; no guest input.. Dialect: Tcl.

The handler constructs a list from the original object vector, publishes it and returns the fixed OK constant; child getters are absent from this function. Prior-result release may affect world state but is not used to choose the intrinsic return code.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Source inspection only; no executed build association.. Channel: Inspected original handler function body; no guest input.. Dialect: Tcl.

The handler constructs a list from the original object vector, publishes it and returns the fixed OK constant; child getters are absent from this function. Prior-result release may affect world state but is not used to choose the intrinsic return code.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Source inspection only; no executed build association.. Channel: Inspected original handler function body; no guest input.. Dialect: Tcl.

The handler constructs a list from the original object vector, publishes it and returns the fixed OK constant; child getters are absent from this function. Prior-result release may affect world state but is not used to choose the intrinsic return code.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Source inspection only; no executed build association.. Channel: Inspected original handler function body; no guest input.. Dialect: Tcl.

The handler constructs a list from the original object vector, publishes it and returns the fixed OK constant; child getters are absent from this function. Prior-result release may affect world state but is not used to choose the intrinsic return code.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Source inspection only; no executed build association.. Channel: Inspected original handler function body; no guest input.. Dialect: Tcl.

The handler constructs a list from the original object vector, publishes it and returns the fixed OK constant; child getters are absent from this function. Prior-result release may affect world state but is not used to choose the intrinsic return code.

### jim

Status: `inspected`. Version: pinned5bac7c99ad65864c87da513e22e2f01703fa4e03. Build: Source inspection only; no executed build association.. Channel: Inspected original handler function body; no guest input.. Dialect: Jim Tcl.

The handler constructs a list from the original object vector, publishes it and returns the fixed OK constant; child getters are absent from this function. Prior-result release may affect world state but is not used to choose the intrinsic return code.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `tcl8.4-source` (source-anchor): [rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.4-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.4-source.json). SHA-256 `2f6f9ee45bd1bf8fc2a55096a0c7bc4272af35fe3489946f7547c2504ae463c1`. JSON pointer `/snippet`. Exact original selected constructor signature/body including unconditional intrinsic return; pinned full source digest.
- `tcl8.5-source` (source-anchor): [rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.5-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.5-source.json). SHA-256 `9b9f74bf01464805b398bd4bd2e6bdcd70ebdf84d70b5326d33d2275b128ea7c`. JSON pointer `/snippet`. Exact original selected constructor signature/body including unconditional intrinsic return; pinned full source digest.
- `tcl8.6-source` (source-anchor): [rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.6-source.json). SHA-256 `605dc0b291fc2cde4d4ea500c5320b3516942cddce949ed34f6a003263e799ea`. JSON pointer `/snippet`. Exact original selected constructor signature/body including unconditional intrinsic return; pinned full source digest.
- `tcl9.0-source` (source-anchor): [rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.0-source.json). SHA-256 `2c038dea4151b159b3c8b49ed9313f16950d5eaabe45bfce04b9087182fdcbc6`. JSON pointer `/snippet`. Exact original selected constructor signature/body including unconditional intrinsic return; pinned full source digest.
- `tcl9.1-source` (source-anchor): [rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.1-source.json). SHA-256 `e872237449e9757c093520b3e729b66283e974104b913a3ef75eaa00fc379f30`. JSON pointer `/snippet`. Exact original selected constructor signature/body including unconditional intrinsic return; pinned full source digest.
- `jim-source` (source-anchor): [rust/tcl-registry/tests/data/native_list_constructor_return/jim-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/jim-source.json). SHA-256 `694a1ab764aae665e062e6add98f6e18a6ae48bbcc53d467548affc14c1d4e26`. JSON pointer `/snippet`. Exact original selected constructor signature/body including unconditional intrinsic return; pinned full source digest.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2889–2904. Full-source SHA-256 `92695cd7731ce21ab5441fb6ce2d23e5cb44f020256a607397ebe836b5a9b14a`; snippet SHA-256 `1d2d131e151897713c552100398dee9a4879bbea042b3746104e959fd883e1ad`; retained evidence `tcl8.4-source`.

```text
Tcl_ListObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    Tcl_Interp *interp;			/* Current interpreter. */
    register int objc;			/* Number of arguments. */
    register Tcl_Obj *CONST objv[];	/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise modify the interpreter's result object to be a list object.
     */
    
    if (objc > 1) {
	Tcl_SetListObj(Tcl_GetObjResult(interp), (objc-1), &(objv[1]));
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2268–2284. Full-source SHA-256 `51be49bd7e373aa319cb2affb341853473cbbf089694650bfff7689d3c925d8e`; snippet SHA-256 `82e203af72a5fe8fe67d7f38e7502546cd76c2742ef9ec1b7250c2dd6dd5c6bb`; retained evidence `tcl8.5-source`.

```text
Tcl_ListObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    register int objc,		/* Number of arguments. */
    register Tcl_Obj *const objv[])
				/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj((objc-1), &(objv[1])));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2431–2447. Full-source SHA-256 `26d3d423720df253aefd7ad412e939165282a1f80fd6cd8c42900875d9627a23`; snippet SHA-256 `a74d9c560c91ec643fd7edff041db9d702da40ae226cbfd45c7075fe4cc0e180`; retained evidence `tcl8.6-source`.

```text
Tcl_ListObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,		/* Number of arguments. */
    Tcl_Obj *const objv[])
				/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj(objc-1, &objv[1]));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2519–2535. Full-source SHA-256 `af564d5d77f8f70cf120dbd49509a8580130cc89352f52424a01920fb0da4fa7`; snippet SHA-256 `be4f6c31e1dc492a8b7e49eb3a96dc46f9634969ebe5df9aac03a58e56f45433`; retained evidence `tcl9.0-source`.

```text
Tcl_ListObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,		/* Number of arguments. */
    Tcl_Obj *const objv[])
				/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj(objc-1, &objv[1]));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2517–2532. Full-source SHA-256 `d5b92a5b9c8c923142db6b76f816cbe6ca857255861a3c3dc53787fa9b335e5a`; snippet SHA-256 `aa64f63faead601643bf58a91cf9ce5d4b8f29f14491ec2dfd8e526c61cd3563`; retained evidence `tcl9.1-source`.

```text
Tcl_ListObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj(objc-1, &objv[1]));
    }
    return TCL_OK;
}

```

jim pinned5bac7c99ad65864c87da513e22e2f01703fa4e03, revision `pinned5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ListCoreCommand`, lines 13466–13473. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `140d628bab218bd7c85321f088065714c399704719aaac3cce75f07ba6a4ccd1`; retained evidence `jim-source`.

```text
static int Jim_ListCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *listObjPtr;

    listObjPtr = Jim_NewListObj(interp, argv + 1, argc - 1);
    Jim_SetResult(interp, listObjPtr);
    return JIM_OK;
}

```


## Consumer bindings

- [rust/tcl-registry/src/native_result.rs](../../../../rust/tcl-registry/src/native_result.rs), `NativeResultContract::list_constructor_completion_code`: Select only the original C/Jim list constructor intrinsic return code after exact argv cardinality; result release, effects, observers and Normal completion remain separate.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `PreparedSourceNativeInvocation::completion_route`: Use the independently selected Leaf+ListArguments intrinsic code under genuine handler dispatch without issuing native Normal or world-effect certificates.
- [rust/tcl-registry/src/native_result.rs](../../../../rust/tcl-registry/src/native_result.rs), `native_result::tests::list_intrinsic_code_retains_unknown_effects_and_requires_native_cardinality` (linked): C5 and Jim empty/dynamic/native-byte operand vectors select only OK; missing dialect, hosted unsupported protocol, unknown argv shape, expansion, offset and different result contract decline. This test binding is not an execution claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

Reproduce exact full source SHA and retained function window/snippet SHA. No executable replay or Rust test is claimed.
