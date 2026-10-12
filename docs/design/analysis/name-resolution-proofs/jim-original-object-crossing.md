# naming.interpreter.jim-original-object-crossing

Kind: `native-observation`

## Problem statement

Independent Jim interpreters cannot borrow arbitrary objects merely because their bytes agree. Original child evaluation and alias calls need separately measured string/completion outcomes and pinned extension source evidence for the crossing owner.

## Question

At the pinned Jim interp extension boundary, what one/multiple-source, counted NUL/Unicode string, alias argument/result and non-OK completion outcomes are observed between independently live parent and child interpreters?

## Conclusion

The original Jim source reports both equality fields1 for empty, spaced, runtime NUL, runtime Unicode and escaped strings; their measured lengths are0,3,3,2,5. Multiple eval arguments return `A B`, while the separately constructed single script returns the nested list `{A B}`. Caught child return and return -code7 both report raw code2 with VALUE; break reports3, continue4 and error1 with ERR. The original parent-frame alias control returns `LOCAL ::N`. The independent counted driver reports ORIGINAL code0 with an empty result; its metadata rows are separate from the guest puts rows. The one original process and compiler command both exit0 with empty stderr. Independently pinned jim-interp.c source reads each crossing object with Jim_GetString plus its count and creates a target-interpreter String, retains the alias prefix as a parent-owned List and copies appended child arguments/results at the selected boundary. These source statements are not native pointer/refcount observations. No arbitrary foreign-object binding, private member/header identity, general frame protocol, option/error transport or C interpreter equivalence follows. The linked Runtime whole-source control consumes the untouched original native script and complete guest rows, first asserting the independent counted VERSION and ORIGINAL0/empty fields. A separate source/API control copies counted String values into the actual independently live target Jim context while retaining source-context currency, and rejects arbitrary foreign or retired contexts. These software ownership assertions are not native pointer/refcount observations; source-selected crossing copies do not loosen the ordinary context binding guard or certify arbitrary foreign-object dispatch.

## Scope

One exact ASCII LF source runs in the fresh pinned Jim core/static-extension C API driver without an added outer source envelope. NUL and Unicode are produced at runtime by explicit ASCII backslash-u escapes. The complete original source/request/driver/ELF/compile and process receipts/streams/launcher are retained with required source/header/archive/build/executable hashes. Full separately pinned jim-interp.c and exact source excerpts distinguish source ownership from public equality/results. C8.4–9.1 and BIG-IP are not tested for this Jim extension question; Rust assertions are not executed by publication.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No C interpreter is executed for this Jim interp-extension crossing question. Public C child APIs and object ownership are independent.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No C interpreter is executed for this Jim interp-extension crossing question. Public C child APIs and object ownership are independent.

### tcl8.6

Status: `not-tested`. Version: 8.6.18. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No C interpreter is executed for this Jim interp-extension crossing question. Public C child APIs and object ownership are independent.

### tcl9.0

Status: `not-tested`. Version: 9.0.4. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No C interpreter is executed for this Jim interp-extension crossing question. Public C child APIs and object ownership are independent.

### tcl9.1

Status: `not-tested`. Version: 9.1.0. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No C interpreter is executed for this Jim interp-extension crossing question. Public C child APIs and object ownership are independent.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Exact pinned current Jim archive/executable/header/build/source and counted driver ELF; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Whole guest rows report equality1/1 and lengths0/3/3/2/5 for empty/spaced/NUL/Unicode/escaped; multi-source concat A B versus single nested {A B}; raw return2, return -code7 still2, break3, continue4, error1; parent-frame values LOCAL ::N. Separate counted driver ORIGINAL0/empty. One compile and one original process exit0/empty stderr. Source-copy ownership is separately pinned, with no native pointer/refcount observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Exact evidence

