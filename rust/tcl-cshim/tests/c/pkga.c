/*
 * tcl-lsp — a language server and toolchain for Tcl
 * Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: AGPL-3.0-or-later
 */

/*
 * pkga.c — a test extension shaped like the `pkga` package Tcl ships in
 * unix/dltest: `pkga_eq` and `pkga_quote` exactly as there, plus `pkga_calc`
 * (Tcl_GetIndexFromObj subcommand dispatch over the whole value API) and a
 * clientData-carrying counter with a delete procedure.
 *
 * The same source compiles against a real Tcl 9 `tcl.h`, which is how the
 * expected strings in the integration tests were captured.
 */

#include "tclshim.h"
#include <string.h>

#if TCL_MAJOR_VERSION < 9
#   define Tcl_Size int
#endif

static int
Pkga_EqObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    int result;
    const char *str1, *str2;
    Tcl_Size len1, len2;
    (void)dummy;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string1 string2");
	return TCL_ERROR;
    }
    str1 = Tcl_GetStringFromObj(objv[1], &len1);
    str2 = Tcl_GetStringFromObj(objv[2], &len2);
    len1 = Tcl_NumUtfChars(str1, len1);
    len2 = Tcl_NumUtfChars(str2, len2);
    if (len1 == len2) {
	result = (Tcl_UtfNcmp(str1, str2, (size_t)len1) == 0);
    } else {
	result = 0;
    }
    Tcl_SetObjResult(interp, Tcl_NewIntObj(result));
    return TCL_OK;
}

static int
Pkga_QuoteObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    (void)dummy;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "value");
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, objv[1]);
    return TCL_OK;
}

static const char *const calcSubcommands[] = {
    "add", "sub", "range", "sum", "neg", "not", "fail", "join", "len", "dup",
    NULL
};
enum calcSubcommand {
    CALC_ADD, CALC_SUB, CALC_RANGE, CALC_SUM, CALC_NEG, CALC_NOT, CALC_FAIL,
    CALC_JOIN, CALC_LEN, CALC_DUP
};

