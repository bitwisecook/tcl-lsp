# naming.encoding.original-byte-name-lookup-and-source-order

Kind: `native-observation`

## Problem statement

Encoding catalogue names are original String bytes, not Unicode identifiers. Source gets an explicit encoding String before opening the file but resolves its catalogue name after successful open. A Unicode validator, replacement-character rewrite, early lookup or ignored encoding operand changes the public result and source effects. Empty system reset is a stored platform default, distinct from the current system encoding.

Source file configuration and reading: Reading a whole file before resolving its source encoding changes the winning error when a file opens but cannot be read. Source must retain early encoding String access, opened channel identity, configuration before read and mandatory same-handle cleanup.

## Question

For the exact counted String names, including literal and modified zero and invalid UTF-8 bytes, what encoding system and explicit source results, errorCode bytes, system changes and file effects occur on each actual initialized provider, and which source stage wins when the file is missing?

Source file configuration and reading: For the owned empty directory, existing ASCII script and absent file, which exact completion, errorCode, system-query and effect bytes occur with UTF-8, unknown and binary encoding on each original provider? Which configuration failure precedes an opened directory read failure?

## Conclusion

All measured C providers use original byte encoding names with a literal-zero C-string extent: utf-8 followed by raw zero and a tail is accepted, uppercase UTF-8 is rejected, and raw FF/C0AF and modified-zero names produce original byte unknown-encoding diagnostics rather than a Unicode validation error. Empty system names restore the actual stored default: identity on the measured C8.4/C8.5 initialized processes and iso8859-1 on the measured C8.6/C9 processes; these are environment observations, not release-derived defaults. Binary is unknown for encoding system. C8.5/C8.6 explicit source accepts empty/binary channel names; C9 rejects them with its no-longer-supported diagnostic and NONE. Every explicit-source missing-file case on C8.5+ reports POSIX ENOENT before encoding lookup, including invalid and unsupported names. C8.4 rejects the explicit form by arity. Jim has no encoding command and accepts only the source filename grammar. Successful existing-file cases execute the exact script and return SOURCE_OK. Full result/errorCode/system/effect bytes remain in the original streams.

Source file configuration and reading: Measured C8.5/C8.6 supported UTF-8 and binary directory cases reach EISDIR, while unknown encoding fails first. Measured C9.0/C9.1 UTF-8 reaches EISDIR; unknown encoding and removed binary configuration fail first. Existing UTF-8 scripts run and absent paths produce ENOENT before lookup on all four providers. Selected original source confirms open before encoding configuration and configuration before read. C8.4/Jim refuse explicit encoding by arity; separate plain-directory cases retain their recorded read errors. Full bytes and separate system-query codes remain captured.

## Scope

Six separately compiled and executed probe processes, each process exit0, contain 41 fresh initialized interpreters: 13 exact public counted String constructor names across system setter, explicit-source existing and explicit-source missing workers, plus two plain-source controls. This is 246 interpreter blocks, not 246 subprocesses. C uses Tcl_CreateInterp/Tcl_Init and Jim uses its recorded initialized provider sequence; each block seeds errorCode and the source effect and observes the current system value separately. Each C block explicitly establishes utf-8 before the tested operation. Exact compiler/link commands, CLI versions, independent same-library versions, original probe ELF and library/header/source bytes and before/after pins are retained. Source excerpts support getter-before-open versus catalogue-after-open stages; public outputs establish file-failure precedence. No callback/updater/free chronology, Native reference-count or protective-hold schedule, complete encoding catalogue, non-ASCII file codec/profile/eof/BOM semantics, Rust build/assertion outcome, universal platform/default or BIG-IP result is established.

Source file configuration and reading: Six separately compiled probe processes, eight fresh initialized interpreters each:48 public operation blocks, four SOURCE_OK successes and44 Guest failures. Compile/CLI/run and six independent same-library diagnostics exit0 with matching versions and all before/after pins equal. Directory cases are distinct from ordinary file controls. This finite observation plus inspected source establishes selected open/configure/read order, not arbitrary VFS callbacks, updater/free chronology, native refcounts, full errno/close behavior, codecs/BOM/EOF, Rust assertions or BIG-IP. The original six-process246-block encoding evidence and fields remain unchanged.

## Provider answers

### tcl8.4

Status: `observed`. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten. Version: tcl8.4 8.4.20.

Original unknown encoding diagnostics retain raw bytes and set NONE. Literal-zero tails truncate; modified zero remains in the name. Empty system reset returns the measured identity default. Binary and uppercase are unknown. All 26 explicit-source calls fail by source fileName arity, with no script effect. Plain source succeeds on the existing file and reports ENOENT on the absent file.

Source file configuration and reading: All seven explicit-encoding workers refuse by source fileName arity with NONE and no source effect. The separate plain-directory worker reaches POSIX EISDIR {illegal operation on a directory}. Unsupported explicit grammar establishes no encoding lookup stage.

### tcl8.5

Status: `observed`. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten. Version: tcl8.5 8.5.19.

System empty restores the measured identity default; binary/uppercase/raw invalid/modified-zero names are unknown with TCL LOOKUP ENCODING and exact original bytes. Existing explicit source accepts empty/binary and the supported names and executes the script; absent files always report POSIX ENOENT before lookup. Literal-zero names use the truncated prefix.

