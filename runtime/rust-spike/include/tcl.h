/*
 * tcl.h -- minimal, API-compatible subset for the Rust-runtime C-extension spike.
 *
 * This header is authored by the *runtime*, not copied from upstream Tcl.  It
 * declares the public Tcl C API surface that the (unmodified) extension source
 * in ../ext/ #includes.  Every type and prototype here is a faithful subset of
 * the real Tcl 9.0 tcl.h / tclDecls.h signatures, so a real extension compiles
 * against it unchanged -- but the *ABI* behind these calls is ours to define
 * (direct C-ABI imports satisfied by the Rust runtime; no 600-slot stubs table).
 *
 * This is the "API compatibility, not ABI compatibility" thesis made concrete:
 * extensions are recompiled from source against this header; we are free to wire
 * the calls however we like underneath.
 *
 * Scope: exactly the symbols Tcl's own dltest/pkga.c uses, plus Tcl_NewStringObj
 * / Tcl_GetString for reuse.  A production runtime widens this to the rest of
 * the public surface (channels, Tcl_ObjType, Tcl_FSRegister, threading, NRE,
 * and the sibling public headers tclOO.h / tclTomMath.h) -- see README.md.
 */
#ifndef SPIKE_TCL_H
#define SPIKE_TCL_H

#include <stddef.h> /* ptrdiff_t, size_t, NULL -- freestanding clang header */
#include <stdint.h> /* int64_t                  -- freestanding clang header */

#ifdef __cplusplus
extern "C" {
#endif

#define TCL_OK 0
#define TCL_ERROR 1

/* Tcl 9: Tcl_Size is ptrdiff_t (32-bit on wasm32). */
typedef ptrdiff_t Tcl_Size;
typedef int64_t Tcl_WideInt;

#define TCL_INDEX_NONE ((Tcl_Size) -1)

/* Mark Foo_Init as an exported/default-visibility symbol for the linker. */
#define DLLEXPORT __attribute__((visibility("default")))

typedef void *ClientData;
typedef struct Tcl_Interp Tcl_Interp; /* opaque to C; real struct lives in Rust */
typedef void *Tcl_Command;
typedef struct Tcl_ObjType Tcl_ObjType;

/*
 * Internal representation union -- mirrors the size (8 bytes) and alignment (8)
 * of the real Tcl_ObjInternalRep on wasm32 so struct Tcl_Obj has the same
 * layout.  Core-API extensions never touch these fields directly.
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
        Tcl_Size size;
    } ptrAndSize;
} Tcl_ObjInternalRep;

typedef struct Tcl_Obj {
    Tcl_Size refCount;
    char *bytes;
    Tcl_Size length;
    const Tcl_ObjType *typePtr;
    Tcl_ObjInternalRep internalRep;
} Tcl_Obj;

typedef int(Tcl_ObjCmdProc)(void *clientData, Tcl_Interp *interp, int objc,
                            Tcl_Obj *const *objv);
typedef void(Tcl_CmdDeleteProc)(void *clientData);

/*
 * The runtime-provided API.  In real tcl.h several of these are macros over a
 * stubs table; here they are plain extern functions -- that difference is the
 * ABI we are choosing, and it is invisible to extension *source*.
 */
extern const char *Tcl_InitStubs(Tcl_Interp *interp, const char *version,
                                 int exact);
extern int Tcl_PkgProvideEx(Tcl_Interp *interp, const char *name,
                            const char *version, const void *clientData);
#define Tcl_PkgProvide(interp, name, version)                                  \
    Tcl_PkgProvideEx(interp, name, version, NULL)

extern Tcl_Command Tcl_CreateObjCommand(Tcl_Interp *interp, const char *cmdName,
                                        Tcl_ObjCmdProc *proc, void *clientData,
                                        Tcl_CmdDeleteProc *deleteProc);

extern char *Tcl_GetStringFromObj(Tcl_Obj *objPtr, Tcl_Size *lengthPtr);
extern char *Tcl_GetString(Tcl_Obj *objPtr);
extern Tcl_Size Tcl_NumUtfChars(const char *src, Tcl_Size length);
extern int Tcl_UtfNcmp(const char *s1, const char *s2, size_t n);

extern void Tcl_SetObjResult(Tcl_Interp *interp, Tcl_Obj *resultObjPtr);
extern Tcl_Obj *Tcl_NewWideIntObj(Tcl_WideInt value);
#define Tcl_NewIntObj(value) Tcl_NewWideIntObj((Tcl_WideInt) (value))
extern Tcl_Obj *Tcl_NewStringObj(const char *bytes, Tcl_Size length);

extern void Tcl_WrongNumArgs(Tcl_Interp *interp, Tcl_Size objc,
                             Tcl_Obj *const *objv, const char *message);

#ifdef __cplusplus
}
#endif

#endif /* SPIKE_TCL_H */
