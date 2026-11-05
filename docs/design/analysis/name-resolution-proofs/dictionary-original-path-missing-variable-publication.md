# naming.dictionary.original-path-missing-variable-publication

Kind: `native-observation`

## Problem statement

Does an original dict path mutator publish an empty missing variable before a later path failure, including a dictionary-valued member and an upvar link? Distinguish C command lookup/write semantics from actual Jim_SetDictKeysVector rather than granting a C lookup frontier to the Jim worker.

## Question

When original dict path operations select a missing variable or array member, what is publicly readable after the path operation fails, and does a linked successful path store reach its original variable?

## Conclusion

Six actual compiles and eighteen fresh initialized host processes exit0 with empty stderr. C8.5–9.1 return the same missing-key failure while the missing root or member remains absent and its separate read fails. Jim returns the same missing-key failure after publishing an empty root or member: its separate existence/read outputs report1/0 and an empty value. The Jim member control also retains keep OLD and the new empty member in array get. All five available providers return the same linked path-set result and both original/linked values. C8.4 returns the original explicit NOT_APPLICABLE branch. The retained Jim worker source independently explains the initial ordinary variable publication; it does not measure physical identity or grant successful software coverage.

## Scope

Exactly three unchanged original ASCII scripts per six pinned initialized providers, each in a fresh host process. Whole original code/result streams, actual version rows, compile/process receipts, ELFs, launcher and required header/library/source/build pins are retained. The original C8.4 availability branch contains a top-level return; its Tcl_EvalEx embedding result is retained independently of raw evaluator Return semantics. Public failed path completion, separate existence/read and successful linked-store values are distinct fields. Source inspection and authored policy tests remain independent from native measurements. No private header/object/root identity, trace chronology, arbitrary dictionary operation, body/frame/compiler entry, editor transformation or BIG-IP observation follows.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh initialized original counted ASCII source via Tcl_EvalEx flags0 or Jim_EvalObj, without an added surrounding catch/eval/puts wrapper. Version: tcl8.4 8.4.20.

All three whole originals return the explicit NOT_APPLICABLE result under the actual Tcl_EvalEx embedding. Three fresh host processes exit0 with empty stderr; complete raw fields remain unchanged.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh initialized original counted ASCII source via Tcl_EvalEx flags0 or Jim_EvalObj, without an added surrounding catch/eval/puts wrapper. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Both failed path-unset controls return catch code1 and key "missing" not known in dictionary; separate existence/read report0/1. The root read says no such variable; the member read says variable isn't array and array get is empty. The linked path-set control reports catch code0 and the identical old BASE added NEW result/original/linked values. Three fresh host processes exit0 with empty stderr; complete raw fields remain unchanged.

### jim

Status: `observed`. Channel: Fresh initialized original counted ASCII source via Tcl_EvalEx flags0 or Jim_EvalObj, without an added surrounding catch/eval/puts wrapper. Version: jim 0.84-9-g5bac7c9.

