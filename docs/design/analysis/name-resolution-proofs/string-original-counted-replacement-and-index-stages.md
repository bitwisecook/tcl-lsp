# naming.string.original-counted-replacement-and-index-stages

Kind: `native-observation`

## Problem statement

A generic Unicode string replacement can erase actual native index conversion, original object aliasing, release-specific storage units and counted-byte append behaviour. Replacement consumers need the selected physical object protocol and its reached operands, rather than a fresh textual recreation.

## Question

For 21 counted original subject/index/replacement cases, including raw UTF bytes, embedded NULs and aliased argv headers, what completion, original/public header fields, result alias identity and exact result bytes occur on each measured C Tcl 8.4–9.1 and current Jim library?

## Conclusion

All six providers return the exact original subject for the four unchanged cases 0/1/7/20. Changed results, original index primaries, counted index acceptance and raw UTF/NUL result bytes differ by provider. Jim after-part bytes can truncate multibyte UTF; C84/85 raw continuation storage on these builds follows signed plain-char promotion, C86 uses 16-bit surrogate units and C9 preserves the four-byte character. Original/public snapshots and result bytes are separate facts; they do not expose unrecorded C Unicode internals or adverse callback chronology.

## Scope

126 fresh native case processes,21 per library, with process exit0 and original streams retained even when the command completion code is1. All unique original input objects have one caller reference; alias roles reuse that header. Snapshots are taken before EvalObjv, after EvalObjv but before result String materialisation, and after result materialisation. Jim charLength is sampled only for actual String primary; C public headers do not expose that cache. Earlier CLI patchlevel and an independent same-static-library diagnostic supply separate version evidence. ASCII C source uses explicit hex/NUL escapes to construct counted objects, without Tcl source decoding. Software assertions, arbitrary backend/build configurations, C private cache/teardown chronology and BIG-IP remain unmeasured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded compiler invocation/output, actual static library, all selected headers/source/configuration bytes and CLI identities. Independent same-library diagnostic queries exact library patchlevel; complete toolchain/load closure is unmeasured.. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only.. Dialect: tcl8.4.

Raw four-byte UTF input is stored as this build's 16-bit units: replacing é in Aé€😀Z produces 4158e282acc3b0efbe9fefbe98efbe805a. Counted ordinary index1-NUL-tail fails; end-1-NUL-tail succeeds. Fresh changed results may have String primary with absent resident bytes, and some retain resident bytes; exact case distinctions are in the streams.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded compiler invocation/output, actual static library, all selected headers/source/configuration bytes and CLI identities. Independent same-library diagnostic queries exact library patchlevel; complete toolchain/load closure is unmeasured.. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only.. Dialect: tcl8.5.

The measured raw four-byte storage output matches8.4. Counted ordinary index1-NUL-tail succeeds, and end-1-NUL-tail succeeds. Fresh changed header/resident states remain case-specific.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded compiler invocation/output, actual static library, all selected headers/source/configuration bytes and CLI identities. Independent same-library diagnostic queries exact library patchlevel; complete toolchain/load closure is unmeasured.. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only.. Dialect: tcl8.6.

Replacing é in Aé€😀Z yields 4158e282aceda0bdedb8805a; deleting character index3 retains the low-surrogate encoded bytes. Counted ordinary and end-suffix indices succeed. Changed results in these cases have String primary and absent resident bytes before their result getter.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded compiler invocation/output, actual static library, all selected headers/source/configuration bytes and CLI identities. Independent same-library diagnostic queries exact library patchlevel; complete toolchain/load closure is unmeasured.. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only.. Dialect: tcl9.0.

Replacing é preserves the four-byte character: 4158e282acf09f98805a; deleting index3 removes it completely. Counted ordinary index succeeds; counted end-suffix index fails. The invalid shared subject/index 2*1 retains no String primary in this case, unlike C8.x.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded compiler invocation/output, actual static library, all selected headers/source/configuration bytes and CLI identities. Independent same-library diagnostic queries exact library patchlevel; complete toolchain/load closure is unmeasured.. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only.. Dialect: tcl9.1.

The measured original replacement outputs and header distinctions match 9.0 for these 21 cases, including raw four-byte units, counted index acceptance/rejection and shared invalid subject/index.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Recorded compiler invocation/output, actual static library, all selected headers/source/configuration bytes and CLI identities. Independent same-library diagnostic queries exact library patchlevel; complete toolchain/load closure is unmeasured.. Channel: Fresh original EvalObjv C-created counted byte objects, one retained caller reference per unique object and no extra alias argv-role references. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands only.. Dialect: jim.

Unchanged cases return the exact subject header. Replacing é in Aé€😀Z yields 4158e282ac; replacing € yields 41c3a958f09f, a truncated UTF sequence with measured String charLength 5. The source worker appends an after-part byte extent computed from character count. Counted NUL bytes remain raw rather than C modified UTF. Shared subject/first index 2*1 succeeds, changes that same original to Index primary and returns 322a58.

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance process is retained for these exact original header inputs.. Channel: No BIG-IP original-header driver capture.. Dialect: F5 iRules.

