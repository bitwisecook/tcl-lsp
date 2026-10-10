# naming.array.jim-original-dictionary-runtime-enumeration

Kind: `implementation-contract`

## Problem statement

Dictionary-backed Jim variables require enumeration through their original dictionary and frame owner. Applying C array-cell enumeration can discard original members and static-copy values.

## Question

Does the selected Runtime Jim array read enumerate genuine dictionary members at the retained frame rather than a C array-cell inventory?

## Conclusion

The Runtime VarStore adapter delegates original dictionary conversion and member/name getters to their existing owners; the shared array core retains its existing result ownership and pattern semantics. The inspected Jim array extension uses its selected variable getter and existing dictionary/List/member operations for get, names and pattern removal. The Runtime adapter requires the actual selected Jim dictionary root and frame; written names do not select a C array cell. Source inspection supplies this bounded explanation independently of authored Runtime test outcomes.

## Scope

Selected Jim084 Runtime dictionary key enumeration, member value read and pattern removal. No native process outcome, no-pattern List header identity, arbitrary array/observer parity or static capture authority follows.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No guest execution receipt for this implementation question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No guest execution receipt for this implementation question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No guest execution receipt for this implementation question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No guest execution receipt for this implementation question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No guest execution receipt for this implementation question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No guest execution receipt for this implementation question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No guest execution receipt for this implementation question.

## Exact evidence

- `source-array_cmd_get` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_static_original/jim-array-enumeration-source.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim-array-enumeration-source.json). SHA-256 `3b9ed9bc574a16b1f86c989cf3eb59c6d69d00f6b1bcc840548ba6fed2e4d64a`. JSON pointer `/source_anchors/0/snippet`. Exact LF-only full-source/snippet join for array_cmd_get; source version/revision unrecorded and no native launch/outcome inferred.
- `source-array_cmd_names` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_static_original/jim-array-enumeration-source.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim-array-enumeration-source.json). SHA-256 `3b9ed9bc574a16b1f86c989cf3eb59c6d69d00f6b1bcc840548ba6fed2e4d64a`. JSON pointer `/source_anchors/1/snippet`. Exact LF-only full-source/snippet join for array_cmd_names; source version/revision unrecorded and no native launch/outcome inferred.
- `source-array_cmd_unset` (source-anchor): [rust/tcl-registry/tests/data/native_procedure_static_original/jim-array-enumeration-source.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim-array-enumeration-source.json). SHA-256 `3b9ed9bc574a16b1f86c989cf3eb59c6d69d00f6b1bcc840548ba6fed2e4d64a`. JSON pointer `/source_anchors/2/snippet`. Exact LF-only full-source/snippet join for array_cmd_unset; source version/revision unrecorded and no native launch/outcome inferred.

## Source inspection

jim Version unrecorded for this source artifact., revision `Source-control revision unrecorded.`, `/workspace/.proofs/native-providers/jimtcl/jim-array.c`, function `array_cmd_get`, lines 62–83. Full-source SHA-256 `ce2bb617dc1f79e80c6466f0485b2570cdedaa03e669cb1e462a062a6af5edda`; snippet SHA-256 `030cd86ca020f9e5e62b271f1ea83f6d307e4c3e61fba16319ecb3b65d0ef7e7`; retained evidence `source-array_cmd_get`.

```text
static int array_cmd_get(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr = Jim_GetVariable(interp, argv[0], JIM_NONE);
    Jim_Obj *patternObj;

    if (!objPtr) {
        return JIM_OK;
    }

    patternObj = (argc == 1) ? NULL : argv[1];

    /* Optimise the "all" case */
    if (patternObj == NULL || Jim_CompareStringImmediate(interp, patternObj, "*")) {
        if (Jim_IsList(objPtr) && Jim_ListLength(interp, objPtr) % 2 == 0) {
            /* A list with an even number of elements */
            Jim_SetResult(interp, objPtr);
            return JIM_OK;
        }
    }

    return Jim_DictMatchTypes(interp, objPtr, patternObj, JIM_DICTMATCH_KEYS, JIM_DICTMATCH_KEYS | JIM_DICTMATCH_VALUES);
}

```

