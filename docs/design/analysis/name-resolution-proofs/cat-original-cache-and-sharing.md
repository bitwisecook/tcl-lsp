# naming.cat.original-cache-and-sharing

Kind: `native-observation`

## Problem statement

A concatenation with identical output can return an input or a newly allocated object depending on sharing, empty operands and cache kind. Reading output bytes before the original physical window would erase the storage being measured.

## Question

How does private TclStringCat preserve original cache, residency and input/result identity across the fixed constructors and sharing axis?

## Conclusion

C9.0/9.1 agree on the retained23-case, two-sharing-axis matrix (46 concatenations per release). For A+A, the unshared route returns the first object and the shared route allocates a distinct result. Empty list/dictionary plus A returns the A operand under either sharing axis. Pure bytearray FF00 pairs project as C3BF C080 C3BF C080. Original before/after/result-before-string and final identity/byte windows remain independent in the312 rows per release.

## Scope

Twenty-three fixed C9-only constructor combinations, each with/without an extra reference to operand zero, compiled against private tclInt.h/tclStringRep.h. Exact full JSONL and13-column projected TSVs are retained. C8.4/8.5/8.6/Jim/BIG-IP are not tested by this corpus, rather than inferred to reject the ABI.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim, bigip

Status: `not-tested`.

No observation of this exact compiled-object question is attached for this provider.

### tcl9.0, tcl9.1

Status: `observed`. Channel: compiled C private TclStringCat original-object ABI; physical snapshots before result string generation. Version: tcl9.0 9.0.4 (manifest release association; probe does not query runtime patchlevel); tcl9.1 9.1.0 (manifest release association; probe does not query runtime patchlevel).

Compiled/run status0; 46 concatenations and 312 retained physical/projection rows. Full final [case,sharing,result identities,result hex] observations: [[0,0,[1,0],"4141"],[0,1,[0,0],"4141"],[1,0,[1,0],"c3bfc080c3bfc080"],[1,1,[0,0],"c3bfc080c3bfc080"],[2,0,[1,0],"c3bfc08041"],[2,1,[0,0],"c3bfc08041"],[3,0,[1,0],"c3bfc080c3bfc080"],[3,1,[0,0],"c3bfc080c3bfc080"],[4,0,[1,0],"c3a9f09f988041"],[4,1,[0,0],"c3a9f09f988041"],[5,0,[1,0],"41c3a9f09f9880"],[5,1,[0,0],"41c3a9f09f9880"],[6,0,[1,0],"41e282ac42"],[6,1,[0,0],"41e282ac42"],[7,0,[1,0],"378042"],[7,1,[0,0],"378042"],[8,0,[1,0],"c3a9f09f9880c3bfc080"],[8,1,[0,0],"c3a9f09f9880c3bfc080"],[9,0,[0,1],"41"],[9,1,[0,1],"41"],[10,0,[0,1],"41"],[10,1,[0,1],"41"],[11,0,[1,0],"3741"],[11,1,[0,0],"3741"],[12,0,[0,1,0],"c3a9f09f9880"],[12,1,[0,1,0],"c3a9f09f9880"],[13,0,[1,0,0],"c3a9f09f9880"],[13,1,[1,0,0],"c3a9f09f9880"],[14,0,[0,0,1],"37"],[14,1,[0,0,1],"37"],[15,0,[0,0,1],""],[15,1,[0,0,1],""],[16,0,[1,0],"4141"],[16,1,[0,0],"4141"],[17,0,[1,0],"4141"],[17,1,[0,0],"4141"],[18,0,[1,0,0],"37"],[18,1,[1,0,0],"37"],[19,0,[1,0,0],"37"],[19,1,[1,0,0],"37"],[20,0,[0,0,1],""],[20,1,[0,0,1],""],[21,0,[0,1,0],"41"],[21,1,[0,1,0],"41"],[22,0,[1,0,0],"4141"],[22,1,[0,0,0],"4141"]]. Every original snapshot is in the linked JSONL; no output rendering is applied before its physical windows.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_cat.rs](../../../../rust/tcl-cmd-core/src/native_cat.rs), `concatenate`: Consumes the independently selected native concatenation recipe and original borrowed operand objects; the native observation does not itself prove Rust execution.
- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_cat_preserves_all_original_cache_and_identity_windows` (linked): Compares each retained original cache/residency/reference window, adjusted known returned-owner reference, input/result identity and exact counted output bytes under C9.0/9.1.

## Evidence

- [rust/tcl-syntax/tests/data/native_object_cat/probe.c](../../../../rust/tcl-syntax/tests/data/native_object_cat/probe.c)
- [rust/tcl-syntax/tests/data/native_object_cat/manifest.json](../../../../rust/tcl-syntax/tests/data/native_object_cat/manifest.json)
- [rust/tcl-syntax/tests/data/native_object_cat/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_object_cat/9.0.4.tsv)
- [rust/tcl-syntax/tests/data/native_object_cat/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_object_cat/9.1.0.tsv)
