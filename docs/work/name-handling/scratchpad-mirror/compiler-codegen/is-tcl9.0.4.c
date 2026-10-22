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
    int range, allowEmpty = 0, end;
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

#define GotLiteral(tokenPtr, word) \
    ((tokenPtr)->type == TCL_TOKEN_SIMPLE_WORD &&			\
     (tokenPtr)[1].size > 1 &&						\
     (tokenPtr)[1].start[0] == word[0] &&				\
     strncmp((tokenPtr)[1].start, (word), (tokenPtr)[1].size) == 0)

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
	if (!GotLiteral(tokenPtr, "-strict")) {
	    return TCL_ERROR;
	}
	tokenPtr = TokenAfter(tokenPtr);
    }
#undef GotLiteral

    /*
     * Compile the code. There are several main classes of check here.
     *	1. Character classes
     *	2. Booleans
     *	3. Integers
     *	4. Floats
     *	5. Lists
     */

    CompileWord(envPtr, tokenPtr, interp, (int)parsePtr->numWords-1);

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
	    OP1(	STR_CLASS, strClassType);
	} else {
	    int over, over2;

	    OP(		DUP);
	    OP1(	STR_CLASS, strClassType);
	    JUMP1(	JUMP_TRUE, over);
	    OP(		POP);
	    PUSH(	"0");
	    JUMP1(	JUMP, over2);
	    FIXJUMP1(over);
	    PUSH(	"");
	    OP(		STR_NEQ);
	    FIXJUMP1(over2);
	}
	return TCL_OK;

    case STR_IS_BOOL:
    case STR_IS_FALSE:
    case STR_IS_TRUE:
	OP(		TRY_CVT_TO_BOOLEAN);
	switch (t) {
	    int over, over2;

	case STR_IS_BOOL:
	    if (allowEmpty) {
		JUMP1(	JUMP_TRUE, over);
		PUSH(	"");
		OP(	STR_EQ);
		JUMP1(	JUMP, over2);
		FIXJUMP1(over);
		OP(	POP);
		PUSH(	"1");
		FIXJUMP1(over2);
	    } else {
		OP4(	REVERSE, 2);
		OP(	POP);
	    }
	    return TCL_OK;
	case STR_IS_TRUE:
	    JUMP1(	JUMP_TRUE, over);
	    if (allowEmpty) {
		PUSH(	"");
		OP(	STR_EQ);
	    } else {
		OP(	POP);
		PUSH(	"0");
	    }
	    FIXJUMP1(	over);
	    OP(		LNOT);
	    OP(		LNOT);
	    return TCL_OK;
	case STR_IS_FALSE:
	    JUMP1(	JUMP_TRUE, over);
	    if (allowEmpty) {
		PUSH(	"");
		OP(	STR_NEQ);
	    } else {
		OP(	POP);
		PUSH(	"1");
	    }
	    FIXJUMP1(	over);
	    OP(		LNOT);
	    return TCL_OK;
	default:
	    break;
	}
    break;

    case STR_IS_DOUBLE: {
	int satisfied, isEmpty;

	if (allowEmpty) {
	    OP(		DUP);
	    PUSH(	"");
	    OP(		STR_EQ);
	    JUMP1(	JUMP_TRUE, isEmpty);
	    OP(		NUM_TYPE);
	    JUMP1(	JUMP_TRUE, satisfied);
	    PUSH(	"0");
	    JUMP1(	JUMP, end);
	    FIXJUMP1(	isEmpty);
	    OP(		POP);
	    FIXJUMP1(	satisfied);
	} else {
	    OP(		NUM_TYPE);
	    JUMP1(	JUMP_TRUE, satisfied);
	    PUSH(	"0");
	    JUMP1(	JUMP, end);
	    TclAdjustStackDepth(-1, envPtr);
	    FIXJUMP1(	satisfied);
	}
	PUSH(		"1");
	FIXJUMP1(	end);
	return TCL_OK;
    }

    case STR_IS_INT:
    case STR_IS_WIDE:
    case STR_IS_ENTIER:
	if (allowEmpty) {
	    int testNumType;

	    OP(		DUP);
	    OP(		NUM_TYPE);
	    OP(		DUP);
	    JUMP1(	JUMP_TRUE, testNumType);
	    OP(		POP);
	    PUSH(	"");
	    OP(		STR_EQ);
	    JUMP1(	JUMP, end);
	    TclAdjustStackDepth(1, envPtr);
	    FIXJUMP1(	testNumType);
	    OP4(	REVERSE, 2);
	    OP(		POP);
	} else {
	    OP(		NUM_TYPE);
	    OP(		DUP);
	    JUMP1(	JUMP_FALSE, end);
	}

	switch (t) {
	case STR_IS_WIDE:
	    PUSH(	"2");
	    OP(		LE);
	    break;
	case STR_IS_INT:
	case STR_IS_ENTIER:
	    PUSH(	"3");
	    OP(		LE);
	    break;
	default:
	    break;
	}
	FIXJUMP1(	end);
	return TCL_OK;
    case STR_IS_DICT:
	range = TclCreateExceptRange(CATCH_EXCEPTION_RANGE, envPtr);
	OP4(		BEGIN_CATCH4, range);
	ExceptionRangeStarts(envPtr, range);
	OP(		DUP);
	OP(		DICT_VERIFY);
	ExceptionRangeEnds(envPtr, range);
	ExceptionRangeTarget(envPtr, range, catchOffset);
	OP(		POP);
	OP(		PUSH_RETURN_CODE);
	OP(		END_CATCH);
	OP(		LNOT);
	return TCL_OK;
    case STR_IS_LIST:
	range = TclCreateExceptRange(CATCH_EXCEPTION_RANGE, envPtr);
	OP4(		BEGIN_CATCH4, range);
	ExceptionRangeStarts(envPtr, range);
	OP(		DUP);
	OP(		LIST_LENGTH);
	OP(		POP);
	ExceptionRangeEnds(envPtr, range);
	ExceptionRangeTarget(envPtr, range, catchOffset);
	OP(		POP);
	OP(		PUSH_RETURN_CODE);
	OP(		END_CATCH);
	OP(		LNOT);
	return TCL_OK;
    }

    return TclCompileBasicMin0ArgCmd(interp, parsePtr, cmdPtr, envPtr);
}
