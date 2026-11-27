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
static int declaration(Interp *i,Obj *name,const char *body) { if(!body) { Obj *v[]={string(i,"oo::define",10),string(i,"C",1),string(i,"property",8),name}; return invoke(i,4,v); } Obj *v[]={string(i,"oo::define",10),string(i,"C",1),string(i,"property",8),name,string(i,"-get",4),string(i,body,(int)strlen(body))}; return invoke(i,6,v); }
static int access(Interp *i,Obj *name,Obj *value) { Obj *v[]={string(i,"o",1),string(i,"configure",9),name,value}; return invoke(i,value?4:3,v); }
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *startup=fresh(); if(!startup)return 2; row(startup,"VERSION",eval(startup,"info patchlevel"),NULL); row(startup,"AVAILABILITY",eval(startup,"info commands ::oo::configurable"),NULL); destroy(startup);
 const char *names[]={"p\xff","p\xed\xa0\x80","p\xed\xa0\x81","p\0tail"}; const int lengths[]={2,4,4,6}; const char *labels[]={"FF","D800","D801","ZERO"};
 for(int k=0;k<4;k++) {
  Interp *i=fresh(); if(!i)return 2; char label[80]; int created=eval(i,"oo::configurable create C; C create o"); snprintf(label,sizeof(label),"%s_CREATE",labels[k]); row(i,label,created,NULL);
  if(!created) {
   Obj *name=string(i,names[k],lengths[k]); hold(name); char dashed[20]; dashed[0]='-'; memcpy(dashed+1,names[k],lengths[k]); Obj *option=string(i,dashed,lengths[k]+1); hold(option);
   printf("INPUT|%s|",labels[k]); hex(names[k],lengths[k]); puts("");
   int code=declaration(i,name,NULL); snprintf(label,sizeof(label),"%s_DEFINE",labels[k]); row(i,label,code,NULL);
   if(!code) { Obj *member=string(i,"VALUE",5); hold(member); Obj *value=list(i,1,&member); hold(value); snprintf(label,sizeof(label),"%s_WRITE",labels[k]); row(i,label,access(i,option,value),NULL); snprintf(label,sizeof(label),"%s_READ",labels[k]); row(i,label,access(i,option,NULL),value); snprintf(label,sizeof(label),"%s_ROSTER",labels[k]); row(i,label,eval(i,"info class properties C"),NULL); release(i,value); release(i,member); }
   release(i,option); release(i,name);
  }
  destroy(i);
 }
 Interp *i=fresh(); if(!i)return 2; int code=eval(i,"oo::configurable create C; C create o"); row(i,"DISTINCT_CREATE",code,NULL);
 if(!code) {
  const char *opaque[]={"p\xed\xa0\x80","p\xed\xa0\x81"}; const char *bodies[]={"return FIRST","return SECOND"};
  for(int k=0;k<2;k++) { char label[80]; Obj *name=string(i,opaque[k],4); hold(name); snprintf(label,sizeof(label),"DISTINCT_DEFINE_%d",k); row(i,label,declaration(i,name,bodies[k]),NULL); release(i,name); }
  for(int k=0;k<2;k++) { char label[80]; char dashed[5]; dashed[0]='-'; memcpy(dashed+1,opaque[k],4); Obj *name=string(i,dashed,5); hold(name); snprintf(label,sizeof(label),"DISTINCT_READ_%d",k); row(i,label,access(i,name,NULL),NULL); release(i,name); }
  row(i,"DISTINCT_ROSTER",eval(i,"info class properties C"),NULL);
 }
 destroy(i);
 const char *invalid[]={"p[bad]","p\\x","","-bad","p::q","p(q)"};
 for(int k=0;k<6;k++) { Interp *j=fresh(); if(!j)return 2; char label[80]; int c=eval(j,"oo::configurable create C"); if(!c) { Obj *name=string(j,invalid[k],(int)strlen(invalid[k])); hold(name); snprintf(label,sizeof(label),"INVALID_%d",k); row(j,label,declaration(j,name,NULL),NULL); release(j,name); } destroy(j); }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
