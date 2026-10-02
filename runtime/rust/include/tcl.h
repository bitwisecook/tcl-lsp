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
 * tcl.h — the C Tcl API an extension is compiled against: one header, two
 * hosts.
 *
 * This is the signature-faithful subset of Tcl 9.0's public header that the
 * project's hosts implement (docs/design/runtime/c-extension-abi.md section
 * 7). It is the only shim there is: the same extension source, recompiled
 * from source and never binary-loaded, runs on
 *
 *   TCL_HOST_NATIVE  the shim (rust/tcl-cshim): trusted host code linked into
 *                    a native process, reached through Interp::load_static or
 *                    a host's `load`; and
 *   TCL_HOST_WASM    the runtime (runtime/rust): an extension compiled to a
 *                    wasm32 module that shares the runtime's linear memory and
 *                    function table and imports the Tcl_* functions it calls.
 *
 * Each host declares the functions it implements and nothing else. A
 * declaration the compiling host does not implement is absent, so calling it
 * is a compile error and never a call that fails at run time. The host is the
 * compilation target (wasm32 is TCL_HOST_WASM, anything else TCL_HOST_NATIVE)
 * unless the build names one with -DTCL_HOST_NATIVE or -DTCL_HOST_WASM;
 * naming both declares both, which checks a source against the header and
 * names no host, since none implements the union.
 *
 * Tcl_Obj has the layout Tcl 9.0 gives it, which extensions read through the
 * reference-count macros below and through objPtr->bytes: refCount, bytes,
 * length, typePtr, then the internal representation. Its fields are the hosts'
 * ABI size type whatever Tcl_Size is, and a source built with
 * -DTCL_MAJOR_VERSION=8 sees Tcl_Size as int, as an 8.x source does, with
 * inline wrappers for the functions that write a size through a pointer.
 *
 * Not declared, because no host implements it: channels, the event loop,
 * threads, Tcl_Eval*, variables, the dict API, object types and the stub
 * tables. See docs/design/runtime/c-extension-shim.md and c-extension-abi.md.
 */

#ifndef TCL_H
#define TCL_H

#include <stdarg.h>
#include <stddef.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * The host this compiles for. Both may be named; neither names the target's.
 */
#if !defined(TCL_HOST_NATIVE) && !defined(TCL_HOST_WASM)
#   if defined(__wasm__)
#	define TCL_HOST_WASM
#   else
#	define TCL_HOST_NATIVE
#   endif
#endif

#ifndef TCL_MAJOR_VERSION
#   define TCL_MAJOR_VERSION 9
#endif
#if TCL_MAJOR_VERSION > 8
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

/*
 * Opaque handles. An extension never sees inside an interpreter, a command
 * token or an object type.
 */
typedef struct Tcl_Interp Tcl_Interp;
typedef struct Tcl_ObjType Tcl_ObjType;
typedef struct Tcl_Command_ *Tcl_Command;
typedef void *ClientData;

typedef long long Tcl_WideInt;
#define TCL_WIDE_INT_TYPE long long

/*
 * The hosts' own size type: the Tcl 9 one, ptrdiff_t, whatever Tcl_Size is
 * below. The exports and the Tcl_Obj fields use it.
 */
typedef ptrdiff_t TclHost_Size;

#if TCL_MAJOR_VERSION > 8
typedef ptrdiff_t Tcl_Size;
#   define TCL_SIZE_MAX ((Tcl_Size)(((size_t)-1)>>1))
#else
typedef int Tcl_Size;
#   define TCL_SIZE_MAX ((Tcl_Size)0x7fffffff)
#endif
#define TCL_INDEX_NONE ((Tcl_Size)-1)

/*
 * A value's internal representation, as Tcl 9.0 declares it. A host keeps its
 * own representation behind it; an extension reads none of its variants.
 */
typedef union Tcl_ObjInternalRep {
    long longValue;
    double doubleValue;
    void *otherValuePtr;
    Tcl_WideInt wideValue;
    struct {
	void *ptr1;
	void *ptr2;
    } twoPtrValue;
    struct {
	void *ptr;
	unsigned long value;
    } ptrAndLongRep;
    struct {
	void *ptr;
	TclHost_Size size;
    } ptrAndSize;
} Tcl_ObjInternalRep;

