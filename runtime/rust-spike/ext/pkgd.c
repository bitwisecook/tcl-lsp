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
 * pkgd.c --
 *
 *	This file contains a simple Tcl package "pkgd" that is intended for
 *	testing the Tcl dynamic loading facilities. It can be used in both
 *	safe and unsafe interpreters.
 *
 * Copyright © 1995 Sun Microsystems, Inc.
 *
 * See the file "license.terms" for information on usage and redistribution of
 * this file, and for a DISCLAIMER OF ALL WARRANTIES.
 */

#undef STATIC_BUILD
#include "tcl.h"

/*
 *----------------------------------------------------------------------
 *
 * Pkgd_SubObjCmd --
 *
 *	This procedure is invoked to process the "pkgd_sub" Tcl command. It
 *	expects two arguments and returns their difference.
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
Pkgd_SubObjCmd(
    void *dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int first, second;
    (void)dummy;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "num num");
	return TCL_ERROR;
    }
    if ((Tcl_GetIntFromObj(interp, objv[1], &first) != TCL_OK)
	    || (Tcl_GetIntFromObj(interp, objv[2], &second) != TCL_OK)) {
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, Tcl_NewIntObj(first - second));
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Pkgd_UnsafeCmd --
 *
 *	This procedure is invoked to process the "pkgd_unsafe" Tcl command. It
 *	just returns a constant string.
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
Pkgd_UnsafeObjCmd(
    void *dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    (void)dummy;
    (void)objc;
    (void)objv;

    Tcl_SetObjResult(interp, Tcl_NewStringObj("unsafe command invoked", TCL_INDEX_NONE));
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Pkgd_Init --
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
Pkgd_Init(
    Tcl_Interp *interp)		/* Interpreter in which the package is to be
				 * made available. */
{
    int code;

    if (Tcl_InitStubs(interp, "8.5-", 0) == NULL) {
	return TCL_ERROR;
    }
    code = Tcl_PkgProvide(interp, "pkgd", "7.3");
    if (code != TCL_OK) {
	return code;
    }
    Tcl_CreateObjCommand(interp, "pkgd_sub", Pkgd_SubObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "pkgd_unsafe", Pkgd_UnsafeObjCmd, NULL,
	    NULL);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * Pkgd_SafeInit --
 *
 *	This is a package initialization procedure, which is called by Tcl
 *	when this package is to be added to a safe interpreter.
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
Pkgd_SafeInit(
    Tcl_Interp *interp)		/* Interpreter in which the package is to be
				 * made available. */
{
    int code;

    if (Tcl_InitStubs(interp, "8.5-", 0) == NULL) {
	return TCL_ERROR;
    }
    code = Tcl_PkgProvide(interp, "pkgd", "7.3");
    if (code != TCL_OK) {
	return code;
    }
    Tcl_CreateObjCommand(interp, "pkgd_sub", Pkgd_SubObjCmd, NULL, NULL);
    return TCL_OK;
}
