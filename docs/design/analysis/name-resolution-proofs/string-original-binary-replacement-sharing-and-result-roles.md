# naming.string.original-binary-replacement-sharing-and-result-roles

Kind: `native-observation`

## Problem statement

String replacement consumers must preserve the physical ByteArray producer, original argument/result roles and actual sharing schedule. Treating binary inputs as fresh Unicode Strings loses native result identity; treating allocator bytes as deterministic data overclaims the native contract.

## Question

For 11 native ByteArray replacement inputs on C Tcl 8.4–9.1, what public original/result primaries, resident-byte fields, alias identities, completions and result String bytes are measured under actual indexed EvalObjv caller references?

## Conclusion

C8 converts these original ByteArrays through its Unicode path. C9 preserves ByteArray primaries on binary branches and returns the original replacement for whole replacement. Both equal-splice caller-reference cases return fresh objects in indexed EvalObjv; this does not prove the separate direct helper cannot splice an unshared subject in place. C9 partial deletion appends to an uninitialised nonzero-length constructor; its recorded leading zeros are not a deterministic result-byte contract.

## Scope

55 fresh case processes and five independent same-static-library version diagnostics, with complete original streams/receipts and whole selected input bytes. Before-command, reached-before-result-getter and materialised snapshots remain distinct. Public object headers do not expose private ByteArray internals or entered EvalObjv temporary references. Direct parsed-index helper behavior, arbitrary build/allocator configurations, adverse callback chronology, Rust assertions, Jim binary equivalence and BIG-IP are not established. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact compiler invocation/output, selected actual static library/source/header/configuration/CLI bytes, and independent same-library version process. Complete system toolchain/load closure is not recorded.. Channel: Fresh Tcl_EvalObjv with native ByteArray inputs. Unique argv objects have one caller reference; case4 adds one genuine subject reference. Tcl_CreateInterp is used without Tcl_Init.. Dialect: tcl8.4.

The subject reaches String primary through Unicode conversion. Partial deletion produces 415a; replacement bytes fe become c3be and NUL becomes c080. Whole replacement is a fresh String result, including cases with an actual extra subject reference. Inverted and empty cases return the original subject. Exact public header fields are case-specific.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact compiler invocation/output, selected actual static library/source/header/configuration/CLI bytes, and independent same-library version process. Complete system toolchain/load closure is not recorded.. Channel: Fresh Tcl_EvalObjv with native ByteArray inputs. Unique argv objects have one caller reference; case4 adds one genuine subject reference. Tcl_CreateInterp is used without Tcl_Init.. Dialect: tcl8.5.

The subject reaches String primary through Unicode conversion. Partial deletion produces 415a; replacement bytes fe become c3be and NUL becomes c080. Whole replacement is a fresh String result, including cases with an actual extra subject reference. Inverted and empty cases return the original subject. Exact public header fields are case-specific.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact compiler invocation/output, selected actual static library/source/header/configuration/CLI bytes, and independent same-library version process. Complete system toolchain/load closure is not recorded.. Channel: Fresh Tcl_EvalObjv with native ByteArray inputs. Unique argv objects have one caller reference; case4 adds one genuine subject reference. Tcl_CreateInterp is used without Tcl_Init.. Dialect: tcl8.6.

The subject reaches String primary through Unicode conversion. Partial deletion produces 415a; replacement bytes fe become c3be and NUL becomes c080. Whole replacement is a fresh String result, including cases with an actual extra subject reference. Inverted and empty cases return the original subject. Exact public header fields are case-specific.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact compiler invocation/output, selected actual static library/source/header/configuration/CLI bytes, and independent same-library version process. Complete system toolchain/load closure is not recorded.. Channel: Fresh Tcl_EvalObjv with native ByteArray inputs. Unique argv objects have one caller reference; case4 adds one genuine subject reference. Tcl_CreateInterp is used without Tcl_Init.. Dialect: tcl9.0.

