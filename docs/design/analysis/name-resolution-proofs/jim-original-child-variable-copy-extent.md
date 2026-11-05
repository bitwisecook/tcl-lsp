# naming.interpreter.jim-original-child-variable-copy-extent

Kind: `native-observation`

## Problem statement

Jim startup-variable copying and ordinary interp object crossings use distinct extents. A counted script/argument/result copy cannot justify a counted startup-variable value or borrow a procedure-local value.

## Question

What startup-variable value extent, Unicode equality, global parent lookup and absent-parent behaviour are reported by the original Jim child control?

## Conclusion

The original Jim guest reports argv source length3 but copied length1 with valueA after a runtime NUL; argc reports the global COUNT value; runtime Unicode argv0 retains length2 with both equality/count fields matching2/2/1; an absent parent variable stays absent (0). The independent driver ORIGINAL completion is0/empty, and one original process/compiler command exit0 with empty stderr. Separately pinned JimInterpCopyVariable uses global variable lookup, Jim_String and Jim_NewStringObj with length -1 for this startup copy purpose. Ordinary counted child script/argument/result copying is a separate question and does not broaden this extent. No native private header/context pointer/refcount observation, all-variable copy, arbitrary local capture or C equivalence follows.

## Scope

One exact ASCII LF original uses runtime backslash-u escapes for NUL/Unicode values, explicit global/local discriminators and the actual Jim child handle API. C8.4–9.1 and BIG-IP are not executed for this Jim-specific question. Whole original request/source/driver/receipts/streams/ELF/provider pins and full Jim interp source are retained; source copy purpose and native public value equality remain distinct.

## Provider answers

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

Exact original guest fields argv3→1/A, global argc COUNT, Unicode argv0 length/equality2/2/1, absent parent0. Driver ORIGINAL0/empty is separate. Pinned startup copy source is CString/global purpose, independently of counted object crossings.

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `not-tested`. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

No process executes this provider for the exact Jim-specific source/API question.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned provider source tree`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `JimInterpCopyVariable and JimInterpCommand`, lines 126–157. Retained reference `native_jim_child_variable_extent306-source-jim-interp.c`.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stderr)
- [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stdout)
- [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stderr)
- [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stdout)
- [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.c](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.c)
- [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.tcl)
- [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/source/jim-interp.c](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/source/jim-interp.c)