Source file configuration and reading: Supported UTF-8 and binary directory cases reach POSIX EISDIR {illegal operation on a directory}; unknown encoding instead reports TCL LOOKUP ENCODING R2286_missing_encoding, with no source effect. Existing UTF-8 executes SOURCE_OK; both missing controls report POSIX ENOENT before lookup. Actual source opens, configures encoding and then reads.

### tcl8.6

Status: `observed`. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten. Version: tcl8.6 8.6.18.

System empty restores the measured iso8859-1 default. Exact byte unknown names retain TCL LOOKUP ENCODING. Existing explicit source accepts empty/binary; absent files report POSIX ENOENT first. Literal-zero prefixes are selected and modified zero is not a terminator.

Source file configuration and reading: Supported UTF-8 and binary directory cases reach POSIX EISDIR {illegal operation on a directory}; unknown encoding instead reports TCL LOOKUP ENCODING R2286_missing_encoding, with no source effect. Existing UTF-8 executes SOURCE_OK; both missing controls report POSIX ENOENT before lookup. Actual source opens, configures encoding and then reads.

### tcl9.0

Status: `observed`. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten. Version: tcl9.0 9.0.4.

System empty restores the measured iso8859-1 default. Existing explicit source rejects empty/binary with the exact no-longer-supported diagnostic and NONE. Missing files report POSIX ENOENT before that configuration/lookup failure. Other original unknown names preserve byte diagnostics and TCL LOOKUP ENCODING.

Source file configuration and reading: Supported UTF-8 and plain directory cases reach POSIX EISDIR {is a directory}. Unknown encoding reports TCL LOOKUP ENCODING R2286_missing_encoding instead; binary reports its exact no-longer-supported diagnostic and NONE before read. Existing UTF-8 executes SOURCE_OK; both missing controls report POSIX ENOENT before lookup.

### tcl9.1

Status: `observed`. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten. Version: tcl9.1 9.1.0.

System empty restores the measured iso8859-1 default. Existing explicit source rejects empty/binary with the exact no-longer-supported diagnostic and NONE. Missing files report POSIX ENOENT before configuration/lookup failure. Literal-zero prefix and original invalid-byte lookup results match the recorded C9.0 cases.

Source file configuration and reading: Supported UTF-8 and plain directory cases reach POSIX EISDIR {is a directory}. Unknown encoding reports TCL LOOKUP ENCODING R2286_missing_encoding instead; binary reports its exact no-longer-supported diagnostic and NONE before read. Existing UTF-8 executes SOURCE_OK; both missing controls report POSIX ENOENT before lookup.

### jim

Status: `observed`. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten. Version: jim 0.84-9-g5bac7c9.

Encoding is unavailable: its original command lookup error is retained for all13 system attempts and each separate before/after system query. All26 explicit source attempts fail the filename-only arity and preserve the seeded errorCode. Plain existing source runs the script and normalizes return to SOURCE_OK; plain missing source uses its measured capitalized No such file or directory and preserves the seeded errorCode.

Source file configuration and reading: All seven explicit-encoding workers refuse by source fileName arity and preserve R2286 ENCODING SEEDED. Encoding system queries separately report invalid command name encoding. Plain-directory reports failed to load file followed by Is a directory and preserves the seeded errorCode; no encoding lookup stage is reached.

### bigip

Status: `not-tested`. Channel: No original BIG-IP input was executed.

No original BIG-IP encoding or source worker observation is available.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 495–519. Line unit: physical LF only. Original byte range [16151, 16877). Retained reference `source-window-0`.

