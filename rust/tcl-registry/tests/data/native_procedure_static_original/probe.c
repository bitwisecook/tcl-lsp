#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { return Jim_NewStringObj(i,p,n); }
static void hold(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int eval(Interp *i,const char *p) { return Jim_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Jim_EvalObjVector(i,n,v); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,long *n) { int size; const char *p=Jim_GetString(o,&size); *n=size; return p; }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { (void)i; return Tcl_NewStringObj(p,n); }
static void hold(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int eval(Interp *i,const char *p) { return Tcl_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Tcl_EvalObjv(i,n,v,0); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,long *n) {
#if TCL_MAJOR_VERSION >= 9
Tcl_Size size;
#else
int size;
#endif
const char *p=Tcl_GetStringFromObj(o,&size); *n=(long)size; return p; }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif
static void hex(const char *p,long n) { for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]); }
/* Pointer equality is sampled before the only result string getter. */
static void row(Interp *i,const char *label,int code,Obj *argument) { int same=argument && result(i)==argument; long n; const char *p=bytes(result(i),&n); printf("%s|%d|%d|",label,code,same); hex(p,n); puts(""); }

static Obj *list(Interp *i,int n,Obj **v) {
#ifdef JIM_PROBE
 return Jim_NewListObj(i,v,n);
#else
 (void)i; return Tcl_NewListObj(n,v);
#endif
}
static int set(Interp *i,Obj *name,Obj *value) { Obj *v[]={string(i,"set",3),name,value}; return invoke(i,3,v); }
static int define(Interp *i,Obj *statics,Obj *body) { Obj *v[]={string(i,"proc",4),string(i,"p",1),string(i,"",0),statics,body}; return invoke(i,5,v); }
static int call(Interp *i) { Obj *v[]={string(i,"p",1)}; return invoke(i,1,v); }
static Obj *body(Interp *i,const char *name,int n) { char out[100]; memcpy(out,"return ${",9); memcpy(out+9,name,n); out[9+n]='}'; return string(i,out,10+n); }
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *startup=fresh(); if(!startup)return 2; row(startup,"VERSION",eval(startup,"info patchlevel"),NULL); destroy(startup);
 const char *names[]={"x","x\xff","x\xed\xa0\x80","x\xed\xa0\x81","x\0tail"};
 const int lengths[]={1,2,4,4,6}; const char *labels[]={"ASCII","FF","D800","D801","ZERO"};
 for(int k=0;k<5;k++) {
  Interp *i=fresh(); if(!i)return 2; char label[80];
  Obj *name=string(i,names[k],lengths[k]); hold(name);
  Obj *member=string(i,"VALUE",5); hold(member); Obj *literal=list(i,1,&member); hold(literal);
  Obj *fields[]={name,literal}; Obj *specifier=list(i,2,fields); hold(specifier); Obj *statics=list(i,1,&specifier); hold(statics);
  Obj *script=body(i,names[k],lengths[k]); hold(script);
  printf("INPUT|%s|",labels[k]); hex(names[k],lengths[k]); puts("");
  int code=define(i,statics,script); snprintf(label,sizeof(label),"%s_LITERAL_DEFINE",labels[k]); row(i,label,code,NULL);
  if(!code) { snprintf(label,sizeof(label),"%s_LITERAL_CALL",labels[k]); row(i,label,call(i),literal); }
  release(i,script); release(i,statics); release(i,specifier); release(i,literal); release(i,member); release(i,name); destroy(i);
 }
 for(int reference=0;reference<2;reference++) {
  Interp *i=fresh(); if(!i)return 2; char label[80]; Obj *name=string(i,"x\xff",2); hold(name); Obj *first=string(i,"FIRST",5); hold(first);
  row(i,"SOURCE_SET",set(i,name,first),NULL); const char *field=reference?"&x\xff":"x\xff"; Obj *specifier=string(i,field,reference?3:2); hold(specifier); Obj *statics=list(i,1,&specifier); hold(statics);
  Obj *script=body(i,"x\xff",2); hold(script); int code=define(i,statics,script); snprintf(label,sizeof(label),"%s_DEFINE",reference?"REFERENCE":"COPY"); row(i,label,code,NULL);
  if(!code) { Obj *second=string(i,"SECOND",6); hold(second); row(i,"SOURCE_REPLACE",set(i,name,second),NULL); snprintf(label,sizeof(label),"%s_CALL",reference?"REFERENCE":"COPY"); row(i,label,call(i),reference?second:first); release(i,second); }
  release(i,script); release(i,statics); release(i,specifier); release(i,first); release(i,name); destroy(i);
 }
 for(int duplicate=0;duplicate<2;duplicate++) {
  Interp *i=fresh(); if(!i)return 2; Obj *name=string(i,"x\xff",2); hold(name); Obj *value=string(i,"V",1); hold(value); Obj *members[]={name,value}; Obj *specifier=duplicate?list(i,2,members):name; hold(specifier); Obj *specs[]={specifier,specifier}; Obj *statics=list(i,duplicate?2:1,specs); hold(statics); Obj *script=body(i,"x\xff",2); hold(script);
  row(i,duplicate?"DUPLICATE_DEFINE":"MISSING_DEFINE",define(i,statics,script),NULL); row(i,duplicate?"DUPLICATE_COMMAND":"MISSING_COMMAND",eval(i,"info commands p"),NULL);
  release(i,script); release(i,statics); release(i,specifier); release(i,value); release(i,name); destroy(i);
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
