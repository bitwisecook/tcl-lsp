// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <stdio.h>
#include <string.h>
#include "jim.h"

static const char *phase;
static Jim_Interp *actual;
static Jim_Obj *head;
static int reset_head;
static const char *head_bytes;
static int head_length;
static const char *override_arg0;

static void sample(const char *kind, Jim_Obj *object)
{
    printf("%s|%s|%d|%s\n", kind, phase, object->bytes != NULL,
        object->typePtr ? object->typePtr->name : "none");
}

static void bytes(const char *kind, int code, Jim_Obj *object)
{
    int length;
    const char *data = Jim_GetString(object, &length);
    printf("%s|%s|%d|", kind, phase, code);
    for (int n = 0; n < length; n++) printf("%02x", (unsigned char)data[n]);
    puts("");
}

static void assign_string(Jim_Obj *object, const char *data, int length)
{
    object->length = length;
    object->bytes = Jim_Alloc(length + 1);
    memcpy(object->bytes, data, length);
    object->bytes[length] = 0;
}

static void update_head(Jim_Obj *object)
{
    sample("RESULT_AT_HEAD_GETTER", Jim_GetResult(actual));
    Jim_Obj *result = Jim_GetResult(actual);
    if (result->bytes) {
        printf("RESIDENT_RESULT_AT_HEAD_GETTER|%s|", phase);
        for (int n = 0; n < result->length; n++) printf("%02x", (unsigned char)result->bytes[n]);
        puts("");
    }
    assign_string(object, head_bytes, head_length);
}

static const Jim_ObjType head_type = {
    "namespace-head-probe", NULL, NULL, update_head, JIM_TYPE_NONE
};

static void update_selector(Jim_Obj *object)
{
    const char *selector = object->internalRep.ptr;
    sample("RESULT_AT_SELECTOR_GETTER", Jim_GetResult(actual));
    if (reset_head) {
        Jim_FreeIntRep(actual, head);
        Jim_InvalidateStringRep(head);
        head->typePtr = &head_type;
    }
    assign_string(object, selector, (int)strlen(selector));
}

static const Jim_ObjType selector_type = {
    "namespace-selector-probe", NULL, NULL, update_selector, JIM_TYPE_NONE
};

static Jim_Interp *fresh(void)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    return Jim_InitStaticExtensions(interp) == JIM_OK ? interp : NULL;
}

static int run(const char *label, const char *selector, int selector_length,
    int argument_count, int renamed, int probe)
{
    actual = fresh();
    if (!actual) return 3;
    if (renamed && Jim_Eval(actual, "rename namespace renamed") != JIM_OK) return 4;
    head_bytes = renamed == 2 ? "renamed\0TAIL" : renamed ? "renamed" : "namespace";
    head_length = renamed == 2 ? 12 : (int)strlen(head_bytes);
    head = Jim_NewStringObj(actual, head_bytes, probe ? (int)strlen(head_bytes) : head_length);
    Jim_IncrRefCount(head);
    Jim_Obj *member = Jim_NewStringObj(actual, selector, selector_length);
    Jim_IncrRefCount(member);
    if (probe) {
        Jim_InvalidateStringRep(member);
        member->typePtr = &selector_type;
        member->internalRep.ptr = (void *)selector;
    }
    Jim_Obj *caller = Jim_NewIntObj(actual, 42);
    Jim_IncrRefCount(caller);
    for (int reuse = 0; reuse < 2; reuse++) {
        char current_phase[128];
        snprintf(current_phase, sizeof(current_phase), "%s_%s", label, reuse ? "REUSE" : "COLD");
        phase = current_phase;
        reset_head = probe;
        if (reuse) Jim_InvalidateStringRep(caller);
        Jim_Obj *arguments[3];
        for (int n = 0; n < 3; n++) {
            arguments[n] = n == 0 && override_arg0
                ? Jim_NewStringObj(actual, override_arg0, -1)
                : Jim_NewIntObj(actual, 84 + n);
            Jim_IncrRefCount(arguments[n]);
        }
        Jim_Obj *argv[] = { head, member, arguments[0], arguments[1], arguments[2] };
        Jim_Obj *vector = Jim_NewListObj(actual, argv, 2 + argument_count);
        Jim_IncrRefCount(vector);
        sample("CALLER_BEFORE", caller);
        sample("MEMBER_BEFORE", member);
        sample("HEAD_BEFORE", head);
        for (int n = 0; n < 3; n++) sample(n == 0 ? "ARG0_BEFORE" : n == 1 ? "ARG1_BEFORE" : "ARG2_BEFORE", arguments[n]);
        int code = Jim_EvalNamespace(actual, vector, caller);
        sample("CALLER_AFTER", caller);
        sample("MEMBER_AFTER", member);
        sample("HEAD_AFTER", head);
        for (int n = 0; n < 3; n++) sample(n == 0 ? "ARG0_AFTER" : n == 1 ? "ARG1_AFTER" : "ARG2_AFTER", arguments[n]);
        sample("RESULT_HEADER", Jim_GetResult(actual));
        bytes("RESULT", code, Jim_GetResult(actual));
        Jim_DecrRefCount(actual, vector);
        for (int n = 0; n < 3; n++) Jim_DecrRefCount(actual, arguments[n]);
    }
    Jim_DecrRefCount(actual, caller);
    Jim_DecrRefCount(actual, member);
    Jim_DecrRefCount(actual, head);
    Jim_FreeInterp(actual);
    return 0;
}

