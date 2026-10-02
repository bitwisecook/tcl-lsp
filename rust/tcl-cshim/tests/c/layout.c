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
 * layout.c — what the authored header declares for Tcl_Obj, and what an
 * extension may do with one, as a C compiler sees it.
 *
 * The shim compiles this against runtime/rust/include/tcl.h so its own
 * `Obj` can be held to the header's layout, and a wasm32 compile of the same
 * file (`make check-c-extension-wasm`) holds the header to the layout the WASM
 * runtime's `TclObj` has (c-extension-abi.md section 4.2). The functions are
 * plain C an extension could contain: reads through the fields and uses of the
 * reference-count macros.
 */

#include <tcl.h>
#include <stddef.h>

#if defined(__wasm32__)
/* The runtime's TclObj: { i32, ptr, i32, ptr, 8-byte union }, 8-aligned. */
_Static_assert(sizeof(Tcl_Obj) == 24, "Tcl_Obj is 24 bytes on wasm32");
_Static_assert(offsetof(Tcl_Obj, refCount) == 0, "refCount at 0");
_Static_assert(offsetof(Tcl_Obj, bytes) == 4, "bytes at 4");
_Static_assert(offsetof(Tcl_Obj, length) == 8, "length at 8");
_Static_assert(offsetof(Tcl_Obj, typePtr) == 12, "typePtr at 12");
_Static_assert(offsetof(Tcl_Obj, internalRep) == 16, "internalRep at 16");
_Static_assert(sizeof(Tcl_ObjInternalRep) == 8, "the union is 8 bytes on wasm32");
#elif defined(__LP64__)
/* Tcl 9.0's own layout on an LP64 target. */
_Static_assert(sizeof(Tcl_Obj) == 48, "Tcl_Obj is 48 bytes on LP64");
_Static_assert(offsetof(Tcl_Obj, refCount) == 0, "refCount at 0");
_Static_assert(offsetof(Tcl_Obj, bytes) == 8, "bytes at 8");
_Static_assert(offsetof(Tcl_Obj, length) == 16, "length at 16");
_Static_assert(offsetof(Tcl_Obj, typePtr) == 24, "typePtr at 24");
_Static_assert(offsetof(Tcl_Obj, internalRep) == 32, "internalRep at 32");
_Static_assert(sizeof(Tcl_ObjInternalRep) == 16, "the union is 16 bytes on LP64");
#endif

/* sizeof(Tcl_Obj) and the offset of each declared field, in declaration order,
 * then sizeof(Tcl_ObjInternalRep). */
const size_t tclshim_test_layout[] = {
    sizeof(Tcl_Obj),
    offsetof(Tcl_Obj, refCount),
    offsetof(Tcl_Obj, bytes),
    offsetof(Tcl_Obj, length),
    offsetof(Tcl_Obj, typePtr),
    offsetof(Tcl_Obj, internalRep),
    sizeof(Tcl_ObjInternalRep),
};

/* Reads through the fields, as an extension is allowed to. */
const char *
tclshim_test_bytes_of(Tcl_Obj *obj)
{
    return obj->bytes;
}

ptrdiff_t
tclshim_test_length_of(Tcl_Obj *obj)
{
    return obj->length;
}

ptrdiff_t
tclshim_test_ref_count_of(Tcl_Obj *obj)
{
    return obj->refCount;
}

/* The reference-count macros. */
int
tclshim_test_shared_of(Tcl_Obj *obj)
{
    return Tcl_IsShared(obj);
}

void
tclshim_test_incr_of(Tcl_Obj *obj)
{
    Tcl_IncrRefCount(obj);
}

void
tclshim_test_decr_of(Tcl_Obj *obj)
{
    Tcl_DecrRefCount(obj);
}
