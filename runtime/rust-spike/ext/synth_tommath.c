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
 * SPIKE -- throwaway proof-of-concept, NOT a real extension. Synthetic bignum
 * extension that compile-checks tclTomMath.h (raw mp_* arithmetic) plus the
 * public Tcl_NewBignumObj in tcl.h. Never run.
 * ============================================================================
 */
#undef STATIC_BUILD
#include "tcl.h"
#include "tclTomMath.h"

DLLEXPORT int Synthbn_Init(Tcl_Interp *interp) {
    if (Tcl_InitStubs(interp, "9.0-", 0) == NULL) {
        return TCL_ERROR;
    }
    mp_int a, b, c;
    mp_init(&a);
    mp_init(&b);
    mp_init(&c);
    mp_set(&a, 40);
    mp_set(&b, 2);
    mp_add(&a, &b, &c);
    Tcl_SetObjResult(interp, Tcl_NewBignumObj(&c));
    mp_clear(&a);
    mp_clear(&b);
    mp_clear(&c);
    Tcl_PkgProvide(interp, "synthbn", "1.0");
    return TCL_OK;
}
