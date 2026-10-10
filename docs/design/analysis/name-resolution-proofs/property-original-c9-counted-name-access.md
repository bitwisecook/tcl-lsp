# naming.property.original-c9-counted-name-access

Kind: `native-observation`

## Problem statement

Using UTF-8 presentation names for C9.0 properties merges opaque declaration and accessor keys even though native C9.0 and C9.1 share counted/CString name purposes. C9.1 physical property caches remain an independent purpose.

## Question

How do the finite original counted property declaration, option, accessor and stored value controls behave in the six selected providers?

## Conclusion

C9.0 and C9.1 preserve the four counted declaration operands and return the original stored list through default accessors. D800 and D801 custom getters remain distinct. The zero operand formats its roster/option with the CString -p extent, independently of original counted name retention. Both releases reject the six forbidden declaration names with the same recorded byte diagnostics. Older C and Jim never reach these property operations because configurable is unavailable. The independently inspected C9.1 GetImplNamesForProperty window formats accessor children from the member CString extent, including its already present dash. The pure member-accessor projection checks the four retained C9.1 member rows independently from declaration-name formatting; this source explanation does not establish a physical property cache capability.

## Scope

Finite pFF, pD800, pD801 and p/raw-zero/tail names; default original-list write/read identity, separate D800/D801 custom getters, six invalid names, and older C/Jim availability controls. Public original object-vector input is distinct from ASCII bootstrap/roster source. No physical property header, epoch, Index cache, object storage layout, unmeasured mutation release, observer closure, source installation/Normal, arbitrary property ordering or BIG-IP result is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA 8e9c5f50c4f32bea679c823622a64b5d7866d2f37e081f5a9f6567faeeb360ea; compiler argv/path/SHA/version and 5 source/header/library/build pins are retained in its provider receipt.. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter.. Dialect: Tcl.

The actual configurable command inventory is empty. All five fresh bootstrap source queries return code 1 with the recorded unavailable-command diagnostic. Property declaration/access and invalid-name controls are not reached; these are availability negatives.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA 260b46b756626d5219eb987a4921941796ef47e439277698d39651b0b27ee9d0; compiler argv/path/SHA/version and 5 source/header/library/build pins are retained in its provider receipt.. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter.. Dialect: Tcl.

The actual configurable command inventory is empty. All five fresh bootstrap source queries return code 1 with the recorded unavailable-command diagnostic. Property declaration/access and invalid-name controls are not reached; these are availability negatives.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA f9035ad4773f073e61ae083779e757d6db23675e553fd01e4279ab59dd6d8b3b; compiler argv/path/SHA/version and 6 source/header/library/build pins are retained in its provider receipt.. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter.. Dialect: Tcl.

The actual configurable command inventory is empty. All five fresh bootstrap source queries return code 1 with the recorded unavailable-command diagnostic. Property declaration/access and invalid-name controls are not reached; these are availability negatives.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA 5a15c3a7d83eeb2efdd3dfa3d53f5ca487f1ca464d1c392e0b40d861d9ac8821; compiler argv/path/SHA/version and 6 source/header/library/build pins are retained in its provider receipt.. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter.. Dialect: Tcl.

All four counted names pFF, pD800, pD801 and p/raw-zero/tail define, write and read successfully; each default read retains the original supplied list object. D800/D801 custom getters remain distinct as FIRST/SECOND. The zero roster is -p, preserving the independent CString formatting extent. All six invalid declarations return the recorded byte diagnostics.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA 76753b900868b46db658a3214dd97baa484ac785a26f177d93ed6a945ac35ce1; compiler argv/path/SHA/version and 6 source/header/library/build pins are retained in its provider receipt.. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter.. Dialect: Tcl.

