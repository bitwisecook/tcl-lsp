# naming.procedure.original-called-name-and-formal-usage-extents

Kind: `native-observation`

## Problem statement

Original called-name and formal name values can contain counted NUL independently of how a wrong-argument usage is assembled and rendered. Required, default and reference-shaped formals require separate public definition/call outcomes rather than a single diagnostic extent inferred from name storage.

## Question

What exact original name/formal lengths, definition completion/results and caught wrong-argument usage do four unchanged NUL-producing called-name, required-formal, default-formal and reference-formal sources return on the six pinned interpreters?

## Conclusion

All twenty-four originals complete with outer guest code0. Their public name/formal lengths are15/1,14/6,13/6 and15/7 for called-name, required, default and reference controls, respectively. Every original procedure definition completes with code0. C8.4–C9.1 definition results are empty; Jim returns the original public command name, including the counted NUL in its called-name control. Every caught call completes with code1. The five C releases retain modified UTF-8 NUL bytes c080 in the whole usage messages: r2286_call<c080>head x, r2286_required a<c080>tail b, r2286_default ?a<c080>tail? b and r2286_reference &a<c080>tail b inside the exact wrong-arguments envelope. Jim called-name usage stops at r2286_call; required-formal usage is r2286_required a b and reference-formal usage is r2286_reference a b, preserving the later b in each; default-formal usage stops at r2286_default ?a. Required/reference-formal truncation and final assembled default/called-name truncation therefore produce distinct finite public messages. Original counted lengths, definition results and caught diagnostic bytes remain independent observations, with no private formal/header/table/key/cell/cache or invocation-operand identity implied. The original request and separate Jim usage/formatter excerpts are retained; their source interpretation cannot add private native state to these public outcomes. All host processes exit0 with empty stderr; no software pass or BIG-IP observation follows.

A marked VM definition links twenty-four exact public called-name/formal usage windows and their original definition/invocation boundaries.

## Scope

Four byte-identical ASCII LF sources, each independently evaluated in a fresh fully initialised interpreter using the unchanged counted-source driver. binary format H* produces literal NUL at evaluation, with no literal NUL/non-ASCII source or added source envelope. Whole public result fields retain original counted lengths, definition completion/result and conditional caught call completion/message; no definition error is replaced with an availability branch. Complete driver/request/source/compiler/ELF/process/stdout/stderr/version and required provider pins are retained. Source anchors are independently hashed read-only channels. No opaque source AST, physical object/refcount, private lookup/frame/formal cell, arbitrary UTF-8/FF name, instruction/admission or executed Rust assertion is supplied. The shared formal_parameter_usage_bytes implementation preserves counted declaration/storage names separately from displayed usage. Jim required and reference formals use a CString extent per required name and retain later formals; a default formal keeps its counted suffix until the separate final message formatter. Runtime and VM use that suffix with the selected called-name and final-message owners. Their linked controls compare all twenty-four unchanged original public completion/results independently in each backend. Explicit Jim binary capability supplies only the original byte-producing source dependency. The pure suffix control and comparison definitions supply no native formal object/header/table/cache/compiler/activation identity or current executed Rust result.

No new original process, compiled opcode, physical formal/header/table identity, private CString stage, source-character channel or software passing outcome is observed by this binding. The C original definitions/invocations retain their actual APIs rather than an unavailable surrogate. Selected compiler and explicit Jim binary adapter are software fixture premises, not general Native entry or library authority.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Four independent fresh fullInit original public source evaluations; separate VERSION and ORIGINAL counted-hex channels with no added source envelope or result normalization. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Four original outer completions code0; lengths15/1,14/6,13/6,15/7;every definition0/empty result and caught call1. Whole usage messages preserve exact c080 NUL encoding and later formal b for required/default/reference controls, with the reference-shaped formal literal ampersand retained. No private formal/storage identity is inferred. Four fresh host processes exit0 with empty stderr.

### jim

Status: `observed`. Channel: Four independent fresh fullInit original public source evaluations; separate VERSION and ORIGINAL counted-hex channels with no added source envelope or result normalization. Version: jim 0.84-9-g5bac7c9.

