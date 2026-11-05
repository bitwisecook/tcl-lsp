# naming.source.original-counted-filename-filesystem-extent

Kind: `native-observation`

## Problem statement

Source filenames have separate original counted String bytes, external filesystem bytes and diagnostic C-string extents. Conflating those extents, borrowing source script codecs or forgiving strict conversion units changes the selected file and winning result.

## Question

How do the original C and Jim source commands convert counted filename bytes, select real byte-named files and report literal/modified-zero and surrogate-unit failures under actual UTF-8, ISO-8859-1 and identity system encoding selections? What exact results, errorCode, effects, selection results and original headers remain?

## Conclusion

The observed Unix C8.4 worker applies C-string extent after actual external conversion, while C8.5 and later refuse converted external zero with EINVAL; reporting retains original String C-string extent. UTF-8 maps resident raw FF to C3 BF. ISO-8859-1 selects raw E9/FF and substitutes question mark for euro on C8, while C9 reports strict EILSEQ. Named identity copies bytes on C8 but is unknown on the measured C9 catalogue. C8.4/C8.5 preserve supplied CESU pairs; C8.6 combines them to four-byte UTF-8; C9 rejects both unpaired units and supplied CESU pairs with EILSEQ. Jim independently supplies raw C-string filenames and preserves seeded errorCode on its observed failures. Real byte-named file effects establish the selected backend. Every reached source operation preserves its original counted header.

## Scope

Eighteen separately compiled original public-object probe processes provide 193 fresh initialized interpreter blocks: filename-extent controls contribute36 (nine source successes,27 Guest errors), filesystem-codec controls contribute133 (60 source successes,52 source Guest errors,21 failed selections that never call source), and strict-UTF8 controls contribute24 (ten source successes,14 Guest errors). There are172 actual source operations and21 selection-only failures. Each collection retains six CLI and six independent same-static-library version diagnostics, exact full input pins, original streams/receipts/probes/runners and lossless driver/version executable bytes. The selected Unix source and output-conversion windows join whole input payloads. Invalid OS byte filenames are retained through raw path hex or lossless JSON. Query result state, operation RETURN and later observers remain distinct. These finite cases establish no arbitrary codec or Windows behavior, callback/updater/free chronology, Native sharing/reference neutrality, same-owner encoding-setting mutation timing, complete historical toolchain/dynamic load closure, source read/configuration precedence, Rust assertion/build result or BIG-IP outcome.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer. Version: tcl8.4 8.4.20.

Filename extents: Under the explicitly selected UTF-8 system encoding, plain existing, literal-zero-tail existing and modified-zero-tail existing all execute the exact file and return SOURCE_OK. All three absent variants report POSIX ENOENT. The literal-zero diagnostic ends at the original zero; modified-zero C080 plus TAIL remains in the original diagnostic even though external conversion terminates filesystem lookup. Success preserves the seeded errorCode and all original path headers remain byte-exact.

Filesystem codecs: The actual UTF-8 selection maps internal é to the UTF-8 file, resident raw FF to C3 BF rather than replacement, and euro to its UTF-8 file. ISO-8859-1 selects raw E9/FF files and substitutes question mark for euro, selecting QUESTION_EXTERNAL. Identity selects the original byte files without Unicode decoding. Literal zero truncates all three backend codecs; modified zero truncates after UTF-8/ISO conversion but remains distinct absent bytes with identity. Ordinary absent names report POSIX ENOENT. Source success preserves seeded errorCode; original counted headers remain exact. Unknown selection fails and source is not called.

Strict UTF-8 filenames: ASCII executes its file. The supplied CESU pair selects the original CESU byte-named file, not the four-byte UTF-8 file. Both unpaired units remain distinct absent external names and report POSIX ENOENT. Success preserves the seeded errorCode and every original path header remains exact.

### tcl8.5

Status: `observed`. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer. Version: tcl8.5 8.5.19.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. All four literal/modified-zero existing/absent variants report POSIX EINVAL without the source effect. Literal zero truncates the original diagnostic while modified C080 plus TAIL remains visible. Original counted path headers remain byte-exact.

Filesystem codecs: UTF-8 selects the é, C3 BF and euro UTF-8 files. ISO-8859-1 selects raw E9/FF and QUESTION_EXTERNAL for euro. Identity is available and selects raw original bytes. Literal external zero refuses with POSIX EINVAL; modified zero refuses under UTF-8/ISO but remains distinct absent bytes under identity and reports ENOENT. Unknown selection fails before source. Source success preserves seeded errorCode and every reached source header is exact afterward.

Strict UTF-8 filenames: ASCII executes its file. The supplied pair selects PAIR_CESU rather than PAIR_UTF8; both unpaired original units report POSIX ENOENT on the guaranteed absent names. Every original counted header remains exact and success preserves seeded errorCode.

### tcl8.6

Status: `observed`. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer. Version: tcl8.6 8.6.18.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. All four literal/modified-zero existing/absent variants report POSIX EINVAL without the source effect. Literal zero truncates the diagnostic and modified C080 plus TAIL is preserved. Original counted path headers remain byte-exact.

Filesystem codecs: UTF-8 selects é, C3 BF for raw FF, and euro UTF-8 files. ISO-8859-1 selects raw E9/FF and substitutes question mark for euro. Identity selects original byte names. Literal external zero and converted modified zero refuse with POSIX EINVAL; identity modified C080 remains an absent distinct path and reports ENOENT. Unknown selection fails without a source call. Reached source path headers remain exact and success preserves seeded errorCode.

Strict UTF-8 filenames: ASCII executes its file. The supplied CESU pair combines to the real four-byte external UTF-8 filename and selects PAIR_UTF8. Both unpaired units remain absent names and report POSIX ENOENT. Original counted headers remain exact and success preserves seeded errorCode.

### tcl9.0

Status: `observed`. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer. Version: tcl9.0 9.0.4.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. All four literal/modified-zero variants report POSIX EINVAL before any source effect; diagnostics use the original String CString extent, not the converted backend extent. Every original counted path header remains byte-exact.

Filesystem codecs: UTF-8 selects é, raw-FF-as-C3-BF and euro UTF-8 files. ISO-8859-1 selects raw E9/FF, but strict euro conversion reports POSIX EILSEQ with no source effect. Converted zero refuses with POSIX EINVAL. All eight identity selections are unknown and never call source, as does the separate unknown codec selection. Source success preserves seeded errorCode and all reached source headers remain exact.

Strict UTF-8 filenames: ASCII executes its file. Unpaired high, unpaired low and the supplied CESU pair each report POSIX EILSEQ with the original filename diagnostic and no source effect. Neither pair file is executed. All original path headers remain exact; ASCII success preserves seeded errorCode.

### tcl9.1

Status: `observed`. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer. Version: tcl9.1 9.1.0.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. Literal and modified zero variants each report POSIX EINVAL without source effects, retaining the distinct original diagnostic extents. Every original counted path header remains byte-exact.

