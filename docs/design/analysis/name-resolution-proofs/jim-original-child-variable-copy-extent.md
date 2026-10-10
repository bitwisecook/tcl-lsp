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

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual selected native child/source API under pinned full process initialisation; exact executable/archive/header/source/build/driver ELF hashes retained; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Exact original guest fields argv3→1/A, global argc COUNT, Unicode argv0 length/equality2/2/1, absent parent0. Driver ORIGINAL0/empty is separate. Pinned startup copy source is CString/global purpose, independently of counted object crossings.

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No process executes this provider for the exact Jim-specific source/API question.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No process executes this provider for the exact Jim-specific source/API question.

### tcl8.6

Status: `not-tested`. Version: 8.6.18. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No process executes this provider for the exact Jim-specific source/API question.

### tcl9.0

Status: `not-tested`. Version: 9.0.4. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No process executes this provider for the exact Jim-specific source/API question.

### tcl9.1

Status: `not-tested`. Version: 9.1.0. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No process executes this provider for the exact Jim-specific source/API question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Exact evidence

- `native_jim_child_variable_extent306-capture.py` (input): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/capture.py). SHA-256 `fab0f0cd02ddab7c5772d1e87e17011f7407f73adbbf84e68992500c10a7d27c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.receipt.json). SHA-256 `17d52e9b54ef836017a7a60f5c0a8099de493bd49c935ca91afef5cd96bc4e5a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-jim-original-jim-child-variable-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/receipt.json). SHA-256 `463ca19c1a8b849cc5b6b06d5bee294931ba263aab7655d1b2c86cf09c03e456`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-jim-original-jim-child-variable-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-jim-original-jim-child-variable-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/original-jim-child-variable-extent/stdout). SHA-256 `5330569aa57963537d1bce45757754820cdc9cbbc74fdceb56ce6b70043fe359`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/jim/probe.elf). SHA-256 `6f1e064d4a3d7451d30405a106407a8b0d689c176299621a934d5a96015b3a16`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-original-request.json` (input): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/original-request.json](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/original-request.json). SHA-256 `6f138d49712f3161c229045765f9ac91cbcdf1e86d31a35b6077821e7cca34b2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-probe.c` (input): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.c](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.c). SHA-256 `5f3ce148bc84c374eec674275763ec575eb3466f21fd860f7009845549f96759`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-probe.tcl` (input): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/probe.tcl). SHA-256 `dfd97898714c2b6d789d7d53162526ba5b2b0140bdf6c3c3dc72fb4b6d99d6eb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-request.json` (input): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/request.json](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/request.json). SHA-256 `6dd5f15e90aa74dc87c564d15890125087ffacc915f0eafbc4bdbfec7f3c1b6f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_variable_extent306-source-jim-interp.c` (source-anchor): [rust/tcl-registry/tests/data/native_jim_child_variable_extent306/source/jim-interp.c](../../../../rust/tcl-registry/tests/data/native_jim_child_variable_extent306/source/jim-interp.c). SHA-256 `6913a5abf6518b9bdf6b953130ae006e4369207baab2d70bd58b4a55bd0edee9`. Whole separately pinned original provider source; source ownership/initialisation is independent of finite public rows.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned provider source tree`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `JimInterpCopyVariable and JimInterpCommand`, lines 126–157. Full-source SHA-256 `6913a5abf6518b9bdf6b953130ae006e4369207baab2d70bd58b4a55bd0edee9`; snippet SHA-256 `67edd4e8dfd5eb6f60d71859e0cabdd327359aab68695bc9045e57e5c215fbb7`; retained evidence `native_jim_child_variable_extent306-source-jim-interp.c`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