```text
Tcl_GetEncoding(interp, name)
    Tcl_Interp *interp;		/* Interp for error reporting, if not NULL. */
    CONST char *name;		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *) Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);
    return LoadEncodingFile(interp, name);
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 765–792. Line unit: physical LF only. Original byte range [23082, 23817). Retained reference `source-window-1`.

```text
Tcl_SetSystemEncoding(interp, name)
    Tcl_Interp *interp;		/* Interp for error reporting, if not NULL. */
    CONST char *name;		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding;
    Encoding *encodingPtr;

    if (!name || !*name) {
	Tcl_MutexLock(&encodingMutex);
	encoding = defaultEncoding;
	encodingPtr = (Encoding *) encoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
    } else {
	encoding = Tcl_GetEncoding(interp, name);
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    Tcl_MutexLock(&encodingMutex);
    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1291–1371. Line unit: physical LF only. Original byte range [39515, 41274). Retained reference `source-window-2`.

```text
LoadEncodingFile(interp, name)
    Tcl_Interp *interp;		/* Interp for error reporting, if not NULL. */
    CONST char *name;		/* The name of the encoding file on disk
				 * and also the name for new encoding. */
{
    int objc, i, ch;
    Tcl_Obj **objv;
    Tcl_Obj *pathPtr;
    Tcl_Channel chan;
    Tcl_Encoding encoding;

    pathPtr = TclGetLibraryPath();
    if (pathPtr == NULL) {
	goto unknown;
    }
    objc = 0;
    Tcl_ListObjGetElements(NULL, pathPtr, &objc, &objv);

    chan = NULL;
    for (i = 0; i < objc; i++) {
	chan = OpenEncodingFile(Tcl_GetString(objv[i]), name);
	if (chan != NULL) {
	    break;
	}
    }

    if (chan == NULL) {
	goto unknown;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    encoding = NULL;
    switch (ch) {
	case 'S': {
	    encoding = LoadTableEncoding(interp, name, ENCODING_SINGLEBYTE,
		    chan);
	    break;
	}
	case 'D': {
	    encoding = LoadTableEncoding(interp, name, ENCODING_DOUBLEBYTE,
		    chan);
	    break;
	}
	case 'M': {
	    encoding = LoadTableEncoding(interp, name, ENCODING_MULTIBYTE,
		    chan);
	    break;
	}
	case 'E': {
	    encoding = LoadEscapeEncoding(name, chan);
	    break;
	}
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_AppendResult(interp, "invalid encoding file \"", name, "\"", NULL);
	if (ch == 'E') {
	    Tcl_AppendResult(interp, " or missing sub-encoding", NULL);
	}
    }
    Tcl_Close(NULL, chan);
    return encoding;

    unknown:
    if (interp != NULL) {
	Tcl_AppendResult(interp, "unknown encoding \"", name, "\"", NULL);
    }
    return NULL;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 423–527. Line unit: physical LF only. Original byte range [10292, 12896). Retained reference `source-window-3`.

```text
Tcl_EncodingObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int index, length;
    Tcl_Encoding encoding;
    char *string;
    Tcl_DString ds;
    Tcl_Obj *resultPtr;

    static CONST char *optionStrings[] = {
	"convertfrom", "convertto", "names", "system",
	NULL
    };
    enum options {
	ENC_CONVERTFROM, ENC_CONVERTTO, ENC_NAMES, ENC_SYSTEM
    };

    if (objc < 2) {
    	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
        return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], optionStrings, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
	case ENC_CONVERTTO:
	case ENC_CONVERTFROM: {
	    Tcl_Obj *data;
	    if (objc == 3) {
		encoding = Tcl_GetEncoding(interp, NULL);
		data = objv[2];
	    } else if (objc == 4) {
		if (TclGetEncodingFromObj(interp, objv[2], &encoding)
			!= TCL_OK) {
		    return TCL_ERROR;
		}
		data = objv[3];
	    } else {
		Tcl_WrongNumArgs(interp, 2, objv, "?encoding? data");
		return TCL_ERROR;
	    }
	    
	    if ((enum options) index == ENC_CONVERTFROM) {
		/*
		 * Treat the string as binary data.
		 */

		string = (char *) Tcl_GetByteArrayFromObj(data, &length);
		Tcl_ExternalToUtfDString(encoding, string, length, &ds);

		/*
		 * Note that we cannot use Tcl_DStringResult here because
		 * it will truncate the string at the first null byte.
		 */

		Tcl_SetStringObj(Tcl_GetObjResult(interp),
			Tcl_DStringValue(&ds), Tcl_DStringLength(&ds));
		Tcl_DStringFree(&ds);
	    } else {
		/*
		 * Store the result as binary data.
		 */

		string = Tcl_GetStringFromObj(data, &length);
		Tcl_UtfToExternalDString(encoding, string, length, &ds);
		resultPtr = Tcl_GetObjResult(interp);
		Tcl_SetByteArrayObj(resultPtr, 
			(unsigned char *) Tcl_DStringValue(&ds),
			Tcl_DStringLength(&ds));
		Tcl_DStringFree(&ds);
	    }

	    Tcl_FreeEncoding(encoding);
	    break;
	}
	case ENC_NAMES: {
	    if (objc > 2) {
		Tcl_WrongNumArgs(interp, 2, objv, NULL);
		return TCL_ERROR;
	    }
	    Tcl_GetEncodingNames(interp);
	    break;
	}
	case ENC_SYSTEM: {
	    if (objc > 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "?encoding?");
		return TCL_ERROR;
	    }
	    if (objc == 2) {
		Tcl_SetStringObj(Tcl_GetObjResult(interp),
			Tcl_GetEncodingName(NULL), -1);
	    } else {
	        return Tcl_SetSystemEncoding(interp,
			Tcl_GetStringFromObj(objv[2], NULL));
	    }
	    break;
	}
    }
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_SourceObjCmd`, lines 1056–1068. Line unit: physical LF only. Original byte range [28791, 29163). Retained reference `source-window-4`.

```text
Tcl_SourceObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "fileName");
	return TCL_ERROR;
    }

    return Tcl_FSEvalFile(interp, objv[1]);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 768–793. Line unit: physical LF only. Original byte range [23644, 24358). Retained reference `source-window-5`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *) Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 988–1015. Line unit: physical LF only. Original byte range [29334, 30056). Retained reference `source-window-6`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding;
    Encoding *encodingPtr;

    if (!name || !*name) {
	Tcl_MutexLock(&encodingMutex);
	encoding = defaultEncoding;
	encodingPtr = (Encoding *) encoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
    } else {
	encoding = Tcl_GetEncoding(interp, name);
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    Tcl_MutexLock(&encodingMutex);
    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1584–1632. Line unit: physical LF only. Original byte range [47153, 48288). Retained reference `source-window-7`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the encoding file on disk and
				 * also the name for new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_AppendResult(interp, "invalid encoding file \"", name, "\"", NULL);
    }
    Tcl_Close(NULL, chan);

    return encoding;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 426–528. Line unit: physical LF only. Original byte range [10204, 12822). Retained reference `source-window-8`.