C Tcl and Jim original-header results do not establish BIG-IP or TMM behaviour.

## Exact evidence

- `original-counted-probe` (input): [runtime/rust/tests/data/native_original_string_replacement629/probe.c](../../../../runtime/rust/tests/data/native_original_string_replacement629/probe.c). SHA-256 `f0fdfdfad94e5b498caa0fa347bbb2e3a059dbfea88eb47dfb6282d6f3bf1c4b`. Exact counted C byte arrays and alias argv roles; original header snapshots precede the result getter.
- `original-request` (input): [runtime/rust/tests/data/native_original_string_replacement629/request.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/request.json). SHA-256 `aac116706ecb7eb75dfb2b184a6900d53ca7a2421d72d403aad1b801fcc2d303`. Original problem and requirements; request entries do not supply outcomes.
- `actual-case-projection` (observation): [runtime/rust/tests/data/native_original_string_replacement629/interpretation.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/interpretation.json). SHA-256 `97e82baff26df6a8473f8f3e21689e527f01f0206403a490d5c5f2bdd02cb747`. All126 actual stream projections with original stream hashes and separate pre-getter/post-getter fields.
- `exact-input-map` (provider): [runtime/rust/tests/data/native_original_string_replacement629/pinned-input-map.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/pinned-input-map.json). SHA-256 `2369f119909abf3ea8a47f1fc8e5cb4236de1f4ea65b63179ef9673766c48cfc`. Exact original pins and complete round-tripped source/header/static-library/compiler/CLI bytes join.
- `exact-input-bytes` (provider): [runtime/rust/tests/data/native_original_string_replacement629/pinned-input-bytes.tar.gz](../../../../runtime/rust/tests/data/native_original_string_replacement629/pinned-input-bytes.tar.gz). SHA-256 `dae9fa2b7b924c466275dca6b6d1e814efcbf582464bb8d058854be56d36dd65`. Complete content-addressed retained pinned inputs; compressed bundle hashes do not claim omitted build/load dependencies.
- `exact-source-windows` (source-anchor): [runtime/rust/tests/data/native_original_string_replacement629/source-windows.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/source-windows.json). SHA-256 `c45e1d25d5133f372968fe8b2f8515056f0d3657c306b2989809a0a0e442e461`. Original Jim replacement and C84/C85 internal storage decoder macro, exact physical LF/source joins.
- `all-original-retention` (provider): [runtime/rust/tests/data/native_original_string_replacement629/retention.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/retention.json). SHA-256 `eb1806928cb708b43a151764a4814208b72faabc6e48546b01b2b19f81961632`. Every complete original stream, receipt and driver image has a canonical lossless retention identity.
- `tcl8.4-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.4.20/receipt.json). SHA-256 `b0bf0f2eae880c240f45be7f01773877376dc3d3b362ac4f5fbb0cce89b9f850`. Actual original compile, earlier CLI version,21 case-member source pins and per-process bindings.
- `tcl8.4-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/8.4.20/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.4.20/same-library-version/receipt.json). SHA-256 `fdc5fc6e7f82bd82f7b5968b80ceb9597d1a3defca3f463804e3951172e26eb1`. Independent diagnostic compiled against identical actual static library; executes after original cases and changes none of their receipts.
- `tcl8.4-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_string_replacement629/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.4.20/same-library-version/execute.stdout). SHA-256 `ef18d09813e9c05fbd397154e6f786ea0221f2ce8981597665e917a08a5c5d7f`. Actual linked library patchlevel query output, independently joined through unchanged static library bytes.
- `tcl8.5-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.5.19/receipt.json). SHA-256 `0a435ada4fc1f9605d16d845cdfed72e46cb57b0fba782217705be9c43024c2d`. Actual original compile, earlier CLI version,21 case-member source pins and per-process bindings.
- `tcl8.5-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/8.5.19/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.5.19/same-library-version/receipt.json). SHA-256 `c727e16b27ce35692cd03e6dcf0438d31a6f09a89958cfedccf02a821de04962`. Independent diagnostic compiled against identical actual static library; executes after original cases and changes none of their receipts.
- `tcl8.5-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_string_replacement629/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.5.19/same-library-version/execute.stdout). SHA-256 `1bd253489464d3ed255f286efdc8deb3768100a5b873e0a062f5026291fcf9bb`. Actual linked library patchlevel query output, independently joined through unchanged static library bytes.
- `tcl8.6-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.6.18/receipt.json). SHA-256 `77c1d77a14e4f0a8567706799822fee9578731f9db7b50e07d60e3f66885c6be`. Actual original compile, earlier CLI version,21 case-member source pins and per-process bindings.
- `tcl8.6-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/8.6.18/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.6.18/same-library-version/receipt.json). SHA-256 `8ce24d18559b7e708d08635cc7c53dde2dec2f029f1baaa1ce53ac8353fc253b`. Independent diagnostic compiled against identical actual static library; executes after original cases and changes none of their receipts.
- `tcl8.6-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_string_replacement629/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/8.6.18/same-library-version/execute.stdout). SHA-256 `dd12d5dc76bfc80ae3033ac87b1932541f275a888adf9e03dabbcb5878b0df8c`. Actual linked library patchlevel query output, independently joined through unchanged static library bytes.
- `tcl9.0-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.0.4/receipt.json). SHA-256 `76087d7ec9f3c0bb723924439daea271b1cc347dcae74c37f48e947fbb959828`. Actual original compile, earlier CLI version,21 case-member source pins and per-process bindings.
- `tcl9.0-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.0.4/same-library-version/receipt.json). SHA-256 `835bc2120c0918c259501cbf8a9816df8d7d1a72e23f6e0df159c22b5af8ec8c`. Independent diagnostic compiled against identical actual static library; executes after original cases and changes none of their receipts.
- `tcl9.0-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_string_replacement629/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. Actual linked library patchlevel query output, independently joined through unchanged static library bytes.
- `tcl9.1-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.1.0/receipt.json). SHA-256 `0591921143f9be72b72cd76f73b5e16c31bf90d32665e9cc4852ce6648c8bcfe`. Actual original compile, earlier CLI version,21 case-member source pins and per-process bindings.
- `tcl9.1-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.1.0/same-library-version/receipt.json). SHA-256 `ae0fd28f84bfd296acad6c81a5984f04e33ab99965fb64b2234b40b7bf6bd46d`. Independent diagnostic compiled against identical actual static library; executes after original cases and changes none of their receipts.
- `tcl9.1-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_string_replacement629/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. Actual linked library patchlevel query output, independently joined through unchanged static library bytes.
- `jim-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/Jim/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/Jim/receipt.json). SHA-256 `26f4e6a5460e3efe983507af9191e66e8e88ea608ca1e5b0aa8804e0cedaaf9d`. Actual original compile, earlier CLI version,21 case-member source pins and per-process bindings.
- `jim-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_string_replacement629/Jim/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/Jim/same-library-version/receipt.json). SHA-256 `12235f46c4d933c9a652d00a755455a92b70a2bd75badb0c25146473047f7b06`. Independent diagnostic compiled against identical actual static library; executes after original cases and changes none of their receipts.
- `jim-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_string_replacement629/Jim/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_string_replacement629/Jim/same-library-version/execute.stdout). SHA-256 `d7ce62ac8c979a247ad4edf0a43c9133dc8b547cdfc8e5b540fccd9bbe848166`. Actual linked library patchlevel query output, independently joined through unchanged static library bytes.
- `exact-source-snippet-0` (source-anchor): [runtime/rust/tests/data/native_original_string_replacement629/source-windows.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/source-windows.json). SHA-256 `c45e1d25d5133f372968fe8b2f8515056f0d3657c306b2989809a0a0e442e461`. JSON pointer `/anchors/0/snippet`. Exact original physical-LF source snippet selected as decoded JSON text, joined to the retained whole source hash.
- `exact-source-snippet-1` (source-anchor): [runtime/rust/tests/data/native_original_string_replacement629/source-windows.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/source-windows.json). SHA-256 `c45e1d25d5133f372968fe8b2f8515056f0d3657c306b2989809a0a0e442e461`. JSON pointer `/anchors/1/snippet`. Exact original physical-LF source snippet selected as decoded JSON text, joined to the retained whole source hash.
- `exact-source-snippet-2` (source-anchor): [runtime/rust/tests/data/native_original_string_replacement629/source-windows.json](../../../../runtime/rust/tests/data/native_original_string_replacement629/source-windows.json). SHA-256 `c45e1d25d5133f372968fe8b2f8515056f0d3657c306b2989809a0a0e442e461`. JSON pointer `/anchors/2/snippet`. Exact original physical-LF source snippet selected as decoded JSON text, joined to the retained whole source hash.