/*
 * A value. bytes is the first byte of the string representation, NUL
 * terminated at offset length, or NULL when there is none: read it through
 * Tcl_GetString or Tcl_GetStringFromObj, which make one. typePtr is NULL in
 * every host.
 */
typedef struct Tcl_Obj {
    TclHost_Size refCount;
    char *bytes;
    TclHost_Size length;
    const Tcl_ObjType *typePtr;
    Tcl_ObjInternalRep internalRep;
} Tcl_Obj;

typedef int (Tcl_ObjCmdProc)(void *clientData, Tcl_Interp *interp, int objc,
	Tcl_Obj *const *objv);
typedef void (Tcl_CmdDeleteProc)(void *clientData);
#if TCL_MAJOR_VERSION > 8
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

/*
 * Reference counts, as Tcl 9.0 defines them: the macros read and write
 * refCount, and a count that falls to zero or below is freed by TclFreeObj. A
 * new object has a count of zero and belongs to whoever first takes a
 * reference to it (Tcl_SetObjResult takes one).
 */
EXTERN void TclFreeObj(Tcl_Obj *objPtr);

#define Tcl_IncrRefCount(objPtr) \
	((void)++(objPtr)->refCount)
#define Tcl_DecrRefCount(objPtr) \
	do { \
	    Tcl_Obj *_objPtr = (objPtr); \
	    if (_objPtr->refCount-- <= 1) { \
		TclFreeObj(_objPtr); \
	    } \
	} while (0)
#define Tcl_IsShared(objPtr) \
	((objPtr)->refCount > 1)

/*
 * There is no stubs table: an extension is recompiled against this header,
 * never loaded against a stub library, so initialising stubs is a no-op that
 * yields the version the host presents.
 */
#define Tcl_InitStubs(interp, version, exact) \
	((void)(interp), (void)(version), (void)(exact), TCL_PATCH_LEVEL)

/*
 * Implemented by both hosts.
 */

/* Command registration. */
EXTERN Tcl_Command Tcl_CreateObjCommand(Tcl_Interp *interp, const char *cmdName,
	Tcl_ObjCmdProc *proc, void *clientData, Tcl_CmdDeleteProc *deleteProc);
EXTERN int Tcl_DeleteCommand(Tcl_Interp *interp, const char *cmdName);

/* Objects: construction. */
EXTERN Tcl_Obj *Tcl_NewStringObj(const char *bytes, TclHost_Size length);
EXTERN Tcl_Obj *Tcl_NewWideIntObj(Tcl_WideInt wideValue);
EXTERN Tcl_Obj *Tcl_NewBooleanObj(int intValue);
EXTERN Tcl_Obj *Tcl_NewDoubleObj(double doubleValue);

/* Objects: reading. */
EXTERN char *Tcl_GetString(Tcl_Obj *objPtr);
EXTERN char *Tcl_GetStringFromObj(Tcl_Obj *objPtr, TclHost_Size *lengthPtr);

/* The interpreter result. */
EXTERN void Tcl_SetObjResult(Tcl_Interp *interp, Tcl_Obj *resultObjPtr);
EXTERN Tcl_Obj *Tcl_GetObjResult(Tcl_Interp *interp);

#if defined(TCL_HOST_WASM)
/*
 * Implemented by the runtime alone.
 */
EXTERN Tcl_Obj *Tcl_NewObj(void);
#endif /* TCL_HOST_WASM */

#if defined(TCL_HOST_NATIVE)
/*
 * Implemented by the shim alone.
 */

/* Objects: construction and copying. */
EXTERN Tcl_Obj *Tcl_NewIntObj(int intValue);
EXTERN Tcl_Obj *Tcl_NewLongObj(long longValue);
EXTERN Tcl_Obj *Tcl_NewListObj(TclHost_Size objc, Tcl_Obj *const objv[]);
EXTERN Tcl_Obj *Tcl_DuplicateObj(Tcl_Obj *objPtr);

/* Objects: reading. */
EXTERN int Tcl_GetIntFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr, int *intPtr);
EXTERN int Tcl_GetLongFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr, long *longPtr);
EXTERN int Tcl_GetWideIntFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
	Tcl_WideInt *widePtr);