```text
Tcl_EncodingObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int index;

    static const char *optionStrings[] = {
	"convertfrom", "convertto", "dirs", "names", "system",
	NULL
    };
    enum options {
	ENC_CONVERTFROM, ENC_CONVERTTO, ENC_DIRS, ENC_NAMES, ENC_SYSTEM
    };

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], optionStrings, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
    case ENC_CONVERTTO:
    case ENC_CONVERTFROM: {
	Tcl_Obj *data;
	Tcl_DString ds;
	Tcl_Encoding encoding;
	int length;
	char *stringPtr;

	if (objc == 3) {
	    encoding = Tcl_GetEncoding(interp, NULL);
	    data = objv[2];
	} else if (objc == 4) {
	    if (Tcl_GetEncodingFromObj(interp, objv[2], &encoding) != TCL_OK) {
		return TCL_ERROR;
	    }
	    data = objv[3];
	} else {
	    Tcl_WrongNumArgs(interp, 2, objv, "?encoding? data");
	    return TCL_ERROR;
	}

	if ((enum options) index == ENC_CONVERTFROM) {
	    /*
	     * Treat the string as binary data.
	     */

	    stringPtr = (char *) Tcl_GetByteArrayFromObj(data, &length);
	    Tcl_ExternalToUtfDString(encoding, stringPtr, length, &ds);

	    /*
	     * Note that we cannot use Tcl_DStringResult here because it will
	     * truncate the string at the first null byte.
	     */

	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    Tcl_DStringValue(&ds), Tcl_DStringLength(&ds)));
	    Tcl_DStringFree(&ds);
	} else {
	    /*
	     * Store the result as binary data.
	     */

	    stringPtr = TclGetStringFromObj(data, &length);
	    Tcl_UtfToExternalDString(encoding, stringPtr, length, &ds);
	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(
		    (unsigned char *) Tcl_DStringValue(&ds),
		    Tcl_DStringLength(&ds)));
	    Tcl_DStringFree(&ds);
	}

	Tcl_FreeEncoding(encoding);
	break;
    }
    case ENC_DIRS:
	return EncodingDirsObjCmd(dummy, interp, objc, objv);
    case ENC_NAMES:
	if (objc > 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	Tcl_GetEncodingNames(interp);
	break;
    case ENC_SYSTEM:
	if (objc > 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?encoding?");
	    return TCL_ERROR;
	}
	if (objc == 2) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    Tcl_GetEncodingName(NULL), -1));
	} else {
	    return Tcl_SetSystemEncoding(interp, TclGetString(objv[2]));
	}
	break;
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `Tcl_SourceObjCmd`, lines 936–966. Line unit: physical LF only. Original byte range [22991, 23739). Retained reference `source-window-9`.

```text
Tcl_SourceObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;

    if (objc != 2 && objc !=4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding name? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    }

    return Tcl_FSEvalFileEx(interp, fileName, encodingName);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1770–1892. Line unit: physical LF only. Original byte range [50144, 53829). Retained reference `source-window-10`.

```text
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

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 755–780. Line unit: physical LF only. Original byte range [22892, 23605). Retained reference `source-window-11`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *)Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 978–1006. Line unit: physical LF only. Original byte range [28602, 29355). Retained reference `source-window-12`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding;
    Encoding *encodingPtr;

    if (!name || !*name) {
	Tcl_MutexLock(&encodingMutex);
	encoding = defaultEncoding;
	encodingPtr = (Encoding *) encoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
    } else {
	encoding = Tcl_GetEncoding(interp, name);
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    Tcl_MutexLock(&encodingMutex);
    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);
    Tcl_FSMountsChanged(NULL);

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1611–1661. Line unit: physical LF only. Original byte range [47338, 48542). Retained reference `source-window-13`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of both the encoding file
				 * and the new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"invalid encoding file \"%s\"", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENCODING", name, (char *)NULL);
    }
    Tcl_Close(NULL, chan);

    return encoding;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdAH.c`, function `EncodingSystemObjCmd`, lines 849–866. Line unit: physical LF only. Original byte range [22819, 23346). Retained reference `source-window-14`.

```text
EncodingSystemObjCmd(
    ClientData dummy,      /* Unused */
    Tcl_Interp* interp,     /* Tcl interpreter */
    int objc,		    /* Number of command line args */
    Tcl_Obj* const objv[])  /* Vector of command line args */
{
    if (objc > 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding?");
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp,
			 Tcl_NewStringObj(Tcl_GetEncodingName(NULL), -1));
    } else {
	return Tcl_SetSystemEncoding(interp, TclGetString(objv[1]));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 985–1015. Line unit: physical LF only. Original byte range [24875, 25627). Retained reference `source-window-15`.

```text
TclNRSourceObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;

    if (objc != 2 && objc !=4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding name? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    }

    return TclNREvalFile(interp, fileName, encodingName);
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1727–1860. Line unit: physical LF only. Original byte range [48406, 52138). Retained reference `source-window-16`.