Filesystem codecs: UTF-8 selects é, raw-FF-as-C3-BF and euro UTF-8 files. ISO-8859-1 selects raw E9/FF but euro reports strict POSIX EILSEQ without source effects. Literal and modified external zero report POSIX EINVAL. Named identity is unavailable: eight failed selections never call source; the separate unknown selection also never calls source. Reached source headers remain exact and success preserves seeded errorCode.

Strict UTF-8 filenames: ASCII executes its file. Each unpaired unit and the supplied CESU pair fail strict conversion with POSIX EILSEQ before any source effect. Both actual pair files remain unexecuted. Every original counted header remains exact and ASCII success preserves seeded errorCode.

### jim

Status: `observed`. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer. Version: jim 0.84-9-g5bac7c9.

Filename extents: Plain existing and literal-zero-tail existing run the file and return SOURCE_OK. Plain absent, literal-zero-tail absent and both modified-zero variants report the original capitalized No such file or directory and preserve the seeded errorCode. The raw modified-zero filename remains a different path; Jim does not use C external filename conversion. Original path headers remain byte-exact.

Filesystem codecs: The independent raw-source worker has no encoding command or C system selection. It selects original UTF-8 é/euro bytes and raw FF. Literal zero truncates to the ASCII prefix; modified C080 remains a distinct absent path. Ordinary absent cases and modified-zero failure retain the original capitalized No such file or directory and seeded errorCode. All eight reached source headers remain exact.

Strict UTF-8 filenames: The independent raw-source worker executes ASCII and the raw CESU pair file. Both unpaired raw names are absent and report capitalized No such file or directory while preserving seeded errorCode. No C UTF-8 system converter is used, and all four original headers remain exact.

### bigip

Status: `not-tested`. Channel: No appliance filename input executed.

No original BIG-IP filename or filesystem codec observation is available.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIOUtil.c`, function `NativeCreateNativeRep`, lines 4024–4057. Line unit: physical LF only. Original byte range [121309, 122218). Retained reference `source-window-0`.

```text
static ClientData 
NativeCreateNativeRep(pathObjPtr)
    Tcl_Obj* pathObjPtr;
{
    char *nativePathPtr;
    Tcl_DString ds;
    Tcl_Obj* validPathObjPtr;
    int len;
    char *str;

    /* Make sure the normalized path is set */
    validPathObjPtr = Tcl_FSGetNormalizedPath(NULL, pathObjPtr);
    if (validPathObjPtr == NULL) {
	return NULL;
    }

    str = Tcl_GetStringFromObj(validPathObjPtr, &len);
#ifdef __WIN32__
    Tcl_WinUtfToTChar(str, len, &ds);
    if (tclWinProcs->useWide) {
	len = Tcl_DStringLength(&ds) + sizeof(WCHAR);
    } else {
	len = Tcl_DStringLength(&ds) + sizeof(char);
    }
#else
    Tcl_UtfToExternalDString(NULL, str, len, &ds);
    len = Tcl_DStringLength(&ds) + sizeof(char);
#endif
    nativePathPtr = ckalloc((unsigned) len);
    memcpy((VOID*)nativePathPtr, (VOID*)Tcl_DStringValue(&ds), (size_t) len);

    Tcl_DStringFree(&ds);
    return (ClientData)nativePathPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIOUtil.c`, function `Tcl_FSEvalFile`, lines 1703–1810. Line unit: physical LF only. Original byte range [52363, 55548). Retained reference `source-window-1`.

```text
int
Tcl_FSEvalFile(interp, pathPtr)
    Tcl_Interp *interp;		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr;		/* Path of file to process.  Tilde-substitution
				 * will be performed on this name. */
{
    int result, length;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    result = TCL_ERROR;
    objPtr = Tcl_NewObj();
    Tcl_IncrRefCount(objPtr);

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
        Tcl_SetErrno(errno);
	Tcl_AppendResult(interp, "couldn't read file \"", 
		Tcl_GetString(pathPtr),
		"\": ", Tcl_PosixError(interp), (char *) NULL);
	goto end;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == (Tcl_Channel) NULL) {
        Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, "couldn't read file \"", 
		Tcl_GetString(pathPtr),
		"\": ", Tcl_PosixError(interp), (char *) NULL);
	goto end;
    }
    /*
     * The eofchar is \32 (^Z).  This is the usual on Windows, but we
     * effect this cross-platform to allow for scripted documents.
     * [Bug: 2040]
     */
    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32");
    /* Try to read first character of stream, so we can
     * check for utf-8 BOM to be handled especially.
     */
    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"", 
		Tcl_GetString(pathPtr),	"\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }
    string = Tcl_GetString(objPtr);
    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them [Bug 3466099].
     */
    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xef\xbb\xbf", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }
    if (Tcl_Close(interp, chan) != TCL_OK) {
        goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = Tcl_GetStringFromObj(objPtr, &length);

#ifdef TCL_TIP280
    /* TIP #280 Force the evaluator to open a frame for a sourced
     * file. */
    iPtr->evalFlags |= TCL_EVAL_FILE;
#endif
    result = Tcl_EvalEx(interp, string, length, 0);
    /* 
     * Now we have to be careful; the script may have changed the
     * iPtr->scriptFile value, so we must reset it without
     * assuming it still points to 'pathPtr'.
     */
    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	char msg[200 + TCL_INTEGER_SPACE];

	/*
	 * Record information telling where the error occurred.
	 */

	sprintf(msg, "\n    (file \"%.150s\" line %d)", Tcl_GetString(pathPtr),
		interp->errorLine);
	Tcl_AddErrorInfo(interp, msg);
    }

    end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1081–1129. Line unit: physical LF only. Original byte range [26796, 28080). Retained reference `source-window-2`.

```text
ClientData
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    int len;
    char *str;

    if (TclFSCwdIsNative()) {
	/*
	 * The cwd is native, which means we can use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = Tcl_GetStringFromObj(validPathPtr, &len);
    Tcl_UtfToExternalDString(NULL, str, len, &ds);
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = ckalloc((unsigned) len);
    memcpy((void*)nativePathPtr, (void*)Tcl_DStringValue(&ds), (size_t) len);

    Tcl_DStringFree(&ds);
    return (ClientData)nativePathPtr;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1769–1892. Line unit: physical LF only. Original byte range [50140, 53829). Retained reference `source-window-3`.

```text
int
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr,		/* Path of file to process. Tilde-substitution
				 * will be performed on this name. */
    const char *encodingName)	/* If non-NULL, then use this encoding for the
				 * file. NULL means use the system encoding. */
{
    int length, result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == (Tcl_Channel) NULL) {
	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	return result;
    }

    /*
     * The eofchar is \32 (^Z). This is the usual on Windows, but we effect
     * this cross-platform to allow for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32");

    /*
     * If the encoding is specified, set it for the channel. Else don't touch
     * it (and use the system encoding) Report error on unknown encoding.
     */

    if (encodingName != NULL) {
	if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
		!= TCL_OK) {
	    Tcl_Close(interp,chan);
	    return result;
	}
    }

    objPtr = Tcl_NewObj();
    Tcl_IncrRefCount(objPtr);
    /* Try to read first character of stream, so we can
     * check for utf-8 BOM to be handled especially.
     */
    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }
    string = Tcl_GetString(objPtr);
    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them [Bug 3466099].
     */
    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xef\xbb\xbf", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = Tcl_GetStringFromObj(objPtr, &length);
    /* TIP #280 Force the evaluator to open a frame for a sourced
     * file. */
    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = Tcl_EvalEx(interp, string, length, 0);

    /*
     * Now we have to be careful; the script may have changed the
     * iPtr->scriptFile value, so we must reset it without assuming it still
     * points to 'pathPtr'.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information telling where the error occurred.
	 */

	const char *pathString = Tcl_GetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : length), pathString,
		(overflow ? "..." : ""), interp->errorLine));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1069–1117. Line unit: physical LF only. Original byte range [26403, 27705). Retained reference `source-window-4`.

