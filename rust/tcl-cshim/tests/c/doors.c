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
 * doors.c — a test extension for what a C command does to its caller beyond
 * answering: it reads, writes and unsets the variables of the frame that called
 * it (Tcl_GetVar2Ex, Tcl_ObjSetVar2, Tcl_UnsetVar2) and evaluates a script there
 * (Tcl_EvalObjEx), the calls a package makes that reach into the interpreter
 * rather than into its own arguments. Each command is one call and what the call
 * answered, so the expected strings in the integration tests, captured by
 * compiling this file against Tcl 9.0.4's own `tcl.h` and loading it into
 * `tclsh9.0`, are the contract: the extension is written once and means the same
 * thing under either header.
 *
 * Its entry point uses the door too, setting the global `doors_loaded`, so a host
 * that opens the door only while a command runs fails to load it.
 */

#include <tcl.h>

/* doors_set name ?index? value: Tcl_ObjSetVar2, answering what it returned. */
static int
Doors_SetObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    Tcl_Obj *stored;
    (void)dummy;

    if (objc != 3 && objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?index? value");
	return TCL_ERROR;
    }
    stored = Tcl_ObjSetVar2(interp, objv[1], objc == 4 ? objv[2] : NULL,
	    objv[objc - 1], TCL_LEAVE_ERR_MSG);
    if (stored == NULL) {
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, stored);
    return TCL_OK;
}

/* doors_global_set name value: the same with TCL_GLOBAL_ONLY. */
static int
Doors_GlobalSetObjCmd(void *dummy, Tcl_Interp *interp, int objc,
	Tcl_Obj *const objv[])
{
    Tcl_Obj *stored;
    (void)dummy;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name value");
	return TCL_ERROR;
    }
    stored = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
	    TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG);
    if (stored == NULL) {
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, stored);
    return TCL_OK;
}

/* doors_get name ?index?: Tcl_GetVar2Ex, the error message left in the result. */
static int
Doors_GetObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    Tcl_Obj *value;
    (void)dummy;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?index?");
	return TCL_ERROR;
    }
    value = Tcl_GetVar2Ex(interp, Tcl_GetString(objv[1]),
	    objc == 3 ? Tcl_GetString(objv[2]) : NULL, TCL_LEAVE_ERR_MSG);
    if (value == NULL) {
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, value);
    return TCL_OK;
}

/*
 * doors_peek name ?index?: Tcl_GetVar2Ex with no flags, answering the value or
 * `<null>`, and leaving the interpreter result as the failed call left it.
 */
static int
Doors_PeekObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    Tcl_Obj *value;
    (void)dummy;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?index?");
	return TCL_ERROR;
    }
    Tcl_ResetResult(interp);
    value = Tcl_GetVar2Ex(interp, Tcl_GetString(objv[1]),
	    objc == 3 ? Tcl_GetString(objv[2]) : NULL, 0);
    if (value == NULL) {
	Tcl_Obj *left = Tcl_GetObjResult(interp);
	Tcl_Size length;
	(void)Tcl_GetStringFromObj(left, &length);
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		length == 0 ? "<null>" : "<null with a message>", -1));
	return TCL_OK;
    }
    Tcl_SetObjResult(interp, value);
    return TCL_OK;
}

/* doors_unset name ?index?: Tcl_UnsetVar2, the error message left in the result. */
static int
Doors_UnsetObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    (void)dummy;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?index?");
	return TCL_ERROR;
    }
    return Tcl_UnsetVar2(interp, Tcl_GetString(objv[1]),
	    objc == 3 ? Tcl_GetString(objv[2]) : NULL, TCL_LEAVE_ERR_MSG);
}

/* doors_eval script: Tcl_EvalObjEx, answering whatever code the script gave. */
static int
Doors_EvalObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    (void)dummy;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "script");
	return TCL_ERROR;
    }
    return Tcl_EvalObjEx(interp, objv[1], 0);
}