Four original outer completions code0; lengths15/1,14/6,13/6,15/7 and every definition0 with original command-name result. Every caught call1: called usage r2286_call;required r2286_required a b;default r2286_default ?a;reference r2286_reference a b. Later b is preserved only in these required/reference controls. Whole original result hex retains counted name NUL independently from rendered usage. Four fresh host processes exit0 with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim Exact independently pinned original source; no executable outcome is inferred from this source anchor., revision `Source-control revision not recorded by this appendix.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimCmdUsage`, lines 11833–11878. Retained reference `native_procedure_usage_extents288-request-jim.c`.

```text
    Jim_Obj *usage = Jim_DuplicateObj(interp, cmdNameObj);

    if (cmd->flags & JIM_CMD_ISPROC) {
        int i;
        for (i = 0; i < cmd->u.proc.argListLen; i++) {
            Jim_AppendString(interp, usage, " ", 1);

            if (i == cmd->u.proc.argsPos) {
                if (cmd->u.proc.arglist[i].defaultObjPtr) {
                    /* Renamed args */
                    Jim_AppendString(interp, usage, "?", 1);
                    Jim_AppendObj(interp, usage, cmd->u.proc.arglist[i].defaultObjPtr);
                    Jim_AppendString(interp, usage, " ...?", -1);
                }
                else {
                    /* We have plain args */
                    Jim_AppendString(interp, usage, "?arg ...?", -1);
                }
            }
            else {
                if (cmd->u.proc.arglist[i].defaultObjPtr) {
                    Jim_AppendString(interp, usage, "?", 1);
                    Jim_AppendObj(interp, usage, cmd->u.proc.arglist[i].nameObjPtr);
                    Jim_AppendString(interp, usage, "?", 1);
                }
                else {
                    const char *arg = Jim_String(cmd->u.proc.arglist[i].nameObjPtr);
                    if (*arg == '&') {
                        arg++;
                    }
                    Jim_AppendString(interp, usage, arg, -1);
                }
            }
        }
    }
    else if (cmd->u.native.usage) {
        if (*cmd->u.native.usage) {
            Jim_AppendStrings(interp, usage, " ", cmd->u.native.usage, NULL);
        }
    }
    else {
        Jim_AppendString(interp, usage, " ...", -1);
    }

    return usage;
}

```

jim Exact independently pinned original source; no executable outcome is inferred from this source anchor., revision `Source-control revision not recorded by this appendix.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimSetProcWrongArgs`, lines 11882–11886. Retained reference `native_procedure_usage_extents288-request-jim.c`.

```text
 */
static void JimSetProcWrongArgs(Jim_Interp *interp, Jim_Obj *procNameObj, Jim_Cmd *cmd)
{
    Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s\"", JimCmdUsage(interp, procNameObj, cmd));
}

```

jim Exact independently pinned original source; no executable outcome is inferred from this source anchor., revision `Source-control revision not recorded by this appendix.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetResultFormatted`, lines 17112–17178. Retained reference `native_procedure_usage_extents288-request-jim.c`.

