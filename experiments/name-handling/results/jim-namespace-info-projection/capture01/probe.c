// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "jim.h"

static void hex(const char *bytes, int length)
{
    for (int n = 0; n < length; n++) printf("%02x", (unsigned char)bytes[n]);
}
static const char *type(Jim_Obj *object)
{
    return object->typePtr ? object->typePtr->name : "none";
}
static int run(const char *label, int empty, int replacement)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    const char *setup =
        "namespace eval RelayInfoSource {proc p {} {}}\n"
        "set force_projection 1\n"
        "rename namespace actual_namespace\n"
        "proc namespace args {\n"
        " if {[lindex $args 0] eq \"canon\" && [llength $args] == 3 && $::force_projection} {\n"
        "  set ::force_projection 0\n"
        "  return ::[actual_namespace canon {*}[lrange $args 1 end]]\n"
        " }\n"
        " tailcall actual_namespace {*}$args\n"
        "}\n";
    if (Jim_Eval(interp, setup) != JIM_OK) return 3;
    if (replacement == 1 && Jim_Eval(interp, "proc lmap {var values body} {return $values}") != JIM_OK) return 4;
    if (replacement == 2 && Jim_Eval(interp,
        "rename string actual_string; proc string args {if {[lindex $args 0] eq {cat}} {return FORGED}; tailcall actual_string {*}$args}") != JIM_OK) return 5;
    const char *input = empty ? "{namespace info} commands ::MissingInfoSource::*" : "{namespace info} commands ::RelayInfoSource::*";
    int code = Jim_Eval(interp, input);
    Jim_Obj *result = Jim_GetResult(interp);
    printf("HEADER|%s|%d|%s|%d|%d\n", label, code, type(result), result->bytes != NULL, result->refCount);
    int count = Jim_ListLength(interp, result);
    printf("LIST|%s|%d\n", label, count);
    for (int n = 0; n < count; n++) {
        Jim_Obj *member = Jim_ListGetIndex(interp, result, n);
        printf("MEMBER|%s|%d|%s|%d|%d|", label, n, type(member), member->bytes != NULL, member->refCount);
        int length;
        const char *bytes = Jim_GetString(member, &length);
        hex(bytes, length);
        printf("\n");
    }
    int length;
    const char *bytes = Jim_GetString(result, &length);
    printf("RESULT|%s|", label);
    hex(bytes, length);
    printf("\n");
    Jim_FreeInterp(interp);
    return 0;
}
int main(void)
{
    const char *labels[] = {"GENUINE", "EMPTY", "RETURN_INPUT_LMAP", "FORGED_CAT"};
    for (int n = 0; n < 4; n++) {
        int code = run(labels[n], n == 1, n < 2 ? 0 : n - 1);
        if (code) return code;
    }
    return 0;
}