- `native_jim_child_object_crossing302-capture.py` (input): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/capture.py). SHA-256 `a9f403c8344404007c3d5acf2033a1cef119f5636c132d2077fe40877f262c1b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/compile.receipt.json). SHA-256 `fa83af8e6160514d238abbb20d684f04a87abe9fe214028e1e97c00f96e0d85a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-jim-original-counted-child-crossing-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/receipt.json). SHA-256 `98f0c8468eff809384f49eb14a4a58f03f357993349fe4e08bf03d56a73eda4e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-jim-original-counted-child-crossing-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/stderr](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-jim-original-counted-child-crossing-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/stdout](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/stdout). SHA-256 `dee37f0d95c5d3050000407360aa8fcb24a5b566162bc2b4e14272404650f4fd`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/probe.elf). SHA-256 `6f1e064d4a3d7451d30405a106407a8b0d689c176299621a934d5a96015b3a16`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-original-request.json` (input): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/original-request.json](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/original-request.json). SHA-256 `622f724962d5f782cead24e7488e01fc1f74e24831e58f150234783f5231a49e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-probe.c` (input): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/probe.c](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/probe.c). SHA-256 `5f3ce148bc84c374eec674275763ec575eb3466f21fd860f7009845549f96759`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-probe.tcl` (input): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/probe.tcl). SHA-256 `a3c7e936edc60ad3cac149ba48099b83992fc6679d903118b5d3a7d2872154cc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-request.json` (input): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/request.json](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/request.json). SHA-256 `c5a15f0345c980501e2351bc9d17ba6ba4878aebec446c2dfcf95d0370476781`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_child_object_crossing302-source-jim-interp.c` (source-anchor): [rust/tcl-registry/tests/data/native_jim_child_object_crossing302/source/jim-interp.c](../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/source/jim-interp.c). SHA-256 `6913a5abf6518b9bdf6b953130ae006e4369207baab2d70bd58b4a55bd0edee9`. Whole independently pinned Jim interp extension source; source-copy/alias ownership is distinct from public runtime equality and completion observations.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned source tree associated with captured Jim executable`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `JimInterpCopyObj and interp_cmd_eval`, lines 13–42. Full-source SHA-256 `6913a5abf6518b9bdf6b953130ae006e4369207baab2d70bd58b4a55bd0edee9`; snippet SHA-256 `00d2e320a65744630cfd8eafe3aaadfa826c4cbe2e4de5a6119b1f9901eba0fe`; retained evidence `native_jim_child_object_crossing302-source-jim-interp.c`.

```text
/* Everything passing between interpreters must be converted to a string */
static Jim_Obj *JimInterpCopyObj(Jim_Interp *target, Jim_Obj *obj)
{
    const char *rep;
    int len;

    rep = Jim_GetString(obj, &len);
    return Jim_NewStringObj(target, rep, len);
}

#define JimInterpCopyResult(to, from) Jim_SetResult((to), JimInterpCopyObj((to), Jim_GetResult((from))))

static int interp_cmd_eval(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int ret;
    Jim_Interp *child = Jim_CmdPrivData(interp);
    Jim_Obj *scriptObj;
    Jim_Obj *targetScriptObj;

    scriptObj = Jim_ConcatObj(interp, argc, argv);
    targetScriptObj = JimInterpCopyObj(child, scriptObj);
    Jim_FreeNewObj(interp, scriptObj);

    Jim_IncrRefCount(targetScriptObj);
    ret = Jim_EvalObj(child, targetScriptObj);
    Jim_DecrRefCount(child, targetScriptObj);

    JimInterpCopyResult(interp, child);
    return ret;
}

```

jim 0.84-9-g5bac7c9, revision `Pinned source tree associated with captured Jim executable`, `/workspace/.proofs/native-providers/jimtcl/jim-interp.c`, function `JimInterpAliasProc and interp_cmd_alias`, lines 55–96. Full-source SHA-256 `6913a5abf6518b9bdf6b953130ae006e4369207baab2d70bd58b4a55bd0edee9`; snippet SHA-256 `c0483931c9238e5aa4360023e9e1627f6f8354cdf90290e9e84614628ee197eb`; retained evidence `native_jim_child_object_crossing302-source-jim-interp.c`.

```text
static int JimInterpAliasProc(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int i, ret;
    Jim_Interp *parent = Jim_GetAssocData(interp, "interp.parent");
    Jim_Obj *targetPrefixObj = Jim_CmdPrivData(interp);
    Jim_Obj *targetScriptObj;

    assert(parent);

    /* Build the complete command */
    targetScriptObj = Jim_DuplicateObj(parent, targetPrefixObj);
    for (i = 1; i < argc; i++) {
        Jim_ListAppendElement(parent, targetScriptObj,
            JimInterpCopyObj(parent, argv[i]));
    }

    Jim_IncrRefCount(targetScriptObj);
    ret = Jim_EvalObj(parent, targetScriptObj);
    Jim_DecrRefCount(parent, targetScriptObj);

    JimInterpCopyResult(interp, parent);
    return ret;
}

static int interp_cmd_alias(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Interp *child = Jim_CmdPrivData(interp);
    Jim_Obj *aliasPrefixList;

    /* The prefix list will be held inside the child, but it still belongs
     * to the parent!
     */

    aliasPrefixList = Jim_NewListObj(interp, argv + 1, argc - 1);
    Jim_IncrRefCount(aliasPrefixList);

    Jim_RegisterCmd(child, Jim_String(argv[0]), NULL, 0, -1, JimInterpAliasProc, JimInterpDelAlias, aliasPrefixList, 0);
    return JIM_OK;
}

static const jim_subcmd_type interp_command_table[] = {
    {   "eval",

```


## Consumer bindings

- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `Interp::copy_original_jim_object_to`: Copy counted original String bytes at the selected crossing into an independently live target context; ordinary foreign-context binding remains refused.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::jim_child_crossing_copies_counted_strings_and_preserves_original_context_refusal` (linked): Actual independently live Runtime Jim object contexts copy counted NUL/Unicode bytes into a distinct target producer; foreign/retired ordinary ownership refuses. Authored API identity is not external native pointer evidence.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::jim_child_counted_crossings_match_original_native_public_rows` (linked): Use exact original302 source and all guest public rows, assert counted VERSION/ORIGINAL0-empty separately, and exclude only reported version metadata from Runtime behaviour comparison. No reconstructed source or native private ownership assertion.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