/* doors_eval_direct script: the same with TCL_EVAL_DIRECT, which only hints. */
static int
Doors_EvalDirectObjCmd(void *dummy, Tcl_Interp *interp, int objc,
	Tcl_Obj *const objv[])
{
    (void)dummy;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "script");
	return TCL_ERROR;
    }
    return Tcl_EvalObjEx(interp, objv[1], TCL_EVAL_DIRECT);
}

/*
 * doors_try script: Tcl_EvalObjEx, answering `{code result}` as the C code saw
 * them and completing normally, whatever the script did.
 */
static int
Doors_TryObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    Tcl_Obj *pair[2];
    int code;
    (void)dummy;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "script");
	return TCL_ERROR;
    }
    code = Tcl_EvalObjEx(interp, objv[1], 0);
    pair[0] = Tcl_NewIntObj(code);
    pair[1] = Tcl_DuplicateObj(Tcl_GetObjResult(interp));
    Tcl_SetObjResult(interp, Tcl_NewListObj(2, pair));
    return TCL_OK;
}

/*
 * doors_eval_twice first second: Tcl_EvalObjEx twice, answering the code of the
 * second, so what the interpreter holds of the first is the second's to replace.
 */
static int
Doors_EvalTwiceObjCmd(void *dummy, Tcl_Interp *interp, int objc,
	Tcl_Obj *const objv[])
{
    (void)dummy;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "first second");
	return TCL_ERROR;
    }
    (void)Tcl_EvalObjEx(interp, objv[1], 0);
    return Tcl_EvalObjEx(interp, objv[2], 0);
}

/*
 * doors_eval_reset script: Tcl_EvalObjEx, then Tcl_ResetResult, answering the
 * code of the evaluation: what a reset takes from the interpreter is the result
 * and the options of the return the script ended in.
 */
static int
Doors_EvalResetObjCmd(void *dummy, Tcl_Interp *interp, int objc,
	Tcl_Obj *const objv[])
{
    int code;
    (void)dummy;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "script");
	return TCL_ERROR;
    }
    code = Tcl_EvalObjEx(interp, objv[1], 0);
    Tcl_ResetResult(interp);
    return code;
}

/*
 * doors_keep name: reads a variable, takes its own reference to the value, unsets
 * the variable and answers the value it still holds, as a command must that keeps
 * a value past a call that can free it.
 */
static int
Doors_KeepObjCmd(void *dummy, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    Tcl_Obj *value;
    (void)dummy;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    value = Tcl_GetVar2Ex(interp, Tcl_GetString(objv[1]), NULL, TCL_LEAVE_ERR_MSG);
    if (value == NULL) {
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(value);
    if (Tcl_UnsetVar2(interp, Tcl_GetString(objv[1]), NULL, TCL_LEAVE_ERR_MSG)
	    != TCL_OK) {
	Tcl_DecrRefCount(value);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, value);
    Tcl_DecrRefCount(value);
    return TCL_OK;
}

int
Doors_Init(Tcl_Interp *interp)
{
    Tcl_Obj *name = Tcl_NewStringObj("doors_loaded", -1);
    Tcl_Obj *stored;

    Tcl_IncrRefCount(name);
    stored = Tcl_ObjSetVar2(interp, name, NULL, Tcl_NewIntObj(1),
	    TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG);
    Tcl_DecrRefCount(name);
    if (stored == NULL) {
	return TCL_ERROR;
    }
    Tcl_CreateObjCommand(interp, "doors_set", Doors_SetObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_global_set", Doors_GlobalSetObjCmd, NULL,
	    NULL);
    Tcl_CreateObjCommand(interp, "doors_get", Doors_GetObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_peek", Doors_PeekObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_unset", Doors_UnsetObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_eval", Doors_EvalObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_eval_direct", Doors_EvalDirectObjCmd,
	    NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_try", Doors_TryObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_keep", Doors_KeepObjCmd, NULL, NULL);
    Tcl_CreateObjCommand(interp, "doors_eval_twice", Doors_EvalTwiceObjCmd, NULL,
	    NULL);
    Tcl_CreateObjCommand(interp, "doors_eval_reset", Doors_EvalResetObjCmd, NULL,
	    NULL);
    return Tcl_PkgProvide(interp, "doors", "1.0");
}
