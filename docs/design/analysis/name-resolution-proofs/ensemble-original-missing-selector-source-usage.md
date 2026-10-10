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

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No source inspection or generic ensemble process for this provider is supplied by this selected C8.5+ source question. C8.4 option and Jim/hosted purposes remain independent.

### tcl8.5

Status: `observed`. Version: Pinned source 8.5.19; no generic ensemble execution/version process.. Build: Existing independent Native292 compile receipt only: archive SHA 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; public header SHA c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5. No new compiler or generic ensemble process.. Channel: Read-only whole source and exact NsEnsembleImplementationCmd excerpt; independent build receipt metadata.. Dialect: Original C Tcl 8.5.19 ensemble source implementation.

Read-only source branch passes/appends the fixed suffix subcommand ?argument ...?. Parameter-prefix construction, original invocation/header, renderer and actual handler execution require their independent premises; this source observation grants no generic public result or software pass.

### tcl8.6

Status: `observed`. Version: Pinned source 8.6.18; no generic ensemble execution/version process.. Build: Existing independent Native292 compile receipt only: archive SHA 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; public header SHA aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245. No new compiler or generic ensemble process.. Channel: Read-only whole source and exact NsEnsembleImplementationCmdNR excerpt; independent build receipt metadata.. Dialect: Original C Tcl 8.6.18 ensemble source implementation.

Read-only source branch passes/appends the fixed suffix subcommand ?arg ...?. Parameter-prefix construction, original invocation/header, renderer and actual handler execution require their independent premises; this source observation grants no generic public result or software pass.

### tcl9.0

Status: `observed`. Version: Pinned source 9.0.4; no generic ensemble execution/version process.. Build: Existing independent Native292 compile receipt only: archive SHA dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; public header SHA eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. No new compiler or generic ensemble process.. Channel: Read-only whole source and exact NsEnsembleImplementationCmdNR excerpt; independent build receipt metadata.. Dialect: Original C Tcl 9.0.4 ensemble source implementation.

Read-only source branch passes/appends the fixed suffix subcommand ?arg ...?. Parameter-prefix construction, original invocation/header, renderer and actual handler execution require their independent premises; this source observation grants no generic public result or software pass.

### tcl9.1

Status: `observed`. Version: Pinned source 9.1.0; no generic ensemble execution/version process.. Build: Existing independent Native292 compile receipt only: archive SHA 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; public header SHA 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. No new compiler or generic ensemble process.. Channel: Read-only whole source and exact NsEnsembleImplementationCmdNR excerpt; independent build receipt metadata.. Dialect: Original C Tcl 9.1.0 ensemble source implementation.

Read-only source branch passes/appends the fixed suffix subcommand ?arg ...?. Parameter-prefix construction, original invocation/header, renderer and actual handler execution require their independent premises; this source observation grants no generic public result or software pass.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No source inspection or generic ensemble process for this provider is supplied by this selected C8.5+ source question. C8.4 option and Jim/hosted purposes remain independent.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No source inspection or generic ensemble process for this provider is supplied by this selected C8.5+ source question. C8.4 option and Jim/hosted purposes remain independent.

## Exact evidence

