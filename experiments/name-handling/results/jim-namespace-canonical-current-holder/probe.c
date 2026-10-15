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


#include <stdio.h>
#include "jim.h"

static void header(const char *field, const char *phase, Jim_Obj *value)
{
    printf("%s|%s|%d|%s\n", field, phase, value->bytes != NULL,
        value->typePtr ? value->typePtr->name : "none");
}

int main(void)
{
    const struct { const char *label; int operands; const char *name; } cases[] = {
        { "CURRENT", 0, "child" },
        { "ABSOLUTE", 1, "::target" },
        { "RELATIVE", 1, "child" },
        { "EXPLICIT", 2, "child" }
    };
    setvbuf(stdout, NULL, _IONBF, 0);
    for (unsigned c = 0; c < sizeof(cases) / sizeof(cases[0]); c++) {
        Jim_Interp *interp = Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
        Jim_Obj *caller = Jim_NewIntObj(interp, 42);
        Jim_Obj *head = Jim_NewStringObj(interp, "namespace", -1);
        Jim_Obj *member = Jim_NewStringObj(interp, "canonical", -1);
        Jim_IncrRefCount(caller);
        Jim_IncrRefCount(head);
        Jim_IncrRefCount(member);
        for (int reuse = 0; reuse < 2; reuse++) {
            char phase[64];
            snprintf(phase, sizeof(phase), "%s_%s", cases[c].label, reuse ? "REUSE" : "COLD");
            if (reuse) Jim_InvalidateStringRep(caller);
            Jim_Obj *prefix = Jim_NewIntObj(interp, 84);
            Jim_Obj *name = Jim_NewStringObj(interp, cases[c].name, -1);
            Jim_IncrRefCount(prefix);
            Jim_IncrRefCount(name);
            Jim_Obj *argv[] = { head, member, name, name };
            if (cases[c].operands == 2) argv[2] = prefix;
            Jim_Obj *vector = Jim_NewListObj(interp, argv, 2 + cases[c].operands);
            Jim_IncrRefCount(vector);
            header("CALLER_BEFORE", phase, caller);
            header("PREFIX_BEFORE", phase, prefix);
            header("NAME_BEFORE", phase, name);
            int code = Jim_EvalNamespace(interp, vector, caller);
            header("CALLER_AFTER", phase, caller);
            header("PREFIX_AFTER", phase, prefix);
            header("NAME_AFTER", phase, name);
            Jim_Obj *result = Jim_GetResult(interp);
            header("RESULT_HEADER", phase, result);
            printf("RESULT_IS_CALLER|%s|%d\n", phase, result == caller);
            printf("RESULT_IS_NAME|%s|%d\n", phase, result == name);
            int length;
            const char *bytes = Jim_GetString(result, &length);
            printf("RESULT|%s|%d|", phase, code);
            for (int k = 0; k < length; k++) printf("%02x", (unsigned char)bytes[k]);
            puts("");
            Jim_DecrRefCount(interp, vector);
            Jim_DecrRefCount(interp, prefix);
            Jim_DecrRefCount(interp, name);
        }
        Jim_DecrRefCount(interp, member);
        Jim_DecrRefCount(interp, head);
        Jim_DecrRefCount(interp, caller);
        Jim_FreeInterp(interp);
    }
    return 0;
}
