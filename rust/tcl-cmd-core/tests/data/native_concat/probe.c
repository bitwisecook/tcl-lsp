/* SPDX-License-Identifier: AGPL-3.0-or-later */
#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size ProbeSize;
#else
typedef int ProbeSize;
#endif
static const char *kind(Tcl_Obj *o) { return o->typePtr ? o->typePtr->name : "NULL"; }
static void hex(const char *p, ProbeSize n) { ProbeSize j; for(j=0;j<n;j++) printf("%02x",(unsigned char)p[j]); }
static int canonical(Tcl_Obj *o) {
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
 return TclListObjIsCanonical(o);
#else
 return o->typePtr && !strcmp(kind(o),"list") && !o->bytes;
#endif
}
static const char *canonicalEmpty;
static void state(Tcl_Obj *o) {
 printf("%s,%d,%d,%d,%ld,",kind(o),o->bytes!=NULL,canonical(o),o->refCount,(long)(o->bytes?o->length:-1));
 if (o->bytes) hex(o->bytes,o->length);
 printf(",storage=%s", !o->bytes ? "absent" : o->bytes == canonicalEmpty ? "canonical" : "allocated");
 if(o->typePtr && !strcmp(kind(o),"string")) {
#if TCL_MAJOR_VERSION >= 9
  printf(",chars=%ld",(long)*(Tcl_Size *)o->internalRep.twoPtrValue.ptr1);
#else
  printf(",chars=%d",*(int *)o->internalRep.twoPtrValue.ptr1);
#endif
 }
}
static Tcl_Obj *list(const char *a,const char *b,Tcl_Obj **child) {
 Tcl_Obj *v[2]; int n=0;
 if(a) {v[n++]=Tcl_NewStringObj(a,-1); if(child) *child=v[0];}
 if(b) v[n++]=Tcl_NewStringObj(b,-1);
 return Tcl_NewListObj(n,v);
}
static Tcl_Obj *parsed(const char *s) { Tcl_Obj *o=Tcl_NewStringObj(s,-1); ProbeSize n; Tcl_ListObjLength(NULL,o,&n); return o; }
static void direct(int id) {
 Tcl_Obj *v[3],*c[3]={NULL,NULL,NULL},*r; int n=0,j; const char *render; ProbeSize length;
 switch(id) {
 case 0: break;
 case 1: v[n++]=Tcl_NewObj(); break;
 case 2: v[n++]=Tcl_NewStringObj("",0); break;
 case 3: v[n++]=list("a","b",&c[0]); break;
 case 4: v[n++]=list(NULL,NULL,NULL); break;
 case 5: v[n++]=list("a",NULL,&c[0]); v[n++]=list("b",NULL,&c[1]); break;
 case 6: v[n++]=list("a",NULL,&c[0]); v[n++]=list("#b",NULL,&c[1]); break;
 case 7: v[n++]=list("#a",NULL,&c[0]); v[n++]=list("b",NULL,&c[1]); break;
 case 8: v[n++]=parsed(" a  {b} "); break;
 case 9: v[n]=list("a","b",&c[0]); Tcl_GetString(v[n++]); break;
 case 10: v[n++]=Tcl_NewStringObj("",0); v[n++]=list("a",NULL,&c[1]); break;
 case 11: v[n++]=Tcl_NewStringObj("  ",-1); v[n++]=list("a",NULL,&c[1]); break;
 case 12: v[n++]=list("a",NULL,&c[0]); v[n++]=list(NULL,NULL,NULL); break;
 case 13: v[n++]=list(NULL,NULL,NULL); v[n++]=list("#b",NULL,&c[1]); break;
 case 14: v[n++]=Tcl_NewStringObj(" a ",-1); v[n++]=Tcl_NewStringObj(" b ",-1); break;
 case 15: v[n++]=Tcl_NewStringObj("",0); v[n++]=Tcl_NewStringObj("",0); break;
 case 16: v[n++]=Tcl_NewStringObj(" ",1); v[n++]=Tcl_NewStringObj("\t",1); break;
 case 17: v[n++]=Tcl_NewStringObj("a\\ ",-1); v[n++]=Tcl_NewStringObj("b",-1); break;
 case 18: v[n++]=Tcl_NewStringObj("a\\\\ ",-1); v[n++]=Tcl_NewStringObj("b",-1); break;
 case 19: v[n++]=list("a",NULL,&c[0]); v[n++]=parsed(" #b "); break;
 default: return;
 }
 for(j=0;j<n;j++) Tcl_IncrRefCount(v[j]);
 printf("D\t%d\tbefore\t",id); for(j=0;j<n;j++) {if(j)putchar(';');state(v[j]);} putchar('\n');
 r=Tcl_ConcatObj(n,v); Tcl_IncrRefCount(r);
 printf("D\t%d\tresult\t",id); state(r); printf("\tsame_first=%d\tbacking_first=%d\tchildren=",n&&r==v[0],n&&r->typePtr&&v[0]->typePtr&&!strcmp(kind(r),"list")&&!strcmp(kind(v[0]),"list")&&r->internalRep.twoPtrValue.ptr1==v[0]->internalRep.twoPtrValue.ptr1);
 for(j=0;j<n;j++) {if(j)putchar(','); printf("%d",c[j]?c[j]->refCount:-1);} putchar('\n');
 printf("D\t%d\tafter\t",id); for(j=0;j<n;j++){if(j)putchar(';');state(v[j]);}putchar('\n');
 render=Tcl_GetStringFromObj(r,&length); printf("D\t%d\tvalue\t",id);hex(render,length);putchar('\n');
 Tcl_DecrRefCount(r); for(j=0;j<n;j++) Tcl_DecrRefCount(v[j]);
}
static int global_count(Interp *ip,Tcl_Obj *o) {int n=0,j; LiteralEntry *le; for(j=0;j<ip->literalTable.numBuckets;j++)for(le=ip->literalTable.buckets[j];le;le=le->nextPtr)if(le->objPtr==o)n++;return n;}
static void compiled(int id) {
 static const char *scripts[]={"concat","concat {}","concat {} {}","concat { } {\t}","concat A","concat {A B} C","concat {A\\ } B","concat {A\\\\ } B"};
 Tcl_Interp *ip=Tcl_CreateInterp(); Tcl_Obj *def[4],*call,*result; Proc *p; ByteCode *bc; int j,code;
 def[0]=Tcl_NewStringObj("proc",-1);def[1]=Tcl_NewStringObj("p",-1);def[2]=Tcl_NewObj();def[3]=Tcl_NewStringObj(scripts[id],-1);
 for(j=0;j<4;j++)Tcl_IncrRefCount(def[j]); code=Tcl_EvalObjv(ip,4,def,0);
 call=Tcl_NewStringObj("p",1);Tcl_IncrRefCount(call); if(code==TCL_OK)code=Tcl_EvalObjv(ip,1,&call,0);
 result=Tcl_GetObjResult(ip);printf("C\t%d\tresult\t%d\t",id,code);state(result);printf("\tglobal=%d\n",global_count((Interp *)ip,result));
 p=TclFindProc((Interp *)ip,"p"); if(p&&p->bodyPtr->typePtr&&!strcmp(kind(p->bodyPtr),"bytecode")) {
 bc=(ByteCode *)p->bodyPtr->internalRep.twoPtrValue.ptr1;
 for(j=0;j<bc->numLitObjects;j++) {Tcl_Obj *o=bc->objArrayPtr[j];printf("C\t%d\tliteral\t%d\t",id,j);state(o);printf("\tglobal=%d\tsame_result=%d\n",global_count((Interp *)ip,o),o==result);}
 }
 Tcl_DecrRefCount(call);for(j=0;j<4;j++)Tcl_DecrRefCount(def[j]);Tcl_DeleteInterp(ip);
}
int main(int argc,char **argv) {int j; (void)argc;Tcl_FindExecutable(argv[0]);{Tcl_Obj *empty=Tcl_NewObj();canonicalEmpty=empty->bytes;Tcl_IncrRefCount(empty);Tcl_DecrRefCount(empty);}for(j=0;j<20;j++)direct(j);for(j=0;j<8;j++)compiled(j);Tcl_Finalize();return 0;}
