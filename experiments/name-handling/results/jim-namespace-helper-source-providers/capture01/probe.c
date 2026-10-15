/* SPDX-License-Identifier: AGPL-3.0-or-later */
/* Observe original SDK procedure objects; no alternate helper implementation. */
#include <stdio.h>
#include "jim.h"

extern int Jim_namespaceInit(Jim_Interp *interp);

static const char *names[] = {
    "namespace delete", "namespace origin", "namespace which", "namespace code",
    "namespace inscope", "namespace import", "namespace info", "namespace upvar",
    "namespace ensemble"
};

static void bytes(Jim_Obj *object)
{
    int count;
    const unsigned char *value = (const unsigned char *)Jim_GetString(object, &count);
    for (int index = 0; index < count; index++) printf("%02x", value[index]);
}

static void object(Jim_Interp *interp, const char *provider, unsigned ordinal,
    const char *role, Jim_Obj *original)
{
    printf("OBJECT|%s|%u|%s|%d|%s|%d\n", provider, ordinal, role,
        original->bytes != NULL, original->typePtr ? original->typePtr->name : "none",
        original->refCount);
    printf("BYTES|%s|%u|%s|", provider, ordinal, role);
    bytes(original);
    puts("");
    int line;
    Jim_Obj *filename = Jim_GetSourceInfo(interp, original, &line);
    printf("SOURCE|%s|%u|%s|%d|", provider, ordinal, role, line);
    bytes(filename);
    puts("");
}

static int sample(Jim_Interp *interp, const char *provider, int required)
{
    for (unsigned index = 0; index < sizeof(names) / sizeof(names[0]); index++) {
        Jim_Obj *name = Jim_NewStringObj(interp, names[index], -1);
        Jim_IncrRefCount(name);
        Jim_Cmd *command = Jim_GetCommand(interp, name, 0);
        printf("HELPER|%s|%u|", provider, index);
        bytes(name);
        printf("|%d|%d\n", command != NULL, command ? command->flags : 0);
        if (command && (command->flags & JIM_CMD_ISPROC)) {
            object(interp, provider, index, "FORMALS", command->u.proc.argListObjPtr);
            object(interp, provider, index, "BODY", command->u.proc.bodyObjPtr);
        }
        Jim_DecrRefCount(interp, name);
        if (required && (!command || !(command->flags & JIM_CMD_ISPROC))) return 10;
    }
    return 0;
}

static Jim_Interp *core(void)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    return interp;
}

int main(int argc, char **argv)
{
    if (argc != 2) return 1;
    setvbuf(stdout, NULL, _IONBF, 0);
    Jim_Interp *interp = core();
    int status = sample(interp, "CORE", 0);
    if (status) return status;
    int code = Jim_namespaceInit(interp);
    printf("SETUP|NAMESPACE_PRIMITIVE|%d\n", code);
    if (code != JIM_OK) return 2;
    status = sample(interp, "NAMESPACE_PRIMITIVE", 0);
    Jim_FreeInterp(interp);
    if (status) return status;

    interp = core();
    code = Jim_InitStaticExtensions(interp);
    printf("SETUP|STATIC|%d\n", code);
    if (code != JIM_OK) return 3;
    status = sample(interp, "STATIC", 1);
    if (status) return status;
    code = Jim_EvalFile(interp, argv[1]);
    printf("SETUP|RAW_AFTER_STATIC|%d\n", code);
    if (code != JIM_OK) return 4;
    status = sample(interp, "RAW_AFTER_STATIC", 1);
    Jim_FreeInterp(interp);
    if (status) return status;

    interp = core();
    code = Jim_EvalFile(interp, argv[1]);
    printf("SETUP|RAW_ONLY|%d\n", code);
    if (code != JIM_OK) return 5;
    status = sample(interp, "RAW_ONLY", 1);
    Jim_FreeInterp(interp);
    return status;
}
