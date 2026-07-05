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
 * SPIKE -- throwaway proof-of-concept, NOT a real extension. This synthetic
 * extension exists only to compile-check the widened tcl.h surface (custom
 * Tcl_ObjType, channels, filesystems, threading, NRE) that the real dltest
 * samples do not exercise. It is never run.
 * ============================================================================
 */
#undef STATIC_BUILD
#include "tcl.h"

/* --- custom Tcl_ObjType --- */
static void MyFree(Tcl_Obj *o) { (void) o; }
static void MyDup(Tcl_Obj *s, Tcl_Obj *d) { (void) s; (void) d; }
static void MyUpdate(Tcl_Obj *o) { (void) o; }
static int MySetAny(Tcl_Interp *i, Tcl_Obj *o) { (void) i; (void) o; return TCL_OK; }
static const Tcl_ObjType myType = {
    .name = "myType",
    .freeIntRepProc = MyFree,
    .dupIntRepProc = MyDup,
    .updateStringProc = MyUpdate,
    .setFromAnyProc = MySetAny,
};

/* --- channel driver --- */
static int MyInput(void *cd, char *buf, int n, int *ec) {
    (void) cd; (void) buf; (void) n; (void) ec; return 0;
}
static int MyOutput(void *cd, const char *buf, int n, int *ec) {
    (void) cd; (void) buf; (void) ec; return n;
}
static void MyWatch(void *cd, int m) { (void) cd; (void) m; }
static int MyGetHandle(void *cd, int d, void **h) {
    (void) cd; (void) d; (void) h; return TCL_ERROR;
}
static const Tcl_ChannelType myChan = {
    .typeName = "mychan",
    .inputProc = MyInput,
    .outputProc = MyOutput,
    .watchProc = MyWatch,
    .getHandleProc = MyGetHandle,
};

/* --- filesystem --- */
static int MyPathIn(Tcl_Obj *p, void **cd) { (void) p; (void) cd; return TCL_ERROR; }
static const Tcl_Filesystem myFs = {
    .typeName = "myfs",
    .structureLength = sizeof(Tcl_Filesystem),
    .version = 1,
    .pathInFilesystemProc = MyPathIn,
};

/* --- threading + NRE --- */
static void MyThread(void *cd) { (void) cd; }
static int MyCmd(void *cd, Tcl_Interp *i, int objc, Tcl_Obj *const *objv) {
    (void) cd; (void) i; (void) objc; (void) objv; return TCL_OK;
}

DLLEXPORT int Synth_Init(Tcl_Interp *interp) {
    if (Tcl_InitStubs(interp, "9.0-", 0) == NULL) {
        return TCL_ERROR;
    }
    Tcl_RegisterObjType(&myType);

    Tcl_Channel ch = Tcl_CreateChannel(&myChan, "mychan0", NULL, 0);
    (void) ch;

    Tcl_FSRegister(NULL, &myFs);

    Tcl_ThreadId tid;
    Tcl_CreateThread(&tid, MyThread, NULL, 0, 0);
    static Tcl_Mutex m;
    Tcl_MutexLock(&m);
    Tcl_MutexUnlock(&m);

    Tcl_NRCreateCommand(interp, "synth_cmd", MyCmd, MyCmd, NULL, NULL);

    Tcl_PkgProvide(interp, "synth", "1.0");
    return TCL_OK;
}