```text
 * Very simple printf-like formatting, designed for error messages.
 *
 * The format may contain up to 5 '%s' or '%#s', corresponding to variable arguments.
 * The resulting string is created and set as the result.
 *
 * Each '%s' should correspond to a regular string parameter.
 * Each '%#s' should correspond to a (Jim_Obj *) parameter.
 * Any other printf specifier is not allowed (but %% is allowed for the % character).
 *
 * e.g. Jim_SetResultFormatted(interp, "Bad option \"%#s\" in proc \"%#s\"", optionObjPtr, procNamePtr);
 *
 * Note: We take advantage of the fact that printf has the same behaviour for both %s and %#s
 *
 * Note that any Jim_Obj parameters with zero ref count will be freed as a result of this call.
 */
void Jim_SetResultFormatted(Jim_Interp *interp, const char *format, ...)
{
    /* Initial space needed */
    int len = strlen(format);
    int extra = 0;
    int n = 0;
    const char *params[5];
    int nobjparam = 0;
    Jim_Obj *objparam[5];
    char *buf;
    va_list args;
    int i;

    va_start(args, format);

    for (i = 0; i < len && n < 5; i++) {
        int l;

        if (strncmp(format + i, "%s", 2) == 0) {
            params[n] = va_arg(args, char *);

            l = strlen(params[n]);
        }
        else if (strncmp(format + i, "%#s", 3) == 0) {
            Jim_Obj *objPtr = va_arg(args, Jim_Obj *);

            params[n] = Jim_GetString(objPtr, &l);
            objparam[nobjparam++] = objPtr;
            Jim_IncrRefCount(objPtr);
        }
        else {
            if (format[i] == '%') {
                i++;
            }
            continue;
        }
        n++;
        extra += l;
    }

    len += extra;
    buf = Jim_Alloc(len + 1);
    len = snprintf(buf, len + 1, format, params[0], params[1], params[2], params[3], params[4]);

    va_end(args);

    Jim_SetResult(interp, Jim_NewStringObjNoAlloc(interp, buf, len));

    for (i = 0; i < nobjparam; i++) {
        Jim_DecrRefCount(interp, objparam[i]);
    }
}

```

## Consumer bindings

- [rust/tcl-syntax/src/formal_params/bytes.rs](../../../../rust/tcl-syntax/src/formal_params/bytes.rs), `formal_parameter_usage_bytes`: Construct selected formal usage separately from counted declaration/storage keys; Jim required/reference per-formal CString extent remains distinct from defaulted counted suffix and final message extent.
- [rust/tcl-registry/src/native_usage.rs](../../../../rust/tcl-registry/src/native_usage.rs), `NativeUsageProtocol::render_procedure_name`: Render selected called-name display bytes independently of formal storage and original invocation identity.
- [rust/tcl-registry/src/native_usage.rs](../../../../rust/tcl-registry/src/native_usage.rs), `NativeUsageProtocol::render_procedure_message`: Apply the selected final usage formatter independently of declaration and per-formal extents.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::proc_wrong_args`: Join selected procedure usage header and shared formal suffix for Runtime presentation; public source comparisons do not identify private ownership.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `proc_usage`: Join selected VM procedure usage header and shared formal suffix without deriving private formal/header/table receipts from public messages.
- [rust/tcl-syntax/src/formal_params/bytes.rs](../../../../rust/tcl-syntax/src/formal_params/bytes.rs), `formal_params::bytes::tests::jim_required_usage_extent_is_independent_of_counted_default_and_storage_names` (linked): Pure suffix construction keeps counted a-NUL-tail declaration/storage names unchanged. Jim required/reference suffixes are a b while the defaulted suffix remains counted ?a-NUL-tail? b before final formatting. It executes no native frame or procedure and supplies no current Rust pass.
- [runtime/rust/src/cmd_info/native_jim_inventory_tests.rs](../../../../runtime/rust/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_procedure_usage_extents_match_all_24_native_public_windows` (linked): Four unchanged whole originals compare all twenty-four Native288 public completion/results, including independent counted lengths, definition completion/result and caught call message. Explicit Jim binary capability only supplies byte construction; no native formal/header/table/cache identity, internal conversion stage or current executed Rust outcome follows.

- [rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs), `original_result`: Decode complete original procedure-usage completion/result fields without inferring formal object/header identity or an internal CString conversion stage.
- [rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_procedure_usage_extents_match_all_24_native_public_windows` (linked): Four unchanged ASCII called-name/required/default/reference-formal programs across six selected software cores compare all24 original whole completion/result windows. Their binary producer supplies NUL at evaluation; lengths, definition code/result and caught invocation message remain independent fields. Jim explicitly registers that byte producer only.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_procedure_usage_extents288](../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288)
- [rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.4.20](../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.4.20)
- [rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.5.19](../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.5.19)
- [rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.6.18](../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.6.18)
- [rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.0.4](../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.0.4)
- [rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.1.0](../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_procedure_usage_extents288/jim](../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/jim)