jim Version unrecorded for this source artifact., revision `Source-control revision unrecorded.`, `/workspace/.proofs/native-providers/jimtcl/jim-array.c`, function `array_cmd_names`, lines 85–94. Full-source SHA-256 `ce2bb617dc1f79e80c6466f0485b2570cdedaa03e669cb1e462a062a6af5edda`; snippet SHA-256 `b9d48e8e15f0111ca90942b6fc6eaaaf1834956ebf7c0dd3b06844703fdcde63`; retained evidence `source-array_cmd_names`.

```text
static int array_cmd_names(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr = Jim_GetVariable(interp, argv[0], JIM_NONE);

    if (!objPtr) {
        return JIM_OK;
    }

    return Jim_DictMatchTypes(interp, objPtr, argc == 1 ? NULL : argv[1], JIM_DICTMATCH_KEYS, JIM_DICTMATCH_KEYS);
}

```

jim Version unrecorded for this source artifact., revision `Source-control revision unrecorded.`, `/workspace/.proofs/native-providers/jimtcl/jim-array.c`, function `array_cmd_unset`, lines 96–135. Full-source SHA-256 `ce2bb617dc1f79e80c6466f0485b2570cdedaa03e669cb1e462a062a6af5edda`; snippet SHA-256 `72f9e7cd3f3ab9cd8cf73abbcd6cbafe01c5cd4254dd207aad575aad1b938120`; retained evidence `source-array_cmd_unset`.

```text
static int array_cmd_unset(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int i;
    int len;
    Jim_Obj *resultObj;
    Jim_Obj *objPtr;
    Jim_Obj **dictValuesObj;

    if (argc == 1 || Jim_CompareStringImmediate(interp, argv[1], "*")) {
        /* Unset the whole array */
        Jim_UnsetVariable(interp, argv[0], JIM_NONE);
        return JIM_OK;
    }

    objPtr = Jim_GetVariable(interp, argv[0], JIM_NONE);

    if (objPtr == NULL) {
        /* Doesn't exist, so nothing to do */
        return JIM_OK;
    }

    dictValuesObj = Jim_DictPairs(interp, objPtr, &len);
    if (dictValuesObj == NULL) {
        /* Variable is not an array - tclsh ignores this and returns nothing - be compatible */
        Jim_SetResultString(interp, "", -1);
        return JIM_OK;
    }

    /* Create a new object with the values which don't match */
    resultObj = Jim_NewDictObj(interp, NULL, 0);

    for (i = 0; i < len; i += 2) {
        if (!Jim_StringMatchObj(interp, argv[1], dictValuesObj[i], 0)) {
            Jim_DictAddElement(interp, resultObj, dictValuesObj[i], dictValuesObj[i + 1]);
        }
    }

    Jim_SetVariable(interp, argv[0], resultObj);
    return JIM_OK;
}

```


## Consumer bindings

- [runtime/rust/src/state_traits.rs](../../../../runtime/rust/src/state_traits.rs), `native_dictionary_array_pairs`: Authentic selected Jim dictionary root at exact frame with existing native name/object/context and dictionary member owners; no C array cell donation.
- [runtime/rust/src/cmd_array.rs](../../../../runtime/rust/src/cmd_array.rs), `cmd_array::tests::jim_dictionary_array_reads_keep_static_copies_and_selected_frames` (linked): Static copied root preserves the captured dictionary while the outer root changes; independent local and upvar roots remain distinct at their actual selected frames.
- [runtime/rust/src/cmd_array.rs](../../../../runtime/rust/src/cmd_array.rs), `cmd_array::tests::jim_array_byte_keys_count_and_unset_without_unicode_projection` (linked): Original opaque dictionary keys enumerate/read/unset through the shared byte and object owners; malformed roots remain guest errors/absence as appropriate.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-runtime",
  "jim_dictionary_array_reads_keep_static_copies_and_selected_frames"
]
```

The linked selectors require the actual Runtime package test environment; no command execution is claimed by this source/implementation record.