```text
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
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
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
	    return result;
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
	goto end;
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
	goto end;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280 Force the evaluator to open a frame for a sourced file.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

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

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 732–757. Line unit: physical LF only. Original byte range [22767, 23480). Retained reference `source-window-17`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *)Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 981–1023. Line unit: physical LF only. Original byte range [29089, 30148). Retained reference `source-window-18`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding = NULL;

    if (name && *name) {
	encoding = Tcl_GetEncoding(interp, name); /* this increases refCount */
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    /* Don't lock (and change anything, bump epoch) if it remains unchanged. */

    if ((encoding ? encoding : defaultEncoding) == systemEncoding) {
	if (encoding) {
	    Tcl_FreeEncoding(encoding); /* paired to Tcl_GetEncoding */

	}

	return TCL_OK;
    }

    /* Checks above ensure this is only called when system encoding changes */
    Tcl_MutexLock(&encodingMutex);
    if (!encoding) {
	encoding = defaultEncoding; /* need increase its refCount */
	((Encoding *)encoding)->refCount++;
    }

    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    Tcl_FSMountsChanged(NULL);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1878–1928. Line unit: physical LF only. Original byte range [57123, 58332). Retained reference `source-window-19`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of both the encoding file
				 * and the new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"invalid encoding file \"%s\"", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENCODING", name, (char *)NULL);
    }
    Tcl_CloseEx(NULL, chan, 0);

    return encoding;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdAH.c`, function `EncodingSystemObjCmd`, lines 820–837. Line unit: physical LF only. Original byte range [23019, 23528). Retained reference `source-window-20`.

