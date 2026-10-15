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
static const char *label;
static Jim_Obj *caller, *target, *body, *token, *extra;
static int worker_code;
static void sample(const char *kind, Jim_Obj *object) {
    printf("%s|%s|%d|%s\n", kind, label, object->bytes != NULL,
        object->typePtr ? object->typePtr->name : "none");
}
static int observe(Jim_Interp *interp, int argc, Jim_Obj *const *argv) {
    printf("INVOKE|%s|%d|%d|%d\n", label, argc, argc > 1 && argv[1] == token, argc > 2 && argv[2] == extra);
    sample("CALLER_INVOKE",caller); sample("TARGET_INVOKE",target); sample("BODY_INVOKE",body); sample("TOKEN_INVOKE",token);sample("EXTRA_INVOKE",extra);
    printf("CURRENT_IS_TARGET|%s|%d\n", label, interp->framePtr->nsObj == target);
    Jim_SetResult(interp,token);return worker_code;
}
static Jim_Interp *fresh(void) {
    Jim_Interp *interp=Jim_CreateInterp();Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp)!=JIM_OK)return NULL;
    Jim_CreateCommand(interp,"observe",observe,NULL,NULL);return interp;
}
static void result_bytes(int code,Jim_Obj *object) {
    int length;const char *bytes=Jim_GetString(object,&length);printf("RESULT|%s|%d|",label,code);
    for(int n=0;n<length;n++)printf("%02x",(unsigned char)bytes[n]);puts("");
}
int main(void) {
    Jim_Interp *v=fresh();if(!v)return 2;label="VERSION";Jim_Eval(v,"info patchlevel");result_bytes(0,Jim_GetResult(v));Jim_FreeInterp(v);
    const struct {const char *name;int direct,args,change,code;} cases[]={
        {"PUBLIC_LIST",0,2,0,JIM_OK},{"PUBLIC_EXTRA",0,3,0,JIM_OK},
        {"DIRECT_LIST",1,2,0,JIM_OK},{"DIRECT_EXTRA",1,3,0,JIM_OK},
        {"PUBLIC_RETURN",0,2,0,JIM_RETURN},{"DIRECT_RETURN",1,2,0,JIM_RETURN},
        {"PUBLIC_ARITY_ZERO",0,0,0,JIM_OK},{"PUBLIC_ARITY_ONE",0,1,0,JIM_OK},
        {"DIRECT_ARITY_ZERO",1,0,0,JIM_OK},{"DIRECT_ARITY_ONE",1,1,0,JIM_OK},
        {"PUBLIC_REPLACED",0,2,1,JIM_OK},{"DIRECT_REPLACED",1,2,1,JIM_OK},
        {"PUBLIC_DELETED",0,2,2,JIM_OK},{"DIRECT_DELETED",1,2,2,JIM_OK}
    };
    for(unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
        Jim_Interp *i=fresh();if(!i)return 3;
        if(cases[n].change && Jim_Eval(i,cases[n].change==1?"rename {namespace inscope} saved; proc {namespace inscope} {name arg args} {return REPLACED}":"rename {namespace inscope} {}")!=JIM_OK)return 4;
        Jim_Obj *head=Jim_NewStringObj(i,cases[n].direct?"namespace inscope":"namespace",-1);Jim_IncrRefCount(head);
        Jim_Obj *member=Jim_NewStringObj(i,"inscope",-1);Jim_IncrRefCount(member);
        caller=Jim_NewIntObj(i,42);Jim_IncrRefCount(caller);
        target=Jim_NewIntObj(i,84);Jim_IncrRefCount(target);
        for(int hot=0;hot<2;hot++) {
            char phase[128];snprintf(phase,sizeof(phase),"%s_%s",cases[n].name,hot?"HOT":"COLD");label=phase;worker_code=cases[n].code;
            if(hot){Jim_InvalidateStringRep(caller);Jim_InvalidateStringRep(target);}
            token=Jim_NewIntObj(i,123);Jim_IncrRefCount(token);extra=Jim_NewIntObj(i,456);Jim_IncrRefCount(extra);
            Jim_Obj *command=Jim_NewStringObj(i,"observe",-1);Jim_Obj *parts[]={command,token};body=Jim_NewListObj(i,parts,2);Jim_IncrRefCount(body);
            Jim_Obj *argv[5];int argc=0;argv[argc++]=head;if(!cases[n].direct)argv[argc++]=member;
            if(cases[n].args>0)argv[argc++]=target;if(cases[n].args>1)argv[argc++]=body;if(cases[n].args>2)argv[argc++]=extra;
            Jim_Obj *vector=Jim_NewListObj(i,argv,argc);Jim_IncrRefCount(vector);
            sample("CALLER_BEFORE",caller);sample("TARGET_BEFORE",target);sample("BODY_BEFORE",body);sample("TOKEN_BEFORE",token);sample("EXTRA_BEFORE",extra);
            int code=Jim_EvalNamespace(i,vector,caller);
            sample("CALLER_AFTER",caller);sample("TARGET_AFTER",target);sample("BODY_AFTER",body);sample("TOKEN_AFTER",token);sample("EXTRA_AFTER",extra);
            printf("RESULT_IS_TOKEN|%s|%d\n",label,Jim_GetResult(i)==token);result_bytes(code,Jim_GetResult(i));
            Jim_DecrRefCount(i,vector);Jim_DecrRefCount(i,body);Jim_DecrRefCount(i,token);Jim_DecrRefCount(i,extra);
        }
        Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,member);Jim_DecrRefCount(i,caller);Jim_DecrRefCount(i,target);Jim_FreeInterp(i);
    }return 0;
}