The subject remains ByteArray primary. Whole replacement returns the exact original replacement; inverted and empty cases return the original subject. Equal splices return fresh ByteArray results under this actual EvalObjv caller schedule, both with one and with two caller references. Resident original Strings do not turn these observed splices into a Unicode path. Ordinary String replacement does reach Unicode. Partial deletion records c080c080415a, but the leading native storage is not initialised by its source producer, so these zeros are not a reusable expected-value guarantee.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact compiler invocation/output, selected actual static library/source/header/configuration/CLI bytes, and independent same-library version process. Complete system toolchain/load closure is not recorded.. Channel: Fresh Tcl_EvalObjv with native ByteArray inputs. Unique argv objects have one caller reference; case4 adds one genuine subject reference. Tcl_CreateInterp is used without Tcl_Init.. Dialect: tcl9.1.

The subject remains ByteArray primary. Whole replacement returns the exact original replacement; inverted and empty cases return the original subject. Equal splices return fresh ByteArray results under this actual EvalObjv caller schedule, both with one and with two caller references. Resident original Strings do not turn these observed splices into a Unicode path. Ordinary String replacement does reach Unicode. Partial deletion records c080c080415a, but the leading native storage is not initialised by its source producer, so these zeros are not a reusable expected-value guarantee.

### jim

Status: `not-tested`. Version: not tested. Build: No genuine native ByteArray producer is available in the selected Jim object model.. Channel: No substitution of an ordinary Jim String for a C ByteArray input.. Dialect: jim.

The C ByteArray question does not establish a Jim binary-object protocol. Independent ordinary String replacement evidence has its own record.

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance original-header driver capture.. Channel: No BIG-IP ByteArray input process.. Dialect: F5 iRules.

Measured C object results do not establish an appliance/TMM protocol.

## Exact evidence

