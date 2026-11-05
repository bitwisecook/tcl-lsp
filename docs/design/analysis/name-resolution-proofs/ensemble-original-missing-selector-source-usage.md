# naming.ensemble.original-missing-selector-source-usage

Kind: `source-anchor`

## Problem statement

A stock info invocation can fail at its actual generic ensemble dispatcher before the info worker. Keeping an info-only suffix and a different generic suffix lets consumers drift and misstates C8.5 diagnostics. The shared ensemble recipe must select the inspected dispatcher suffix, while C8.4 option dispatch and Jim scripted/core info remain separate purposes.

## Question

Which exact fixed missing-selector usage suffix does each inspected original C ensemble execution dispatcher append, independently of info command identity?

## Conclusion

The independently pinned original C8.5 NsEnsembleImplementationCmd passes subcommand ?argument ...? to Tcl_WrongNumArgs when its selector is missing. The inspected C8.6, C9.0 and C9.1 NsEnsembleImplementationCmdNR branches construct their parameter prefix and append subcommand ?arg ...?. NativeEnsembleConfigurationProtocol::missing_selector_usage selects that fixed suffix under the genuine C release; the info missing-selector purpose delegates to it for C8.5+. Runtime and VM consume the same selected suffix while original invocation words, parameters, active configuration, rendering and result allocation retain independent owners. C8.4 option dispatch and Jim purposes remain separate. These are exact source branch facts and a pure software selection contract, not observed generic-ensemble execution, original object/table/cache identity or a reached Runtime/VM result.

## Scope

Read-only inspection retains four whole original C source files, their exact dispatcher excerpts, the unchanged original question request and four independent Native292 compilation receipts. Those compilation receipts supply existing build metadata only; no new compiler/provider/Rust process runs and no generic ensemble invocation is observed by this appendix. Native292 public info messages remain a separate question. C8.4, Jim and BIG-IP are not inspected here. The fixed missing-selector suffix is distinct from parameter object quoting, original header storage, result allocation and Native dispatch admission. One linked pure API control checks selected C8.5 versus C8.6+ suffix and refusal outside its exact selected release/family.

## Provider answers

### tcl8.4, jim, bigip

Status: `not-tested`.

No source inspection or generic ensemble process for this provider is supplied by this selected C8.5+ source question. C8.4 option and Jim/hosted purposes remain independent.

### tcl8.5

Status: `inspected`. Channel: Read-only whole source and exact NsEnsembleImplementationCmd excerpt; independent build receipt metadata. Version: tcl8.5 Pinned source 8.5.19; no generic ensemble execution/version process.

Read-only source branch passes/appends the fixed suffix subcommand ?argument ...?. Parameter-prefix construction, original invocation/header, renderer and actual handler execution require their independent premises; this source observation grants no generic public result or software pass.

### tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Read-only whole source and exact NsEnsembleImplementationCmdNR excerpt; independent build receipt metadata. Version: tcl8.6 Pinned source 8.6.18; no generic ensemble execution/version process; tcl9.0 Pinned source 9.0.4; no generic ensemble execution/version process; tcl9.1 Pinned source 9.1.0; no generic ensemble execution/version process.

Read-only source branch passes/appends the fixed suffix subcommand ?arg ...?. Parameter-prefix construction, original invocation/header, renderer and actual handler execution require their independent premises; this source observation grants no generic public result or software pass.

## Source inspection

tcl8.5 Pinned original source 8.5.19; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NsEnsembleImplementationCmd`, lines 6025–6053. Retained reference `ensemble-missing-selector-source297-8.5.19-tclNamesp.c`.

```text
static int
NsEnsembleImplementationCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    EnsembleConfig *ensemblePtr = clientData;
				/* The ensemble itself. */
    Tcl_Obj **tempObjv;		/* Space used to construct the list of
				 * arguments to pass to the command that
				 * implements the ensemble subcommand. */
    int result;			/* The result of the subcommand execution. */
    Tcl_Obj *prefixObj;		/* An object containing the prefix words of
				 * the command that implements the
				 * subcommand. */
    Tcl_HashEntry *hPtr;	/* Used for efficient lookup of fully
				 * specified but not yet cached command
				 * names. */
    Tcl_Obj **prefixObjv;	/* The list of objects to substitute in as the
				 * target command prefix. */
    int prefixObjc;		/* Size of prefixObjv of course! */
    int reparseCount = 0;	/* Number of reparses. */

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "subcommand ?argument ...?");
	return TCL_ERROR;
    }

