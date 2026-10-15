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
static Jim_Obj *caller;
static int deleted;
static void sample(const char *kind, Jim_Obj *object) {
    printf("%s|%s|%d|%s\n",kind,label,object->bytes != NULL,
        object->typePtr ? object->typePtr->name : "none");
}
static void update_argument(Jim_Obj *object) {
    const char *payload=object->internalRep.ptr;
    sample("CALLER_AT_NAME_GETTER",caller);
    object->length=(int)strlen(payload);
    object->bytes=Jim_Alloc(object->length+1);
    memcpy(object->bytes,payload,object->length+1);
}
static const Jim_ObjType argument_type={"delete-name-probe",NULL,NULL,update_argument,JIM_TYPE_NONE};
static Jim_Obj *name(Jim_Interp *i,const char *payload) {
    Jim_Obj *o=Jim_NewStringObj(i,"",0);Jim_InvalidateStringRep(o);
    o->typePtr=&argument_type;o->internalRep.ptr=(void *)payload;Jim_IncrRefCount(o);return o;
}
static void deleted_command(Jim_Interp *i,void *unused) {
    (void)i;(void)unused;deleted++;
    printf("COMMAND_DELETE|%s|%d\n",label,deleted);
}
static int target(Jim_Interp *i,int argc,Jim_Obj *const *argv) {
    (void)argc;(void)argv;Jim_SetResultString(i,"VALUE",-1);return JIM_OK;
}
static Jim_Interp *fresh(void) {
    Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);
    return Jim_InitStaticExtensions(i)==JIM_OK ? i:NULL;
}
static void setup(Jim_Interp *i) {
    Jim_CreateCommand(i,"42::84::target",target,NULL,deleted_command);
    Jim_CreateCommand(i,"42::85::target",target,NULL,deleted_command);
    Jim_SetVariableStr(i,"42::84::v",Jim_NewIntObj(i,7));
    Jim_SetVariableStr(i,"42::85::v",Jim_NewIntObj(i,8));
}
static void result_bytes(int code,Jim_Obj *o) {
    int len;const char *bytes=Jim_GetString(o,&len);printf("RESULT|%s|%d|",label,code);
    for(int n=0;n<len;n++)printf("%02x",(unsigned char)bytes[n]);puts("");
}
int main(void) {
    Jim_Interp *version=fresh();if(!version)return 2;label="VERSION";
    Jim_Eval(version,"info patchlevel");result_bytes(0,Jim_GetResult(version));Jim_FreeInterp(version);
    const struct {const char *name;int direct,count,change,mutates;const char *payload;} cases[]={
      {"PUBLIC_ZERO",0,0,0,0,"84"},{"DIRECT_ZERO",1,0,0,0,"84"},
      {"PUBLIC_ROOT",0,1,0,0,"::"},{"DIRECT_ROOT",1,1,0,0,"::"},
      {"PUBLIC_EMPTY",0,1,0,0,""},{"DIRECT_EMPTY",1,1,0,0,""},
      {"PUBLIC_RELATIVE",0,1,0,1,"84"},{"DIRECT_RELATIVE",1,1,0,1,"84"},
      {"PUBLIC_ABSOLUTE",0,1,0,1,"::42::84"},{"DIRECT_ABSOLUTE",1,1,0,1,"::42::84"},
      {"PUBLIC_MULTI",0,2,0,1,"84"},{"DIRECT_MULTI",1,2,0,1,"84"},
      {"PUBLIC_REPLACED",0,1,1,0,"84"},{"DIRECT_REPLACED",1,1,1,0,"84"},
      {"PUBLIC_DELETED",0,1,2,0,"84"},{"DIRECT_DELETED",1,1,2,0,"84"}
    };
    for(unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
      Jim_Interp *i=fresh();if(!i)return 3;label="SETUP";setup(i);
      if(cases[n].change && Jim_Eval(i,cases[n].change==1 ?
          "rename {namespace delete} saved; proc {namespace delete} {args} {return $args}":
          "rename {namespace delete} {}")!=JIM_OK)return 4;
      Jim_Obj *head=Jim_NewStringObj(i,cases[n].direct?"namespace delete":"namespace",-1);Jim_IncrRefCount(head);
      Jim_Obj *member=Jim_NewStringObj(i,"delete",-1);Jim_IncrRefCount(member);
      caller=Jim_NewIntObj(i,42);Jim_IncrRefCount(caller);
      for(int reuse=0;reuse<2;reuse++) {
        if(reuse && cases[n].mutates){label="RESTORE";setup(i);}
        char phase[128];snprintf(phase,sizeof(phase),"%s_%s",cases[n].name,reuse?"REUSE":"COLD");label=phase;
        if(reuse)Jim_InvalidateStringRep(caller);deleted=0;
        Jim_Obj *first=name(i,cases[n].payload),*second=name(i,"85");
        Jim_Obj *argv[4];int argc=0;argv[argc++]=head;if(!cases[n].direct)argv[argc++]=member;
        if(cases[n].count>0)argv[argc++]=first;if(cases[n].count>1)argv[argc++]=second;
        Jim_Obj *vector=Jim_NewListObj(i,argv,argc);Jim_IncrRefCount(vector);
        sample("CALLER_BEFORE",caller);sample("FIRST_BEFORE",first);sample("SECOND_BEFORE",second);
        int code=Jim_EvalNamespace(i,vector,caller);
        sample("CALLER_AFTER",caller);sample("FIRST_AFTER",first);sample("SECOND_AFTER",second);
        printf("DELETIONS|%s|%d\n",label,deleted);
        if(cases[n].change==1){int len;Jim_Obj **v;int ok=Jim_ListGetElements(i,Jim_GetResult(i),&len,&v)==JIM_OK;
          printf("REPLACEMENT_ARG_IS_ORIGINAL|%s|%d\n",label,ok && len==1 && v[0]==first);}
        result_bytes(code,Jim_GetResult(i));
        Jim_DecrRefCount(i,vector);Jim_DecrRefCount(i,first);Jim_DecrRefCount(i,second);
      }
      Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,member);Jim_DecrRefCount(i,caller);
      label="INTERP_TEARDOWN";Jim_FreeInterp(i);
    }
    return 0;
}
