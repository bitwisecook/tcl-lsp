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
static Jim_Obj *caller,*space,*other,*local,*extra,*last;
static void sample(const char *kind,Jim_Obj *o) {
    printf("%s|%s|%d|%s\n",kind,label,o->bytes!=NULL,o->typePtr?o->typePtr->name:"none");
}
static void update(Jim_Obj *o) {
    const char *s=o->internalRep.ptr;sample("CALLER_AT_NAME_GETTER",caller);
    o->length=(int)strlen(s);o->bytes=Jim_Alloc(o->length+1);memcpy(o->bytes,s,o->length+1);
}
static const Jim_ObjType type={"upvar-name-probe",NULL,NULL,update,JIM_TYPE_NONE};
static Jim_Obj *name(Jim_Interp *i,const char *s) {
    Jim_Obj *o=Jim_NewStringObj(i,"",0);Jim_InvalidateStringRep(o);o->typePtr=&type;
    o->internalRep.ptr=(void *)s;Jim_IncrRefCount(o);return o;
}
static void bytes(const char *kind,int code,Jim_Obj *o) {
    int len;const char *s=Jim_GetString(o,&len);printf("%s|%s|%d|",kind,label,code);
    for(int n=0;n<len;n++)printf("%02x",(unsigned char)s[n]);puts("");
}
static int observe(Jim_Interp *i,int argc,Jim_Obj *const *argv) {
    printf("INVOKE|%s|%d|%d|%d|%d|%d\n",label,argc,i->framePtr->nsObj==caller,
      argc>3 && argv[3]==local,argc>3 && argv[3]==i->emptyObj,argc>5 && argv[5]==last);
    sample("CALLER_INVOKE",caller);sample("SPACE_INVOKE",space);sample("OTHER_INVOKE",other);
    sample("LOCAL_INVOKE",local);sample("EXTRA_INVOKE",extra);sample("LAST_INVOKE",last);
    if(argc>2)bytes("TARGET",0,argv[2]);if(argc>4)bytes("TARGET_SECOND",0,argv[4]);
    Jim_SetResult(i,argc>3?argv[3]:i->emptyObj);return JIM_OK;
}
static Jim_Interp *fresh(void) {
    Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);
    return Jim_InitStaticExtensions(i)==JIM_OK?i:NULL;
}
int main(void) {
    setvbuf(stdout,NULL,_IONBF,0);
    Jim_Interp *v=fresh();if(!v)return 2;label="VERSION";Jim_Eval(v,"info patchlevel");bytes("RESULT",0,Jim_GetResult(v));Jim_FreeInterp(v);
    const struct {const char *name;int direct,count,change,absolute;} cases[]={
      {"PUBLIC_ZERO",0,0,0,0},{"DIRECT_ZERO",1,0,0,0},
      {"PUBLIC_NAMESPACE_ONLY",0,1,0,0},{"DIRECT_NAMESPACE_ONLY",1,1,0,0},
      {"PUBLIC_PAIR",0,3,0,0},{"DIRECT_PAIR",1,3,0,0},
      {"PUBLIC_ABSOLUTE",0,3,0,1},{"DIRECT_ABSOLUTE",1,3,0,1},
      {"PUBLIC_ODD",0,2,0,0},{"DIRECT_ODD",1,2,0,0},
      {"PUBLIC_MULTI",0,5,0,0},{"DIRECT_MULTI",1,5,0,0},
      {"PUBLIC_REPLACED",0,3,1,0},{"DIRECT_REPLACED",1,3,1,0},
      {"PUBLIC_DELETED",0,3,2,0},{"DIRECT_DELETED",1,3,2,0}
    };
    for(unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
      Jim_Interp *i=fresh();if(!i)return 3;Jim_CreateCommand(i,"upvar",observe,NULL,NULL);
      if(cases[n].change && Jim_Eval(i,cases[n].change==1?
        "rename {namespace upvar} saved; proc {namespace upvar} {ns args} {return $args}":
        "rename {namespace upvar} {}")!=JIM_OK)return 4;
      Jim_Obj *head=Jim_NewStringObj(i,cases[n].direct?"namespace upvar":"namespace",-1);Jim_IncrRefCount(head);
      Jim_Obj *member=Jim_NewStringObj(i,"upvar",-1);Jim_IncrRefCount(member);
      caller=Jim_NewIntObj(i,42);Jim_IncrRefCount(caller);
      for(int reuse=0;reuse<2;reuse++) {
        char phase[128];snprintf(phase,sizeof(phase),"%s_%s",cases[n].name,reuse?"REUSE":"COLD");label=phase;
        if(reuse)Jim_InvalidateStringRep(caller);
        space=cases[n].absolute?name(i,"::84"):Jim_NewIntObj(i,84);if(!cases[n].absolute)Jim_IncrRefCount(space);
        other=name(i,"v");local=Jim_NewIntObj(i,123);Jim_IncrRefCount(local);
        extra=name(i,"other");last=Jim_NewIntObj(i,456);Jim_IncrRefCount(last);
        Jim_Obj *argv[7];int argc=0;argv[argc++]=head;if(!cases[n].direct)argv[argc++]=member;
        Jim_Obj *parts[]={space,other,local,extra,last};for(int k=0;k<cases[n].count;k++)argv[argc++]=parts[k];
        Jim_Obj *vector=Jim_NewListObj(i,argv,argc);Jim_IncrRefCount(vector);
        sample("CALLER_BEFORE",caller);sample("SPACE_BEFORE",space);sample("OTHER_BEFORE",other);sample("LOCAL_BEFORE",local);sample("EXTRA_BEFORE",extra);sample("LAST_BEFORE",last);
        int code=Jim_EvalNamespace(i,vector,caller);
        sample("CALLER_AFTER",caller);sample("SPACE_AFTER",space);sample("OTHER_AFTER",other);sample("LOCAL_AFTER",local);sample("EXTRA_AFTER",extra);sample("LAST_AFTER",last);
        printf("RESULT_IS_LOCAL|%s|%d\n",label,Jim_GetResult(i)==local);
        if(cases[n].change==1)printf("REPLACEMENT_OTHER_IS_ORIGINAL|%s|%d\n",label,
          Jim_ListLength(i,Jim_GetResult(i))==2 && Jim_ListGetIndex(i,Jim_GetResult(i),0)==other);
        bytes("RESULT",code,Jim_GetResult(i));
        Jim_DecrRefCount(i,vector);Jim_DecrRefCount(i,space);Jim_DecrRefCount(i,other);Jim_DecrRefCount(i,local);Jim_DecrRefCount(i,extra);Jim_DecrRefCount(i,last);
      }
      Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,member);Jim_DecrRefCount(i,caller);Jim_FreeInterp(i);
    }
    const char *scripts[]={
      "namespace eval 42::84 {variable v ORIGINAL}; namespace eval 42 {namespace upvar 84 v linked; set linked CHANGED}; set ::42::84::v",
      "namespace eval 42::84 {variable v ORIGINAL}; namespace eval 42 {namespace upvar 84 v; set {} CHANGED}; set ::42::84::v",
      "namespace eval 42 {namespace upvar 84}",
      "namespace upvar"
    };
    for(int n=0;n<4;n++){Jim_Interp *i=fresh();if(!i)return 5;char phase[64];snprintf(phase,sizeof(phase),"REAL_WORKER_%d",n);label=phase;int code=Jim_Eval(i,scripts[n]);bytes("REAL_RESULT",code,Jim_GetResult(i));Jim_FreeInterp(i);}
    return 0;
}