- `original-counted-probe` (input): [runtime/rust/tests/data/native_original_binary_replacement633/probe.c](../../../../runtime/rust/tests/data/native_original_binary_replacement633/probe.c). SHA-256 `27e1cca41100e9e7e074c5ef579dde973ecd943d611a3ad454317ef4e9beff2e`. Actual ByteArray constructors, seven owner entries for six unique objects plus one explicit subject reference, and original public snapshots before the result getter.
- `original-request` (input): [runtime/rust/tests/data/native_original_binary_replacement633/request.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/request.json). SHA-256 `de080f86e6626ade524cd69c57f216a3b35f7ffef046468b60ca84e9551ce26d`. Question and input requirements; no expected output is supplied.
- `actual-case-projection` (observation): [runtime/rust/tests/data/native_original_binary_replacement633/interpretation.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/interpretation.json). SHA-256 `a4cbec20e4b0f7e5dadc9cd66a0f630d787add63da6b0d8d51f7642fd13916e3`. All55 raw stream projections; process completion and command completion remain separate.
- `exact-input-map` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/pinned-input-map.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/pinned-input-map.json). SHA-256 `786e0911ab11840f7963969a2a95ece23499bda348cee1b3c8dabea450fa8d67`. All original selected compiler/library/source/header/CLI/input pins and content-addressed archive join.
- `exact-input-bytes` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/pinned-input-bytes.tar.gz](../../../../runtime/rust/tests/data/native_original_binary_replacement633/pinned-input-bytes.tar.gz). SHA-256 `db7adab418ea82b0b088e77014a6c08635e288fbbe34e643c6c646c6d7b2a987`. Whole retained original inputs, independently hash-verified after decompression; system toolchain/load closure remains outside the recorded scope.
- `exact-source-windows` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. Actual selected command/helper/setter source explains C8 Unicode conversion, C9 result identity and unspecified partial-deletion prefix.
- `all-original-retention` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/retention.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/retention.json). SHA-256 `70340122eec938cd76e9bc8399e105257dd179eede98c068e8b81f4e3ad2ede8`. Original receipts, raw streams and executable images retained without truncation.
- `tcl8.4-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/receipt.json). SHA-256 `b84f5a85d525e2b718ee541f0368e221ae1395a375375cffb6b7249941ed957a`. Actual original selected compile/link recipe, CLI version, input pins and fresh case processes.
- `tcl8.4-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/same-library-version/receipt.json). SHA-256 `7dba01d55a657b807c07a51e4fd9e220110554af8917d768869b44c6c2a5ea70`. Independent later diagnostic using the identical actual static library and headers; original case streams are unchanged.
- `tcl8.4-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/same-library-version/execute.stdout). SHA-256 `ef18d09813e9c05fbd397154e6f786ea0221f2ce8981597665e917a08a5c5d7f`. Direct selected-library version query and patchlevel output.
- `tcl8.5-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/receipt.json). SHA-256 `b0f8c0ecf368248601f3d325d8aefe6a675ae07dde97718b4160bdfc9daa0405`. Actual original selected compile/link recipe, CLI version, input pins and fresh case processes.
- `tcl8.5-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/same-library-version/receipt.json). SHA-256 `53d91a5ae0c26885b732ce3886fbf69185df28fae9c79f00bfb58b3bfbc2c00d`. Independent later diagnostic using the identical actual static library and headers; original case streams are unchanged.
- `tcl8.5-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/same-library-version/execute.stdout). SHA-256 `1bd253489464d3ed255f286efdc8deb3768100a5b873e0a062f5026291fcf9bb`. Direct selected-library version query and patchlevel output.
- `tcl8.6-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/receipt.json). SHA-256 `f2668326c95384e6dead2308af3caacebbc8568fb29213c55e69c78aa113306a`. Actual original selected compile/link recipe, CLI version, input pins and fresh case processes.
- `tcl8.6-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/same-library-version/receipt.json). SHA-256 `55d73feb60fb5c0f9b0e6810b76912aaaeec253853c8901e1ff99a2e6828de2a`. Independent later diagnostic using the identical actual static library and headers; original case streams are unchanged.
- `tcl8.6-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/same-library-version/execute.stdout). SHA-256 `dd12d5dc76bfc80ae3033ac87b1932541f275a888adf9e03dabbcb5878b0df8c`. Direct selected-library version query and patchlevel output.
- `tcl9.0-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/receipt.json). SHA-256 `671b6579e6e57cc92026636e3ee15912a503dc1e7d3607f40e2fc620e35843b1`. Actual original selected compile/link recipe, CLI version, input pins and fresh case processes.
- `tcl9.0-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/same-library-version/receipt.json). SHA-256 `366b3da86fb981d3d464e03d105fd39ebb2af46b79052a364efc62f08881f3a8`. Independent later diagnostic using the identical actual static library and headers; original case streams are unchanged.
- `tcl9.0-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. Direct selected-library version query and patchlevel output.
- `tcl9.1-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/receipt.json). SHA-256 `0a89aabed282018fdc6f2a8f2ca4a819782d1cd7c04b94a387c3a2fa43ee8806`. Actual original selected compile/link recipe, CLI version, input pins and fresh case processes.
- `tcl9.1-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/same-library-version/receipt.json). SHA-256 `82ca20463dc7905ecfe4b9118132afda6369dd033632b0273cd576270e957641`. Independent later diagnostic using the identical actual static library and headers; original case streams are unchanged.
- `tcl9.1-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. Direct selected-library version query and patchlevel output.
- `exact-source-snippet-0` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/0/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-1` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/1/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-2` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/2/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-3` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/3/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-4` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/4/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-5` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/5/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-6` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/6/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-7` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/7/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-8` (source-anchor): [runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json](../../../../runtime/rust/tests/data/native_original_binary_replacement633/source-windows.json). SHA-256 `27fa0042cee275af78ca3bbe6fd60178848df9e7f47b7a6ad3fb06116997c171`. JSON pointer `/anchors/8/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_StringObjCmd STR_REPLACE case`, lines 2189–2225. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `280cc8dfad4c37b40301812db3277fe1202ad56e9a08b28095ffd1663a92636a`; retained evidence `exact-source-snippet-0`.

```text
	case STR_REPLACE: {
	    Tcl_UniChar *ustring1;
	    int first, last;

	    if (objc < 5 || objc > 6) {
	        Tcl_WrongNumArgs(interp, 2, objv,
				 "string first last ?string?");
		return TCL_ERROR;
	    }

	    ustring1 = Tcl_GetUnicodeFromObj(objv[2], &length1);
	    length1--;

	    if ((TclGetIntForIndex(interp, objv[3], length1, &first) != TCL_OK)
		    || (TclGetIntForIndex(interp, objv[4], length1,
			    &last) != TCL_OK)) {
		return TCL_ERROR;
	    }

	    if ((last < first) || (last < 0) || (first > length1)) {
		Tcl_SetObjResult(interp, objv[2]);
	    } else {
		if (first < 0) {
		    first = 0;
		}

		Tcl_SetUnicodeObj(resultPtr, ustring1, first);
		if (objc == 6) {
		    Tcl_AppendObjToObj(resultPtr, objv[5]);
		}
		if (last < length1) {
		    Tcl_AppendUnicodeToObj(resultPtr, ustring1 + last + 1,
			    length1 - last);
		}
	    }
	    break;
	}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2258–2303. Full-source SHA-256 `fe2c67a2097ece2e07d9345a5edf416b5fbc891276349e360f9c25da1cc8f348`; snippet SHA-256 `12fc6487147f00d261fa80876bcbe6e2dd3e7dec79b835fd54d88c81818061f2`; retained evidence `exact-source-snippet-1`.

