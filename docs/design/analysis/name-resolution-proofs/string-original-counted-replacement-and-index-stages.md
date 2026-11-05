# naming.string.original-counted-replacement-and-index-stages

Kind: `native-observation`

## Problem statement

A generic Unicode string replacement can erase actual native index conversion, original object aliasing, release-specific storage units and counted-byte append behaviour. Replacement consumers need the selected physical object protocol and its reached operands, rather than a fresh textual recreation.

## Question

For 21 counted original subject/index/replacement cases, including raw UTF bytes, embedded NULs and aliased argv headers, what completion, original/public header fields, result alias identity and exact result bytes occur on each measured C Tcl 8.4–9.1 and current Jim library?

## Conclusion

All six providers return the exact original subject for the four unchanged cases 0/1/7/20. Changed results, original index primaries, counted index acceptance and raw UTF/NUL result bytes differ by provider. Jim after-part bytes can truncate multibyte UTF; C84/85 raw continuation storage on these builds follows signed plain-char promotion, C86 uses 16-bit surrogate units and C9 preserves the four-byte character. Original/public snapshots and result bytes are separate facts; they do not expose unrecorded C Unicode internals or adverse callback chronology.

## Scope

126 fresh native case processes,21 per library, with process exit0 and original streams retained even when the command completion code is1. All unique original input objects have one caller reference; alias roles reuse that header. Snapshots are taken before EvalObjv, after EvalObjv but before result String materialisation, and after result materialisation. Jim charLength is sampled only for actual String primary; C public headers do not expose that cache. Earlier CLI patchlevel and an independent same-static-library diagnostic supply separate version evidence. ASCII C source uses explicit hex/NUL escapes to construct counted objects, without Tcl source decoding. Software assertions, arbitrary backend/build configurations, C private cache/teardown chronology and BIG-IP remain unmeasured. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only. Version: tcl8.4 8.4.20.

Raw four-byte UTF input is stored as this build's 16-bit units: replacing é in Aé€😀Z produces 4158e282acc3b0efbe9fefbe98efbe805a. Counted ordinary index1-NUL-tail fails; end-1-NUL-tail succeeds. Fresh changed results may have String primary with absent resident bytes, and some retain resident bytes; exact case distinctions are in the streams.

### tcl8.5

Status: `observed`. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only. Version: tcl8.5 8.5.19.

The measured raw four-byte storage output matches8.4. Counted ordinary index1-NUL-tail succeeds, and end-1-NUL-tail succeeds. Fresh changed header/resident states remain case-specific.

### tcl8.6

Status: `observed`. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only. Version: tcl8.6 8.6.18.

Replacing é in Aé€😀Z yields 4158e282aceda0bdedb8805a; deleting character index3 retains the low-surrogate encoded bytes. Counted ordinary and end-suffix indices succeed. Changed results in these cases have String primary and absent resident bytes before their result getter.

### tcl9.0

Status: `observed`. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only. Version: tcl9.0 9.0.4.

Replacing é preserves the four-byte character: 4158e282acf09f98805a; deleting index3 removes it completely. Counted ordinary index succeeds; counted end-suffix index fails. The invalid shared subject/index 2*1 retains no String primary in this case, unlike C8.x.

### tcl9.1

Status: `observed`. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only. Version: tcl9.1 9.1.0.

The measured original replacement outputs and header distinctions match 9.0 for these 21 cases, including raw four-byte units, counted index acceptance/rejection and shared invalid subject/index.

### jim

Status: `observed`. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only. Version: jim 0.84-9-g5bac7c9.

Unchanged cases return the exact subject header. Replacing é in Aé€😀Z yields 4158e282ac; replacing € yields 41c3a958f09f, a truncated UTF sequence with measured String charLength 5. The source worker appends an after-part byte extent computed from character count. Counted NUL bytes remain raw rather than C modified UTF. Shared subject/first index 2*1 succeeds, changes that same original to Index primary and returns 322a58.

### bigip

Status: `not-tested`. Channel: No BIG-IP original-header driver capture.

