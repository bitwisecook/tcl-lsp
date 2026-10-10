# naming.source.jim-original-concatenation-ownership

Kind: `implementation-contract`

## Problem statement

Reconstructing unary Jim cat from rendered bytes loses its original object and reaches a getter that the original worker bypasses. Conversely, multiple operands require each checked original String getter and a fresh String receiver. Recipe selection and physical liveness must stay independent of equal bytes or unavailable Logical purpose.

## Question

How do Jim string cat zero, unary and multiple-operand owners preserve the original transfer/getter/backing order while refusing retired, foreign or selected-unavailable materialization without creating Native admission?

## Conclusion

Pinned Jim OPT_CAT returns argv[2] for one operand, creates Jim_NewStringObj(empty,0) for zero or multiple operands, and calls Jim_AppendObj for each multiple operand. Jim_AppendObj reaches the source Jim_GetString before Jim_AppendString converts the fresh receiver through SetStringFromAny; that conversion marks character length unknown and StringAppendString allocates backing even for the empty append. The shared concatenate_jim owner preserves that selected order. VM and Runtime callers independently validate actual materialization purpose and a sole original live header. Four linked software definitions describe plain empty, unary same-object/reference retention without rendering, multiple fresh unknown-count allocated String, foreign/retired refusal and unavailable Logical purpose with no physical fallback. No completed assertion result is attached.

The [original InfoCommands literal observation](compiler-original-info-commands-literal-resolution.md), [deferred Jim Script owner](jim-original-deferred-script-ownership.md) and [object-materialization purpose](invocation-original-object-materialization-purpose.md) keep their independent scopes.

## Scope

Exactly four source excerpts from original current Jim revision 5bac7c99ad65864c87da513e22e2f01703fa4e03 and four owned VM software definitions. External provider outcomes remain not tested; zero/unary have no original process or private refcount/header measurement. Original InfoCommands literal capture independently observes its multiple-operand NamespaceInfo child String and public fields only; it does not measure the zero/unary cat cases or certify the whole current Info comparator. Equal result bytes, retained source branch, a source profile, generic compatibility or result construction supplies no actual Native worker/table/frame/argv identity, application CABI issuer, compiler/body admission or arbitrary object-clone permission. Typed access/refusal and current actual/logical/standalone materialization authority remain separate. Original Jim scalar Boolean/expression-truth and deferred Script ownership are separate purposes.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original process for this concatenation ownership question. Dialect: tcl8.4.

No original external provider process or source comparison answers this Jim software concatenation ownership contract.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original process for this concatenation ownership question. Dialect: tcl8.5.

No original external provider process or source comparison answers this Jim software concatenation ownership contract.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original process for this concatenation ownership question. Dialect: tcl8.6.

No original external provider process or source comparison answers this Jim software concatenation ownership contract.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original process for this concatenation ownership question. Dialect: tcl9.0.

No original external provider process or source comparison answers this Jim software concatenation ownership contract.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original process for this concatenation ownership question. Dialect: tcl9.1.

No original external provider process or source comparison answers this Jim software concatenation ownership contract.

### jim

Status: `not-tested`. Version: 0.84-9-g5bac7c9. Build: Pinned original source interpretation only. Channel: No original process for this concatenation ownership question. Dialect: jim.

Four exact Jim source windows explain zero/unary/multiple worker transfer and append order; no original provider process, private ownership/header/refcount or successful software outcome is supplied.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original process for this concatenation ownership question. Dialect: bigip.

No original external provider process or source comparison answers this Jim software concatenation ownership contract.

## Exact evidence