## Source inspection

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimStringReplaceObj`, lines 2770–2803. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `9a09f9e3a77eb997558eda8f39b18f428ed0700c8205945fe2c374adc66e8b2b`; retained evidence `exact-source-snippet-0`.

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

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 2593–2612. Full-source SHA-256 `f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a`; snippet SHA-256 `53ced9beda30f5e22afc28d2e88c66088d004f0d81cbd1c2047c331b4273fef1`; retained evidence `exact-source-snippet-1`.

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

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 3670–3675. Full-source SHA-256 `72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa`; snippet SHA-256 `0dc12159aed2bbede178c2a157ee8f5076f180f60b21e11a3c4ae79444215483`; retained evidence `exact-source-snippet-2`.

```text
#define TclUtfToUniChar(str, chPtr) \
	((((unsigned char) *(str)) < 0xC0) ?		\
	    ((*(chPtr) = (Tcl_UniChar) *(str)), 1)	\
	    : Tcl_UtfToUniChar(str, chPtr))

/*

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Retain the unchanged probe and all21 inputs. Original compile/launch vectors and exact environments are in each provider receipt; each case uses a new process. Recreate libraries from their separately supported build configuration and verify the selected headers/static-library bytes, or treat rebuilt bytes as a new experiment. The pinned input tar preserves every measured whole input, while omitted system linker/toolchain dependencies remain outside its closure. Same-library version diagnostics are separate processes, not rows inserted into the original cases.