```text
StringRplcCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_UniChar *ustring;
    int first, last, length;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
    length--;

    if (TclGetIntForIndexM(interp, objv[2], length, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], length, &last) != TCL_OK){
	return TCL_ERROR;
    }

    if ((last < first) || (last < 0) || (first > length)) {
	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
	length--;

	if (first < 0) {
	    first = 0;
	}

	resultPtr = Tcl_NewUnicodeObj(ustring, first);
	if (objc == 5) {
	    Tcl_AppendObjToObj(resultPtr, objv[4]);
	}
	if (last < length) {
	    Tcl_AppendUnicodeToObj(resultPtr, ustring + last + 1,
		    length - last);
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2329–2392. Full-source SHA-256 `c2fe2deb95449c94c2719dca830d4e57ec4a436741199da39122d772888e849e`; snippet SHA-256 `68c46d9d79f432a239f39f7ebe513bd54bcf881e58ba3cb4b430e701a1f5a7a6`; retained evidence `exact-source-snippet-2`.

```text
StringRplcCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_UniChar *ustring;
    int first, last, length, end;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
    end = length - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK){
	return TCL_ERROR;
    }

    /*
     * The following test screens out most empty substrings as
     * candidates for replacement. When they are detected, no
     * replacement is done, and the result is the original string,
     */
    if ((last < 0) ||		/* Range ends before start of string */
	    (first > end) ||	/* Range begins after end of string */
	    (last < first)) {	/* Range begins after it starts */

	/*
	 * BUT!!! when (end < 0) -- an empty original string -- we can
	 * have (first <= end < 0 <= last) and an empty string is permitted
	 * to be replaced.
	 */
	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	/*
	 * We are re-fetching in case the string argument is same value as
	 * an index argument, and shimmering cost us our ustring.
	 */

	ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
	end = length-1;

	if (first < 0) {
	    first = 0;
	}

	resultPtr = Tcl_NewUnicodeObj(ustring, first);
	if (objc == 5) {
	    Tcl_AppendObjToObj(resultPtr, objv[4]);
	}
	if (last < end) {
	    Tcl_AppendUnicodeToObj(resultPtr, ustring + last + 1,
		    end - last);
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2392–2448. Full-source SHA-256 `080e201d1a09760aad4fedd9835e2091fd663d405d32d034f32dd0918d9006cc`; snippet SHA-256 `457c02a56bd84e0abbd36450af56eca3afc7d09443901f2945d0f384bc0812e5`; retained evidence `exact-source-snippet-3`.

```text
StringRplcCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * The following test screens out most empty substrings as candidates for
     * replacement. When they are detected, no replacement is done, and the
     * result is the original string.
     */

    if ((last < 0) ||	/* Range ends before start of string */
	    (first > end) ||	/* Range begins after end of string */
	    (last < first)) {	/* Range begins after it starts */
	/*
	 * BUT!!! when (end < 0) -- an empty original string -- we can
	 * have (first <= end < 0 <= last) and an empty string is permitted
	 * to be replaced.
	 */

	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	if (first < 0) {
	    first = TCL_INDEX_START;
	}
	if (last > end) {
	    last = end;
	}

	resultPtr = TclStringReplace(interp, objv[1], first,
		last + 1 - first, (objc == 5) ? objv[4] : NULL,
		TCL_STRING_IN_PLACE);

	if (resultPtr == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `TclStringReplace`, lines 4189–4304. Full-source SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`; snippet SHA-256 `042f4ccded53127fb70a28760d34fba835483c1d57d52f78c17fcf718bce92ea`; retained evidence `exact-source-snippet-4`.

```text
TclStringReplace(
    Tcl_Interp *interp,		/* For error reporting, may be NULL */
    Tcl_Obj *objPtr,		/* String to act upon */
    Tcl_Size first,		/* First index to replace */
    Tcl_Size count,		/* How many chars to replace */
    Tcl_Obj *insertPtr,		/* Replacement string, may be NULL */
    int flags)			/* TCL_STRING_IN_PLACE => attempt in-place */
{
    int inPlace = flags & TCL_STRING_IN_PLACE;
    Tcl_Obj *result;

    /* Replace nothing with nothing */
    if ((insertPtr == NULL) && (count <= 0)) {
	if (inPlace) {
	    return objPtr;
	} else {
	    return Tcl_DuplicateObj(objPtr);
	}
    }
    if (first < 0) {
	first = 0;
    }

    /*
     * The caller very likely had to call Tcl_GetCharLength() or similar
     * to be able to process index values.  This means it is likely that
     * objPtr is either a proper "byte-array" or a "string" or else it has
     * a known and short string rep.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size numBytes = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &numBytes);

	if (insertPtr == NULL) {
	    /* Replace something with nothing. */

	    assert ( first <= numBytes ) ;
	    assert ( count <= numBytes ) ;
	    assert ( first + count <= numBytes ) ;

	    result = Tcl_NewByteArrayObj(NULL, numBytes - count);/* PANIC? */
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Replace everything */
	if ((first == 0) && (count == numBytes)) {
	    return insertPtr;
	}

	if (TclIsPureByteArray(insertPtr)) {
	    Tcl_Size newBytes = 0;
	    unsigned char *iBytes
		    = Tcl_GetBytesFromObj(NULL, insertPtr, &newBytes);

	    if (count == newBytes && inPlace && !Tcl_IsShared(objPtr)) {
		/*
		 * Removal count and replacement count are equal.
		 * Other conditions permit. Do in-place splice.
		 */

		memcpy(bytes + first, iBytes, count);
		Tcl_InvalidateStringRep(objPtr);
		return objPtr;
	    }

	    if (newBytes > (TCL_SIZE_MAX - (numBytes - count))) {
		if (interp) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "max size for a Tcl value (%" TCL_SIZE_MODIFIER "d bytes) exceeded",
			    TCL_SIZE_MAX));
		    Tcl_SetErrorCode(interp, "TCL", "MEMORY", (char *)NULL);
		}
		return NULL;
	    }
	    result = Tcl_NewByteArrayObj(NULL, numBytes - count + newBytes);
								/* PANIC? */
	    Tcl_SetByteArrayLength(result, 0);
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, iBytes, newBytes);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Flow through to try other approaches below */
    }

    /*
     * TODO: Figure out how not to generate a Tcl_UniChar array rep
     * when it can be determined objPtr->bytes points to a string of
     * all single-byte characters so we can index it directly.
     */

    /* The traditional implementation... */
    {
	Tcl_Size numChars;
	Tcl_UniChar *ustring = Tcl_GetUnicodeFromObj(objPtr, &numChars);

	/* TODO: Is there an in-place option worth pursuing here? */

	result = Tcl_NewUnicodeObj(ustring, first);
	if (insertPtr) {
	    Tcl_AppendObjToObj(result, insertPtr);
	}
	if ((first + count) < numChars) {
	    Tcl_AppendUnicodeToObj(result, ustring + first + count,
		    numChars - first - count);
	}

	return result;
    }
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 315–341. Full-source SHA-256 `5660d9f82d02f1ca213a0e9e0b96ff959a81d6f6a474229fb7eada8edfde40bf`; snippet SHA-256 `27908deda03f1103aa76bf019a780f614b3c6b03b2cc654846a68346702a8991`; retained evidence `exact-source-snippet-5`.

```text
Tcl_SetByteArrayObj(
    Tcl_Obj *objPtr,		/* Object to initialize as a ByteArray. */
    const unsigned char *bytes,	/* The array of bytes to use as the new value.
				 * May be NULL even if numBytes > 0. */
    Tcl_Size numBytes)		/* Number of bytes in the array,
				 * must be >= 0 */
{
    ByteArray *byteArrayPtr;
    Tcl_ObjInternalRep ir;

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_SetByteArrayObj");
    }
    TclInvalidateStringRep(objPtr);

    assert(numBytes >= 0);
    byteArrayPtr = (ByteArray *)Tcl_Alloc(BYTEARRAY_SIZE(numBytes));
    byteArrayPtr->used = numBytes;
    byteArrayPtr->allocated = numBytes;

    if ((bytes != NULL) && (numBytes > 0)) {
	memcpy(byteArrayPtr->bytes, bytes, numBytes);
    }
    SET_BYTEARRAY(&ir, byteArrayPtr);

    Tcl_StoreInternalRep(objPtr, &properByteArrayType, &ir);
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2457–2513. Full-source SHA-256 `62be408d1faa29a35d3e30eacc4986fd9f1a973f7efcd474b3a2a9082ded512f`; snippet SHA-256 `d04d12c1fa0a543dc0ef66c7cbc4362119af47e18cd60e8ec72e44bdf13910cb`; retained evidence `exact-source-snippet-6`.

```text
StringRplcCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * The following test screens out most empty substrings as candidates for
     * replacement. When they are detected, no replacement is done, and the
     * result is the original string.
     */

    if ((last < 0) ||		/* Range ends before start of string */
	    (first > end) ||	/* Range begins after end of string */
	    (last < first)) {	/* Range begins after it starts */
	/*
	 * BUT!!! when (end < 0) -- an empty original string -- we can
	 * have (first <= end < 0 <= last) and an empty string is permitted
	 * to be replaced.
	 */

	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	if (first < 0) {
	    first = TCL_INDEX_START;
	}
	if (last > end) {
	    last = end;
	}

	resultPtr = TclStringReplace(interp, objv[1], first,
		last + 1 - first, (objc == 5) ? objv[4] : NULL,
		TCL_STRING_IN_PLACE);

	if (resultPtr == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `TclStringReplace`, lines 4197–4312. Full-source SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`; snippet SHA-256 `042f4ccded53127fb70a28760d34fba835483c1d57d52f78c17fcf718bce92ea`; retained evidence `exact-source-snippet-7`.

```text
TclStringReplace(
    Tcl_Interp *interp,		/* For error reporting, may be NULL */
    Tcl_Obj *objPtr,		/* String to act upon */
    Tcl_Size first,		/* First index to replace */
    Tcl_Size count,		/* How many chars to replace */
    Tcl_Obj *insertPtr,		/* Replacement string, may be NULL */
    int flags)			/* TCL_STRING_IN_PLACE => attempt in-place */
{
    int inPlace = flags & TCL_STRING_IN_PLACE;
    Tcl_Obj *result;

    /* Replace nothing with nothing */
    if ((insertPtr == NULL) && (count <= 0)) {
	if (inPlace) {
	    return objPtr;
	} else {
	    return Tcl_DuplicateObj(objPtr);
	}
    }
    if (first < 0) {
	first = 0;
    }

    /*
     * The caller very likely had to call Tcl_GetCharLength() or similar
     * to be able to process index values.  This means it is likely that
     * objPtr is either a proper "byte-array" or a "string" or else it has
     * a known and short string rep.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size numBytes = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &numBytes);

	if (insertPtr == NULL) {
	    /* Replace something with nothing. */

	    assert ( first <= numBytes ) ;
	    assert ( count <= numBytes ) ;
	    assert ( first + count <= numBytes ) ;

	    result = Tcl_NewByteArrayObj(NULL, numBytes - count);/* PANIC? */
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Replace everything */
	if ((first == 0) && (count == numBytes)) {
	    return insertPtr;
	}

	if (TclIsPureByteArray(insertPtr)) {
	    Tcl_Size newBytes = 0;
	    unsigned char *iBytes
		    = Tcl_GetBytesFromObj(NULL, insertPtr, &newBytes);

	    if (count == newBytes && inPlace && !Tcl_IsShared(objPtr)) {
		/*
		 * Removal count and replacement count are equal.
		 * Other conditions permit. Do in-place splice.
		 */

		memcpy(bytes + first, iBytes, count);
		Tcl_InvalidateStringRep(objPtr);
		return objPtr;
	    }

	    if (newBytes > (TCL_SIZE_MAX - (numBytes - count))) {
		if (interp) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "max size for a Tcl value (%" TCL_SIZE_MODIFIER "d bytes) exceeded",
			    TCL_SIZE_MAX));
		    Tcl_SetErrorCode(interp, "TCL", "MEMORY", (char *)NULL);
		}
		return NULL;
	    }
	    result = Tcl_NewByteArrayObj(NULL, numBytes - count + newBytes);
								/* PANIC? */
	    Tcl_SetByteArrayLength(result, 0);
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, iBytes, newBytes);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Flow through to try other approaches below */
    }

    /*
     * TODO: Figure out how not to generate a Tcl_UniChar array rep
     * when it can be determined objPtr->bytes points to a string of
     * all single-byte characters so we can index it directly.
     */

    /* The traditional implementation... */
    {
	Tcl_Size numChars;
	Tcl_UniChar *ustring = Tcl_GetUnicodeFromObj(objPtr, &numChars);

	/* TODO: Is there an in-place option worth pursuing here? */

	result = Tcl_NewUnicodeObj(ustring, first);
	if (insertPtr) {
	    Tcl_AppendObjToObj(result, insertPtr);
	}
	if ((first + count) < numChars) {
	    Tcl_AppendUnicodeToObj(result, ustring + first + count,
		    numChars - first - count);
	}

	return result;
    }
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 317–343. Full-source SHA-256 `2c4d8facc256ec7e135164f92119d4dcf5f98fc3369c0b35a1e89ba950d61b7a`; snippet SHA-256 `27908deda03f1103aa76bf019a780f614b3c6b03b2cc654846a68346702a8991`; retained evidence `exact-source-snippet-8`.

```text
Tcl_SetByteArrayObj(
    Tcl_Obj *objPtr,		/* Object to initialize as a ByteArray. */
    const unsigned char *bytes,	/* The array of bytes to use as the new value.
				 * May be NULL even if numBytes > 0. */
    Tcl_Size numBytes)		/* Number of bytes in the array,
				 * must be >= 0 */
{
    ByteArray *byteArrayPtr;
    Tcl_ObjInternalRep ir;

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_SetByteArrayObj");
    }
    TclInvalidateStringRep(objPtr);

    assert(numBytes >= 0);
    byteArrayPtr = (ByteArray *)Tcl_Alloc(BYTEARRAY_SIZE(numBytes));
    byteArrayPtr->used = numBytes;
    byteArrayPtr->allocated = numBytes;

    if ((bytes != NULL) && (numBytes > 0)) {
	memcpy(byteArrayPtr->bytes, bytes, numBytes);
    }
    SET_BYTEARRAY(&ir, byteArrayPtr);

    Tcl_StoreInternalRep(objPtr, &properByteArrayType, &ir);
}

```


## Consumer bindings

- [rust/tcl-vm/src/value_ops/native_jim_index.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_index.rs), `original_indexed_binary_inputs_match_supported_native_public_projections` (linked): Software source definition checks indexed binary inputs match supported native public projections.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `original_indexed_binary_inputs_match_supported_native_public_projections` (linked): Software source definition checks indexed binary inputs match supported native public projections.

A named test is a coverage binding, not a claim that it executed.

## Replay

Use the unchanged counted-object probe and all11 inputs. Each provider receipt records its exact compile/link/launch vector and environment; each input runs in a fresh process. Verify the original selected library/header bytes or treat rebuilt bytes as an independent experiment. Do not copy the partial-deletion leading byte observation into a deterministic software expectation. Direct helper sharing is a distinct execution purpose.
