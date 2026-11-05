# naming.list.original-jim-source-length-conversion

Kind: `native-observation`

## Problem statement

A source object has a genuine original primary and source metadata. Treating it as an arbitrary Unknown value loses its selected conversion; treating every Unknown as Source would invent that authority.

## Question

What original operand primaries/results occur across three unchanged concat channels, and what exact primary/resident-byte/child-source conversion does Jim_ListLength perform on an independently created Jim_SetSourceInfo object?

## Conclusion

All C providers report original0 word, original1 and original2 exact bytes A-double-space-B-space-C, both operand primaries none, and SOURCE_LENGTH NOT_APPLICABLE. Jim reports original0 word followed by one space, original1 A-double-space-B-space-C with both operand primaries source, and original2 A-space-B-space-C. In the independent Jim Source API channel, BEFORE reports source with resident bytes1; AFTER reports list, length2, resident bytes1 and unchanged original bytes41202042. The first list child reports primary source, line17 and filename source-check.tcl. All six processes/compiler commands exit0 with empty stderr. The driver uses one fresh interpreter for the three original source channels and a separate fresh Jim interpreter for the dedicated Source construction/conversion. The original object field observers and source anchors establish only this finite primary/source conversion; arbitrary Unknown values, C Source equivalence, variable/name-table identity, private ownership counters, allocator behaviour, full stock classes or VM/native dispatch are not inferred. Linked implementation controls recognise only the authentic JimSource physical class with its selected Jim protocol and actual source producer. Unknown/foreign/C classes refuse rather than borrow Source conversion. Source-aware Length retains original resident bytes and child filename/line through the shared conversion owner. Runtime/VM API controls compare the three exact Source fields independently of the original concat-channel whole-code/result controls; their software object assertions do not widen the finite native primary/source observations or establish arbitrary allocator/header/name ownership.

## Scope

Three exact ASCII originals deliberately retain adjacent spaces and no final LF, with the complete unchanged C99 driver and one actual process per provider. The public source completion/type fields stay separate from the Jim-only constructor/ListLength channel and its source filename/line. Whole originals/request/driver/receipts/streams/ELFs and required header/archive/build/source pins are retained. Jim source/header copies and exact conversion excerpts are independently pinned; BIG-IP and Rust assertions are not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Compile this exact ASCII C99 probe against the same six independently pinned Native236 headers/archives/build sources and linker arguments. Fresh full Tcl_Init or Jim core/static extensions. Call executable with no arguments; retain full original stdout/stderr/exit and actual linked patchlevel. No outer source wrapper, no result normalization. The dedicated Source-length API channel uses its own fresh Jim interpreter and reports C NOT_APPLICABLE; it is separate from the three public source evaluations. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Original completion values word / A-double-space-B C / A-double-space-B C; both operands none. The separate Source API row is NOT_APPLICABLE and supplies no C Source object conversion.

### jim

Status: `observed`. Channel: Compile this exact ASCII C99 probe against the same six independently pinned Native236 headers/archives/build sources and linker arguments. Fresh full Tcl_Init or Jim core/static extensions. Call executable with no arguments; retain full original stdout/stderr/exit and actual linked patchlevel. No outer source wrapper, no result normalization. The dedicated Source-length API channel uses its own fresh Jim interpreter and reports C NOT_APPLICABLE; it is separate from the three public source evaluations. Version: jim 0.84-9-g5bac7c9.

Original completion values word-space / A-double-space-B C / A B C; both operands source. Independent Source API: source/resident1 → list/length2/resident1 with41202042; first child source/line17/source-check.tcl.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned original Jim provider source`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetSourceInfo`, lines 3749–3774. Retained reference `native_source_list_length241-jim.c`.

```text
void Jim_SetSourceInfo(Jim_Interp *interp, Jim_Obj *objPtr,
    Jim_Obj *fileNameObj, int lineNumber)
{
    JimPanic((Jim_IsShared(objPtr), "Jim_SetSourceInfo called with shared object"));
    Jim_FreeIntRep(interp, objPtr);
    Jim_IncrRefCount(fileNameObj);
    objPtr->internalRep.sourceValue.fileNameObj = fileNameObj;
    objPtr->internalRep.sourceValue.lineNumber = lineNumber;
    objPtr->typePtr = &sourceObjType;
}

/**
 * Similar to ScriptObjAddTokens(), but for subst objects.
 */
static void SubstObjAddTokens(Jim_Interp *interp, struct ScriptObj *script,
    ParseTokenList *tokenlist)
{
    int i;
    struct ScriptToken *token;

    token = script->token = Jim_Alloc(sizeof(ScriptToken) * tokenlist->count);

    for (i = 0; i < tokenlist->count; i++) {
        const ParseToken *t = &tokenlist->list[i];

        /* Create a token for 't' */

```

jim 0.84-9-g5bac7c9, revision `Pinned original Jim provider source`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `SetListFromAny`, lines 6917–7010. Retained reference `native_source_list_length241-jim.c`.

