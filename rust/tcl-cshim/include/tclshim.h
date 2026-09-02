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
 * tclshim.h — the C Tcl API subset the tcl-cshim crate implements.
 *
 * An existing extension compiles against this header with at most an include
 * swap (`#include "tcl.h"` -> `#include "tclshim.h"`). Every declaration here
 * is implemented; nothing is declared that is not. The exported symbols keep
 * their real Tcl names and the Tcl 9 ABI (`Tcl_Size` is `ptrdiff_t`); the
 * `TCL_SHIM_TCL_MAJOR=8` switch adapts 8.x sources with inline wrappers, the
 * same way Tcl 9's own header does for its 8.x compatibility mode.
 *
 * The C-required mangling lives here and in the Rust crate — variadic
 * functions are inline C that fan out into fixed-arity exports, the freeing
 * conventions of Tcl_SetResult are resolved before the call crosses into
 * Rust — so the engine-neutral interface underneath never sees any of it.
 * See docs/design/c-extension-shim.md.
 */

#ifndef TCLSHIM_H
#define TCLSHIM_H

#include <stdarg.h>
#include <stddef.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef TCL_SHIM_TCL_MAJOR
#   define TCL_SHIM_TCL_MAJOR 9
#endif

#define TCL_MAJOR_VERSION TCL_SHIM_TCL_MAJOR
#if TCL_SHIM_TCL_MAJOR > 8
#   define TCL_MINOR_VERSION 0
#   define TCL_VERSION "9.0"
#   define TCL_PATCH_LEVEL "9.0.4"
#else
#   define TCL_MINOR_VERSION 6
#   define TCL_VERSION "8.6"
#   define TCL_PATCH_LEVEL "8.6.16"
#endif

#ifndef DLLEXPORT
#   if defined(_WIN32)
#	define DLLEXPORT __declspec(dllexport)
#   else
#	define DLLEXPORT
#   endif
#endif
#ifndef EXTERN
#   define EXTERN extern
#endif

/* Opaque handles. An extension never sees inside either. */
typedef struct Tcl_Interp Tcl_Interp;
typedef struct Tcl_Obj Tcl_Obj;
typedef struct Tcl_Command_ *Tcl_Command;
typedef void *ClientData;

typedef long long Tcl_WideInt;
#define TCL_WIDE_INT_TYPE long long

/* The shim's own ABI size type: the Tcl 9 one, whatever Tcl_Size is below. */
typedef ptrdiff_t TclShim_Size;

#if TCL_SHIM_TCL_MAJOR > 8
typedef ptrdiff_t Tcl_Size;
#   define TCL_SIZE_MAX ((Tcl_Size)(((size_t)-1)>>1))
#else
typedef int Tcl_Size;
#   define TCL_SIZE_MAX ((Tcl_Size)0x7fffffff)
#endif
#define TCL_INDEX_NONE ((Tcl_Size)-1)

typedef int (Tcl_ObjCmdProc)(void *clientData, Tcl_Interp *interp, int objc,
	Tcl_Obj *const *objv);
typedef void (Tcl_CmdDeleteProc)(void *clientData);
#if TCL_SHIM_TCL_MAJOR > 8
typedef void (Tcl_FreeProc)(void *blockPtr);
#else
typedef void (Tcl_FreeProc)(char *blockPtr);
#endif

#define TCL_OK		0
#define TCL_ERROR	1
#define TCL_RETURN	2
#define TCL_BREAK	3
#define TCL_CONTINUE	4

#define TCL_STATIC	((Tcl_FreeProc *) 0)
#define TCL_VOLATILE	((Tcl_FreeProc *) 1)
#define TCL_DYNAMIC	((Tcl_FreeProc *) 3)

#define TCL_EXACT		1
#define TCL_NULL_OK		32
#define TCL_INDEX_TEMP_TABLE	64

/* Command registration. */
EXTERN Tcl_Command Tcl_CreateObjCommand(Tcl_Interp *interp, const char *cmdName,
	Tcl_ObjCmdProc *proc, void *clientData, Tcl_CmdDeleteProc *deleteProc);
EXTERN int Tcl_DeleteCommand(Tcl_Interp *interp, const char *cmdName);