EXTERN int Tcl_GetBooleanFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr, int *intPtr);
EXTERN int Tcl_GetDoubleFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
	double *doublePtr);
EXTERN int Tcl_GetIndexFromObjStruct(Tcl_Interp *interp, Tcl_Obj *objPtr,
	const void *tablePtr, TclHost_Size offset, const char *msg, int flags,
	void *indexPtr);

/* Lists. */
EXTERN int Tcl_ListObjAppendElement(Tcl_Interp *interp, Tcl_Obj *listPtr,
	Tcl_Obj *objPtr);
EXTERN int Tcl_ListObjGetElements(Tcl_Interp *interp, Tcl_Obj *listPtr,
	TclHost_Size *objcPtr, Tcl_Obj ***objvPtr);
EXTERN int Tcl_ListObjLength(Tcl_Interp *interp, Tcl_Obj *listPtr,
	TclHost_Size *lengthPtr);

/* The interpreter's error state. */
EXTERN void Tcl_ResetResult(Tcl_Interp *interp);
EXTERN void Tcl_WrongNumArgs(Tcl_Interp *interp, TclHost_Size objc,
	Tcl_Obj *const objv[], const char *message);
EXTERN void Tcl_SetObjErrorCode(Tcl_Interp *interp, Tcl_Obj *errorObjPtr);
/* Fixed-arity exports behind the variadic inline functions below. */
EXTERN void TclShim_SetResultString(Tcl_Interp *interp, const char *result);
EXTERN void TclShim_AppendResultString(Tcl_Interp *interp, const char *piece);

/* Packages. */
EXTERN int Tcl_PkgProvideEx(Tcl_Interp *interp, const char *name,
	const char *version, const void *clientData);

/* UTF-8 helpers the canonical test extensions lean on. */
EXTERN TclHost_Size Tcl_NumUtfChars(const char *src, TclHost_Size length);
EXTERN int Tcl_UtfNcmp(const char *s1, const char *s2, size_t n);

/*
 * Variadic entry points. C variadics cannot be defined from Rust on the
 * stable toolchain, so each is an inline C function that fans the NULL-
 * terminated argument list out into fixed-arity exports. This is the host
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
#endif /* TCL_HOST_NATIVE */

#if TCL_MAJOR_VERSION < 9
/*
 * 8.x sources pass `int *` where the hosts' ABI wants `ptrdiff_t *`. The
 * wrappers convert through a temporary; each is defined before the macro that
 * renames the source's call to it, so the wrapper body still reaches the real
 * export.
 */
static inline char *
TclHost8_GetStringFromObj(Tcl_Obj *objPtr, int *lengthPtr)
{
    TclHost_Size length;
    char *bytes = Tcl_GetStringFromObj(objPtr, &length);

    if (lengthPtr != NULL) {
	*lengthPtr = (int)length;
    }
    return bytes;
}

#   define Tcl_GetStringFromObj TclHost8_GetStringFromObj

#   if defined(TCL_HOST_NATIVE)
static inline int
TclHost8_ListObjGetElements(Tcl_Interp *interp, Tcl_Obj *listPtr, int *objcPtr,
	Tcl_Obj ***objvPtr)
{
    TclHost_Size objc;
    int code = Tcl_ListObjGetElements(interp, listPtr, &objc, objvPtr);

    if (objcPtr != NULL) {
	*objcPtr = (int)objc;
    }
    return code;
}

static inline int
TclHost8_ListObjLength(Tcl_Interp *interp, Tcl_Obj *listPtr, int *lengthPtr)
{
    TclHost_Size length;
    int code = Tcl_ListObjLength(interp, listPtr, &length);

    if (lengthPtr != NULL) {
	*lengthPtr = (int)length;
    }
    return code;
}

#	define Tcl_ListObjGetElements TclHost8_ListObjGetElements
#	define Tcl_ListObjLength TclHost8_ListObjLength
#   endif /* TCL_HOST_NATIVE */
#endif /* TCL_MAJOR_VERSION < 9 */

#ifdef __cplusplus
}
#endif

#endif /* TCL_H */