```text
EncodingSystemObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp* interp,     /* Tcl interpreter */
    int objc,		    /* Number of command line args */
    Tcl_Obj* const objv[])  /* Vector of command line args */
{
    if (objc > 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding?");
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp,
		Tcl_NewStringObj(Tcl_GetEncodingName(NULL), -1));
    } else {
	return Tcl_SetSystemEncoding(interp, TclGetString(objv[1]));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 1104–1158. Line unit: physical LF only. Original byte range [28601, 30131). Retained reference `source-window-21`.

```text
TclNRSourceObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;
    int result;
    void **pkgFiles = NULL;
    void *names = NULL;

    if (objc < 2 || objc > 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding encoding? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    } else if (objc == 3) {
	/* Handle undocumented -nopkg option. This should only be
	 * used by the internal ::tcl::Pkg::source utility function. */
	static const char *const nopkgoptions[] = {
	    "-nopkg", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], nopkgoptions,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	pkgFiles = (void **)Tcl_GetAssocData(interp, "tclPkgFiles", NULL);
	/* Make sure that during the following TclNREvalFile no filenames
	 * are recorded for inclusion in the "package files" command */
	names = *pkgFiles;
	*pkgFiles = NULL;
    }
    result = TclNREvalFile(interp, fileName, encodingName);
    if (pkgFiles) {
	/* restore "tclPkgFiles" assocdata to how it was. */
	*pkgFiles = names;
    }
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1691–1826. Line unit: physical LF only. Original byte range [44401, 48127). Retained reference `source-window-22`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter that evaluates the script. */
    Tcl_Obj *pathPtr,		/* Pathname of the file to process.
				 * Tilde-substitution is performed on this
				 * pathname. */
    const char *encodingName)	/* Either the name of an encoding or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_Size length;
    int result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }

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
	Tcl_CloseEx(interp,chan,0);
	return result;
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
	goto end;
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
	goto end;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

    /*
     * Restore the original iPtr->scriptFile value, but because the value may
     * have hanged during evaluation, don't assume it currently points to
     * pathPtr.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information about where the error occurred.
	 */

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : (int)length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 740–765. Line unit: physical LF only. Original byte range [23064, 23777). Retained reference `source-window-23`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *)Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 989–1026. Line unit: physical LF only. Original byte range [29375, 30429). Retained reference `source-window-24`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding = NULL;

    if (name && *name) {
	encoding = Tcl_GetEncoding(interp, name); /* this increases refCount */
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    /* Don't lock (and change anything, bump epoch) if it remains unchanged. */

    if ((encoding ? encoding : defaultEncoding) == systemEncoding) {
	if (encoding) {
	    Tcl_FreeEncoding(encoding); /* paired to Tcl_GetEncoding */
	}
	return TCL_OK;
    }

    /* Checks above ensure this is only called when system encoding changes */
    Tcl_MutexLock(&encodingMutex);
    if (!encoding) {
	encoding = defaultEncoding; /* need increase its refCount */
	((Encoding *)encoding)->refCount++;
    }

    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    Tcl_FSMountsChanged(NULL);

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 2420–2470. Line unit: physical LF only. Original byte range [76100, 77309). Retained reference `source-window-25`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of both the encoding file
				 * and the new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"invalid encoding file \"%s\"", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENCODING", name, (char *)NULL);
    }
    Tcl_CloseEx(NULL, chan, 0);

    return encoding;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdAH.c`, function `EncodingSystemObjCmd`, lines 841–858. Line unit: physical LF only. Original byte range [25580, 26085). Retained reference `source-window-26`.

```text
EncodingSystemObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Tcl interpreter */
    Tcl_Size objc,		/* Number of command line args */
    Tcl_Obj *const *objv)	/* Vector of command line args */
{
    if (objc > 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding?");
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp,
		Tcl_NewStringObj(Tcl_GetEncodingName(NULL), -1));
    } else {
	return Tcl_SetSystemEncoding(interp, TclGetString(objv[1]));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 1169–1223. Line unit: physical LF only. Original byte range [31813, 33346). Retained reference `source-window-27`.

```text
TclNRSourceObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;
    int result;
    void **pkgFiles = NULL;
    void *names = NULL;

    if (objc < 2 || objc > 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding encoding? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    } else if (objc == 3) {
	/* Handle undocumented -nopkg option. This should only be
	 * used by the internal ::tcl::Pkg::source utility function. */
	static const char *const nopkgoptions[] = {
	    "-nopkg", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], nopkgoptions,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	pkgFiles = (void **)Tcl_GetAssocData(interp, "tclPkgFiles", NULL);
	/* Make sure that during the following TclNREvalFile no filenames
	 * are recorded for inclusion in the "package files" command */
	names = *pkgFiles;
	*pkgFiles = NULL;
    }
    result = TclNREvalFile(interp, fileName, encodingName);
    if (pkgFiles) {
	/* restore "tclPkgFiles" assocdata to how it was. */
	*pkgFiles = names;
    }
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1690–1825. Line unit: physical LF only. Original byte range [44472, 48198). Retained reference `source-window-28`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter that evaluates the script. */
    Tcl_Obj *pathPtr,		/* Pathname of the file to process.
				 * Tilde-substitution is performed on this
				 * pathname. */
    const char *encodingName)	/* Either the name of an encoding or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_Size length;
    int result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }

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
	Tcl_CloseEx(interp,chan,0);
	return result;
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
	goto end;
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
	goto end;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

    /*
     * Restore the original iPtr->scriptFile value, but because the value may
     * have hanged during evaluation, don't assume it currently points to
     * pathPtr.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information about where the error occurred.
	 */

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : (int)length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SourceCoreCommand`, lines 16711–16716. Line unit: physical LF only. Original byte range [529211, 529412). Retained reference `source-window-29`.

```text
static int Jim_SourceCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retval = Jim_EvalFile(interp, Jim_String(argv[1]));

    return retval == JIM_RETURN ? JIM_OK : retval;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 347–386. Line unit: physical LF only. Original byte range [12281, 13438). Retained reference `source-window-30`.

```text
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

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 544–634. Line unit: physical LF only. Original byte range [17524, 20200). Retained reference `source-window-31`.

```text
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;

    if (encodingsInitialized) {
	return;
    }

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings. Note that the UTF-8 to UTF-8
     * translation is not a no-op, because it will turn a stream of improperly
     * formed UTF-8 into a properly formed stream.
     */

    type.encodingName	= "identity";
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;

    defaultEncoding	= Tcl_CreateEncoding(&type);
    tclIdentityEncoding = Tcl_GetEncoding(NULL, type.encodingName);
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

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    {
	TableEncodingData *dataPtr = (TableEncodingData *)
		ckalloc(sizeof(TableEncodingData));
	unsigned size;
	unsigned short i;

	memset(dataPtr, 0, sizeof(TableEncodingData));
	dataPtr->fallback = '?';

	size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
	dataPtr->toUnicode = (unsigned short **) ckalloc(size);
	memset(dataPtr->toUnicode, 0, size);
	dataPtr->fromUnicode = (unsigned short **) ckalloc(size);
	memset(dataPtr->fromUnicode, 0, size);

	dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
	dataPtr->fromUnicode[0] = (unsigned short *)
		(dataPtr->fromUnicode + 256);
	for (i=1 ; i<256 ; i++) {
	    dataPtr->toUnicode[i] = emptyPage;
	    dataPtr->fromUnicode[i] = emptyPage;
	}

	for (i=0 ; i<256 ; i++) {
	    dataPtr->toUnicode[0][i] = i;
	    dataPtr->fromUnicode[0][i] = i;
	}

	type.encodingName	= "iso8859-1";
	type.toUtfProc		= Iso88591ToUtfProc;
	type.fromUtfProc	= Iso88591FromUtfProc;
	type.freeProc		= TableFreeProc;
	type.nullSize		= 1;
	type.clientData		= dataPtr;
	Tcl_CreateEncoding(&type);
    }

    encodingsInitialized = 1;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 533–622. Line unit: physical LF only. Original byte range [16889, 19612). Retained reference `source-window-32`.

```text
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

    if (encodingsInitialized) {
	return;
    }

    isLe.s = 1;
    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= "identity";
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

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
    type.clientData	= INT2PTR(isLe.c);
    Tcl_CreateEncoding(&type);

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)ckalloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)ckalloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)ckalloc(size);
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

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 524–659. Line unit: physical LF only. Original byte range [16679, 20842). Retained reference `source-window-33`.

```text
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

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 527–667. Line unit: physical LF only. Original byte range [16867, 21139). Retained reference `source-window-34`.

```text
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

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_SourceObjCmd`, lines 1056–1068. Line unit: physical LF only. Original byte range [28791, 29163). Retained reference `read646-source-window-0`.

```text
Tcl_SourceObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "fileName");
	return TCL_ERROR;
    }

    return Tcl_FSEvalFile(interp, objv[1]);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `Tcl_SourceObjCmd`, lines 936–966. Line unit: physical LF only. Original byte range [22991, 23739). Retained reference `read646-source-window-1`.

```text
Tcl_SourceObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;

    if (objc != 2 && objc !=4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding name? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    }

    return Tcl_FSEvalFileEx(interp, fileName, encodingName);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1770–1892. Line unit: physical LF only. Original byte range [50144, 53829). Retained reference `read646-source-window-2`.

```text
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

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 985–1015. Line unit: physical LF only. Original byte range [24875, 25627). Retained reference `read646-source-window-3`.