#define RUN(...) do { int status = run(__VA_ARGS__); if (status) return status; } while (0)

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    actual = fresh();
    if (!actual) return 2;
    phase = "VERSION";
    int code = Jim_Eval(actual, "info patchlevel");
    bytes("RESULT", code, Jim_GetResult(actual));
    Jim_FreeInterp(actual);
    const char *members[] = { "canonical", "code", "current", "delete", "ensemble",
        "eval", "export", "import", "inscope", "origin", "parent", "qualifiers", "tail", "upvar", "which" };
    for (unsigned n = 0; n < sizeof(members) / sizeof(members[0]); n++) {
        for (int count = 0; count <= 3; count++) {
            char label[128];
            snprintf(label, sizeof(label), "TABLE_%s_%d", members[n], count);
            RUN(label, members[n], (int)strlen(members[n]), count, 0, 0);
        }
    }
    RUN("PREFIX_ARITY", "upv", 3, 0, 0, 0);
    RUN("EXACT_NUL_ARITY", "upvar\0TAIL", 10, 0, 0, 0);
    RUN("PREFIX_NUL", "upv\0TAIL", 8, 0, 0, 0);
    RUN("EMPTY", "", 0, 0, 0, 0);
    RUN("AMBIGUOUS", "c", 1, 0, 0, 0);
    RUN("MISSING", "unavailable", 11, 0, 0, 0);
    RUN("CODE_PREFIX_ARITY", "co", 2, 0, 0, 0);
    RUN("CODE_PREFIX_HELPER", "co", 2, 1, 0, 0);
    RUN("RENAMED_ARITY", "upv", 3, 0, 1, 0);
    RUN("RENAMED_NUL_HEAD_ARITY", "upv", 3, 0, 2, 0);
    RUN("HEAD_GETTER_ARITY", "upv", 3, 0, 0, 1);
    RUN("HEAD_GETTER_NUL_ARITY", "upv", 3, 0, 2, 1);
    RUN("RENAMED_MISSING", "unavailable", 11, 0, 1, 0);
    RUN("RENAMED_EMPTY", "", 0, 0, 1, 0);
    RUN("HEAD_GETTER_NUL_MISSING", "unavailable", 11, 0, 2, 1);
    RUN("COMMANDS", "-commands", 9, 0, 0, 0);
    RUN("COMMANDS_EXTRAS", "-commands", 9, 2, 0, 0);
    RUN("HELP", "-help", 5, 0, 0, 0);
    override_arg0 = "upv";
    RUN("HELP_MEMBER", "-help", 5, 1, 0, 0);
    RUN("HELP_MEMBER_EXTRAS", "-help", 5, 2, 0, 0);
    override_arg0 = "-commands";
    RUN("HELP_COMMANDS", "-help", 5, 1, 0, 0);
    override_arg0 = "unavailable";
    RUN("HELP_MISSING", "-help", 5, 1, 0, 0);
    override_arg0 = NULL;
    return 0;
}