```text
void *
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    const char *str;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    int len;

    if (TclFSCwdIsNative() || Tcl_FSGetPathType(pathPtr) == TCL_PATH_ABSOLUTE) {
	/*
	 * The cwd is native (or path is absolute), use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = Tcl_GetStringFromObj(validPathPtr, &len);
    Tcl_UtfToExternalDString(NULL, str, len, &ds);
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = (char *)ckalloc(len);
    memcpy(nativePathPtr, Tcl_DStringValue(&ds), len);

    Tcl_DStringFree(&ds);
    return nativePathPtr;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIOUtil.c`, function `TclNREvalFile`, lines 1862–1966. Line unit: physical LF only. Original byte range [52139, 55130). Retained reference `source-window-5`.

```text
int
TclNREvalFile(
    Tcl_Interp *interp,		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr,		/* Path of file to process. Tilde-substitution
				 * will be performed on this name. */
    const char *encodingName)	/* If non-NULL, then use this encoding for the
				 * file. NULL means use the system encoding. */
{
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile, *objPtr;
    Interp *iPtr;
    Tcl_Channel chan;
    const char *string;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }

    /*
     * The eofchar is \32 (^Z). This is the usual on Windows, but we effect
     * this cross-platform to allow for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32 {}");

    /*
     * If the encoding is specified, set it for the channel. Else don't touch
     * it (and use the system encoding) Report error on unknown encoding.
     */

    if (encodingName != NULL) {
	if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
		!= TCL_OK) {
	    Tcl_Close(interp,chan);
	    return TCL_ERROR;
	}
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Try to read first character of stream, so we can check for utf-8 BOM to
     * be handled especially.
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xEF\xBB\xBF", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);

    /*
     * TIP #280: Force the evaluator to open a frame for a sourced file.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    TclNRAddCallback(interp, EvalFileCallback, oldScriptFile, pathPtr, objPtr,
	    NULL);
    return TclNREvalObjEx(interp, objPtr, 0, NULL, INT_MIN);
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1141–1193. Line unit: physical LF only. Original byte range [28566, 29983). Retained reference `source-window-6`.

```text
void *
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    const char *str;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    Tcl_Size len;

    if (TclFSCwdIsNative() || Tcl_FSGetPathType(pathPtr) == TCL_PATH_ABSOLUTE) {
	/*
	 * The cwd is native (or path is absolute), use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = TclGetStringFromObj(validPathPtr, &len);
    if (Tcl_UtfToExternalDStringEx(NULL, NULL, str, len, 0, &ds, NULL) != TCL_OK) {
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = (char *)Tcl_Alloc(len);
    memcpy(nativePathPtr, Tcl_DStringValue(&ds), len);

    Tcl_DStringFree(&ds);
    return nativePathPtr;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIOUtil.c`, function `TclNREvalFile`, lines 1828–1934. Line unit: physical LF only. Original byte range [48128, 51181). Retained reference `source-window-7`.

```text
int
TclNREvalFile(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the script. */
    Tcl_Obj *pathPtr,		/* Pathname of a file containing the script to
				 * evaluate. Tilde-substitution is performed on
				 * this pathname. */
    const char *encodingName)	/* The name of an encoding to use, or NULL to
				 *  use the utf-8 encoding. */
{
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile, *objPtr;
    Interp *iPtr;
    Tcl_Channel chan;
    const char *string;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    TclPkgFileSeen(interp, TclGetString(pathPtr));

    /*
     * The eof character is \x1A (^Z). Tcl uses it on every platform to allow
     * for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\x1A");

    /*
     * If the encoding is specified, set the channel to that encoding.
     * Otherwise use utf-8.  If the encoding is unknown report an error.
     */

    if (encodingName == NULL) {
	encodingName = "utf-8";
    }
    if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
	    != TCL_OK) {
	Tcl_CloseEx(interp, chan, 0);
	return TCL_ERROR;
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Read first character of stream to check for utf-8 BOM
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters.
     * Otherwise, replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, TCL_INDEX_NONE,
	    memcmp(string, "\xEF\xBB\xBF", 3)) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    TclNRAddCallback(interp, EvalFileCallback, oldScriptFile, pathPtr, objPtr,
	    NULL);
    return TclNREvalObjEx(interp, objPtr, 0, NULL, INT_MIN);
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1143–1195. Line unit: physical LF only. Original byte range [28657, 30074). Retained reference `source-window-8`.

```text
void *
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    const char *str;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    Tcl_Size len;

    if (TclFSCwdIsNative() || Tcl_FSGetPathType(pathPtr) == TCL_PATH_ABSOLUTE) {
	/*
	 * The cwd is native (or path is absolute), use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = TclGetStringFromObj(validPathPtr, &len);
    if (Tcl_UtfToExternalDStringEx(NULL, NULL, str, len, 0, &ds, NULL) != TCL_OK) {
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = (char *)Tcl_Alloc(len);
    memcpy(nativePathPtr, Tcl_DStringValue(&ds), len);

    Tcl_DStringFree(&ds);
    return nativePathPtr;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIOUtil.c`, function `TclNREvalFile`, lines 1827–1933. Line unit: physical LF only. Original byte range [48199, 51251). Retained reference `source-window-9`.

```text
int
TclNREvalFile(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the script. */
    Tcl_Obj *pathPtr,		/* Pathname of a file containing the script to
				 * evaluate. Tilde-substitution is performed on
				 * this pathname. */
    const char *encodingName)	/* The name of an encoding to use, or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile, *objPtr;
    Interp *iPtr;
    Tcl_Channel chan;
    const char *string;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    TclPkgFileSeen(interp, TclGetString(pathPtr));

    /*
     * The eof character is \x1A (^Z). Tcl uses it on every platform to allow
     * for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\x1A");

    /*
     * If the encoding is specified, set the channel to that encoding.
     * Otherwise use utf-8.  If the encoding is unknown report an error.
     */

    if (encodingName == NULL) {
	encodingName = "utf-8";
    }
    if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
	    != TCL_OK) {
	Tcl_CloseEx(interp, chan, 0);
	return TCL_ERROR;
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Read first character of stream to check for utf-8 BOM
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters.
     * Otherwise, replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, TCL_INDEX_NONE,
	    memcmp(string, "\xEF\xBB\xBF", 3)) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    TclNRAddCallback(interp, EvalFileCallback, oldScriptFile, pathPtr, objPtr,
	    NULL);
    return TclNREvalObjEx(interp, objPtr, 0, NULL, INT_MIN);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SourceCoreCommand`, lines 16711–16716. Line unit: physical LF only. Original byte range [529211, 529412). Retained reference `source-window-10`.

