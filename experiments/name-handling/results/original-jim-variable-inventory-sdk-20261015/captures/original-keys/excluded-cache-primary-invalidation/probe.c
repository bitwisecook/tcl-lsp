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
static const Jim_ObjType argument_type = {"inventory-argument-probe", NULL, NULL, update_argument, JIM_TYPE_NONE};
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
    Jim_Interp *i=fresh();if(!i)return 2;
    label="VERSION";Jim_Eval(i,"info patchlevel");result_bytes(0,Jim_GetResult(i));
    Jim_Obj *name=Jim_NewIntObj(i,123);Jim_IncrRefCount(name);
    Jim_Obj *value=Jim_NewIntObj(i,789);Jim_IncrRefCount(value);
    if(Jim_SetVariable(i,name,value)!=JIM_OK)return 3;
    Jim_Obj *head=Jim_NewStringObj(i,"info",-1);Jim_IncrRefCount(head);
    Jim_Obj *nons=Jim_NewStringObj(i,"-nons",-1);Jim_IncrRefCount(nons);
    Jim_Obj *globals=Jim_NewStringObj(i,"globals",-1);Jim_IncrRefCount(globals);
    for(int filtered=0;filtered<2;filtered++) {
        label=filtered?"FILTERED":"UNFILTERED";
        Jim_SetEmptyResult(i);Jim_InvalidateStringRep(name);
        Jim_Obj *pattern=Jim_NewIntObj(i,123);Jim_IncrRefCount(pattern);
        Jim_Obj *argv[]={head,nons,globals,pattern};
        sample("KEY_BEFORE",name);sample("PATTERN_BEFORE",pattern);
        printf("KEY_REFS_BEFORE|%s|%d\n",label,name->refCount);
        int code=Jim_EvalObjVector(i,filtered?4:3,argv);
        sample("KEY_AFTER",name);sample("PATTERN_AFTER",pattern);
        printf("KEY_REFS_AFTER|%s|%d\n",label,name->refCount);
        Jim_Obj *result=Jim_GetResult(i);int length=Jim_ListLength(i,result),found=0;
        for(int n=0;n<length;n++){Jim_Obj *member;Jim_ListIndex(i,result,n,&member,JIM_NONE);if(member==name)found++;}
        printf("KEY_IS_ORIGINAL|%s|%d\n",label,found);
        result_bytes(code,result);Jim_DecrRefCount(i,pattern);
    }
    Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,nons);Jim_DecrRefCount(i,globals);Jim_DecrRefCount(i,name);Jim_DecrRefCount(i,value);Jim_FreeInterp(i);return 0;
}