All four counted names pFF, pD800, pD801 and p/raw-zero/tail define, write and read successfully; each default read retains the original supplied list object. D800/D801 custom getters remain distinct as FIRST/SECOND. The zero roster is -p, preserving the independent CString formatting extent. All six invalid declarations return the recorded byte diagnostics.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA fd1a0c94d6808e943cdc300aeeddb3a556e908c884acfa59352e141074a3249d; compiler argv/path/SHA/version and 5 source/header/library/build pins are retained in its provider receipt.. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter.. Dialect: Jim Tcl.

The actual configurable command inventory is empty. All five fresh bootstrap source queries return code 1 with the recorded unavailable-command diagnostic. Property declaration/access and invalid-name controls are not reached; these are availability negatives.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance execution is attached.

## Exact evidence

- `probe.c` (input): [rust/tcl-registry/tests/data/native_property_counted_original/probe.c](../../../../rust/tcl-registry/tests/data/native_property_counted_original/probe.c). SHA-256 `04b7c2d0042b26ab25dc8569bbc4cc6bf215994b8d47dd1f4077796b85a8ecd6`. Exact original counted native String/list operands with result pointer sampled before sole getter.
- `queue.json` (input): [rust/tcl-registry/tests/data/native_property_counted_original/queue.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/queue.json). SHA-256 `fd1e3b9d6ee1c6f464d5eabf145d163e3926e4c801db6840111aa43a33a45e50`. Finite requested controls and channels; criteria only.
- `capture.py` (input): [rust/tcl-registry/tests/data/native_property_counted_original/capture.py](../../../../rust/tcl-registry/tests/data/native_property_counted_original/capture.py). SHA-256 `78f015446ee85eed6e9b027d1bc68990dd565bd269c7605181000a3d4d04f194`. Retained actual compiler/launcher protocol and provider pins.
- `receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_counted_original/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/receipt.json). SHA-256 `4f85ddcab7f281460a0a42e799ae19feb311355aa3e05c2a38a77339818d6eef`. Six complete provider captures with exact source/library/header/compiler/version and stream associations.
- `tcl8.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/receipt.json). SHA-256 `fe8b60daf38d4205f7e535348cdbbda3709ed5a4f8667f7e259a5bec10b681f9`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/stdout.tsv). SHA-256 `1f40b77059fed8059261a08b1736e6547e1d1b061bec3f7a85e9fca1f832f4a1`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.5-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/receipt.json). SHA-256 `fdf2d2f401f80f4ce74f384a80170c2e511d883c03a191ce6ba0b1e52ad6e6cb`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/stdout.tsv). SHA-256 `1ca2cab6b4d4e05c4c1247b11aec1c0a00f068f0daa54a9917a5e4c9f11ddb6e`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.5-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.5-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.6-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/receipt.json). SHA-256 `e71c05fc24168a9c1f40ea220493634986cef60b82c963e71fa40264eb5b24c9`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/stdout.tsv). SHA-256 `33b060e3d3b9ecdcbf8eacf8d53e5a28649a98fb5e28052aa0de81c6547f48f0`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.6-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl8.6-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/receipt.json). SHA-256 `921b70ee08366bd2aedfc7062eb258f0e1b018eb07248f733c1a1f6b4f94e855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/stdout.tsv). SHA-256 `cd4c3e3d591df8a7137b89824c185af2db0a68ce4bf05a56a4ad98dc5c1783cd`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.1-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/receipt.json). SHA-256 `83dbde163800adc882fe7f0f706b1d6c8aa626ed387a45a2dabd728c7c358356`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/stdout.tsv). SHA-256 `674d40a2fb329ebfa256f447ec227fce88455bf225698052f4486fa33571b7be`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.1-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `tcl9.1-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_property_counted_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/jim/receipt.json). SHA-256 `2677287f8570ca3725bd0794ca7d968a74b3c5853c062b7d1f41479033864c8a`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `jim-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_property_counted_original/jim/stdout.tsv). SHA-256 `6a1244d2ca113d7e4ac76f7af65503256dbd20bfb1377260d1dc4525586ab5f1`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_property_counted_original/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_property_counted_original/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_property_counted_original/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact immutable capture; complete process status, version and provider joins remain in its receipt.
- `member-accessor-source` (source-anchor): [rust/tcl-registry/tests/data/native_property_counted_original/member-accessor-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_property_counted_original/member-accessor-source-anchors.json). SHA-256 `4bf6bcd75c5bfc117d4abd4db82f9fadc8328fd1c997295f43f45cd5013a13ce`. JSON pointer `/source_anchors/0/snippet`. Exact LF-window/full-file C91 source association for formatting original configuration member children.

## Source inspection

tcl9.1 9.1.0, revision `Recorded retained tcl9.1.0 full source; source-control revision unrecorded.`, `tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetImplNamesForProperty`, lines 165–184. Full-source SHA-256 `c791423ae9b49c877bea8da8a7c2b74b40520e600863341e8d12a63b10c2aaaf`; snippet SHA-256 `9eabc989c950132cb543ccca7e8398951a6ec94dff2a82b40093503f94e7531a`; retained evidence `member-accessor-source`.

```text
static PropertyName *
GetImplNamesForProperty(
    Tcl_Obj *objPtr)
{
    Tcl_ObjInternalRep *irPtr = TclFetchInternalRep(objPtr, &propNameType);
    if (irPtr) {
	return GET_PROPERTY_NAME(irPtr);
    }

    // No cached version; make it

    const char *propName = TclGetString(objPtr);
    PropertyName *namePtr = (PropertyName *) Tcl_Alloc(sizeof(PropertyName));
    namePtr->readerName = Tcl_ObjPrintf("<ReadProp%s>", propName);
    namePtr->writerName = Tcl_ObjPrintf("<WriteProp%s>", propName);
    Tcl_IncrRefCount(namePtr->readerName);
    Tcl_IncrRefCount(namePtr->writerName);
    SET_PROPERTY_NAME(objPtr, namePtr);
    return namePtr;
}