```text
static int Jim_SourceCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retval = Jim_EvalFile(interp, Jim_String(argv[1]));

    return retval == JIM_RETURN ? JIM_OK : retval;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalFile`, lines 12120–12168. Line unit: physical LF only. Original byte range [384075, 385640). Retained reference `source-window-11`.

```text
int Jim_EvalFile(Jim_Interp *interp, const char *filename)
{
    FILE *fp;
    char *buf;
    Jim_Obj *scriptObjPtr;
    Jim_Obj *filenameObj, *oldFilenameObj;
    int retcode = JIM_ERR;
    int readlen;
#define READ_BUF_SIZE 256

   if ((fp = fopen(filename, "rt")) == NULL) {
        Jim_SetResultFormatted(interp, "couldn't read file \"%s\": %s", filename, strerror(errno));
        return JIM_ERR;
    }
    scriptObjPtr = Jim_NewStringObj(interp, NULL, 0);

    buf = Jim_Alloc(READ_BUF_SIZE);
    while ((readlen = fread(buf, 1, READ_BUF_SIZE, fp)) > 0) {
        Jim_AppendString(interp, scriptObjPtr, buf, readlen);
    }
    Jim_Free(buf);
    if (ferror(fp)) {
        fclose(fp);
        Jim_SetResultFormatted(interp, "failed to load file \"%s\": %s", filename, strerror(errno));
        Jim_FreeNewObj(interp, scriptObjPtr);
        return retcode;
    }
    fclose(fp);

    /* Convert the stringObjType to a sourceObjType with filename and line */
    filenameObj = Jim_NewStringObj(interp, filename, -1);
    Jim_SetSourceInfo(interp, scriptObjPtr, filenameObj, 1);
    oldFilenameObj = JimPushInterpObj(interp->currentFilenameObj, filenameObj);

    retcode = Jim_EvalObj(interp, scriptObjPtr);

    JimPopInterpObj(interp, interp->currentFilenameObj, oldFilenameObj);

    /* Handle the JIM_RETURN return code */
    if (retcode == JIM_RETURN) {
        if (--interp->returnLevel <= 0) {
            retcode = interp->returnCode;
            interp->returnCode = JIM_OK;
            interp->returnLevel = 0;
        }
    }

    return retcode;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `BinaryProc`, lines 1846–1888. Line unit: physical LF only. Original byte range [55178, 56671). Retained reference `source-window-12`.

```text
static int
BinaryProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	srcReadPtr, dstWrotePtr, dstCharsPtr)
    ClientData clientData;	/* Not used. */
    CONST char *src;		/* Source string (unknown encoding). */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy((void *) dst, (void *) src, (size_t) srcLen);
    return result;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `TableFromUtfProc`, lines 2412–2516. Line unit: physical LF only. Original byte range [72899, 76015). Retained reference `source-window-13`.

```text
static int 
TableFromUtfProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	srcReadPtr, dstWrotePtr, dstCharsPtr)
    ClientData clientData;	/* TableEncodingData that specifies
				 * encoding. */
    CONST char *src;		/* Source string in UTF-8. */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted.  This
				 * may be less than the original source length
				 * if there was a problem converting some
				 * source characters. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    CONST char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd, *prefixBytes;
    Tcl_UniChar ch;
    int result, len, word, numChars;
    TableEncodingData *dataPtr;
    unsigned short **fromUnicode;
    
    result = TCL_OK;    

    dataPtr = (TableEncodingData *) clientData;
    prefixBytes = dataPtr->prefixBytes;
    fromUnicode = dataPtr->fromUnicode;
    
    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

#if TCL_UTF_MAX > 3
	/*
	 * This prevents a crash condition.  More evaluation is required
	 * for full support of int Tcl_UniChar. [Bug 1004065]
	 */
	if (ch & 0xffff0000) {
	    word = 0;
	} else
#endif
	    word = fromUnicode[(ch >> 8)][ch & 0xff];

	if ((word == 0) && (ch != 0)) {
	    if (flags & TCL_ENCODING_STOPONERROR) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
	    word = dataPtr->fallback; 
	}
	if (prefixBytes[(word >> 8)] != 0) {
	    if (dst + 1 > dstEnd) {
		result = TCL_CONVERT_NOSPACE;
		break;
	    }
	    dst[0] = (char) (word >> 8);
	    dst[1] = (char) word;
	    dst += 2;
	} else {
	    if (dst > dstEnd) {
		result = TCL_CONVERT_NOSPACE;
		break;
	    }
	    dst[0] = (char) word;
	    dst++;
	} 
	src += len;
    }
    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `BinaryProc`, lines 2059–2100. Line unit: physical LF only. Original byte range [60808, 62189). Retained reference `source-window-14`.

```text
static int
BinaryProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, (size_t) srcLen);
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 2821–2908. Line unit: physical LF only. Original byte range [83361, 85745). Retained reference `source-window-15`.

```text
static int
Iso88591FromUtfProc(
    ClientData clientData,	/* Ignored. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;

    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	Tcl_UniChar ch;
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xff) {
	    if (flags & TCL_ENCODING_STOPONERROR) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }

	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = (Tcl_UniChar) '?';
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `BinaryProc`, lines 2113–2157. Line unit: physical LF only. Original byte range [61425, 62897). Retained reference `source-window-16`.

```text
static int
BinaryProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if ((flags & TCL_ENCODING_CHAR_LIMIT) && srcLen > *dstCharsPtr) {
	srcLen = *dstCharsPtr;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, srcLen);
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 3006–3099. Line unit: physical LF only. Original byte range [88094, 90629). Retained reference `source-window-17`.

```text
static int
Iso88591FromUtfProc(
    ClientData clientData,	/* Ignored. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result = TCL_OK, numChars;
    Tcl_UniChar ch = 0;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xFF
#if TCL_UTF_MAX == 4
		|| ((ch >= 0xD800) && (len < 3))
#endif
		) {
	    if (flags & TCL_ENCODING_STOPONERROR) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
#if TCL_UTF_MAX == 4
	    if ((ch >= 0xD800) && (len < 3)) {
		len = 4;
	    }
#endif
	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = (Tcl_UniChar) '?';
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `BinaryProc`, lines 2380–2420. Line unit: physical LF only. Original byte range [71239, 72466). Retained reference `source-window-18`.

```text
static int
BinaryProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if ((flags & TCL_ENCODING_CHAR_LIMIT) && srcLen > *dstCharsPtr) {
	srcLen = *dstCharsPtr;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, srcLen);
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 3768–3848. Line unit: physical LF only. Original byte range [111142, 113288). Retained reference `source-window-19`.