```text
static int SetListFromAny(Jim_Interp *interp, struct Jim_Obj *objPtr)
{
    struct JimParserCtx parser;
    const char *str;
    int strLen;
    Jim_Obj *fileNameObj;
    int linenr;

    if (objPtr->typePtr == &listObjType) {
        return JIM_OK;
    }

    /* Optimise dict -> list for object with no string rep.  */
    if (Jim_IsDict(objPtr) && objPtr->bytes == NULL) {
        Jim_Dict *dict = objPtr->internalRep.dictValue;
        /* To convert to a list we need to:
         * 1. Take ownership of the table
         * 2. Discard the hash table
         * 3. Free the dict structure
         */

        /* 1. Switch the internal rep */
        objPtr->typePtr = &listObjType;
        objPtr->internalRep.listValue.len = dict->len;
        objPtr->internalRep.listValue.maxLen = dict->maxLen;
        objPtr->internalRep.listValue.ele = dict->table;

        /* 2. Discard the hash table */
        Jim_Free(dict->ht);

        /* 3. Free the dict structure */
        Jim_Free(dict);
        return JIM_OK;
    }

    /* Try to preserve information about filename / line number */
    fileNameObj = Jim_GetSourceInfo(interp, objPtr, &linenr);
    Jim_IncrRefCount(fileNameObj);

    /* Get the string representation */
    str = Jim_GetString(objPtr, &strLen);

    /* Free the old internal repr just now and initialize the
     * new one just now. The string->list conversion can't fail. */
    Jim_FreeIntRep(interp, objPtr);
    objPtr->typePtr = &listObjType;
    objPtr->internalRep.listValue.len = 0;
    objPtr->internalRep.listValue.maxLen = 0;
    objPtr->internalRep.listValue.ele = NULL;

    /* Convert into a list */
    if (strLen) {
        JimParserInit(&parser, str, strLen, linenr);
        while (!parser.eof) {
            Jim_Obj *elementPtr;

            JimParseList(&parser);
            if (parser.tt != JIM_TT_STR && parser.tt != JIM_TT_ESC)
                continue;
            elementPtr = JimParserGetTokenObj(interp, &parser);
            Jim_SetSourceInfo(interp, elementPtr, fileNameObj, parser.tline);
            elementPtr->taint = objPtr->taint;
            ListAppendElement(objPtr, elementPtr);
        }
    }
    Jim_DecrRefCount(interp, fileNameObj);
    return JIM_OK;
}

Jim_Obj *Jim_NewListObj(Jim_Interp *interp, Jim_Obj *const *elements, int len)
{
    Jim_Obj *objPtr;

    objPtr = Jim_NewObj(interp);
    objPtr->typePtr = &listObjType;
    objPtr->bytes = NULL;
    objPtr->taint = 0;
    objPtr->internalRep.listValue.ele = NULL;
    objPtr->internalRep.listValue.len = 0;
    objPtr->internalRep.listValue.maxLen = 0;

    if (len) {
        ListInsertElements(objPtr, 0, len, elements);
    }

    return objPtr;
}

/* Return a vector of Jim_Obj with the elements of a Jim list, and the
 * length of the vector. Note that the user of this function should make
 * sure that the list object can't shimmer while the vector returned
 * is in use, this vector is the one stored inside the internal representation
 * of the list object. This function is not exported, extensions should
 * always access to the List object elements using Jim_ListGetIndex(). */

```

## Consumer bindings

- [rust/tcl-registry/src/native_stock_list.rs](../../../../rust/tcl-registry/src/native_stock_list.rs), `NativeStockListInputClass`: Keep authentic JimSource distinct from Unknown and C physical classes; selected source conversion does not grant other stock classes.
- [rust/tcl-registry/src/native_stock_list/tests.rs](../../../../rust/tcl-registry/src/native_stock_list/tests.rs), `native_stock_list::tests::original_jim_source_length_requires_its_own_dialect_and_physical_class` (linked): Pure selected JimSource protocol/class permits its own source-aware length path; C/Unknown/foreign or lost producer cannot borrow that path.
- [runtime/rust/src/native_source_tests.rs](../../../../runtime/rust/src/native_source_tests.rs), `native_source::tests::original_jim_source_length_preserves_resident_bytes_and_child_source` (linked): Match original Source-before/after/child fields with actual selected software Source owner, resident bytes and filename/line; no arbitrary Unknown-to-Source grant.
- [runtime/rust/src/cmd_list.rs](../../../../runtime/rust/src/cmd_list.rs), `cmd_list::tests::native_concat_preserves_selected_engine_and_object_representation` (linked): Compare separate unchanged original concat channels and selected engine/operand representation; dedicated Source API metadata stays independent.
- [rust/tcl-vm/src/value_source_tests.rs](../../../../rust/tcl-vm/src/value_source_tests.rs), `value::native_source_tests::original_jim_source_length_preserves_resident_bytes_and_child_source` (linked): VM selected Source owner matches the three dedicated original API rows without promoting Unknown/C classes or borrowing absent source ownership.
- [rust/tcl-vm/src/value_source_tests.rs](../../../../rust/tcl-vm/src/value_source_tests.rs), `value::native_source_tests::original_source_length_and_concat_channels_match_all_native_providers` (linked): Use all three unchanged concat originals and complete six-provider native completion columns; compare the separate Jim Source API channel independently.

## Evidence

- [rust/tcl-registry/tests/data/native_source_list_length241/8.4.20](../../../../rust/tcl-registry/tests/data/native_source_list_length241/8.4.20)
- [rust/tcl-registry/tests/data/native_source_list_length241/8.5.19](../../../../rust/tcl-registry/tests/data/native_source_list_length241/8.5.19)
- [rust/tcl-registry/tests/data/native_source_list_length241/8.6.18](../../../../rust/tcl-registry/tests/data/native_source_list_length241/8.6.18)
- [rust/tcl-registry/tests/data/native_source_list_length241/9.0.4](../../../../rust/tcl-registry/tests/data/native_source_list_length241/9.0.4)
- [rust/tcl-registry/tests/data/native_source_list_length241/9.1.0](../../../../rust/tcl-registry/tests/data/native_source_list_length241/9.1.0)
- [rust/tcl-registry/tests/data/native_source_list_length241](../../../../rust/tcl-registry/tests/data/native_source_list_length241)
- [rust/tcl-registry/tests/data/native_source_list_length241/jim](../../../../rust/tcl-registry/tests/data/native_source_list_length241/jim)