Both failed path-unset controls return catch code1 and key "missing" not known in dictionary; separate existence/read report1/0 with an empty value. The member control retains keep OLD member {} in array get. The linked path-set control reports catch code0 and the identical old BASE added NEW result/original/linked values. Three fresh host processes exit0 with empty stderr; complete raw fields remain unchanged.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim Pinned original Jim worker source; independent source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c`, function `Jim_SetDictKeysVector`, lines 8138–8218. Retained reference `native_dictionary_path_publication319-source-inspection-jim.c`.

```text
int Jim_SetDictKeysVector(Jim_Interp *interp, Jim_Obj *varNamePtr,
    Jim_Obj *const *keyv, int keyc, Jim_Obj *newObjPtr, int flags)
{
    Jim_Obj *varObjPtr, *objPtr, *dictObjPtr;
    int shared, i;

    varObjPtr = objPtr = Jim_GetVariable(interp, varNamePtr, flags);
    if (objPtr == NULL) {
        if (newObjPtr == NULL && (flags & JIM_MUSTEXIST)) {
            /* Cannot remove a key from non existing var */
            return JIM_ERR;
        }
        varObjPtr = objPtr = Jim_NewDictObj(interp, NULL, 0);
        if (Jim_SetVariable(interp, varNamePtr, objPtr) != JIM_OK) {
            return JIM_ERR;
        }
    }
    if ((shared = Jim_IsShared(objPtr)))
        varObjPtr = objPtr = Jim_DuplicateObj(interp, objPtr);
    for (i = 0; i < keyc; i++) {
        dictObjPtr = objPtr;

        /* Check if it's a valid dictionary */
        if (SetDictFromAny(interp, dictObjPtr) != JIM_OK) {
            goto err;
        }

        if (i == keyc - 1) {
            /* Last key: Note that error on unset with missing last key is OK */
            if (Jim_DictAddElement(interp, objPtr, keyv[keyc - 1], newObjPtr) != JIM_OK) {
                if (newObjPtr || (flags & JIM_MUSTEXIST)) {
                    goto err;
                }
            }
            if (newObjPtr) {
                varObjPtr->taint |= newObjPtr->taint;
            }
            break;
        }

        /* Check if the given key exists. */
        Jim_InvalidateStringRep(dictObjPtr);
        if (Jim_DictKey(interp, dictObjPtr, keyv[i], &objPtr,
                newObjPtr ? JIM_NONE : JIM_ERRMSG) == JIM_OK) {
            /* This key exists at the current level.
             * Make sure it's not shared!. */
            if (Jim_IsShared(objPtr)) {
                objPtr = Jim_DuplicateObj(interp, objPtr);
                DictAddElement(interp, dictObjPtr, keyv[i], objPtr);
            }
        }
        else {
            /* Key not found. If it's an [unset] operation
             * this is an error. Only the last key may not
             * exist. */
            if (newObjPtr == NULL) {
                goto err;
            }
            /* Otherwise set an empty dictionary
             * as key's value. */
            objPtr = Jim_NewDictObj(interp, NULL, 0);
            DictAddElement(interp, dictObjPtr, keyv[i], objPtr);
        }
    }
    /* XXX: Is this necessary? */
    Jim_InvalidateStringRep(objPtr);
    Jim_InvalidateStringRep(varObjPtr);
    if (Jim_SetVariable(interp, varNamePtr, varObjPtr) != JIM_OK) {
        return JIM_ERR;
    }

    if (!(flags & JIM_NORESULT)) {
        Jim_SetResult(interp, varObjPtr);
    }
    return JIM_OK;
  err:
    if (shared) {
        Jim_FreeNewObj(interp, varObjPtr);
    }
    return JIM_ERR;
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_dictionary.rs](../../../../rust/tcl-registry/src/native_dictionary.rs), `InvocationDialect::native_dictionary_path_publication`: Select C original-name versus Jim original-variable path publication as a purpose, without issuing physical object/frame facts.
- [rust/tcl-vm/src/interp/native_dictionary.rs](../../../../rust/tcl-vm/src/interp/native_dictionary.rs), `Vm::jim_dictionary_path_update_bytes`: Use original ordinary variable lookup/publication for the selected Jim worker, independently of the C primitive lookup frontier.
- [runtime/rust/src/cmd_dict.rs](../../../../runtime/rust/src/cmd_dict.rs), `set`: Retain actual selected dictionary path update policy and public completion through the Runtime native handler.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `dictionary_path_update_bytes`: Delegate selected original counted path operands to the actual VM path owner and keep C/Jim lookup frontiers distinct.
- [rust/tcl-registry/src/native_dictionary.rs](../../../../rust/tcl-registry/src/native_dictionary.rs), `native_dictionary::path_publication_tests::original_path_publication_keeps_jim_worker_separate_from_c_lookup_frontier` (linked): Pure selected policy keeps the Jim worker ordinary-variable publication distinct from C primitive GET, rejects absent/foreign purposes and grants no native physical receiver or new process observation.
- [runtime/rust/src/cmd_dict.rs](../../../../runtime/rust/src/cmd_dict.rs), `cmd_dict::native_path_publication_tests::original_path_missing_variable_publication_matches_all_15_available_native_windows` (linked): Replay all three unchanged original sources under C8.5/8.6/9.0/9.1 and Jim and compare complete original public code/result windows. C8.4 original NOT_APPLICABLE captures remain independently retained with purpose absence, without raw Return/API Ok normalization. No backend pass, private identity or arbitrary path-operation equivalence is inferred.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_path_publication_tests::original_path_missing_variable_publication_matches_all_15_available_native_windows` (linked): Replay all three unchanged original sources under C8.5/8.6/9.0/9.1 and Jim and compare complete original public code/result windows. C8.4 original NOT_APPLICABLE captures remain independently retained with purpose absence, without raw Return/API Ok normalization. No backend pass, private identity or arbitrary path-operation equivalence is inferred.
- [runtime/rust/src/cmd_dict/original_path_tests.rs](../../../../runtime/rust/src/cmd_dict/original_path_tests.rs), `cmd_dict::original_path_tests::original_dictionary_generic_unset_missing_root_and_sugar_required_unset_remain_distinct` (linked): Finite source-defined control: original dictionary generic unset missing root and sugar required unset remain distinct. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 0 | 1 | 0 | 0 |
| `isolated` | 0 | 1 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_dictionary_path_publication319](../../../../rust/tcl-registry/tests/data/native_dictionary_path_publication319)
- [rust/tcl-registry/tests/data/native_dictionary_path_publication319/8.4.20](../../../../rust/tcl-registry/tests/data/native_dictionary_path_publication319/8.4.20)
- [rust/tcl-registry/tests/data/native_dictionary_path_publication319/8.5.19](../../../../rust/tcl-registry/tests/data/native_dictionary_path_publication319/8.5.19)
- [rust/tcl-registry/tests/data/native_dictionary_path_publication319/8.6.18](../../../../rust/tcl-registry/tests/data/native_dictionary_path_publication319/8.6.18)
- [rust/tcl-registry/tests/data/native_dictionary_path_publication319/9.0.4](../../../../rust/tcl-registry/tests/data/native_dictionary_path_publication319/9.0.4)
- [rust/tcl-registry/tests/data/native_dictionary_path_publication319/9.1.0](../../../../rust/tcl-registry/tests/data/native_dictionary_path_publication319/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_dictionary_path_publication319/jim](../../../../rust/tcl-registry/tests/data/native_dictionary_path_publication319/jim)
