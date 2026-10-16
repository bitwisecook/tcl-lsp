// SPDX-License-Identifier: AGPL-3.0-or-later
#include "jim-config.h"
#include "jim.h"
#include <stdio.h>
#include <string.h>

static Jim_Obj *body;
static Jim_Obj *deletion_name;
static int getters;
static int body_frees;
static int result_frees;
static int old_slot;
static int old_refs;
static int before_delete;
static int after_delete;
static int deletion_code;

static void update_body(Jim_Obj *object)
{
    static const char source[] = "return ORIGINAL_BODY";
    ++getters;
    object->bytes = Jim_Alloc(sizeof(source));
    memcpy(object->bytes, source, sizeof(source));
    object->length = (int)sizeof(source) - 1;
}
static void free_body(Jim_Interp *interp, Jim_Obj *object)
{
    (void)interp;
    (void)object;
    ++body_frees;
}
static const Jim_ObjType body_type = {
    "physicalBodyOriginal", free_body, NULL, update_body, 0
};
static void free_result(Jim_Interp *interp, Jim_Obj *object)
{
    ++result_frees;
    old_slot = Jim_GetResult(interp) == object;
    old_refs = object->refCount;
    before_delete = body->refCount;
    deletion_code = Jim_DeleteCommand(interp, deletion_name);
    after_delete = body->refCount;
}
static const Jim_ObjType result_type = {
    "physicalBodyOldResult", free_result, NULL, NULL, 0
};
static void report(Jim_Interp *interp, Jim_Obj *name, const char *phase, int code)
{
    printf("ROW|%s|code=%d|same_body=%d|body_string=%d|body_type=%s|body_refs=%d|body_getters=%d|name_string=%d|name_type=%s\n",
        phase, code, Jim_GetResult(interp) == body, body->bytes != NULL,
        body->typePtr ? body->typePtr->name : "none", body->refCount, getters,
        name->bytes != NULL, name->typePtr ? name->typePtr->name : "none");
}
int main(void)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_Obj *declaration[4];
    Jim_Obj *query[3];
    Jim_Cmd *info;
    int i;
    int code;
    printf("VERSION|%s\n", JIM_GITVERSION);
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    body = Jim_NewObj(interp);
    body->bytes = NULL;
    body->length = 0;
    body->typePtr = &body_type;
    declaration[0] = Jim_NewStringObj(interp, "proc", -1);
    declaration[1] = Jim_NewStringObj(interp, "::Holder::p", -1);
    declaration[2] = Jim_NewListObj(interp, NULL, 0);
    declaration[3] = body;
    for (i = 0; i < 4; ++i) Jim_IncrRefCount(declaration[i]);
    code = Jim_EvalObjVector(interp, 4, declaration);
    printf("DEFINE|code=%d|body_refs=%d|body_getters=%d|body_string=%d\n",
        code, body->refCount, getters, body->bytes != NULL);
    if (code != JIM_OK) return 3;
    for (i = 0; i < 4; ++i) Jim_DecrRefCount(interp, declaration[i]);
    query[0] = Jim_NewStringObj(interp, "info", -1);
    query[1] = Jim_NewStringObj(interp, "body", -1);
    query[2] = Jim_NewStringObj(interp, "::Holder::p", -1);
    for (i = 0; i < 3; ++i) Jim_IncrRefCount(query[i]);
    info = Jim_GetCommand(interp, query[0], JIM_ERRMSG);
    if (!info || (info->flags & JIM_CMD_ISPROC)) return 4;
    code = info->u.native.cmdProc(interp, 3, query);
    report(interp, query[2], "PRIMITIVE_COLD", code);
    Jim_SetEmptyResult(interp);
    Jim_InvalidateStringRep(query[2]);
    code = info->u.native.cmdProc(interp, 3, query);
    report(interp, query[2], "PRIMITIVE_WARM", code);
    Jim_SetEmptyResult(interp);
    code = Jim_EvalObjVector(interp, 3, query);
    report(interp, query[2], "FULL_EVAL", code);
    Jim_SetEmptyResult(interp);
    deletion_name = Jim_NewStringObj(interp, "::Holder::p", -1);
    Jim_IncrRefCount(deletion_name);
    {
        Jim_Obj *prior = Jim_NewObj(interp);
        prior->bytes = NULL;
        prior->length = 0;
        prior->typePtr = &result_type;
        Jim_SetResult(interp, prior);
    }
    code = info->u.native.cmdProc(interp, 3, query);
    report(interp, query[2], "PRIMITIVE_OLD_RESULT_DELETE", code);
    printf("CALLBACK|count=%d|old_slot=%d|old_refs=%d|before=%d|after=%d|delete=%d|body_frees=%d\n",
        result_frees, old_slot, old_refs, before_delete, after_delete, deletion_code, body_frees);
    Jim_SetEmptyResult(interp);
    printf("CLEANUP|body_frees=%d|body_getters=%d\n", body_frees, getters);
    Jim_DecrRefCount(interp, deletion_name);
    for (i = 0; i < 3; ++i) Jim_DecrRefCount(interp, query[i]);
    Jim_FreeInterp(interp);
    return 0;
}