```


## Consumer bindings

- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `NativePropertyLookupProtocol::member_accessor_names`: Selects exact C91 already-dashed original configuration member CString formatting independently from the C90/C91 pure declaration-name formatter.
- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `native_property_lookup::tests::original_c9_name_recipe_matches_native_declaration_boundaries_without_cache_authority` (linked): Compares the six actual C9 declaration diagnostics and counted property projections; selects name recipes for C90/C91 while physical lookup stays C91-only, and older C/Jim/vendor purposes abstain.
- [rust/tcl-vm/src/cmd_oo/native_property_counted_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_property_counted_tests.rs), `cmd_oo::native_property_counted_tests::original_property_names_and_values_match_native_c9_counted_access` (linked): Recreates actual C90/C91 core plus separately installed compiler, enters the exact bootstrap source channel, then compares original counted declaration/read/write/roster bytes, stored list identity, distinct opaque getters and invalid errors. Unavailable original-argv factory controls bind their independent naming.property.original-configurable-availability-counted-argv question; Native source refusal is not reinterpreted as a guest result.
- [runtime/rust/src/cmd_oo/native_property_counted_tests.rs](../../../../runtime/rust/src/cmd_oo/native_property_counted_tests.rs), `cmd_oo::native_property_counted_tests::original_property_names_and_values_match_native_c9_counted_access` (linked): Independently compares original Runtime result pointers before its only getter and the same finite counted name/argv controls with no host refusal. No Rust execution result is inferred.
- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `native_property_lookup::tests::original_c91_property_member_accessors_preserve_the_native_dash_once` (linked): Consumes the four actual C91 member roster/read/write rows, then joins cached dashed-member formatting to independent declaration-name formatting; CString zero extent is retained. No physical capability or execution result follows from the pure comparison.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_property_counted_original/capture.py"
]
```

The retained runner describes the exact captured absolute paths, source/compiler/header/library pins and stdout/stderr associations. A replay requires matching providers and a fresh output directory. Native observations establish only these finite controls; linked Rust assertions are not execution receipts.
