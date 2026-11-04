# naming.info.version-source-owner

Kind: `source-anchor`

## Problem statement

Using C version globals for a Jim info query makes a valid fresh Jim interpreter fail when no Tcl global exists. Conversely, copying a native probe git version into a different Rust engine would claim an unrelated build. The selected report source must distinguish a live C global, a genuinely supplied Jim build description and the independent Jim core-release fallback.

## Question

Which implementation source supplies info patchlevel on C Tcl and Jim, and what does Jim use when no git build description is supplied?

## Conclusion

Inspected C workers read live global tcl_patchLevel. Jim uses its own JIM_GITVERSION when compiled, otherwise formats JIM_VERSION as the core release. The Rust adapter has no separately supplied Jim git-build description and therefore reports the selected core release 0.84, independently of Tcl globals. No native executable version or git revision is borrowed for that adapter.

## Scope

Exact pinned C5 InfoPatchLevelCmd windows and current pinned Jim_InfoCoreCommand switch window. Source inspection supports the reporting purpose and fallback; it does not prove Rust execution, arbitrary version strings, headers, caches or engine equivalence.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Source inspection only; exact original full source hash 92695cd7731ce21ab5441fb6ce2d23e5cb44f020256a607397ebe836b5a9b14a. No new executable or Rust outcome inferred.. Channel: Inspected original implementation function window; no interpreter input channel.. Dialect: Tcl.

C reads the live tcl_patchLevel global with TCL_GLOBAL_ONLY and reports a missing variable; this is distinct from Jim compile-time reporting.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Source inspection only; exact original full source hash 51be49bd7e373aa319cb2affb341853473cbbf089694650bfff7689d3c925d8e. No new executable or Rust outcome inferred.. Channel: Inspected original implementation function window; no interpreter input channel.. Dialect: Tcl.

C reads the live tcl_patchLevel global with TCL_GLOBAL_ONLY and reports a missing variable; this is distinct from Jim compile-time reporting.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Source inspection only; exact original full source hash 26d3d423720df253aefd7ad412e939165282a1f80fd6cd8c42900875d9627a23. No new executable or Rust outcome inferred.. Channel: Inspected original implementation function window; no interpreter input channel.. Dialect: Tcl.

C reads the live tcl_patchLevel global with TCL_GLOBAL_ONLY and reports a missing variable; this is distinct from Jim compile-time reporting.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Source inspection only; exact original full source hash af564d5d77f8f70cf120dbd49509a8580130cc89352f52424a01920fb0da4fa7. No new executable or Rust outcome inferred.. Channel: Inspected original implementation function window; no interpreter input channel.. Dialect: Tcl.

C reads the live tcl_patchLevel global with TCL_GLOBAL_ONLY and reports a missing variable; this is distinct from Jim compile-time reporting.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Source inspection only; exact original full source hash d5b92a5b9c8c923142db6b76f816cbe6ca857255861a3c3dc53787fa9b335e5a. No new executable or Rust outcome inferred.. Channel: Inspected original implementation function window; no interpreter input channel.. Dialect: Tcl.

C reads the live tcl_patchLevel global with TCL_GLOBAL_ONLY and reports a missing variable; this is distinct from Jim compile-time reporting.

### jim

Status: `inspected`. Version: 0.84 source / 5bac7c99. Build: Source inspection only; exact original full source hash fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867. No new executable or Rust outcome inferred.. Channel: Inspected original implementation function window; no interpreter input channel.. Dialect: Jim Tcl.

Jim uses JIM_GITVERSION when defined, otherwise falls through to INFO_VERSION formatting JIM_VERSION / 100 and % 100. No Tcl version globals are read.

### bigip

Status: `not-tested`. Version: not inspected. Build: not recorded. Channel: not inspected. Dialect: F5 iRules.

No appliance implementation inspected.

## Historical references

- `info-source-tcl8.4` (source-anchor): [rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/8.4.20-info-version.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/8.4.20-info-version.json). JSON pointer `/snippet`. Exact retained function window and whole original source digest for the independently selected report source.
- `info-source-tcl8.5` (source-anchor): [rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/8.5.19-info-version.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/8.5.19-info-version.json). JSON pointer `/snippet`. Exact retained function window and whole original source digest for the independently selected report source.
- `info-source-tcl8.6` (source-anchor): [rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/8.6.18-info-version.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/8.6.18-info-version.json). JSON pointer `/snippet`. Exact retained function window and whole original source digest for the independently selected report source.
- `info-source-tcl9.0` (source-anchor): [rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/9.0.4-info-version.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/9.0.4-info-version.json). JSON pointer `/snippet`. Exact retained function window and whole original source digest for the independently selected report source.
- `info-source-tcl9.1` (source-anchor): [rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/9.1.0-info-version.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/9.1.0-info-version.json). JSON pointer `/snippet`. Exact retained function window and whole original source digest for the independently selected report source.
- `info-source-jim` (source-anchor): [rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/jim-info-version.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/source-anchors/jim-info-version.json). JSON pointer `/snippet`. Exact retained function window and whole original source digest for the independently selected report source.