```text
static int
Iso88591FromUtfProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result = TCL_OK, numChars;
    Tcl_UniChar ch = 0;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xFF) {
	    if (PROFILE_STRICT(flags)) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = '?'; /* Profiles TCL8 and REPLACE */
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `BinaryProc`, lines 2922–2962. Line unit: physical LF only. Original byte range [90136, 91363). Retained reference `source-window-20`.

```text
static int
BinaryProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if ((flags & TCL_ENCODING_CHAR_LIMIT) && srcLen > *dstCharsPtr) {
	srcLen = *dstCharsPtr;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, srcLen);
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 4573–4653. Line unit: physical LF only. Original byte range [136492, 138638). Retained reference `source-window-21`.

```text
static int
Iso88591FromUtfProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result = TCL_OK, numChars;
    Tcl_UniChar ch = 0;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xFF) {
	    if (PROFILE_STRICT(flags)) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = '?'; /* Profiles TCL8 and REPLACE */
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `TclInitEncodingSubsystem`, lines 346–386. Line unit: physical LF only. Original byte range [12276, 13438). Retained reference `source-window-22`.

```text
void
TclInitEncodingSubsystem()
{
    Tcl_EncodingType type;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);
    
    /*
     * Create a few initial encodings.  Note that the UTF-8 to UTF-8 
     * translation is not a no-op, because it will turn a stream of
     * improperly formed UTF-8 into a properly formed stream.
     */

    type.encodingName	= "identity";
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;

    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfExtToUtfIntProc;
    type.fromUtfProc	= UtfIntToUtfExtProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.encodingName   = "unicode";
    type.toUtfProc	= UnicodeToUtfProc;
    type.fromUtfProc    = UtfToUnicodeProc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `TclInitEncodingSubsystem`, lines 523–659. Line unit: physical LF only. Original byte range [16674, 20842). Retained reference `source-window-23`.

```text
void
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;
    TableEncodingData *dataPtr;
    unsigned size;
    unsigned short i;
    union {
	char c;
	short s;
    } isLe;
    int leFlags;

    if (encodingsInitialized) {
	return;
    }

    /* Note: This DEPENDS on TCL_ENCODING_LE being defined in least sig byte */
    isLe.s = 1;
    leFlags = isLe.c ? TCL_ENCODING_LE : 0;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= NULL;
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfToUtfProc;
    type.fromUtfProc	= UtfToUtfProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= INT2PTR(ENCODING_UTF);
    tclUtf8Encoding = Tcl_CreateEncoding(&type);
    type.clientData	= NULL;
    type.encodingName	= "cesu-8";
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc	= UtfToUcs2Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "ucs-2le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf32ToUtfProc;
    type.fromUtfProc	= UtfToUtf32Proc;
    type.freeProc	= NULL;
    type.nullSize	= 4;
    type.encodingName	= "utf-32le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc    = UtfToUtf16Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "utf-16le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

#ifndef TCL_NO_DEPRECATED
    type.encodingName	= "unicode";
    Tcl_CreateEncoding(&type);
#endif

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)Tcl_Alloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->fromUnicode, 0, size);

    dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
    dataPtr->fromUnicode[0] = (unsigned short *) (dataPtr->fromUnicode + 256);
    for (i=1 ; i<256 ; i++) {
	dataPtr->toUnicode[i] = emptyPage;
	dataPtr->fromUnicode[i] = emptyPage;
    }

    for (i=0 ; i<256 ; i++) {
	dataPtr->toUnicode[0][i] = i;
	dataPtr->fromUnicode[0][i] = i;
    }

    type.encodingName	= "iso8859-1";
    type.toUtfProc	= Iso88591ToUtfProc;
    type.fromUtfProc	= Iso88591FromUtfProc;
    type.freeProc	= TableFreeProc;
    type.nullSize	= 1;
    type.clientData	= dataPtr;
    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    encodingsInitialized = 1;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `TclInitEncodingSubsystem`, lines 526–667. Line unit: physical LF only. Original byte range [16862, 21139). Retained reference `source-window-24`.

```text
void
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;
    TableEncodingData *dataPtr;
    unsigned size;
    unsigned short i;
    union {
	char c;
	short s;
    } isLe;
    int leFlags;

    if (encodingsInitialized) {
	return;
    }

    /* Note: This DEPENDS on TCL_ENCODING_LE being defined in least sig byte */
    isLe.s = 1;
    leFlags = isLe.c ? TCL_ENCODING_LE : 0;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= NULL;
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfToUtfProc;
    type.fromUtfProc	= UtfToUtfProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclUtf8Encoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "cesu-8";
    type.toUtfProc	= Cesu8ToUtfProc;
    type.fromUtfProc	= UtfToCesu8Proc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc	= UtfToUcs2Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "ucs-2le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf32ToUtfProc;
    type.fromUtfProc	= UtfToUtf32Proc;
    type.freeProc	= NULL;
    type.nullSize	= 4;
    type.encodingName	= "utf-32le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc    = UtfToUtf16Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "utf-16le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

#ifndef TCL_NO_DEPRECATED
    type.encodingName	= "unicode";
    Tcl_CreateEncoding(&type);
#endif

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)Tcl_Alloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->fromUnicode, 0, size);

    dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
    dataPtr->fromUnicode[0] = (unsigned short *) (dataPtr->fromUnicode + 256);
    for (i=1 ; i<256 ; i++) {
	dataPtr->toUnicode[i] = emptyPage;
	dataPtr->fromUnicode[i] = emptyPage;
    }

    for (i=0 ; i<256 ; i++) {
	dataPtr->toUnicode[0][i] = i;
	dataPtr->fromUnicode[0][i] = i;
    }

    type.encodingName	= "iso8859-1";
    type.toUtfProc	= Iso88591ToUtfProc;
    type.fromUtfProc	= Iso88591FromUtfProc;
    type.freeProc	= TableFreeProc;
    type.nullSize	= 1;
    type.clientData	= dataPtr;
    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    encodingsInitialized = 1;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1494–1613. Line unit: physical LF only. Original byte range [45751, 49495). Retained reference `source-window-25`.

```text
int
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1744–1863. Line unit: physical LF only. Original byte range [54661, 58459). Retained reference `source-window-26`.

```text
int
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";	/* Avoid ABSAN warnings about ops on NULL pointers */
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2007–2105. Line unit: physical LF only. Original byte range [60882, 64085). Retained reference `source-window-27`.