static int
Pkga_CalcObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    int index;
    (void)dummy;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "subcommand ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], calcSubcommands, "subcommand", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum calcSubcommand)index) {
    case CALC_ADD: {
	int first, second;

	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "n m");
	    return TCL_ERROR;
	}
	if (Tcl_GetIntFromObj(interp, objv[2], &first) != TCL_OK
		|| Tcl_GetIntFromObj(interp, objv[3], &second) != TCL_OK) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewIntObj(first + second));
	return TCL_OK;
    }
    case CALC_SUB: {
	long first, second;

	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "n m");
	    return TCL_ERROR;
	}
	if (Tcl_GetLongFromObj(interp, objv[2], &first) != TCL_OK
		|| Tcl_GetLongFromObj(interp, objv[3], &second) != TCL_OK) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewLongObj(first - second));
	return TCL_OK;
    }
    case CALC_RANGE: {
	int count, i;
	Tcl_Obj *list;

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "count");
	    return TCL_ERROR;
	}
	if (Tcl_GetIntFromObj(interp, objv[2], &count) != TCL_OK) {
	    return TCL_ERROR;
	}
	list = Tcl_NewListObj(0, NULL);
	for (i = 0; i < count; i++) {
	    Tcl_ListObjAppendElement(interp, list, Tcl_NewIntObj(i));
	}
	Tcl_SetObjResult(interp, list);
	return TCL_OK;
    }
    case CALC_SUM: {
	Tcl_Size count, i;
	Tcl_Obj **elements;
	Tcl_WideInt total = 0, item;

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "list");
	    return TCL_ERROR;
	}
	if (Tcl_ListObjGetElements(interp, objv[2], &count, &elements) != TCL_OK) {
	    return TCL_ERROR;
	}
	for (i = 0; i < count; i++) {
	    if (Tcl_GetWideIntFromObj(interp, elements[i], &item) != TCL_OK) {
		return TCL_ERROR;
	    }
	    total += item;
	}
	Tcl_SetObjResult(interp, Tcl_NewWideIntObj(total));
	return TCL_OK;
    }
    case CALC_NEG: {
	double value;

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "x");
	    return TCL_ERROR;
	}
	if (Tcl_GetDoubleFromObj(interp, objv[2], &value) != TCL_OK) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewDoubleObj(-value));
	return TCL_OK;
    }
    case CALC_NOT: {
	int value;

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "boolean");
	    return TCL_ERROR;
	}
	if (Tcl_GetBooleanFromObj(interp, objv[2], &value) != TCL_OK) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(!value));
	return TCL_OK;
    }
    case CALC_FAIL: {
	char message[64];

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "message");
	    return TCL_ERROR;
	}
	strncpy(message, Tcl_GetString(objv[2]), sizeof(message) - 1);
	message[sizeof(message) - 1] = '\0';
	Tcl_SetResult(interp, message, TCL_VOLATILE);
	Tcl_SetErrorCode(interp, "PKGA", "FAIL", message, (char *)NULL);
	return TCL_ERROR;
    }
    case CALC_JOIN:
	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "a b");
	    return TCL_ERROR;
	}
	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, Tcl_GetString(objv[2]), "+",
		Tcl_GetString(objv[3]), (char *)NULL);
	return TCL_OK;
    case CALC_LEN: {
	Tcl_Size length;

	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "list");
	    return TCL_ERROR;
	}
	if (Tcl_ListObjLength(interp, objv[2], &length) != TCL_OK) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, Tcl_NewWideIntObj((Tcl_WideInt)length));
	return TCL_OK;
    }
    case CALC_DUP: {
	Tcl_Obj *copy, *pair[2];

	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "list element");
	    return TCL_ERROR;
	}
	/* Append to a duplicate, never to the argument: the result shows the
	 * original beside the extended copy. */
	copy = Tcl_DuplicateObj(objv[2]);
	if (Tcl_IsShared(copy)) {
	    Tcl_SetResult(interp, (char *)"duplicate unexpectedly shared",
		    TCL_STATIC);
	    return TCL_ERROR;
	}
	if (Tcl_ListObjAppendElement(interp, copy, objv[3]) != TCL_OK) {
	    Tcl_DecrRefCount(copy);
	    return TCL_ERROR;
	}
	pair[0] = objv[2];
	pair[1] = copy;
	Tcl_SetObjResult(interp, Tcl_NewListObj(2, pair));
	return TCL_OK;
    }
    }
    return TCL_OK;
}

/* A counter carried through clientData, with a delete procedure. It is a
 * process-wide static shared by every interpreter that loads the package
 * (tests load it concurrently), so it is never reset. */
struct counter {
    int calls;
    int deleted;
};
static struct counter pkgaCounter;

static int
Pkga_CountObjCmd(void *clientData, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    struct counter *counter = (struct counter *)clientData;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }
    counter->calls++;
    Tcl_SetObjResult(interp, Tcl_NewIntObj(counter->calls));
    return TCL_OK;
}

static void
Pkga_CountDeleteProc(void *clientData)
{
    ((struct counter *)clientData)->deleted = 1;
}

static int
Pkga_ForgetObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    (void)dummy;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, Tcl_NewIntObj(
	    Tcl_DeleteCommand(interp, "pkga_count") == 0 && pkgaCounter.deleted));
    return TCL_OK;
}

DLLEXPORT int
Pkga_Init(Tcl_Interp *interp)
{
    int code;

    if (Tcl_InitStubs(interp, "8.5-", 0) == NULL) {
	return TCL_ERROR;
    }
    code = Tcl_PkgProvide(interp, "pkga", "1.0");
    if (code != TCL_OK) {
	return code;
    }
    Tcl_CreateObjCommand(interp, "pkga_eq", Pkga_EqObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "pkga_quote", Pkga_QuoteObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "pkga_calc", Pkga_CalcObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "pkga_count", Pkga_CountObjCmd, &pkgaCounter,
	    Pkga_CountDeleteProc);
    Tcl_CreateObjCommand(interp, "pkga_forget", Pkga_ForgetObjCmd, NULL, NULL);
    return TCL_OK;
}
