TclCompileStringIsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    static const char *const isClasses[] = {
	"alnum",	"alpha",	"ascii",	"control",
	"boolean",	"dict",		"digit",	"double",
	"entier",	"false",	"graph",	"integer",
	"list",		"lower",	"print",	"punct",
	"space",	"true",		"upper",
	"wideinteger", "wordchar",	"xdigit",	NULL
    };
    enum isClassesEnum {
	STR_IS_ALNUM,	STR_IS_ALPHA,	STR_IS_ASCII,	STR_IS_CONTROL,
	STR_IS_BOOL,	STR_IS_DICT,	STR_IS_DIGIT,	STR_IS_DOUBLE,
	STR_IS_ENTIER,	STR_IS_FALSE,	STR_IS_GRAPH,	STR_IS_INT,
	STR_IS_LIST,	STR_IS_LOWER,	STR_IS_PRINT,	STR_IS_PUNCT,
	STR_IS_SPACE,	STR_IS_TRUE,	STR_IS_UPPER,
	STR_IS_WIDE,	STR_IS_WORD,	STR_IS_XDIGIT
    } t;
    int allowEmpty = 0;
    Tcl_ExceptionRange range;
    Tcl_BytecodeLabel end;
    InstStringClassType strClassType;
    Tcl_Obj *isClass;

    if (parsePtr->numWords < 3 || parsePtr->numWords > 6) {
	return TCL_ERROR;
    }
    TclNewObj(isClass);
    if (!TclWordKnownAtCompileTime(tokenPtr, isClass)) {
	Tcl_DecrRefCount(isClass);
	return TCL_ERROR;
    } else if (Tcl_GetIndexFromObj(interp, isClass, isClasses, "class", 0,
	    &t) != TCL_OK) {
	Tcl_DecrRefCount(isClass);
	TclCompileSyntaxError(interp, envPtr);
	return TCL_OK;
    }
    Tcl_DecrRefCount(isClass);

    /*
     * Cannot handle the -failindex option at all, and that's the only legal
     * way to have more than 4 arguments.
     */

    if (parsePtr->numWords != 3 && parsePtr->numWords != 4) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(tokenPtr);
    if (parsePtr->numWords == 3) {
	allowEmpty = 1;
    } else {
	if (!IS_TOKEN_PREFIX(tokenPtr, 2, "-strict")) {
	    return TCL_ERROR;
	}
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Compile the code. There are several main classes of check here.
     *	1. Character classes
     *	2. Booleans
     *	3. Integers
     *	4. Floats
     *	5. Lists
     */

    PUSH_TOKEN(			tokenPtr, parsePtr->numWords - 1);

    switch (t) {
    case STR_IS_ALNUM:
	strClassType = STR_CLASS_ALNUM;
	goto compileStrClass;
    case STR_IS_ALPHA:
	strClassType = STR_CLASS_ALPHA;
	goto compileStrClass;
    case STR_IS_ASCII:
	strClassType = STR_CLASS_ASCII;
	goto compileStrClass;
    case STR_IS_CONTROL:
	strClassType = STR_CLASS_CONTROL;
	goto compileStrClass;
    case STR_IS_DIGIT:
	strClassType = STR_CLASS_DIGIT;
	goto compileStrClass;
    case STR_IS_GRAPH:
	strClassType = STR_CLASS_GRAPH;
	goto compileStrClass;
    case STR_IS_LOWER:
	strClassType = STR_CLASS_LOWER;
	goto compileStrClass;
    case STR_IS_PRINT:
	strClassType = STR_CLASS_PRINT;
	goto compileStrClass;
    case STR_IS_PUNCT:
	strClassType = STR_CLASS_PUNCT;
	goto compileStrClass;
    case STR_IS_SPACE:
	strClassType = STR_CLASS_SPACE;
	goto compileStrClass;
    case STR_IS_UPPER:
	strClassType = STR_CLASS_UPPER;
	goto compileStrClass;
    case STR_IS_WORD:
	strClassType = STR_CLASS_WORD;
	goto compileStrClass;
    case STR_IS_XDIGIT:
	strClassType = STR_CLASS_XDIGIT;
    compileStrClass:
	if (allowEmpty) {
	    OP1(		STR_CLASS, strClassType);
	} else {
	    Tcl_BytecodeLabel over, over2;

	    OP(			DUP);
	    OP1(		STR_CLASS, strClassType);
	    FWDJUMP(		JUMP_TRUE, over);
	    OP(			POP);
	    PUSH(		"0");
	    FWDJUMP(		JUMP, over2);
	    FWDLABEL(	over);
	    OP(			IS_EMPTY);
	    OP(			LNOT);
	    FWDLABEL(	over2);
	}
	return TCL_OK;

    case STR_IS_BOOL:
    case STR_IS_FALSE:
    case STR_IS_TRUE:
	OP(			TRY_CVT_TO_BOOLEAN);
	switch (t) {
	    Tcl_BytecodeLabel over, over2;

	case STR_IS_BOOL:
	    if (allowEmpty) {
		FWDJUMP(	JUMP_TRUE, over);
		OP(		IS_EMPTY);
		FWDJUMP(	JUMP, over2);
		FWDLABEL(over);
		OP(		POP);
		PUSH(		"1");
		FWDLABEL(over2);
	    } else {
		OP(		SWAP);
		OP(		POP);
	    }
	    return TCL_OK;
	case STR_IS_TRUE:
	    FWDJUMP(		JUMP_TRUE, over);
	    if (allowEmpty) {
		OP(		IS_EMPTY);
	    } else {
		OP(		POP);
		PUSH(		"0");
	    }
	    FWDJUMP(		JUMP, over2);
	    FWDLABEL(	over);
	    // Normalize the boolean value
	    OP(			LNOT);
	    OP(			LNOT);
	    FWDLABEL(	over2);
	    return TCL_OK;
	case STR_IS_FALSE:
	    FWDJUMP(		JUMP_TRUE, over);
	    if (allowEmpty) {
		OP(		IS_EMPTY);
	    } else {
		OP(		POP);
		PUSH(		"0");
	    }
	    FWDJUMP(		JUMP, over2);
	    FWDLABEL(	over);
	    OP(			LNOT);
	    FWDLABEL(	over2);
	    return TCL_OK;
	default:
	    break;
	}
    break;

    case STR_IS_DOUBLE: {
	Tcl_BytecodeLabel satisfied, isEmpty;

	if (allowEmpty) {
	    OP(			DUP);
	    OP(			IS_EMPTY);
	    FWDJUMP(		JUMP_TRUE, isEmpty);
	    OP(			NUM_TYPE);
	    FWDJUMP(		JUMP_TRUE, satisfied);
	    PUSH(		"0");
	    FWDJUMP(		JUMP, end);
	    FWDLABEL(	isEmpty);
	    OP(			POP);
	    FWDLABEL(	satisfied);
	} else {
	    OP(			NUM_TYPE);
	    FWDJUMP(		JUMP_TRUE, satisfied);
	    PUSH(		"0");
	    FWDJUMP(		JUMP, end);
	    STKDELTA(-1);
	    FWDLABEL(	satisfied);
	}
	PUSH(			"1");
	FWDLABEL(	end);
	return TCL_OK;
    }

    case STR_IS_INT:
    case STR_IS_WIDE:
    case STR_IS_ENTIER:
	if (allowEmpty) {
	    Tcl_BytecodeLabel testNumType;

	    OP(			DUP);
	    OP(			NUM_TYPE);
	    OP(			DUP);
	    FWDJUMP(		JUMP_TRUE, testNumType);
	    OP(			POP);
	    OP(			IS_EMPTY);
	    FWDJUMP(		JUMP, end);
	    STKDELTA(+1);
	    FWDLABEL(	testNumType);
	    OP(			SWAP);
	    OP(			POP);
	} else {
	    OP(			NUM_TYPE);
	    OP(			DUP);
	    FWDJUMP(		JUMP_FALSE, end);
	}

	switch (t) {
	case STR_IS_WIDE:
	    PUSH(		"2");
	    OP(			LE);
	    break;
	case STR_IS_INT:
	case STR_IS_ENTIER:
	    PUSH(		"3");
	    OP(			LE);
	    break;
	default:
	    break;
	}
	FWDLABEL(	end);
	return TCL_OK;
    case STR_IS_DICT:
	range = MAKE_CATCH_RANGE();
	OP4(			BEGIN_CATCH, range);
	OP(			DUP);
	CATCH_RANGE(range) {
	    OP(			DICT_VERIFY);
	}
	CATCH_TARGET(	range);
	OP(			POP);
	OP(			PUSH_RETURN_CODE);
	OP(			END_CATCH);
	OP(			LNOT);
	return TCL_OK;
    case STR_IS_LIST:
	range = MAKE_CATCH_RANGE();
	OP4(			BEGIN_CATCH, range);
	OP(			DUP);
	CATCH_RANGE(range) {
	    OP(			LIST_LENGTH);
	}
	OP(			POP);
	CATCH_TARGET(	range);
	OP(			POP);
	OP(			PUSH_RETURN_CODE);
	OP(			END_CATCH);
	OP(			LNOT);
	return TCL_OK;
    }

    return TclCompileBasicMin0ArgCmd(interp, parsePtr, cmdPtr, envPtr);
}