```text
static int 
UtfToUtfProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	     srcReadPtr, dstWrotePtr, dstCharsPtr, pureNullMode)
    ClientData clientData;	/* Not used. */
    CONST char *src;		/* Source string in UTF-8. */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted.  This
				 * may be less than the original source length
				 * if there was a problem converting some
				 * source characters. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode;		/* Convert embedded nulls from
				 * internal representation to real
				 * null-bytes or vice versa */

{
    CONST char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;
    
    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 &&
	    !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are
	     * in input mode, so that they get converted to 0xc080.
	     */
	    *dst++ = *src++;
	} else if (pureNullMode == 1 &&
		   UCHAR(*src) == 0xc0 &&
		   UCHAR(*(src+1)) == 0x80) {
	    /* 
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */
	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /* Always check before using Tcl_UtfToUniChar. Not doing
	     * can so cause it run beyond the endof the buffer!  If we
	     * happen such an incomplete char its bytes are made to
	     * represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2217–2313. Line unit: physical LF only. Original byte range [66200, 69291). Retained reference `source-window-28`.

```text
static int
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. */
{
    const char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xc080.
	     */

	    *dst++ = *src++;
	} else if (pureNullMode == 1 && UCHAR(*src) == 0xc0 &&
		(src + 1 < srcEnd) && UCHAR(*(src+1)) == 0x80) {
	    /*
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using Tcl_UtfToUniChar. Not doing can so
	     * cause it run beyond the endof the buffer! If we happen such an
	     * incomplete char its byts are made to represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2275–2420. Line unit: physical LF only. Original byte range [66906, 71888). Retained reference `source-window-29`.

```text
static int
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. Also combine or separate surrogate pairs */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    Tcl_UniChar *chPtr = (Tcl_UniChar *) statePtr;

    if (flags & TCL_ENCODING_START) {
    	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - ((pureNullMode == 1) ? 4 : TCL_UTF_MAX);

    for (numChars = 0; src < srcEnd && numChars <= charLimit; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (pureNullMode == 0))) {
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xC080.
	     */

	    *dst++ = *src++;
	    *chPtr = 0; /* reset surrogate handling */
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd)
		&& (UCHAR(src[1]) == 0x80) && (pureNullMode == 1)) {
	    /*
	     * Convert 0xC080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    *chPtr = 0; /* reset surrogate handling */
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using TclUtfToUniChar. Not doing can so
	     * cause it run beyond the end of the buffer! If we happen such an
	     * incomplete char its bytes are made to represent themselves
	     * unless the user has explicitly asked to be told.
	     */

	    if ((flags & TCL_ENCODING_STOPONERROR) && (pureNullMode == 0)) {
		result = TCL_CONVERT_MULTIBYTE;
		break;
	    }
	    *chPtr = UCHAR(*src);
	    src += 1;
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	} else {
	    size_t len = TclUtfToUniChar(src, chPtr);
	    if ((len < 2) && (*chPtr != 0) && (flags & TCL_ENCODING_STOPONERROR)
		    && ((*chPtr & ~0x7FF) != 0xD800) && (pureNullMode == 0)) {
		result = TCL_CONVERT_SYNTAX;
		break;
	    }
	    src += len;
	    if ((*chPtr & ~0x7FF) == 0xD800) {
		Tcl_UniChar low;
		/* A surrogate character is detected, handle especially */
#if TCL_UTF_MAX <= 4
	    if ((len < 3) && ((src[3 - len] & 0xC0) != 0x80)) {
	    /* It's invalid. See [ed29806ba] */
		*chPtr = UCHAR(src[-1]);
		dst += Tcl_UniCharToUtf(*chPtr, dst);
		continue;
	    }
#endif
		low = *chPtr;
		len = (src <= srcEnd-3) ? Tcl_UtfToUniChar(src, &low) : 0;
		if (((low & ~0x3FF) != 0xDC00) || (*chPtr & 0x400)) {
		    *dst++ = (char) (((*chPtr >> 12) | 0xE0) & 0xEF);
		    *dst++ = (char) (((*chPtr >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((*chPtr | 0x80) & 0xBF);
		    *chPtr = 0; /* reset surrogate handling */
		    continue;
		} else if ((TCL_UTF_MAX > 3) || (pureNullMode == 1)) {
		    int full = (((*chPtr & 0x3FF) << 10) | (low & 0x3FF)) + 0x10000;
		    *dst++ = (char) (((full >> 18) | 0xF0) & 0xF7);
		    *dst++ = (char) (((full >> 12) | 0x80) & 0xBF);
		    *dst++ = (char) (((full >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((full | 0x80) & 0xBF);
			*chPtr = 0; /* reset surrogate handling */
		    src += len;
		    continue;
		}
	    }
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2440–2743. Line unit: physical LF only. Original byte range [72928, 82701). Retained reference `source-window-30`.

```text
static int
UtfToUtfProc(
    void *clientData,		/* additional flags */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* TCL_ENCODING_* conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    int ch;
    int profile;

    if (flags & TCL_ENCODING_START) {
	/* *statePtr will hold high surrogate in a split surrogate pair */
	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    /*
     * If output is UTF-8 or encoding for Tcl's internal encoding,
     * max space needed is TCL_UTF_MAX. Otherwise, need 6 bytes (CESU-8)
     */
    dstEnd = dst + dstLen - ((flags & (ENCODING_INPUT|ENCODING_UTF)) ? TCL_UTF_MAX : 6);

    /*
     * Macro to output an isolated high surrogate when it is not followed
     * by a low surrogate. NOT to be called for strict profile since
     * that should raise an error.
     */
#define OUTPUT_ISOLATEDSURROGATE                                \
    do {                                                        \
	Tcl_UniChar high;                                       \
	if (PROFILE_REPLACE(profile)) {                         \
	    high = UNICODE_REPLACE_CHAR;                        \
	} else {                                                \
	    high = (Tcl_UniChar)(ptrdiff_t) *statePtr;          \
	}                                                       \
	assert(!(flags & ENCODING_UTF)); /* Must be CESU-8 */   \
	assert(HIGH_SURROGATE(high));                           \
	assert(!PROFILE_STRICT(profile));                       \
	dst += Tcl_UniCharToUtf(high, dst);                     \
	*statePtr = 0; /* Reset state */                        \
    } while (0)

    /*
     * Macro to check for isolated surrogate and either break with
     * an error if profile is strict, or output an appropriate
     * character for replace and tcl8 profiles and continue.
     */
#define CHECK_ISOLATEDSURROGATE                                         \
    if (*statePtr) {                                                    \
	if (PROFILE_STRICT(profile)) {                                  \
	    result = TCL_CONVERT_SYNTAX;                                \
	    break;                                                      \
	}                                                               \
	OUTPUT_ISOLATEDSURROGATE;                                       \
	continue; /* Rerun loop so length checks etc. repeated */       \
    } else                                                              \
	(void) 0

    profile = ENCODING_PROFILE_GET(flags);
    for (numChars = 0; src < srcEnd && numChars <= charLimit; numChars++) {

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (flags & ENCODING_INPUT))) {

	    CHECK_ISOLATEDSURROGATE;
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to \xC0\x80.
	     */
	    *dst++ = *src++;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		 (UCHAR(src[1]) == 0x80) &&
		 (!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */

	    CHECK_ISOLATEDSURROGATE;
	    if (!PROFILE_TCL8(profile) && (flags & ENCODING_INPUT)) {
		if (PROFILE_REPLACE(profile)) {
		    dst += Tcl_UniCharToUtf(UNICODE_REPLACE_CHAR, dst);
		    src += 2;
		} else {
		    /* PROFILE_STRICT */
		    result = TCL_CONVERT_SYNTAX;
		    break;
		}
	    } else {
		/*
		 * Convert 0xC080 to real nulls when we are in output mode,
		 * irrespective of the profile.
		 */
		*dst++ = 0;
		src += 2;
	    }

	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence not because there are insufficient
	     * bytes in source buffer (have already checked that above) but
	     * because the UTF-8 sequence is truncated.
	     */

	    CHECK_ISOLATEDSURROGATE;

	    if (flags & ENCODING_INPUT) {
		/* Incomplete bytes for modified UTF-8 target */
		if (PROFILE_STRICT(profile)) {
		    result = (flags & TCL_ENCODING_CHAR_LIMIT)
			    ? TCL_CONVERT_MULTIBYTE
			    : TCL_CONVERT_SYNTAX;
		    break;
		}
	    }
	    if (PROFILE_REPLACE(profile)) {
		ch = UNICODE_REPLACE_CHAR;
		++src;
	    } else {
		/* TCL_ENCODING_PROFILE_TCL8 */
		char chbuf[2];
		chbuf[0] = UCHAR(*src++);
		chbuf[1] = 0;
		TclUtfToUniChar(chbuf, &ch);
	    }
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    /* Have a complete character */
	    size_t len = TclUtfToUniChar(src, &ch);

	    Tcl_UniChar savedSurrogate = (Tcl_UniChar) (ptrdiff_t)*statePtr;
	    *statePtr = 0; /* Reset surrogate */

	    if (flags & ENCODING_INPUT) {
		if (((len < 2) && (ch != 0)) || ((ch > 0xFFFF) && !(flags & ENCODING_UTF))) {
		    if (PROFILE_STRICT(profile)) {
			result = TCL_CONVERT_SYNTAX;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    }
		}
	    }

	    const char *saveSrc = src;
	    src += len;
	    if (!(flags & ENCODING_UTF) && !(flags & ENCODING_INPUT)
		    && (ch > 0x7FF)) {
		assert(savedSurrogate == 0); /* Since this flag combo
						will never set *statePtr */
		if (ch > 0xFFFF) {
		    /* CESU-8 6-byte sequence for chars > U+FFFF */
		    ch -= 0x10000;
		    *dst++ = 0xED;
		    *dst++ = (char) (((ch >> 16) & 0x0F) | 0xA0);
		    *dst++ = (char) (((ch >> 10) & 0x3F) | 0x80);
		    ch = (ch & 0x03FF) | 0xDC00;
		}
		*dst++ = (char)(((ch >> 12) | 0xE0) & 0xEF);
		*dst++ = (char)(((ch >> 6) | 0x80) & 0xBF);
		*dst++ = (char)((ch | 0x80) & 0xBF);
		continue;
	    } else if (SURROGATE(ch)) {
		if ((flags & ENCODING_UTF)) {
		    /* UTF-8, not CESU-8, so surrogates should not appear */
		    if (PROFILE_STRICT(profile)) {
			result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
			src = saveSrc;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    } else {
			/* PROFILE_TCL8 - output as is */
		    }
		} else {
		    /* CESU-8 */
		    if (LOW_SURROGATE(ch)) {
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    ch = 0x10000 + ((savedSurrogate - 0xd800) << 10) + (ch - 0xdc00);
			} else {
			    /* Isolated low surrogate */
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Tcl8 profile. Output low surrogate as is */
			    }
			}
		    } else {
			assert(HIGH_SURROGATE(ch));
			/* Save the high surrogate */
			*statePtr = (Tcl_EncodingState) (ptrdiff_t) ch;
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Output the isolated high surrogate */
				ch = savedSurrogate;
			    }
			} else {
			    /* High surrogate saved in *statePtr. Do not output anything just yet. */
			    --numChars; /* Cancel the increment at end of loop */
			    continue;
			}
		    }
		}
	    } else {
		/* Normal character */
		CHECK_ISOLATEDSURROGATE;
	    }

	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    /* Check if an high surrogate left over */
    if (*statePtr) {
	assert(!(flags & ENCODING_UTF)); /* CESU-8, Not UTF-8 */
	if (!(flags & TCL_ENCODING_END)) {
	    /* More data coming */
	} else {
	    /* No more data coming */
	    if (PROFILE_STRICT(profile)) {
		result = (flags & ENCODING_INPUT)
		    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
	    } else {
		if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		} else {
		    ch = (Tcl_UniChar) (ptrdiff_t) *statePtr;
		}
		if (dst < dstEnd) {
		    dst += Tcl_UniCharToUtf(ch, dst);
		    ++numChars;
		} else {
		    /* No room in destination */
		    result = TCL_CONVERT_NOSPACE;
		}
	    }
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2964–3152. Line unit: physical LF only. Original byte range [91365, 96533). Retained reference `source-window-31`.

```text
static int
UtfToUtfProc(
    void *clientData,
    const char *src,
    int srcLen,
    int flags,
    Tcl_EncodingState *statePtr,
    char *dst,
    int dstLen,
    int *srcReadPtr,
    int *dstWrotePtr,
    int *dstCharsPtr)
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars = 0, charLimit = INT_MAX;
    int ch;
    int profile;

    *statePtr = 0;
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    dstEnd = dst + dstLen - TCL_UTF_MAX;

    /* Checks if a source byte can be copied directly to destination */
#define BYTE_COPYABLE(byte_) \
    (UCHAR(byte_) < 0x80 && ((UCHAR(byte_) != 0)))

    profile = ENCODING_PROFILE_GET(flags);

    for (numChars = 0; src < srcEnd && numChars < charLimit; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}

	if (BYTE_COPYABLE(*src)) {
	    /*
	     * Common case fast path for non-nul ASCII bytes.
	     *
	     * Because we resolved any pending surrogate above (before the
	     * loop), *statePtr is guaranteed to be 0 here, so no
	     * CHECK_ISOLATEDSURROGATE() call is needed.
	     */
	    const char *p = src;
	    ptrdiff_t copyCount = srcEnd - src;
	    if (charLimit != INT_MAX) {
		if (copyCount > (charLimit - numChars)) {
		    copyCount = charLimit - numChars;
		}
	    }
	    if (copyCount > (dstEnd - dst + 1)) {
		copyCount = dstEnd - dst + 1;
	    }
	    const char *srcStop = src + copyCount;
	    while (p < srcStop && BYTE_COPYABLE(*p)) {
		*dst++ = *p++;
	    }
	    numChars += (int)(p - src) - 1;
	    src = p;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		(UCHAR(src[1]) == 0x80) &&
		(!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */
	    if (!PROFILE_TCL8(profile) && (flags & ENCODING_INPUT)) {
		if (PROFILE_REPLACE(profile)) {
		    dst += Tcl_UniCharToUtf(UNICODE_REPLACE_CHAR, dst);
		    src += 2;
		} else {
		    /* PROFILE_STRICT */
		    result = TCL_CONVERT_SYNTAX;
		    break;
		}
	    } else {
		*dst++ = 0;
		src += 2;
	    }
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence (truncated UTF-8, not just end of
	     * source buffer — that is caught by the srcClose check above).
	     */
	    if (flags & ENCODING_INPUT) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & TCL_ENCODING_CHAR_LIMIT)
			    ? TCL_CONVERT_MULTIBYTE
			    : TCL_CONVERT_SYNTAX;
		    break;
		}
	    }
	    if (PROFILE_REPLACE(profile)) {
		ch = UNICODE_REPLACE_CHAR;
		++src;
	    } else {
		/* TCL_ENCODING_PROFILE_TCL8 */
		char chbuf[2];
		chbuf[0] = UCHAR(*src++);
		chbuf[1] = 0;
		TclUtfToUniChar(chbuf, &ch);
	    }
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    /* Have a complete character */
	    size_t len = TclUtfToUniChar(src, &ch);

	    /*
	     * For invalid inputs, Tcl_UtfToUniChar will return the byte
	     * itself and len == 1. We know this is invalid because valid
	     * single byte ASCII is already handled above except for 0.
	     * For the output case, Tcl should not have invalid byte sequences
	     * internally but if it does, garbage in, garbage out. This is
	     * historical behavior that should be changed imho. See
	     * ticket [b69e00ecf6]. TODO.
	     */
	    if (flags & ENCODING_INPUT) {
		if ((len < 2) && (ch != 0)) {
		    if (PROFILE_STRICT(profile)) {
			result = TCL_CONVERT_SYNTAX;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    }
		}
	    }

	    const char *saveSrc = src;
	    src += len;

	    if (SURROGATE(ch)) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
		    src = saveSrc;
		    break;
		} else if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		}
		/* PROFILE_TCL8: fall through and output as-is */
	    }
	    /* Normal character (or surrogate resolved to replacement/as-is) */
	    assert(ch >= 0 && ch <= 0x10FFFF);
	    if (ch == 0) {
		if (flags & ENCODING_INPUT) {
		    *dst++ = (char)0xC0;
		    *dst++ = (char)0x80;
		} else {
		    *dst++ = 0;
		}
	    } else if ((unsigned)ch < 0x800) {
		assert(ch >= 0x80);
		*dst++ = (char)(0xC0 | (ch >> 6));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else if ((unsigned)ch < 0x10000) {
		*dst++ = (char)(0xE0 | (ch >> 12));
		*dst++ = (char)(0x80 | ((ch >> 6) & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else {
		*dst++ = (char)(0xF0 | (ch >> 18));
		*dst++ = (char)(0x80 | ((ch >> 12) & 0x3F));
		*dst++ = (char)(0x80 | ((ch >> 6)  & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    }
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;

#undef BYTE_COPYABLE
}

```

## Consumer bindings

- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `prepare_source_filename`: Keep selected external backend bytes, original diagnostic extent and counted source label separate under actual C/Jim filename facts.
- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `external_iso88591_filename`: Reuse selected original unit decoding for actual ISO external bytes and C8 fallback/C9 strict filesystem error.
- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `filename_encoding_error`: Present reached strict external-conversion failure as filesystem EILSEQ, independently of converter capability absence.
- [rust/tcl-platform/src/lib.rs](../../../../rust/tcl-platform/src/lib.rs), `Filesystem::source_filename_bytes`: Supply an independent actual filesystem converter from already selected native units; no raw dialect decoder or source script codec donation.
- [rust/tcl-vm/src/command/native_encoding_source.rs](../../../../rust/tcl-vm/src/command/native_encoding_source.rs), `SourceOps::system_encoding`: Read actual mutable system setting at reached filesystem conversion, with original Name/operand/currency purpose retained separately.
- [runtime/rust/src/interp/native_encoding_source.rs](../../../../runtime/rust/src/interp/native_encoding_source.rs), `SourceOps::system_encoding`: Read current Runtime system encoding at reached source filename conversion under genuine original argv and command currency.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_filename_external_zero_has_separate_backend_and_report_extents` (linked): Keep selected C external conversion/C-string or EINVAL backend extent separate from original diagnostic bytes and counted source label; Jim retains raw CString purpose.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_filename_iso88591_and_identity_select_actual_external_bytes` (linked): The existing selected native decoder supplies ISO units; C8 fallback and C9 strict EILSEQ remain separate from byte-preserving identity and source labels.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_c9_identity_name_uses_actual_catalogue_instead_of_builtin_codec` (linked): C9 named identity uses actual catalogue knowledge: genuine miss yields the selected unknown diagnostic, while supplied installed identity remains possible.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_filename_utf8_surrogates_keep_selected_conversion_and_error_owner` (linked): Preserve C84/85 supplied pair bytes, C86 pair combination and C9 strict unpaired/pair EILSEQ through the same external worker, without a missing-converter or Host classification.
- [rust/tcl-vm/src/command/native_encoding_source/tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_source/tests.rs), `command::native_encoding_source::tests::original_filename_utf8_conversion_preserves_counted_header_and_lookup_effects` (linked): Original VM source uses selected external filename bytes and preserves the complete counted operand header while distinguishing C84 prefix effects from later converted-zero EINVAL.
- [rust/tcl-vm/src/command/native_encoding_source/tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_source/tests.rs), `command::native_encoding_source::tests::original_filename_iso88591_host_default_keeps_plain_source_working` (linked): Actual supplied Host ISO-8859-1 default permits ordinary source through the selected filename worker without an additional custom converter query.
- [runtime/rust/src/interp/native_encoding_source/tests.rs](../../../../runtime/rust/src/interp/native_encoding_source/tests.rs), `interp::native_encoding_source::tests::original_filename_external_zero_refuses_before_open_and_keeps_report_bytes` (linked): Runtime original argv uses external zero refusal before backend open, preserves complete original counted header and presents original CString report bytes.
- [runtime/rust/src/interp/native_encoding_source/tests.rs](../../../../runtime/rust/src/interp/native_encoding_source/tests.rs), `interp::native_encoding_source::tests::original_filename_iso88591_selected_system_keeps_plain_source_working` (linked): Runtime consumes the actual selected ISO-8859-1 system setting for an ordinary source filename and preserves exact file/evaluation effects without custom converter queries.

## Evidence

- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/same-library-version)
- [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/same-library-version](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/same-library-version)
- [runtime/rust/src/interp](../../../../runtime/rust/src/interp)
- [runtime/rust/src/interp/native_encoding_source](../../../../runtime/rust/src/interp/native_encoding_source)
- [rust/tcl-cmd-core/src/encoding](../../../../rust/tcl-cmd-core/src/encoding)
- [rust/tcl-cmd-core/src/encoding/original_names](../../../../rust/tcl-cmd-core/src/encoding/original_names)
- [rust/tcl-platform/src](../../../../rust/tcl-platform/src)
- [rust/tcl-vm/src/command](../../../../rust/tcl-vm/src/command)
- [rust/tcl-vm/src/command/native_encoding_source](../../../../rust/tcl-vm/src/command/native_encoding_source)