/* Objects: construction and reference counting. */
EXTERN Tcl_Obj *Tcl_NewStringObj(const char *bytes, TclShim_Size length);
EXTERN Tcl_Obj *Tcl_NewIntObj(int intValue);
EXTERN Tcl_Obj *Tcl_NewLongObj(long longValue);
EXTERN Tcl_Obj *Tcl_NewWideIntObj(Tcl_WideInt wideValue);
EXTERN Tcl_Obj *Tcl_NewBooleanObj(int intValue);
EXTERN Tcl_Obj *Tcl_NewDoubleObj(double doubleValue);
EXTERN Tcl_Obj *Tcl_NewListObj(TclShim_Size objc, Tcl_Obj *const objv[]);
EXTERN void Tcl_IncrRefCount(Tcl_Obj *objPtr);
EXTERN void Tcl_DecrRefCount(Tcl_Obj *objPtr);
EXTERN int Tcl_IsShared(Tcl_Obj *objPtr);
EXTERN Tcl_Obj *Tcl_DuplicateObj(Tcl_Obj *objPtr);

/* Objects: reading. */
EXTERN char *Tcl_GetString(Tcl_Obj *objPtr);
EXTERN char *Tcl_GetStringFromObj(Tcl_Obj *objPtr, TclShim_Size *lengthPtr);
EXTERN int Tcl_GetIntFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr, int *intPtr);
EXTERN int Tcl_GetLongFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr, long *longPtr);
EXTERN int Tcl_GetWideIntFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
	Tcl_WideInt *widePtr);
EXTERN int Tcl_GetBooleanFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr, int *intPtr);
EXTERN int Tcl_GetDoubleFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
	double *doublePtr);
EXTERN int Tcl_GetIndexFromObjStruct(Tcl_Interp *interp, Tcl_Obj *objPtr,
	const void *tablePtr, TclShim_Size offset, const char *msg, int flags,
	void *indexPtr);

/* Lists. */
EXTERN int Tcl_ListObjAppendElement(Tcl_Interp *interp, Tcl_Obj *listPtr,
	Tcl_Obj *objPtr);
EXTERN int Tcl_ListObjGetElements(Tcl_Interp *interp, Tcl_Obj *listPtr,
	TclShim_Size *objcPtr, Tcl_Obj ***objvPtr);
EXTERN int Tcl_ListObjLength(Tcl_Interp *interp, Tcl_Obj *listPtr,
	TclShim_Size *lengthPtr);

/* The interpreter result and error state. */
EXTERN void Tcl_SetObjResult(Tcl_Interp *interp, Tcl_Obj *resultObjPtr);
EXTERN Tcl_Obj *Tcl_GetObjResult(Tcl_Interp *interp);
EXTERN void Tcl_ResetResult(Tcl_Interp *interp);
EXTERN void Tcl_WrongNumArgs(Tcl_Interp *interp, TclShim_Size objc,
	Tcl_Obj *const objv[], const char *message);
EXTERN void Tcl_SetObjErrorCode(Tcl_Interp *interp, Tcl_Obj *errorObjPtr);
/* Fixed-arity exports behind the variadic inline functions below. */
EXTERN void TclShim_SetResultString(Tcl_Interp *interp, const char *result);
EXTERN void TclShim_AppendResultString(Tcl_Interp *interp, const char *piece);

/* Packages. */
EXTERN int Tcl_PkgProvideEx(Tcl_Interp *interp, const char *name,
	const char *version, const void *clientData);

/* UTF-8 helpers the canonical test extensions lean on. */
EXTERN TclShim_Size Tcl_NumUtfChars(const char *src, TclShim_Size length);
EXTERN int Tcl_UtfNcmp(const char *s1, const char *s2, size_t n);

/*
 * Variadic entry points. C variadics cannot be defined from Rust on the
 * stable toolchain, so each is an inline C function that fans the NULL-
 * terminated argument list out into fixed-arity exports. This is the shim
 * absorbing a C idiom, exactly as the design requires.
 */
static inline void
Tcl_AppendResult(Tcl_Interp *interp, ...)
{
    va_list ap;
    const char *piece;

    va_start(ap, interp);
    while ((piece = va_arg(ap, const char *)) != NULL) {
	TclShim_AppendResultString(interp, piece);
    }
    va_end(ap);
}

