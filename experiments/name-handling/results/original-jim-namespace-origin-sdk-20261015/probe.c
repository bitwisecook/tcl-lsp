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
#include <string.h>
#include "jim.h"
static Jim_Obj *observed_namespace;
static const char *label;
static const char *payload;
static int payload_length;
static void update_argument(Jim_Obj *object) {
    printf("NAMESPACE_AT_ARG_GETTER|%s|%d|%s\n", label, observed_namespace->bytes != NULL,
        observed_namespace->typePtr ? observed_namespace->typePtr->name : "none");
    object->bytes = Jim_Alloc(payload_length + 1);
    memcpy(object->bytes, payload, payload_length);
    object->bytes[payload_length] = 0;
    object->length = payload_length;
}
static const Jim_ObjType argument_type = {"origin-argument-probe", NULL, NULL, update_argument, JIM_TYPE_NONE};
static Jim_Interp *fresh(void) {
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    return Jim_InitStaticExtensions(interp) == JIM_OK ? interp : NULL;
}
static void sample(const char *kind, Jim_Obj *object) {
    printf("%s|%s|%d|%s\n", kind, label, object->bytes != NULL,
        object->typePtr ? object->typePtr->name : "none");
}
static void result_bytes(int code, Jim_Obj *object) {
    int length;
    const char *bytes = Jim_GetString(object, &length);
    printf("RESULT|%s|%d|", label, code);
    for (int n = 0; n < length; n++) printf("%02x", (unsigned char)bytes[n]);
    puts("");
}
int main(void) {
    Jim_Interp *version = fresh();
    if (!version) return 2;
    label = "VERSION";
    if (Jim_Eval(version, "info patchlevel") != JIM_OK) return 3;
    result_bytes(0, Jim_GetResult(version));
    Jim_FreeInterp(version);
    static const char target[] = "target";
    static const char absolute[] = "::target";
    static const char alias[] = "alias_target";
    static const char chain[] = "alias_chain";
    static const char nul[] = "target\0TAIL";
    static const char missing[] = "missing";
    static const char empty[] = "";
    const struct { const char *name, *bytes; int length, direct, argc, change; } cases[] = {
        {"PUBLIC_RELATIVE", target, sizeof(target)-1, 0, 1, 0},
        {"PUBLIC_ABSOLUTE", absolute, sizeof(absolute)-1, 0, 1, 0},
        {"PUBLIC_ALIAS", alias, sizeof(alias)-1, 0, 1, 0},
        {"PUBLIC_CHAIN", chain, sizeof(chain)-1, 0, 1, 0},
        {"PUBLIC_NUL", nul, sizeof(nul)-1, 0, 1, 0},
        {"PUBLIC_MISSING", missing, sizeof(missing)-1, 0, 1, 0},
        {"PUBLIC_EMPTY", empty, 0, 0, 1, 0},
        {"DIRECT_RELATIVE", target, sizeof(target)-1, 1, 1, 0},
        {"DIRECT_ABSOLUTE", absolute, sizeof(absolute)-1, 1, 1, 0},
        {"DIRECT_ALIAS", alias, sizeof(alias)-1, 1, 1, 0},
        {"DIRECT_CHAIN", chain, sizeof(chain)-1, 1, 1, 0},
        {"DIRECT_NUL", nul, sizeof(nul)-1, 1, 1, 0},
        {"DIRECT_MISSING", missing, sizeof(missing)-1, 1, 1, 0},
        {"DIRECT_EMPTY", empty, 0, 1, 1, 0},
        {"PUBLIC_ARITY_ZERO", target, sizeof(target)-1, 0, 0, 0},
        {"PUBLIC_ARITY_TWO", target, sizeof(target)-1, 0, 2, 0},
        {"DIRECT_ARITY_ZERO", target, sizeof(target)-1, 1, 0, 0},
        {"DIRECT_ARITY_TWO", target, sizeof(target)-1, 1, 2, 0},
        {"PUBLIC_REPLACED", target, sizeof(target)-1, 0, 1, 1},
        {"DIRECT_REPLACED", target, sizeof(target)-1, 1, 1, 1},
        {"PUBLIC_DELETED", target, sizeof(target)-1, 0, 1, 2},
        {"DIRECT_DELETED", target, sizeof(target)-1, 1, 1, 2}
    };
    for (unsigned n = 0; n < sizeof(cases)/sizeof(cases[0]); n++) {
        Jim_Interp *interp = fresh();
        if (!interp) return 4;
        if (Jim_Eval(interp, "proc 42::target {} {return VALUE}; proc target {} {return GLOBAL}; alias 42::alias_target 42::target; alias 42::alias_chain 42::alias_target") != JIM_OK) return 6;
        if (cases[n].change && Jim_Eval(interp, cases[n].change == 1
            ? "rename {namespace origin} saved; proc {namespace origin} arg {return REPLACED}"
            : "rename {namespace origin} {}") != JIM_OK) return 5;
        Jim_Obj *ns = Jim_NewIntObj(interp, 42); Jim_IncrRefCount(ns);
        observed_namespace = ns; payload = cases[n].bytes; payload_length = cases[n].length;
        Jim_Obj *head = Jim_NewStringObj(interp, cases[n].direct ? "namespace origin" : "namespace", -1);
        Jim_IncrRefCount(head);
        Jim_Obj *member = Jim_NewStringObj(interp, "origin", -1); Jim_IncrRefCount(member);
        for (int hot = 0; hot < 2; hot++) {
            char phase[128]; snprintf(phase, sizeof(phase), "%s_%s", cases[n].name, hot ? "HOT" : "COLD"); label = phase;
            if (hot) Jim_InvalidateStringRep(ns);
            Jim_Obj *arg = Jim_NewStringObj(interp, "", 0); Jim_IncrRefCount(arg);
            Jim_InvalidateStringRep(arg); arg->typePtr = &argument_type;
            Jim_Obj *elements[4]; int argc = 0;
            elements[argc++] = head;
            if (!cases[n].direct) elements[argc++] = member;
            for (int j = 0; j < cases[n].argc; j++) elements[argc++] = arg;
            Jim_Obj *body = Jim_NewListObj(interp, elements, argc); Jim_IncrRefCount(body);
            sample("NAMESPACE_BEFORE", ns); sample("ARG_BEFORE", arg);
            int code = Jim_EvalNamespace(interp, body, ns);
            sample("NAMESPACE_AFTER", ns); sample("ARG_AFTER", arg);
            Jim_Obj *result = Jim_GetResult(interp);
            printf("RESULT_IS_ARG|%s|%d\n", label, result == arg);
            if (code == JIM_OK && result != arg && result->typePtr && !strcmp(result->typePtr->name, "list")) {
                int count = Jim_ListLength(interp, result);
                Jim_Obj *child = count == 4 ? Jim_ListGetIndex(interp, result, 3) : NULL;
                printf("RESULT_LIST|%s|%d|%d\n", label, count, child == arg);
            }
            result_bytes(code, result);
            Jim_DecrRefCount(interp, body); Jim_DecrRefCount(interp, arg);
        }
        Jim_DecrRefCount(interp, head); Jim_DecrRefCount(interp, member); Jim_DecrRefCount(interp, ns);
        Jim_FreeInterp(interp);
    }
    return 0;
}