## Source inspection

tcl8.4 8.4.20, revision `Pinned Tcl 8.4.20 source`, `generic/tclCmdIL.c`, function `InfoPatchLevelCmd`, lines 1845–1870. Retained reference `info-source-tcl8.4`.

```text
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    CONST char *patchlevel;

    if (objc != 2) {
        Tcl_WrongNumArgs(interp, 2, objv, NULL);
        return TCL_ERROR;
    }

    patchlevel = Tcl_GetVar(interp, "tcl_patchLevel",
            (TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG));
    if (patchlevel != NULL) {
        Tcl_SetStringObj(Tcl_GetObjResult(interp), patchlevel, -1);
        return TCL_OK;
    }
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoProcsCmd --

```

tcl8.5 8.5.19, revision `Pinned Tcl 8.5.19 source`, `generic/tclCmdIL.c`, function `InfoPatchLevelCmd`, lines 1658–1683. Retained reference `info-source-tcl8.5`.

```text
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *patchlevel;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }

    patchlevel = Tcl_GetVar(interp, "tcl_patchLevel",
	    (TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG));
    if (patchlevel != NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(patchlevel, -1));
	return TCL_OK;
    }
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoProcsCmd --

```

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18 source`, `generic/tclCmdIL.c`, function `InfoPatchLevelCmd`, lines 1813–1838. Retained reference `info-source-tcl8.6`.

```text
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *patchlevel;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }

    patchlevel = Tcl_GetVar(interp, "tcl_patchLevel",
	    (TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG));
    if (patchlevel != NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(patchlevel, -1));
	return TCL_OK;
    }
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoProcsCmd --

```

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4 source`, `generic/tclCmdIL.c`, function `InfoPatchLevelCmd`, lines 1807–1832. Retained reference `info-source-tcl9.0`.

```text
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *patchlevel;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }

    patchlevel = Tcl_GetVar2(interp, "tcl_patchLevel", NULL,
	    (TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG));
    if (patchlevel != NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(patchlevel, -1));
	return TCL_OK;
    }
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoProcsCmd --

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0 source`, `generic/tclCmdIL.c`, function `InfoPatchLevelCmd`, lines 1790–1815. Retained reference `info-source-tcl9.1`.

```text
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    const char *patchlevel;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }

    patchlevel = Tcl_GetVar2(interp, "tcl_patchLevel", NULL,
	    (TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG));
    if (patchlevel != NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(patchlevel, -1));
	return TCL_OK;
    }
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InfoProcsCmd --

```

jim jim, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `Jim_InfoCoreCommand`, lines 16274–16287. Retained reference `info-source-jim`.

```text
        case INFO_PATCHLEVEL:
           /* bootstrap jimsh doesn't have this so fall through */
#ifdef JIM_GITVERSION
            Jim_SetResultString(interp, JIM_GITVERSION, -1);
            return JIM_OK;
#endif

        case INFO_VERSION:{
                char versionbuf[64];
                snprintf(versionbuf, sizeof(versionbuf), "%d.%d", JIM_VERSION / 100, JIM_VERSION % 100);
                Jim_SetResultString(interp, versionbuf, -1);
                return JIM_OK;
            }


```


## Consumer bindings

- [rust/tcl-registry/src/native_info_version.rs](../../../../rust/tcl-registry/src/native_info_version.rs), `InvocationDialect::native_info_version_source`: Select the actual report source, independently of caller variable lookup and external build version observations.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `info_version_report`: Consume live C global or selected Jim fallback report without creating foreign bootstrap variables.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `info_version_report`: Apply the same reporting purpose on the excluded Runtime port.
- [rust/tcl-registry/src/native_info_version.rs](../../../../rust/tcl-registry/src/native_info_version.rs), `native_info_version::tests::selected_version_report_keeps_jim_independent_of_tcl_globals` (linked): Select C live-global and Jim core-report sources independently; wrong engine member declines.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info::tests::actual_jim_version_report_does_not_read_tcl_bootstrap_globals` (linked): A selected fresh Jim core reports 0.84 without Tcl globals and ignores an unrelated Tcl patchLevel write; no git-build claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

Source inspection is reproduced by comparing the whole source SHA-256, exact line window and snippet SHA-256 from the pinned original source. Public capture replay does not execute this source-inspection question. Rust coverage outcome requires a separate actual test receipt.