- `jim-cat-whole-source` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Unchanged official whole original Jim source, shared byte-exact; source interpretation only, no new execution.
- `jim-cat-source-audit` (source-anchor): [rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-audit.json](../../../../rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-audit.json). SHA-256 `851f7b30d45fec677e538043ad17763db9725360c82953e78651e19c047bc13a`. Exact original source revision/hash/window index from the accepted source packet; immutable source interpretation, not provider observation.
- `jim-cat-source-window-0` (source-anchor): [rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/SetStringFromAny-2467-2490.txt](../../../../rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/SetStringFromAny-2467-2490.txt). SHA-256 `6f8d5ff9cb1f9a93363096779fc91b9ec85d87ce8b7d55e679994dc9a203e5b5`. Exact counted original source excerpt; transfer/getter/backing order is source interpretation and issues no original process/header/refcount result.
- `jim-cat-source-window-1` (source-anchor): [rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/StringAppendString-2561-2595.txt](../../../../rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/StringAppendString-2561-2595.txt). SHA-256 `239e74aca4a954ec667f14ea5e218b2cf64fd2b60d5b9dd8ba1652f9a8490faf`. Exact counted original source excerpt; transfer/getter/backing order is source interpretation and issues no original process/header/refcount result.
- `jim-cat-source-window-2` (source-anchor): [rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/Jim_AppendObj-2596-2610.txt](../../../../rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/Jim_AppendObj-2596-2610.txt). SHA-256 `2f4138f46aab99c3925068ce459014f5bdee29d303ea05014937fadcf6d1885b`. Exact counted original source excerpt; transfer/getter/backing order is source interpretation and issues no original process/header/refcount result.
- `jim-cat-source-window-3` (source-anchor): [rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/OPT_CAT-14849-14871.txt](../../../../rust/tcl-vm/tests/data/jim_original_concatenation_ownership/source-windows/OPT_CAT-14849-14871.txt). SHA-256 `795ee733eee38d695d4035e4f018dfb14693e2c470043dad4b67dad8e0ea4a51`. Exact counted original source excerpt; transfer/getter/backing order is source interpretation and issues no original process/header/refcount result.
- `jim-cat-version-recipe` (source-anchor): [rust/tcl-registry/tests/data/native_mathop_jim_original_source/request.json](../../../../rust/tcl-registry/tests/data/native_mathop_jim_original_source/request.json). SHA-256 `8228eae6950ab50c2ee30c2789e055dc6755ec8fe8f610bff88887b2d8534c19`. Unchanged current-Jim request retains the exact source/version recipe; its independent public mathop/helper outputs do not measure zero/unary cat or private ownership.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `SetStringFromAny`, lines 2467–2490. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `6f8d5ff9cb1f9a93363096779fc91b9ec85d87ce8b7d55e679994dc9a203e5b5`; retained evidence `jim-cat-whole-source`.

```text
{
    if (objPtr->typePtr != &stringObjType) {
        /* Get a fresh string representation. */
        if (objPtr->bytes == NULL) {
            /* Invalid string repr. Generate it. */
            JimPanic((objPtr->typePtr->updateStringProc == NULL, "UpdateStringProc called against '%s' type.", objPtr->typePtr->name));
            objPtr->typePtr->updateStringProc(objPtr);
        }
        /* Free any other internal representation. */
        Jim_FreeIntRep(interp, objPtr);
        /* Set it as string, i.e. just set the maxLength field. */
        objPtr->typePtr = &stringObjType;
        objPtr->internalRep.strValue.maxLength = objPtr->length;
        /* Don't know the utf-8 length yet */
        objPtr->internalRep.strValue.charLength = -1;
    }
    return JIM_OK;
}

/**
 * Returns the length of the object string in chars, not bytes.
 *
 * These may be different for a utf-8 string.
 */

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `StringAppendString`, lines 2561–2595. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `239e74aca4a954ec667f14ea5e218b2cf64fd2b60d5b9dd8ba1652f9a8490faf`; retained evidence `jim-cat-whole-source`.

```text
static void StringAppendString(Jim_Obj *objPtr, const char *str, int len)
{
    int needlen;

    if (len == -1)
        len = strlen(str);
    needlen = objPtr->length + len;
    if (objPtr->internalRep.strValue.maxLength < needlen ||
        objPtr->internalRep.strValue.maxLength == 0) {
        needlen *= 2;
        /* Inefficient to alloc for less than 8 bytes */
        if (needlen < 7) {
            needlen = 7;
        }
        if (objPtr->bytes == JimEmptyStringRep) {
            objPtr->bytes = Jim_Alloc(needlen + 1);
        }
        else {
            objPtr->bytes = Jim_Realloc(objPtr->bytes, needlen + 1);
        }
        objPtr->internalRep.strValue.maxLength = needlen;
    }
    memcpy(objPtr->bytes + objPtr->length, str, len);
    objPtr->bytes[objPtr->length + len] = '\0';

    if (objPtr->internalRep.strValue.charLength >= 0) {
        /* Update the utf-8 char length */
        objPtr->internalRep.strValue.charLength += utf8_strlen(objPtr->bytes + objPtr->length, len);
    }
    objPtr->length += len;
}