- `ensemble-missing-selector-source297-8.5.19-independent-info292-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.5.19-independent-info292-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.5.19-independent-info292-compile.receipt.json). SHA-256 `92b36c7426600ed77ff8d022f07173b8bafc164203032ff4f4437f242269b5eb`. Exact existing independent Native292 compilation receipt retained as build metadata only; no generic ensemble execution or new compilation.
- `ensemble-missing-selector-source297-8.5.19-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.5.19-tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.5.19-tclNamesp.c). SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-8.5.19-tclNamesp.c-6025-6053.txt` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.5.19-tclNamesp.c-6025-6053.txt](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.5.19-tclNamesp.c-6025-6053.txt). SHA-256 `0b5cd811b1372fe44af95f18c2f7a8be2821951694bd32a6692eeeb984454301`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-8.6.18-independent-info292-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.6.18-independent-info292-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.6.18-independent-info292-compile.receipt.json). SHA-256 `6e9433414d044125e139dfa184aa375aad7f62d1f91c687c918889c221bc50f2`. Exact existing independent Native292 compilation receipt retained as build metadata only; no generic ensemble execution or new compilation.
- `ensemble-missing-selector-source297-8.6.18-tclEnsemble.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.6.18-tclEnsemble.c](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.6.18-tclEnsemble.c). SHA-256 `2c86e298e6beaad843ca15885ba454367117cdcbed43c509c69110521159a642`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-8.6.18-tclEnsemble.c-1677-1723.txt` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.6.18-tclEnsemble.c-1677-1723.txt](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/8.6.18-tclEnsemble.c-1677-1723.txt). SHA-256 `380b52525046dec123c3a11e034fdec02daa926c65dd8d86f720863ec2d3bbdd`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-9.0.4-independent-info292-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.0.4-independent-info292-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.0.4-independent-info292-compile.receipt.json). SHA-256 `592f0da1fe061ceb61facfee8eb8a40ba7fe0cd86103f185cc54de51d5cdfff7`. Exact existing independent Native292 compilation receipt retained as build metadata only; no generic ensemble execution or new compilation.
- `ensemble-missing-selector-source297-9.0.4-tclEnsemble.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.0.4-tclEnsemble.c](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.0.4-tclEnsemble.c). SHA-256 `6d0c49c5a53f2c7f89eafb3dc9113b4f2f418e48d6b9256f3c99150be9dd12e7`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-9.0.4-tclEnsemble.c-1748-1793.txt` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.0.4-tclEnsemble.c-1748-1793.txt](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.0.4-tclEnsemble.c-1748-1793.txt). SHA-256 `6a67583a17cfa58fdc9d29bb52bdfe6da92ff34def705f67cb039be583df6c05`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-9.1.0-independent-info292-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.1.0-independent-info292-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.1.0-independent-info292-compile.receipt.json). SHA-256 `914e2aae7f2e087f41d42d956c4e1e5ae0305e2ae638a81c4d828f5c16814a5b`. Exact existing independent Native292 compilation receipt retained as build metadata only; no generic ensemble execution or new compilation.
- `ensemble-missing-selector-source297-9.1.0-tclEnsemble.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.1.0-tclEnsemble.c](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.1.0-tclEnsemble.c). SHA-256 `3ad1e9efbb692a831d5d4b7545e3030868621532bb62ba94c551c0ce42983664`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-9.1.0-tclEnsemble.c-1749-1794.txt` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.1.0-tclEnsemble.c-1749-1794.txt](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/9.1.0-tclEnsemble.c-1749-1794.txt). SHA-256 `4bb38a0eb7fac957078455a74e977b8e3391c76c39b3cd176009207383ab179c`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `ensemble-missing-selector-source297-request.json` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/request.json](../../../../rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297/request.json). SHA-256 `3934a4024b350ce9f1db89cfcaa3227aded405beedbf3a9ddcf26e4e3812efe3`. Exact independently pinned original C source, dispatcher excerpt or read-only original question request; no provider execution or private object observation.
- `naming-ensemble-original-missing-selector-source-usage-runtime-rust-src-interp.rs` (implementation): [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs). SHA-256 `a35a824908f06db26fc08fecd5e6b018e30bd9b4ceeb2538fefc7e4b3d088c33`. Current source/API owner and software assertion definition; no executed Rust or native provider result.
- `naming-ensemble-original-missing-selector-source-usage-rust-tcl-registry-src-native_ensemble.rs` (implementation): [rust/tcl-registry/src/native_ensemble.rs](../../../../rust/tcl-registry/src/native_ensemble.rs). SHA-256 `e487096650aa40e243771afef534a5213c88566ea91c75053968effe1d6b57ac`. Current source/API owner and software assertion definition; no executed Rust or native provider result.
- `naming-ensemble-original-missing-selector-source-usage-rust-tcl-registry-src-native_index_lookup.rs` (implementation): [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs). SHA-256 `ad791cb98aa7cdc2760cba3c7f3d31daf40cdb268788bda6ca62a571f6633b3a`. Current source/API owner and software assertion definition; no executed Rust or native provider result.
- `naming-ensemble-original-missing-selector-source-usage-rust-tcl-vm-src-interp.rs` (implementation): [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs). SHA-256 `dbbcf66e209f9a6a8a6593268894016e8d2732b413528a876a82487eb45fe714`. Current source/API owner and software assertion definition; no executed Rust or native provider result.

## Source inspection

tcl8.5 Pinned original source 8.5.19; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NsEnsembleImplementationCmd`, lines 6025–6053. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `0b5cd811b1372fe44af95f18c2f7a8be2821951694bd32a6692eeeb984454301`; retained evidence `ensemble-missing-selector-source297-8.5.19-tclNamesp.c`.

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

tcl8.6 Pinned original source 8.6.18; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `NsEnsembleImplementationCmdNR`, lines 1677–1723. Full-source SHA-256 `2c86e298e6beaad843ca15885ba454367117cdcbed43c509c69110521159a642`; snippet SHA-256 `380b52525046dec123c3a11e034fdec02daa926c65dd8d86f720863ec2d3bbdd`; retained evidence `ensemble-missing-selector-source297-8.6.18-tclEnsemble.c`.

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

tcl9.0 Pinned original source 9.0.4; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `NsEnsembleImplementationCmdNR`, lines 1748–1793. Full-source SHA-256 `6d0c49c5a53f2c7f89eafb3dc9113b4f2f418e48d6b9256f3c99150be9dd12e7`; snippet SHA-256 `6a67583a17cfa58fdc9d29bb52bdfe6da92ff34def705f67cb039be583df6c05`; retained evidence `ensemble-missing-selector-source297-9.0.4-tclEnsemble.c`.

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

tcl9.1 Pinned original source 9.1.0; independent build metadata only., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `NsEnsembleImplementationCmdNR`, lines 1749–1794. Full-source SHA-256 `3ad1e9efbb692a831d5d4b7545e3030868621532bb62ba94c551c0ce42983664`; snippet SHA-256 `4bb38a0eb7fac957078455a74e977b8e3391c76c39b3cd176009207383ab179c`; retained evidence `ensemble-missing-selector-source297-9.1.0-tclEnsemble.c`.

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
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Interp::ensemble_wrong_args`: Consume the same selected suffix with independently retained invocation words, parameter spelling and usage/header protocol; no observed native private object is recreated by this byte recipe.
- [rust/tcl-registry/src/native_ensemble.rs](../../../../rust/tcl-registry/src/native_ensemble.rs), `native_ensemble::configuration_surface_tests::actual_ensemble_missing_selector_usage_is_selected_independently_of_info` (linked): Pure selected protocol keeps C8.5 argument and C8.6/C9.0/C9.1 arg suffixes equal to their inspected dispatcher recipes; info delegates for those releases. C8.4, Jim, hosted and missing release/family cannot borrow the generic C recipe. This definition is no executed Rust or generic native ensemble result.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