```

tcl8.6 Pinned original source 8.6.18; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `NsEnsembleImplementationCmdNR`, lines 1677–1723. Retained reference `ensemble-missing-selector-source297-8.6.18-tclEnsemble.c`.

```text
static int
NsEnsembleImplementationCmdNR(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    EnsembleConfig *ensemblePtr = (EnsembleConfig *)clientData;
				/* The ensemble itself. */
    Tcl_Obj *prefixObj;		/* An object containing the prefix words of
				 * the command that implements the
				 * subcommand. */
    Tcl_HashEntry *hPtr;	/* Used for efficient lookup of fully
				 * specified but not yet cached command
				 * names. */
    int reparseCount = 0;	/* Number of reparses. */
    Tcl_Obj *errorObj;		/* Used for building error messages. */
    Tcl_Obj *subObj;
    int subIdx;

    /*
     * Must recheck objc since numParameters might have changed. See test
     * namespace-53.9.
     */

  restartEnsembleParse:
    subIdx = 1 + ensemblePtr->numParameters;
    if (objc < subIdx + 1) {
	/*
	 * No subcommand argument. Make error message.
	 */

	Tcl_DString buf;	/* Message being built */

	Tcl_DStringInit(&buf);
	if (ensemblePtr->parameterList) {
	    Tcl_DStringAppend(&buf,
		    TclGetString(ensemblePtr->parameterList), -1);
	    TclDStringAppendLiteral(&buf, " ");
	}
	TclDStringAppendLiteral(&buf, "subcommand ?arg ...?");
	Tcl_WrongNumArgs(interp, 1, objv, Tcl_DStringValue(&buf));
	Tcl_DStringFree(&buf);

	return TCL_ERROR;
    }

```

tcl9.0 Pinned original source 9.0.4; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `NsEnsembleImplementationCmdNR`, lines 1748–1793. Retained reference `ensemble-missing-selector-source297-9.0.4-tclEnsemble.c`.

```text
static int
NsEnsembleImplementationCmdNR(
    void *clientData,		/* The ensemble this is the impl. of. */
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    EnsembleConfig *ensemblePtr = (EnsembleConfig *) clientData;
				/* The ensemble itself. */
    Tcl_Obj *prefixObj;		/* An object containing the prefix words of
				 * the command that implements the
				 * subcommand. */
    Tcl_HashEntry *hPtr;	/* Used for efficient lookup of fully
				 * specified but not yet cached command
				 * names. */
    int reparseCount = 0;	/* Number of reparses. */
    Tcl_Obj *errorObj;		/* Used for building error messages. */
    Tcl_Obj *subObj;
    Tcl_Size subIdx;

    /*
     * Must recheck objc since numParameters might have changed. See test
     * namespace-53.9.
     */

  restartEnsembleParse:
    subIdx = 1 + ensemblePtr->numParameters;
    if (objc < subIdx + 1) {
	/*
	 * No subcommand argument. Make error message.
	 */

	Tcl_DString buf;	/* Message being built */

	Tcl_DStringInit(&buf);
	if (ensemblePtr->parameterList) {
	    TclDStringAppendObj(&buf, ensemblePtr->parameterList);
	    TclDStringAppendLiteral(&buf, " ");
	}
	TclDStringAppendLiteral(&buf, "subcommand ?arg ...?");
	Tcl_WrongNumArgs(interp, 1, objv, Tcl_DStringValue(&buf));
	Tcl_DStringFree(&buf);

	return TCL_ERROR;
    }

```

tcl9.1 Pinned original source 9.1.0; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `NsEnsembleImplementationCmdNR`, lines 1749–1794. Retained reference `ensemble-missing-selector-source297-9.1.0-tclEnsemble.c`.

```text
static int
NsEnsembleImplementationCmdNR(
    void *clientData,		/* The ensemble this is the impl. of. */
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    EnsembleConfig *ensemblePtr = (EnsembleConfig *) clientData;
				/* The ensemble itself. */
    Tcl_Obj *prefixObj;		/* An object containing the prefix words of
				 * the command that implements the
				 * subcommand. */
    Tcl_HashEntry *hPtr;	/* Used for efficient lookup of fully
				 * specified but not yet cached command
				 * names. */
    int reparseCount = 0;	/* Number of reparses. */
    Tcl_Obj *errorObj;		/* Used for building error messages. */
    Tcl_Obj *subObj;
    Tcl_Size subIdx;

    /*
     * Must recheck objc since numParameters might have changed. See test
     * namespace-53.9.
     */

  restartEnsembleParse:
    subIdx = 1 + ensemblePtr->numParameters;
    if (objc < subIdx + 1) {
	/*
	 * No subcommand argument. Make error message.
	 */

	Tcl_DString buf;	/* Message being built */

	Tcl_DStringInit(&buf);
	if (ensemblePtr->parameterList) {
	    TclDStringAppendObj(&buf, ensemblePtr->parameterList);
	    TclDStringAppendLiteral(&buf, " ");
	}
	TclDStringAppendLiteral(&buf, "subcommand ?arg ...?");
	Tcl_WrongNumArgs(interp, 1, objv, Tcl_DStringValue(&buf));
	Tcl_DStringFree(&buf);

	return TCL_ERROR;
    }

```

## Consumer bindings

- [rust/tcl-registry/src/native_ensemble.rs](../../../../rust/tcl-registry/src/native_ensemble.rs), `NativeEnsembleConfigurationProtocol::missing_selector_usage`: Select the inspected fixed generic C ensemble suffix under the actual release/family; no original invocation, parameter/header or execution identity.
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `InvocationDialect::native_info_original_missing_selector_usage`: Select C8.4 option usage separately and delegate C8.5+ fixed suffix to its genuine generic ensemble purpose; Jim keeps its independent presenter.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::ensemble_wrong_args_for_invocation`: Append the selected fixed suffix while retaining original invocation/parameter and rendering owners separately; missing protocol refuses.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::ensemble_wrong_args`: Consume the same selected suffix with independently retained invocation words, parameter spelling and usage/header protocol; no observed native private object is recreated by this byte recipe.
- [rust/tcl-registry/src/native_ensemble.rs](../../../../rust/tcl-registry/src/native_ensemble.rs), `native_ensemble::configuration_surface_tests::actual_ensemble_missing_selector_usage_is_selected_independently_of_info` (linked): Pure selected protocol keeps C8.5 argument and C8.6/C9.0/C9.1 arg suffixes equal to their inspected dispatcher recipes; info delegates for those releases. C8.4, Jim, hosted and missing release/family cannot borrow the generic C recipe. This definition is no executed Rust or generic native ensemble result.

## Evidence

- [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297)
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs)
- [rust/tcl-registry/src/native_ensemble.rs](../../../../rust/tcl-registry/src/native_ensemble.rs)
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs)
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs)
