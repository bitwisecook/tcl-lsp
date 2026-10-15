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
static const char *phase;
static Jim_Obj *caller;
struct payload {const char *bytes;int length;};
static void sample(const char *kind,Jim_Obj *object) {
 printf("%s|%s|%d|%s\n",kind,phase,object->bytes!=NULL,object->typePtr?object->typePtr->name:"none");
}
static void update(Jim_Obj *object) {
 const struct payload *p=object->internalRep.ptr;
 sample("CALLER_AT_NAME_GETTER",caller);
 object->bytes=Jim_Alloc(p->length+1);memcpy(object->bytes,p->bytes,p->length);
 object->bytes[p->length]=0;object->length=p->length;
}
static const Jim_ObjType type={"namespace-text-name-probe",NULL,NULL,update,JIM_TYPE_NONE};
static void result(Jim_Interp *i,int code,Jim_Obj *argument) {
 Jim_Obj *r=Jim_GetResult(i);sample("RESULT_HEADER",r);
 printf("RESULT_IS_ARGUMENT|%s|%d\n",phase,r==argument);
 printf("RESULT_IS_EMPTY|%s|%d\n",phase,r==i->emptyObj);
 int length;const char *bytes=Jim_GetString(r,&length);
 printf("RESULT|%s|%d|",phase,code);
 for(int k=0;k<length;k++)printf("%02x",(unsigned char)bytes[k]);puts("");
}
int main(void) {
 setvbuf(stdout,NULL,_IONBF,0);
 const struct payload names[]={
 {"absent::child",13},{"absent",6},{"::absent::child",15},{"::",2},{"",0},
 {"::\0child",8},{"absent::x\0::tail",16},{":",1},{":::tail",7}
 };
 const struct {const char *label;int count,index;} cases[]={
 {"ZERO",0,-1},{"INTEGER",1,-1},{"RELATIVE",1,0},{"PLAIN",1,1},
 {"ABSOLUTE",1,2},{"ROOT",1,3},{"EMPTY",1,4},{"NUL_ROOT",1,5},
 {"NUL_RELATIVE",1,6},{"ONE_COLON",1,7},{"TRIPLE_COLON",1,8},{"WRONG_ARITY",2,0}
 };
 const char *members[]={"qualifiers","tail"};
 for(int operation=0;operation<2;operation++) {
 for(unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
  Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return 2;
  caller=Jim_NewIntObj(i,42);Jim_IncrRefCount(caller);
  Jim_Obj *head=Jim_NewStringObj(i,"namespace",-1);Jim_IncrRefCount(head);
  Jim_Obj *member=Jim_NewStringObj(i,members[operation],-1);Jim_IncrRefCount(member);
  for(int reuse=0;reuse<2;reuse++) {
   char label[128];snprintf(label,sizeof(label),"%s_%s_%s",operation?"TAIL":"QUALIFIERS",cases[n].label,reuse?"REUSE":"COLD");phase=label;
   if(reuse)Jim_InvalidateStringRep(caller);
   Jim_Obj *argument;
   if(cases[n].index<0)argument=Jim_NewIntObj(i,84);
   else {argument=Jim_NewStringObj(i,"",0);Jim_InvalidateStringRep(argument);argument->typePtr=&type;argument->internalRep.ptr=(void*)&names[cases[n].index];}
   Jim_IncrRefCount(argument);
   Jim_Obj *extra=Jim_NewIntObj(i,123);Jim_IncrRefCount(extra);
   Jim_Obj *argv[]={head,member,argument,extra};
   Jim_Obj *vector=Jim_NewListObj(i,argv,2+cases[n].count);Jim_IncrRefCount(vector);
   sample("CALLER_BEFORE",caller);sample("ARGUMENT_BEFORE",argument);sample("EXTRA_BEFORE",extra);
   int code=Jim_EvalNamespace(i,vector,caller);
   sample("CALLER_AFTER",caller);sample("ARGUMENT_AFTER",argument);sample("EXTRA_AFTER",extra);
   result(i,code,argument);
   Jim_DecrRefCount(i,vector);Jim_DecrRefCount(i,argument);Jim_DecrRefCount(i,extra);
  }
  Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,member);Jim_DecrRefCount(i,caller);Jim_FreeInterp(i);
 }
 }
 return 0;
}
