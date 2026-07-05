// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/*
 * pkgt.c --
 *
 *	This file contains a simple Tcl package "pkgt" that is intended for
 *	testing the Tcl dynamic loading facilities.
 *
 * Copyright © 1995 Sun Microsystems, Inc.
 *
 * See the file "license.terms" for information on usage and redistribution of
 * this file, and for a DISCLAIMER OF ALL WARRANTIES.
 */

#undef STATIC_BUILD
#include "tcl.h"

static int TraceProc2 (
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size level,
    const char *command,
    Tcl_Command commandInfo,
    Tcl_Size objc,
    struct Tcl_Obj *const *objv)
{
    (void)clientData;
    (void)interp;
    (void)level;
    (void)command;
    (void)commandInfo;
    (void)objc;
    (void)objv;

    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Pkgt_EqObjCmd2 --
 *
 *	This procedure is invoked to process the "pkgt_eq" Tcl command. It
 *	expects two arguments and returns 1 if they are the same, 0 if they
 *	are different.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
Pkgt_EqObjCmd2(
    void *dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_WideInt result;
    const char *str1, *str2;
    Tcl_Size len1, len2;
    (void)dummy;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv,  "string1 string2");
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
    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(result));
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Pkgt_Init --
 *
 *	This is a package initialization procedure, which is called by Tcl
 *	when this package is to be added to an interpreter.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

DLLEXPORT int
Pkgt_Init(
    Tcl_Interp *interp)		/* Interpreter in which the package is to be
				 * made available. */
{
    int code;

    if (Tcl_InitStubs(interp, "9.0-", 0) == NULL) {
	return TCL_ERROR;
    }
    code = Tcl_PkgProvide(interp, "pkgt", "1.0");
    if (code != TCL_OK) {
	return code;
    }
    Tcl_CreateObjCommand2(interp, "pkgt_eq", Pkgt_EqObjCmd2, NULL, NULL);
    Tcl_CreateObjTrace2(interp, 0, 0, TraceProc2, NULL, NULL);
    return TCL_OK;
}
