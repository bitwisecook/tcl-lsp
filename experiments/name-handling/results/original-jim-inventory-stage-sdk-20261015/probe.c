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
    Jim_Interp *v=fresh();if(!v)return 2;label="VERSION";Jim_Eval(v,"info patchlevel");result_bytes(0,Jim_GetResult(v));Jim_FreeInterp(v);
    const struct {const char *name;int nons,count,all,rooted;} cases[]={
        {"RAW_PATTERN",1,1,0,0},{"RAW_NONE",1,0,0,0},{"RAW_ALL",1,2,1,0},{"RAW_ARITY",1,2,0,0},
        {"AWARE_PATTERN",0,1,0,0},{"AWARE_ALL",0,2,1,0},{"AWARE_ARITY",0,2,0,0},
        {"ROOTED_RAW_PATTERN",1,1,0,1},{"ROOTED_AWARE_ARITY",0,2,0,1}
    };
    for(unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
        Jim_Interp *i=fresh();if(!i)return 3;
        if(Jim_Eval(i,"proc 42::target {} {}")!=JIM_OK)return 4;
        Jim_Obj *head=Jim_NewStringObj(i,cases[n].rooted?"::info":"info",-1);Jim_IncrRefCount(head);
        Jim_Obj *ns=Jim_NewIntObj(i,42);Jim_IncrRefCount(ns);observed_namespace=ns;
        Jim_Obj *nons=Jim_NewStringObj(i,"-nons",-1);Jim_IncrRefCount(nons);
        Jim_Obj *commands=Jim_NewStringObj(i,"commands",-1);Jim_IncrRefCount(commands);
        Jim_Obj *all=Jim_NewStringObj(i,"-all",-1);Jim_IncrRefCount(all);
        Jim_Obj *other=Jim_NewStringObj(i,"other",-1);Jim_IncrRefCount(other);
        for(int hot=0;hot<2;hot++) {
            char phase[128];snprintf(phase,sizeof(phase),"%s_%s",cases[n].name,hot?"HOT":"COLD");label=phase;
            if(hot)Jim_InvalidateStringRep(ns);
            payload="::42::target";payload_length=strlen(payload);
            Jim_Obj *arg=Jim_NewStringObj(i,"",0);Jim_IncrRefCount(arg);Jim_InvalidateStringRep(arg);arg->typePtr=&argument_type;
            Jim_Obj *argv[6];int argc=0;argv[argc++]=head;if(cases[n].nons)argv[argc++]=nons;argv[argc++]=commands;
            if(cases[n].count>0)argv[argc++]=cases[n].all?all:arg;
            if(cases[n].count>1)argv[argc++]=cases[n].all?arg:other;
            Jim_Obj *list=Jim_NewListObj(i,argv,argc);Jim_IncrRefCount(list);
            sample("NAMESPACE_BEFORE",ns);sample("ARG_BEFORE",arg);
            int code=Jim_EvalNamespace(i,list,ns);
            sample("NAMESPACE_AFTER",ns);sample("ARG_AFTER",arg);
            result_bytes(code,Jim_GetResult(i));Jim_DecrRefCount(i,list);Jim_DecrRefCount(i,arg);
        }
        Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,ns);Jim_DecrRefCount(i,nons);Jim_DecrRefCount(i,commands);Jim_DecrRefCount(i,all);Jim_DecrRefCount(i,other);Jim_FreeInterp(i);
    }return 0;
}