C Tcl and Jim original-header results do not establish BIG-IP or TMM behaviour.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimStringReplaceObj`, lines 2770–2803. Retained reference `exact-source-snippet-0`.

```text
Jim_Obj *JimStringReplaceObj(Jim_Interp *interp,
    Jim_Obj *strObjPtr, Jim_Obj *firstObjPtr, Jim_Obj *lastObjPtr, Jim_Obj *newStrObj)
{
    int first, last;
    const char *str;
    int len, rangeLen;
    Jim_Obj *objPtr;

    len = Jim_Utf8Length(interp, strObjPtr);

    if (JimStringGetRange(interp, firstObjPtr, lastObjPtr, len, &first, &last, &rangeLen) != JIM_OK) {
        return NULL;
    }

    if (last < first) {
        return strObjPtr;
    }

    str = Jim_String(strObjPtr);

    /* Before part */
    objPtr = Jim_NewStringObjUtf8(interp, str, first);

    /* Replacement */
    if (newStrObj) {
        Jim_AppendObj(interp, objPtr, newStrObj);
    }

    /* After part */
    Jim_AppendString(interp, objPtr, str + utf8_index(str, last + 1), len - last - 1);

    return objPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 2593–2612. Retained reference `exact-source-snippet-1`.

```text
 * It checks to see if we have a one-byte utf char before calling
 * the real Tcl_UtfToUniChar, as this will save a lot of time for
 * primarily ascii string handling. The macro's expression result
 * is 1 for the 1-byte case or the result of Tcl_UtfToUniChar.
 * The ANSI C "prototype" for this macro is:
 *
 * EXTERN int TclUtfToUniChar _ANSI_ARGS_((CONST char *string,
 *					   Tcl_UniChar *ch));
 *----------------------------------------------------------------
 */

#define TclUtfToUniChar(str, chPtr) \
	((((unsigned char) *(str)) < 0xC0) ? \
	    ((*(chPtr) = (Tcl_UniChar) *(str)), 1) \
	    : Tcl_UtfToUniChar(str, chPtr))

/*
 *----------------------------------------------------------------
 * Macro used by the Tcl core to compare Unicode strings.  On
 * big-endian systems we can use the more efficient memcmp, but

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 3670–3675. Retained reference `exact-source-snippet-2`.

```text
#define TclUtfToUniChar(str, chPtr) \
	((((unsigned char) *(str)) < 0xC0) ?		\
	    ((*(chPtr) = (Tcl_UniChar) *(str)), 1)	\
	    : Tcl_UtfToUniChar(str, chPtr))

/*

```

## Consumer bindings

- [rust/tcl-syntax/src/native_tcl_utf.rs](../../../../rust/tcl-syntax/src/native_tcl_utf.rs), `legacy_storage_plain_char_promotion_is_separate_from_public_unit_decoding` (linked): Software source definition checks legacy storage plain char promotion is separate from public unit decoding.
- [rust/tcl-syntax/src/raw_string.rs](../../../../rust/tcl-syntax/src/raw_string.rs), `original_jim_after_part_uses_the_measured_character_derived_byte_extent` (linked): Software source definition checks jim after part uses the measured character derived byte extent.
- [rust/tcl-vm/src/value_ops/native_jim_index.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_index.rs), `original_replacement_matches_every_native_observed_projection` (linked): Software source definition checks replacement matches every native observed projection.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `original_replacement_matches_every_native_observed_projection` (linked): Software source definition checks replacement matches every native observed projection.

## Evidence

- [runtime/rust/tests/data/native_original_string_replacement629/probe.c](../../../../runtime/rust/tests/data/native_original_string_replacement629/probe.c)
- [runtime/rust/tests/data/native_original_string_replacement629/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.4.20/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_string_replacement629/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.5.19/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_string_replacement629/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.6.18/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_string_replacement629/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.0.4/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_string_replacement629/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.1.0/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_string_replacement629/Jim/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/Jim/same-library-version/execute.stdout)
