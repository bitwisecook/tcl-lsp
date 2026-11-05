# naming.interpreter.original-child-bootstrap-context

Kind: `native-observation`

## Problem statement

A full parent command catalogue does not establish a child catalogue or copied startup variables. The actual selected C and Jim child APIs need finite direct public queries before source/backend initialisation advice.

## Question

With the actual supported child API in the pinned fully initialised C/Jim process, which of eight queried commands occur in the child and which six explicitly supplied parent variables are copied?

## Conclusion

Every C provider reports namespace/package/interp/array/binary present and pack/unpack/class absent in its child; all six supplied parent variables argv/argc/argv0/jim::argv0/jim::exe/jim::lineedit are absent there. Jim reports all eight queried commands present and each of the six supplied variable values copied exactly as the original copy-name strings. All six original processes and compiler commands exit0 with empty stderr; the independent driver ORIGINAL completion is0/empty. Pinned Jim extension source separately registers core and static extensions and copies the six global variables. The finite public presence queries establish neither a full command roster nor backend handler support, private object/header identity, a C/Jim API equivalence or general child capability. The shared NativeChildInitialisation descriptor selects only implemented child bootstrap members and the six original global startup operands. NativeJimInterpreterCopyPurpose keeps counted object crossings separate from the startup-variable CString extent; a NUL-truncated startup value cannot donate a general object-copy protocol. The linked Runtime bootstrap control asserts the five supported namespace/package/interp/array/binary entries, actual scripted binary procedure and distinct software copy owners, without claiming implementation of every command present in the native Jim child. Separate direct host API controls retain a retired Jim owner or unavailable observed-frame refusal, preserve the prior parent result and withhold the child table/command/success result. The C control covers automatic, named and genuinely nested constructor owners and carries the exact typed nested failure to the parent. These software failures are not native failure observations, guest completion/options, private native identity or successful bootstrap grants.

## Scope

One unchanged ASCII LF source selects C interp-create versus the actual Jim interp handle API in a fresh full parent process. It explicitly supplies six parent variables and queries eight child names. Whole input/actual commands/streams/ELFs/receipts and independently required provider/header/archive/build/source hashes are retained. Full Jim interp/static-loader source is inspected independently. BIG-IP and Rust assertions are not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The five namespace/package/interp/array/binary child names are present; pack/unpack/class and all six supplied parent variables are absent. Whole guest rows and independent ORIGINAL0/empty retained; no exhaustive roster/backend/private-object grant.

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

All eight queried child commands and all six exact supplied copy-name variables are present. Whole guest rows and independent ORIGINAL0/empty retained; no exhaustive roster/backend/private-object grant.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned provider source tree`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `JimInterpCopyVariable and JimInterpCommand`, lines 126–157. Retained reference `native_child_bootstrap_context303-source-jim-interp.c`.

```text
static void JimInterpCopyVariable(Jim_Interp *target, Jim_Interp *source, const char *var, const char *default_value)
{
    Jim_Obj *value = Jim_GetGlobalVariableStr(source, var, JIM_NONE);
    const char *str;

    str = value ? Jim_String(value) : default_value;
    if (str) {
        Jim_SetGlobalVariableStr(target, var, Jim_NewStringObj(target, str, -1));
    }
}

/**
 * [interp] creates a new interpreter.
 */
static int JimInterpCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Interp *child;
    char buf[34];
    int i;
    static const char * const copyvars[] = {
        "argv", "argc", "argv0", "jim::argv0", "jim::exe", "jim::lineedit", NULL
    };

    /* Create the interpreter command */
    child = Jim_CreateInterp();
    Jim_RegisterCoreCommands(child);
    Jim_InitStaticExtensions(child);

    /* Copy some core variables to the new interpreter */
    for (i = 0; copyvars[i]; i++) {
        JimInterpCopyVariable(child, interp, copyvars[i], NULL);
    }

```

jim 0.84-9-g5bac7c9, revision `Pinned provider source tree`, `/workspace/.proofs/native-providers/jimtcl/_load-static-exts.c`, function `Jim_InitStaticExtensions`, lines 1–67. Retained reference `native_child_bootstrap_context303-source-_load-static-exts.c`.

```text

/* autogenerated - do not edit */
#include "jim.h"
#include "jimautoconf.h"
int Jim_InitStaticExtensions(Jim_Interp *interp)

