# naming.procedure.original-usage-called-name-and-formatter

Kind: `source-anchor`

## Problem statement

Applying C procedure-name list quoting to a multiword Jim command changes the measured namespace-helper error. Separately, treating counted lookup/header bytes as the final formatter extent can retain bytes after NUL that Jim snprintf does not publish. The shared renderer must preserve original invocation identity separately from presentation.

## Question

How does current Jim construct a procedure wrong-argument usage from the original called-name object, and at what extent boundary does its final error formatter consume that assembled usage?

## Conclusion

The separately pinned Jim source shows JimCmdUsage beginning with Jim_DuplicateObj of the original cmdNameObj and appending selected formal/native usage. This procedure-called-name path applies no C list-element quoting to the called object. JimSetProcWrongArgs passes the assembled usage through %#s to Jim_SetResultFormatted. The shown formatter obtains the counted object string/length for sizing, then snprintf consumes its CString extent and publishes the returned formatted length. These are source-branch facts, including a final formatter boundary distinct from the original called-name or assembled object. The shared NativeUsageProtocol procedure-name and message renderers select presentation bytes only. Runtime and VM consume that selected recipe while original call words, formal declaration, actual resolver/table/activation ownership and ensemble rewrite premises remain independent. The existing public multiword helper arity captures are separate native questions; this source inspection neither executes a NUL procedure name nor proves a current command/header/cache identity, private lifetime, C formatter law or a Rust pass.

## Scope

Read-only inspection retains the original request, whole exact jim.c and three exact source excerpts for JimCmdUsage, JimSetProcWrongArgs and Jim_SetResultFormatted with full hashes/ranges. Jim source findings are explicitly source-only; all five C providers and BIG-IP are not tested by this Jim-specific question. Software renderers are current source/API owners, independent of native254/256/258/274 public arity outcomes. No new provider/compiler/Rust process, procedure-NUL completion or private identity observation is supplied. The marked canonical renderer control is an implementation byte-construction assertion, including a chosen NUL input, not a native procedure-NUL experiment or a current Rust pass. Actual procedure definition/formal/called-name completion observations retain a separate native question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This Jim-specific source inspection supplies no provider execution or C/BIG-IP source/formatting result. Current software rendering/assertion definitions do not issue original invocation, object/header or activation ownership.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This Jim-specific source inspection supplies no provider execution or C/BIG-IP source/formatting result. Current software rendering/assertion definitions do not issue original invocation, object/header or activation ownership.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This Jim-specific source inspection supplies no provider execution or C/BIG-IP source/formatting result. Current software rendering/assertion definitions do not issue original invocation, object/header or activation ownership.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This Jim-specific source inspection supplies no provider execution or C/BIG-IP source/formatting result. Current software rendering/assertion definitions do not issue original invocation, object/header or activation ownership.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This Jim-specific source inspection supplies no provider execution or C/BIG-IP source/formatting result. Current software rendering/assertion definitions do not issue original invocation, object/header or activation ownership.

### jim

Status: `inspected`. Version: Exact independently pinned jim.c; executable version not measured by this appendix.. Build: Read-only source content only; no interpreter/compiler/build invocation.. Channel: Whole original source plus three exact independently hashed function excerpts.. Dialect: Jim original source implementation.

Read-only source inspection: JimCmdUsage duplicates original cmdNameObj before appending formal usage; JimSetProcWrongArgs passes the assembled object via %#s; Jim_SetResultFormatted sizes from counted length then snprintf consumes CString extent. No procedure-NUL execution, actual table/header/lifetime or Rust pass is observed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This Jim-specific source inspection supplies no provider execution or C/BIG-IP source/formatting result. Current software rendering/assertion definitions do not issue original invocation, object/header or activation ownership.

## Exact evidence

