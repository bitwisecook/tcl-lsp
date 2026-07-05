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
 * ============================================================================
 * SPIKE -- throwaway proof-of-concept. NOT the final design. Representative
 * subset of the TclOO C API, enough to compile a real OO extension (pkgooa.c).
 * See docs/design/runtime/c-extension-abi.md.
 * ============================================================================
 *
 * tclOO.h -- authored, API-compatible subset of Tcl's object-system C API.
 *
 * NOTE on pkgooa.c: that sample deliberately introspects the *stub table*
 * (`tclOOStubsPtr->tcl_CopyObjectInstance`) -- a binary-ABI concept our
 * direct-linking model does not use. We provide the `TclOOStubs` shape and a
 * nominal `tclOOStubsPtr` so the source compiles; the address-comparison the
 * sample performs is not meaningful under our ABI. This is exactly the kind of
 * stubs-introspecting extension the design doc calls out as needing a nominal
 * (populated-with-our-functions) stub table for source compatibility.
 */
#ifndef SPIKE_TCLOO_H
#define SPIKE_TCLOO_H

#include "tcl.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Tcl_Object_ *Tcl_Object;
typedef struct Tcl_Class_ *Tcl_Class;
typedef struct Tcl_Method_ *Tcl_Method;
typedef struct Tcl_ObjectContext_ *Tcl_ObjectContext;

/* ---- representative public OO C API ---- */
extern Tcl_Object Tcl_CopyObjectInstance(Tcl_Interp *interp, Tcl_Object oPtr,
                                         const char *name, const char *nsName);
extern Tcl_Object Tcl_GetObjectFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr);
extern Tcl_Obj *Tcl_GetObjectName(Tcl_Interp *interp, Tcl_Object object);
extern Tcl_Class Tcl_GetObjectAsClass(Tcl_Object object);
extern Tcl_Object Tcl_NewObjectInstance(Tcl_Interp *interp, Tcl_Class cls,
                                        const char *nameStr,
                                        const char *nsNameStr, Tcl_Size objc,
                                        Tcl_Obj *const *objv, Tcl_Size skip);
extern Tcl_Class Tcl_GetClassAsObject(Tcl_Class clazz);
extern int Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
                                       Tcl_ObjectContext context, Tcl_Size objc,
                                       Tcl_Obj *const *objv, Tcl_Size skip);

/* ---- stub table shape (introspected by some extensions) ---- */
typedef struct TclOOStubs {
    int magic;
    void *hooks;
    Tcl_Object (*tcl_CopyObjectInstance)(Tcl_Interp *, Tcl_Object, const char *,
                                         const char *);
    /* The remaining ~30 slots are NULL in test fixtures; size generously so a
     * positional initializer (as in pkgooa.c) does not overflow the struct. */
    void *slot[64];
} TclOOStubs;

extern const TclOOStubs *tclOOStubsPtr;

/* Returns the version string (non-NULL) on success, NULL on failure. */
extern const char *Tcl_OOInitStubs(Tcl_Interp *interp);

#ifdef __cplusplus
}
#endif

#endif /* SPIKE_TCLOO_H */