static inline void
Tcl_SetErrorCode(Tcl_Interp *interp, ...)
{
    va_list ap;
    const char *piece;
    Tcl_Obj *code = Tcl_NewListObj(0, NULL);

    va_start(ap, interp);
    while ((piece = va_arg(ap, const char *)) != NULL) {
	Tcl_ListObjAppendElement(NULL, code, Tcl_NewStringObj(piece, -1));
    }
    va_end(ap);
    Tcl_SetObjErrorCode(interp, code);
}

/*
 * Tcl_SetResult's freeing convention is resolved here: the string is always
 * copied, so TCL_STATIC and TCL_VOLATILE need nothing more, TCL_DYNAMIC is
 * freed with the C allocator, and any other value is a caller-supplied free
 * procedure to call.
 */
static inline void
Tcl_SetResult(Tcl_Interp *interp, char *result, Tcl_FreeProc *freeProc)
{
    TclShim_SetResultString(interp, result);
    if (freeProc == TCL_DYNAMIC) {
	free(result);
    } else if (freeProc != TCL_STATIC && freeProc != TCL_VOLATILE) {
	freeProc(result);
    }
}

#define Tcl_PkgProvide(interp, name, version) \
	Tcl_PkgProvideEx((interp), (name), (version), NULL)

/*
 * Tcl_GetIndexFromObjStruct encodes the width of *indexPtr in the flags
 * word, as Tcl 9's header does; Tcl_GetIndexFromObj is the char*-table
 * special case.
 */
#define Tcl_GetIndexFromObjStruct(interp, objPtr, tablePtr, offset, msg, flags, indexPtr) \
	((Tcl_GetIndexFromObjStruct)((interp), (objPtr), (tablePtr), (offset), \
	    (msg), (flags) | (int)(sizeof(*(indexPtr)) << 1), (indexPtr)))
#define Tcl_GetIndexFromObj(interp, objPtr, tablePtr, msg, flags, indexPtr) \
	Tcl_GetIndexFromObjStruct((interp), (objPtr), (tablePtr), sizeof(char *), \
	    (msg), (flags), (indexPtr))

/*
 * There is no stubs table: the shim is linked, not loaded against a stub
 * library, so initialising stubs is a no-op that yields the version the
 * shim presents.
 */
#define Tcl_InitStubs(interp, version, exact) \
	((void)(interp), (void)(version), (void)(exact), TCL_PATCH_LEVEL)

#if TCL_SHIM_TCL_MAJOR < 9
/*
 * 8.x sources pass `int *` where the shim's ABI wants `ptrdiff_t *`. The
 * wrappers convert through a temporary; each is defined before the macro
 * that renames the source's call to it, so the wrapper body still reaches
 * the real export.
 */
static inline char *
TclShim8_GetStringFromObj(Tcl_Obj *objPtr, int *lengthPtr)
{
    TclShim_Size length;
    char *bytes = Tcl_GetStringFromObj(objPtr, &length);

    if (lengthPtr != NULL) {
	*lengthPtr = (int)length;
    }
    return bytes;
}

static inline int
TclShim8_ListObjGetElements(Tcl_Interp *interp, Tcl_Obj *listPtr, int *objcPtr,
	Tcl_Obj ***objvPtr)
{
    TclShim_Size objc;
    int code = Tcl_ListObjGetElements(interp, listPtr, &objc, objvPtr);

    if (objcPtr != NULL) {
	*objcPtr = (int)objc;
    }
    return code;
}

static inline int
TclShim8_ListObjLength(Tcl_Interp *interp, Tcl_Obj *listPtr, int *lengthPtr)
{
    TclShim_Size length;
    int code = Tcl_ListObjLength(interp, listPtr, &length);

    if (lengthPtr != NULL) {
	*lengthPtr = (int)length;
    }
    return code;
}

#   define Tcl_GetStringFromObj TclShim8_GetStringFromObj
#   define Tcl_ListObjGetElements TclShim8_ListObjGetElements
#   define Tcl_ListObjLength TclShim8_ListObjLength
#endif

#ifdef __cplusplus
}
#endif

#endif /* TCLSHIM_H */