- `procedure-usage-source285-JimCmdUsage.c.txt` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_usage_source285/JimCmdUsage.c.txt](../../../../rust/tcl-registry/tests/data/native_procedure_usage_source285/JimCmdUsage.c.txt). SHA-256 `cd284667e1a21dad2255ad2f9f0ff09a32929234ad98fa88bdb432eae12e673e`. Exact independently pinned original Jim source, source excerpt or read-only question request; no provider execution/physical object assertion.
- `procedure-usage-source285-JimSetProcWrongArgs.c.txt` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_usage_source285/JimSetProcWrongArgs.c.txt](../../../../rust/tcl-registry/tests/data/native_procedure_usage_source285/JimSetProcWrongArgs.c.txt). SHA-256 `e46d076ea5318725573531d8f01dd95281ce819df333d449b6dc7cf4e561572f`. Exact independently pinned original Jim source, source excerpt or read-only question request; no provider execution/physical object assertion.
- `procedure-usage-source285-Jim_SetResultFormatted.c.txt` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_usage_source285/Jim_SetResultFormatted.c.txt](../../../../rust/tcl-registry/tests/data/native_procedure_usage_source285/Jim_SetResultFormatted.c.txt). SHA-256 `6594903ebcd01fd3242fcaeed9be04baa434799636b02db02d814dbc87a279d1`. Exact independently pinned original Jim source, source excerpt or read-only question request; no provider execution/physical object assertion.
- `procedure-usage-source285-jim.c` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_usage_source285/jim.c](../../../../rust/tcl-registry/tests/data/native_procedure_usage_source285/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Exact independently pinned original Jim source, source excerpt or read-only question request; no provider execution/physical object assertion.
- `procedure-usage-source285-request.json` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_usage_source285/request.json](../../../../rust/tcl-registry/tests/data/native_procedure_usage_source285/request.json). SHA-256 `eaaab69f3e58d45bd20e3c4566197868d22109015ef852ca54a58d66fed9fb61`. Exact independently pinned original Jim source, source excerpt or read-only question request; no provider execution/physical object assertion.
- `naming-procedure-original-usage-called-name-and-formatter-runtime-rust-src-interp.rs` (implementation): [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs). SHA-256 `9c4b822bf730c22c2c6914788124311ce80cc114ff9c4312cf2ebf21bddac9c8`. Current source/API presentation owner or marked assertion definition; no executed Rust/provider outcome.
- `naming-procedure-original-usage-called-name-and-formatter-rust-tcl-registry-src-native_usage.rs` (implementation): [rust/tcl-registry/src/native_usage.rs](../../../../rust/tcl-registry/src/native_usage.rs). SHA-256 `3167b042af3939531550b079268951c6b8cfe865ff5c17e7e754741f77b85036`. Current source/API presentation owner or marked assertion definition; no executed Rust/provider outcome.
- `naming-procedure-original-usage-called-name-and-formatter-rust-tcl-vm-src-exec.rs` (implementation): [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs). SHA-256 `bfe79b7841b2bd050fa20d56ed2459c2a928ab6ce683546ad86910f97a67c8c6`. Current source/API presentation owner or marked assertion definition; no executed Rust/provider outcome.

## Source inspection

jim Version/revision not recorded by this source-only appendix; whole content SHA independently pinned., revision `Read-only exact source content; no source-control revision claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimCmdUsage`, lines 11833–11878. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `cd284667e1a21dad2255ad2f9f0ff09a32929234ad98fa88bdb432eae12e673e`; retained evidence `procedure-usage-source285-jim.c`.

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

jim Version/revision not recorded by this source-only appendix; whole content SHA independently pinned., revision `Read-only exact source content; no source-control revision claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimSetProcWrongArgs`, lines 11882–11886. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `e46d076ea5318725573531d8f01dd95281ce819df333d449b6dc7cf4e561572f`; retained evidence `procedure-usage-source285-jim.c`.

```text
 */
static void JimSetProcWrongArgs(Jim_Interp *interp, Jim_Obj *procNameObj, Jim_Cmd *cmd)
{
    Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s\"", JimCmdUsage(interp, procNameObj, cmd));
}

```

jim Version/revision not recorded by this source-only appendix; whole content SHA independently pinned., revision `Read-only exact source content; no source-control revision claimed.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetResultFormatted`, lines 17112–17178. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `6594903ebcd01fd3242fcaeed9be04baa434799636b02db02d814dbc87a279d1`; retained evidence `procedure-usage-source285-jim.c`.

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

- [rust/tcl-registry/src/native_usage.rs](../../../../rust/tcl-registry/src/native_usage.rs), `NativeUsageProtocol::render_procedure_name`: Select procedure-called-name presentation: Jim preserves called bytes while C selected recipe can apply its independent list quoting. Does not reconstruct genuine call identity.
- [rust/tcl-registry/src/native_usage.rs](../../../../rust/tcl-registry/src/native_usage.rs), `NativeUsageProtocol::render_procedure_message`: Present assembled procedure usage under its independently selected final formatter extent; byte rendering supplies no command/header/activation ownership.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::proc_wrong_args`: Consume selected usage recipe with independently retained original call/formal/ensemble rewrite premises.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `proc_usage`: Consume selected procedure usage presentation independently of actual ProcDef/call/header/admission ownership.
- [rust/tcl-registry/src/native_usage.rs](../../../../rust/tcl-registry/src/native_usage.rs), `native_usage::tests::procedure_usage_preserves_the_selected_name_and_final_formatter_boundary` (linked): Pure selected procedure presenter retains raw multiword Jim called-name bytes and applies its final CString boundary to already assembled p-NUL-tail usage. C counted presentation remains separately selected. This authored byte-renderer control does not execute a NUL procedure or prove original invocation/header/cache/activation identity; separately pinned source excerpts establish source facts only.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