```text
TclNRSourceObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;

    if (objc != 2 && objc !=4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding name? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    }

    return TclNREvalFile(interp, fileName, encodingName);
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1727–1860. Line unit: physical LF only. Original byte range [48406, 52138). Retained reference `read646-source-window-4`.

```text
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
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
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
	    return result;
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
	goto end;
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
	goto end;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280 Force the evaluator to open a frame for a sourced file.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

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

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 1104–1158. Line unit: physical LF only. Original byte range [28601, 30131). Retained reference `read646-source-window-5`.

```text
TclNRSourceObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;
    int result;
    void **pkgFiles = NULL;
    void *names = NULL;

    if (objc < 2 || objc > 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding encoding? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    } else if (objc == 3) {
	/* Handle undocumented -nopkg option. This should only be
	 * used by the internal ::tcl::Pkg::source utility function. */
	static const char *const nopkgoptions[] = {
	    "-nopkg", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], nopkgoptions,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	pkgFiles = (void **)Tcl_GetAssocData(interp, "tclPkgFiles", NULL);
	/* Make sure that during the following TclNREvalFile no filenames
	 * are recorded for inclusion in the "package files" command */
	names = *pkgFiles;
	*pkgFiles = NULL;
    }
    result = TclNREvalFile(interp, fileName, encodingName);
    if (pkgFiles) {
	/* restore "tclPkgFiles" assocdata to how it was. */
	*pkgFiles = names;
    }
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1691–1826. Line unit: physical LF only. Original byte range [44401, 48127). Retained reference `read646-source-window-6`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter that evaluates the script. */
    Tcl_Obj *pathPtr,		/* Pathname of the file to process.
				 * Tilde-substitution is performed on this
				 * pathname. */
    const char *encodingName)	/* Either the name of an encoding or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_Size length;
    int result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }

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
	Tcl_CloseEx(interp,chan,0);
	return result;
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
	goto end;
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
	goto end;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

    /*
     * Restore the original iPtr->scriptFile value, but because the value may
     * have hanged during evaluation, don't assume it currently points to
     * pathPtr.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information about where the error occurred.
	 */

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : (int)length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 1169–1223. Line unit: physical LF only. Original byte range [31813, 33346). Retained reference `read646-source-window-7`.

```text
TclNRSourceObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;
    int result;
    void **pkgFiles = NULL;
    void *names = NULL;

    if (objc < 2 || objc > 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding encoding? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    } else if (objc == 3) {
	/* Handle undocumented -nopkg option. This should only be
	 * used by the internal ::tcl::Pkg::source utility function. */
	static const char *const nopkgoptions[] = {
	    "-nopkg", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], nopkgoptions,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	pkgFiles = (void **)Tcl_GetAssocData(interp, "tclPkgFiles", NULL);
	/* Make sure that during the following TclNREvalFile no filenames
	 * are recorded for inclusion in the "package files" command */
	names = *pkgFiles;
	*pkgFiles = NULL;
    }
    result = TclNREvalFile(interp, fileName, encodingName);
    if (pkgFiles) {
	/* restore "tclPkgFiles" assocdata to how it was. */
	*pkgFiles = names;
    }
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1690–1825. Line unit: physical LF only. Original byte range [44472, 48198). Retained reference `read646-source-window-8`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter that evaluates the script. */
    Tcl_Obj *pathPtr,		/* Pathname of the file to process.
				 * Tilde-substitution is performed on this
				 * pathname. */
    const char *encodingName)	/* Either the name of an encoding or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_Size length;
    int result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }

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
	Tcl_CloseEx(interp,chan,0);
	return result;
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
	goto end;
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
	goto end;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

    /*
     * Restore the original iPtr->scriptFile value, but because the value may
     * have hanged during evaluation, don't assume it currently points to
     * pathPtr.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information about where the error occurred.
	 */

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : (int)length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SourceCoreCommand`, lines 16711–16716. Line unit: physical LF only. Original byte range [529211, 529412). Retained reference `read646-source-window-9`.

```text
static int Jim_SourceCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retval = Jim_EvalFile(interp, Jim_String(argv[1]));

    return retval == JIM_RETURN ? JIM_OK : retval;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIOUtil.c`, function `Tcl_FSEvalFile`, lines 1704–1810. Line unit: physical LF only. Original byte range [52367, 55548). Retained reference `read646-source-window-10`.

```text
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

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalFile`, lines 12120–12168. Line unit: physical LF only. Original byte range [384075, 385640). Retained reference `read646-source-window-11`.

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

## Consumer bindings

- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `resolve_original_encoding_name`: Resolve only the original encoding-name C-string extent, with selected C diagnostics, independent reset/channel purpose and an actual catalogue capability for names beyond the implemented codec subset.
- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `prepare_source_file`: Coordinate original operand String, file open, encoding selection, read and same-handle close. Check each stage on success and failure; evaluation and the independently selected filesystem filename facts retain their own purposes.
- [rust/tcl-registry/src/native_encoding.rs](../../../../rust/tcl-registry/src/native_encoding.rs), `InvocationDialect::native_encoding_name_protocol`: Select actual C encoding-name presentation independently of original object authority; Jim encoding-command absence stays explicit.
- [rust/tcl-registry/src/native_encoding.rs](../../../../rust/tcl-registry/src/native_encoding.rs), `InvocationDialect::native_source_file_protocol`: Select actual source default and explicit grammar purpose without issuing file, String or catalogue authority.
- [rust/tcl-platform/src/lib.rs](../../../../rust/tcl-platform/src/lib.rs), `EncodingCatalogue`: Supply actual byte-name lookup capability; a known miss and unavailable knowledge remain different results.
- [rust/tcl-platform/src/lib.rs](../../../../rust/tcl-platform/src/lib.rs), `SourceFile`: Retain one actually opened handle for the separate read and mandatory close stages.
- [rust/tcl-host-native/src/lib.rs](../../../../rust/tcl-host-native/src/lib.rs), `NativeFs::open_source_bytes`: Open the actual original byte path before reading and retain the owned file handle for source configuration, read and close.
- [runtime/rust/src/mem_fs.rs](../../../../runtime/rust/src/mem_fs.rs), `MemFs::open_source_bytes`: Capture the current VFS file backing at open and expose read separately, preserving that handle through cleanup.
- [rust/tcl-vm/src/command/native_encoding_source.rs](../../../../rust/tcl-vm/src/command/native_encoding_source.rs), `set_system`: Resolve original system encoding bytes under the actual Name scope and header, capturing reset default independently of mutable current encoding.
- [rust/tcl-vm/src/command/native_encoding_source.rs](../../../../rust/tcl-vm/src/command/native_encoding_source.rs), `source`: Retain original Name operation, selected source operands, package/frame effects and full completion through shared file staging.
- [runtime/rust/src/interp/native_encoding_source.rs](../../../../runtime/rust/src/interp/native_encoding_source.rs), `lookup`: Resolve actual argv encoding-name bytes using their original header/currency and supplied Host catalogue.
- [runtime/rust/src/cmd_fs.rs](../../../../runtime/rust/src/cmd_fs.rs), `source_cmd`: Retain original command scope and argv through selected source encoding, file stages and evaluation.
- [runtime/rust/src/cmd_misc.rs](../../../../runtime/rust/src/cmd_misc.rs), `encoding_cmd`: Retain original command scope and argv through system encoding lookup and mutation.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_byte_names_keep_c_extent_raw_diagnostics_and_actual_reset_default` (linked): Resolve the original encoding-name C-string extent, preserve invalid/case/modified-zero bytes in selected diagnostics, and restore the supplied reset default without a catalogue query.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_channel_pseudo_encodings_stay_distinct_from_system_reset_and_catalogue_absence` (linked): Keep channel empty/binary handling separate from system reset, and distinguish unavailable catalogue knowledge from a genuine missing-name diagnostic.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_source_stage_failure_stops_before_later_lookup_or_backend` (linked): Check the original String and open stage on both outcomes; an earlier getter, file or currentness failure prevents later catalogue or backend work.
- [rust/tcl-vm/src/command/native_encoding_source/tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_source/tests.rs), `command::native_encoding_source::tests::original_system_names_keep_resident_bytes_zero_queries_and_captured_default` (linked): Use resident original names without Host catalogue queries for supported encodings/reset, and retain exact unknown bytes plus the previously captured default.
- [rust/tcl-vm/src/command/native_encoding_source/tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_source/tests.rs), `command::native_encoding_source::tests::original_source_opens_before_lookup_and_consumes_selected_encoding` (linked): Consume the supplied encoding when reading the script and preserve missing-file precedence before a reached encoding lookup.
- [runtime/rust/src/interp/native_encoding_source/tests.rs](../../../../runtime/rust/src/interp/native_encoding_source/tests.rs), `interp::native_encoding_source::tests::original_source_argument_encoding_is_consumed_and_missing_file_precedes_lookup` (linked): The actual Runtime argv owner supplies the selected encoding to source conversion; a missing file reports ENOENT before an encoding catalogue query.
- [runtime/rust/src/interp/native_encoding_source/tests.rs](../../../../runtime/rust/src/interp/native_encoding_source/tests.rs), `interp::native_encoding_source::tests::original_system_reset_and_catalogue_callback_keep_original_currency` (linked): Restore the independently captured encoding default and retain the exact original currency refusal after a supplied catalogue callback changes and restores interpreter context.

## Evidence

- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/8.4.20](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/8.4.20)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/8.5.19](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/8.5.19)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/8.6.18](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/8.6.18)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/9.0.4](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/9.0.4)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/9.1.0](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/9.1.0)
- [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/Jim](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/646/Jim)
- [runtime](../../../../runtime)
- [runtime/rust/src](../../../../runtime/rust/src)
- [runtime/rust/src/interp](../../../../runtime/rust/src/interp)
- [rust/tcl-cmd-core/src](../../../../rust/tcl-cmd-core/src)
- [rust/tcl-cmd-core/src/encoding](../../../../rust/tcl-cmd-core/src/encoding)
- [rust/tcl-host-native](../../../../rust/tcl-host-native)
- [rust/tcl-platform](../../../../rust/tcl-platform)
- [rust/tcl-registry](../../../../rust/tcl-registry)
- [rust/tcl-vm/src](../../../../rust/tcl-vm/src)
- [rust/tcl-vm/src/command](../../../../rust/tcl-vm/src/command)