{
	extern int Jim_stdlibInit(Jim_Interp *);
	extern int Jim_aioInit(Jim_Interp *);
	extern int Jim_arrayInit(Jim_Interp *);
	extern int Jim_clockInit(Jim_Interp *);
	extern int Jim_ensembleInit(Jim_Interp *);
	extern int Jim_eventloopInit(Jim_Interp *);
	extern int Jim_execInit(Jim_Interp *);
	extern int Jim_fileInit(Jim_Interp *);
	extern int Jim_historyInit(Jim_Interp *);
	extern int Jim_interpInit(Jim_Interp *);
	extern int Jim_jsonInit(Jim_Interp *);
	extern int Jim_jsonencodeInit(Jim_Interp *);
	extern int Jim_loadInit(Jim_Interp *);
	extern int Jim_namespaceInit(Jim_Interp *);
	extern int Jim_nshelperInit(Jim_Interp *);
	extern int Jim_ooInit(Jim_Interp *);
	extern int Jim_packInit(Jim_Interp *);
	extern int Jim_packageInit(Jim_Interp *);
	extern int Jim_posixInit(Jim_Interp *);
	extern int Jim_readdirInit(Jim_Interp *);
	extern int Jim_regexpInit(Jim_Interp *);
	extern int Jim_signalInit(Jim_Interp *);
	extern int Jim_syslogInit(Jim_Interp *);
	extern int Jim_tclcompatInit(Jim_Interp *);
	extern int Jim_tclprefixInit(Jim_Interp *);
	extern int Jim_zlibInit(Jim_Interp *);
	extern int Jim_binaryInit(Jim_Interp *);
	extern int Jim_globInit(Jim_Interp *);
	extern int Jim_treeInit(Jim_Interp *);
	Jim_stdlibInit(interp);
	Jim_aioInit(interp);
	Jim_arrayInit(interp);
	Jim_clockInit(interp);
	Jim_ensembleInit(interp);
	Jim_eventloopInit(interp);
	Jim_execInit(interp);
	Jim_fileInit(interp);
	Jim_historyInit(interp);
	Jim_interpInit(interp);
	Jim_jsonInit(interp);
	Jim_jsonencodeInit(interp);
	Jim_loadInit(interp);
	Jim_namespaceInit(interp);
	Jim_nshelperInit(interp);
	Jim_ooInit(interp);
	Jim_packInit(interp);
	Jim_packageInit(interp);
	Jim_posixInit(interp);
	Jim_readdirInit(interp);
	Jim_regexpInit(interp);
	Jim_signalInit(interp);
	Jim_syslogInit(interp);
	Jim_tclcompatInit(interp);
	Jim_tclprefixInit(interp);
	Jim_zlibInit(interp);
	Jim_binaryInit(interp);
	Jim_globInit(interp);
	Jim_treeInit(interp);
	return JIM_OK;
}

```

## Consumer bindings

- [rust/tcl-registry/src/special_vars.rs](../../../../rust/tcl-registry/src/special_vars.rs), `NativeChildInitialisation`: Select implemented child distribution bootstrap and exact six original global startup operands independently of the finite native command presence queries.
- [rust/tcl-registry/src/special_vars.rs](../../../../rust/tcl-registry/src/special_vars.rs), `NativeJimInterpreterCopyPurpose`: Keep counted object crossing separate from original CString startup-variable copying.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `Interp::initialise_original_jim_child`: Require actual parent/child ownership and selected implemented distribution bootstrap before publication; preserve original global copy extents.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `Interp::copy_original_jim_object_with_purpose_to`: Copy only the selected original extent into an independently live target context; foreign context binding does not loosen.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::jim_child_bootstrap_copies_global_variable_extents_without_parent_object_sharing` (linked): Actual parent/child software owners retain the six original global startup values and selected CString NUL/Unicode extents; only five supported command entries and actual scripted binary are asserted. Distinct software pointers do not observe native private ownership or the complete extension roster.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::jim_child_constructor_refusal_preserves_typed_cause_and_parent_result` (linked): Direct retired Jim owner or actual observed policy without entered frame preserves typed host refusal and prior parent result, with no child table/command/success result. Native presence/copy success rows do not measure these failures.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::c_child_constructor_refusal_keeps_owner_cause_without_success_result` (linked): Five selected C source/API owners cover automatic, named and genuine nested constructor paths; the actual nested typed host cause reaches the parent without changing prior result or publishing a child success handle. No native guest failure or private identity is observed.

## Evidence

- [rust/tcl-registry/tests/data/native_child_bootstrap_context303](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/8.4.20/original-child-bootstrap-context](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/8.4.20/original-child-bootstrap-context)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/8.5.19/original-child-bootstrap-context](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/8.5.19/original-child-bootstrap-context)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/8.6.18/original-child-bootstrap-context](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/8.6.18/original-child-bootstrap-context)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/9.0.4/original-child-bootstrap-context](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/9.0.4/original-child-bootstrap-context)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/9.1.0/original-child-bootstrap-context](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/9.1.0/original-child-bootstrap-context)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/jim/original-child-bootstrap-context](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/jim/original-child-bootstrap-context)