/* Higher level API to append strings to objects.
 * Object must not be unshared for each of these.
 */

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_AppendObj`, lines 2596–2610. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `2f4138f46aab99c3925068ce459014f5bdee29d303ea05014937fadcf6d1885b`; retained evidence `jim-cat-whole-source`.

```text
void Jim_AppendString(Jim_Interp *interp, Jim_Obj *objPtr, const char *str, int len)
{
    JimPanic((Jim_IsShared(objPtr), "Jim_AppendString called with shared object"));
    SetStringFromAny(interp, objPtr);
    StringAppendString(objPtr, str, len);
}

void Jim_AppendObj(Jim_Interp *interp, Jim_Obj *objPtr, Jim_Obj *appendObjPtr)
{
    int len;
    const char *str = Jim_GetString(appendObjPtr, &len);
    Jim_AppendString(interp, objPtr, str, len);
    objPtr->taint |= appendObjPtr->taint;
}


```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `OPT_CAT`, lines 14849–14871. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `795ee733eee38d695d4035e4f018dfb14693e2c470043dad4b67dad8e0ea4a51`; retained evidence `jim-cat-whole-source`.

```text
        case OPT_CAT:{
                Jim_Obj *objPtr;
                if (argc == 3) {
                    /* optimise the one-arg case */
                    objPtr = argv[2];
                }
                else {
                    int i;

                    objPtr = Jim_NewStringObj(interp, "", 0);

                    for (i = 2; i < argc; i++) {
                        Jim_AppendObj(interp, objPtr, argv[i]);
                    }
                }
                Jim_SetResult(interp, objPtr);
                return JIM_OK;
            }

        case OPT_COMPARE:
        case OPT_EQUAL:
            {
                /* n is the number of remaining option args */

```


## Consumer bindings

- [rust/tcl-cmd-core/src/native_cat.rs](../../../../rust/tcl-cmd-core/src/native_cat.rs), `concatenate_jim`: Apply the selected Jim zero/unary/multiple original-object cat transfer and checked source-getter-before-fresh-receiver String order; caller liveness and recipe selection remain required.
- [rust/tcl-cmd-core/src/string.rs](../../../../rust/tcl-cmd-core/src/string.rs), `cat`: Consume a positively selected original Jim result or propagate its typed refusal; generic fallback is reached only when the adapter did not select that worker.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `Vm::native_jim_string_cat`: Select the actual tagged object-materialization protocol, check a sole original header and reach each original multiple-operand getter; selected unavailable Logical purpose refuses before physical fallback.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `Interp::native_jim_string_cat`: Retain actual selected name/object purpose, liveness and checked original getter/integer formatter; delegate the shared Jim cat recipe without creating Native admission.
- [rust/tcl-syntax/src/value.rs](../../../../rust/tcl-syntax/src/value.rs), `ValueOps::native_jim_string_cat`: Distinguish selected unavailable worker error from an adapter that did not select Jim; standalone/authored compatibility is not native execution or original-object authority.
- [rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs), `value_ops::native_jim_string_cat_tests::jim_cat_zero_and_single_operand_keep_their_distinct_original_source_contracts` (linked): The selected Jim software fixture creates a plain resident empty object for zero operands, while a live unary List returns the same owned original with exactly one additional reference and no List/member String materialisation; dropping the returned owner removes that reference. Zero/unary are source/API contracts, not original process observations.
- [rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs), `value_ops::native_jim_string_cat_tests::jim_cat_multiple_original_operands_create_unknown_count_string_backing` (linked): Multiple operands reach original getters and publish a distinct Jim String with unknown character count and allocated backing; :: plus ::alpha retains exact ::::alpha bytes, while two empty originals still create allocated String backing. The independent NativeInfo capture supplies only its own multiple-operand child window.
- [rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs), `value_ops::native_jim_string_cat_tests::jim_cat_refuses_retired_and_foreign_original_getters_without_rebuilding` (linked): A retired unary lifetime view refuses the original header; a foreign C9 List supplied among Jim operands refuses without reconstructing its members or strings, preserving the genuine prefix and original cache. This is an owned software refusal control.
- [rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_string_cat_tests.rs), `value_ops::native_jim_string_cat_tests::jim_cat_selected_unavailable_logical_purpose_does_not_reach_physical_engine` (linked): A positively selected unavailable Logical materialization purpose refuses zero/unary cat without reaching the physical Jim engine; the original integer stays stringless and retains its primary. Source fallback cannot replace a positively selected unavailable Logical purpose.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact source windows and software/API definitions only; no process or passing implementation assertion supplied. Restore retained source revision/window bytes to review source order. Original Info multiple-operand observations remain independent and unchanged; new zero/unary or provider execution requires its own exact request and receipts.
