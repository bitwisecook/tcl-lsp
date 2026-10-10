#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Size;
#define INC(i,o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
static Obj *string(Interp *i, const char *s, Size n) { return Jim_NewStringObj(i,s,n); }
static const char *bytes(Obj *o, Size *n) { return Jim_GetString(o,n); }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static int vector(Interp *i, int n, Obj **v) { return Jim_EvalObjVector(i,n,v); }
static int source(Interp *i, const char *s) { return Jim_Eval(i,s); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
#define INC(i,o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
static Obj *string(Interp *i, const char *s, Size n) { (void)i; return Tcl_NewStringObj(s,n); }
static const char *bytes(Obj *o, Size *n) { return Tcl_GetStringFromObj(o,n); }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static int vector(Interp *i, int n, Obj **v) { return Tcl_EvalObjv(i,n,v,0); }
static int source(Interp *i, const char *s) { return Tcl_EvalEx(i,s,(Size)strlen(s),0); }
static unsigned calls;
static void hex(const char *s, Size n) { for (Size p=0;p<n;p++) printf("%02x",(unsigned char)s[p]); }
static int watch(ClientData d, Interp *i, int n, Obj *const *v) {
 (void)d; calls++; printf("CALL|%u|%d",calls,n);
 for(int p=1;p<n;p++) { Size count;const char *s=bytes(v[p],&count);putchar('|');hex(s,count); }
 puts("");Tcl_ResetResult(i);return TCL_OK;
}
#endif
#ifdef JIM_PROBE
static void hex(const char *s, Size n) { for (Size p=0;p<n;p++) printf("%02x",(unsigned char)s[p]); }
#endif
static int invoke(Interp *i, const char **words, int count, Obj *name, Obj *last) {
 Obj *v[8]; for(int p=0;p<count;p++) { v[p]=string(i,words[p],-1);INC(i,v[p]); }
 v[count]=name; INC(i,name);int n=count+1;
 if(last) { v[n++]=last;INC(i,last); }
 int code=vector(i,n,v); for(int p=0;p<n;p++) DEC(i,v[p]);return code;
}
static int trace(Interp *i, Obj *name, int remove) {
 Obj *v[6];int n=0;
#if !defined(JIM_PROBE) && TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
 const char *before[]={"trace",remove?"vdelete":"variable"};
 const char *after[]={"w","watch"};
#else
 const char *before[]={"trace",remove?"remove":"add","variable"};
 const char *after[]={"write","watch"};
#endif
 for(unsigned p=0;p<sizeof(before)/sizeof(before[0]);p++)v[n++]=string(i,before[p],-1);
 v[n++]=name;
 for(unsigned p=0;p<sizeof(after)/sizeof(after[0]);p++)v[n++]=string(i,after[p],-1);
 for(int p=0;p<n;p++)INC(i,v[p]);int code=vector(i,n,v);for(int p=0;p<n;p++)DEC(i,v[p]);return code;
}
static void outcome(Interp *i,const char *kind,const char *label,int code) {
 Size n;const char *s=bytes(result(i),&n);printf("%s|%s|%d|",kind,label,code);hex(s,n);puts("");
}
static void info(Interp *i,const char *label,Obj *name) {
#if !defined(JIM_PROBE) && TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
 const char *words[]={"trace","vinfo"};
#else
 const char *words[]={"trace","info","variable"};
#endif
 int code=invoke(i,words,sizeof(words)/sizeof(words[0]),name,NULL);outcome(i,"INFO",label,code);
}
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 static const char raw[]={'v',0,'t','a','i','l','(','k',')'};
 static const char alternate[]={'v',0,'o','t','h','e','r','(','j',')'};
 const char *labels[]={"RAW_COUNTED","NUMERIC_ESCAPE","BINARY00"};
 for(int test=0;test<3;test++) {
#ifdef JIM_PROBE
  Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return 2;
#else
  Interp *i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)return 2;Tcl_CreateObjCommand(i,"watch",watch,NULL,NULL);calls=0;
#endif
  if(source(i,"info patchlevel")!=0)return 3;outcome(i,"VERSION",labels[test],0);
  Obj *name;
  if(test==0)name=string(i,raw,sizeof(raw));
  else { const char *input=test==1?"set name v\\u0000tail(k)":"set name [binary format H* 76007461696c286b29]";if(source(i,input)!=0)return 4;name=result(i); }
  INC(i,name);const char *primary=name->typePtr?name->typePtr->name:"none";
  Size count;const char *s=bytes(name,&count);printf("SUBJECT|%s|%s|",labels[test],primary);hex(s,count);puts("");
  int code=trace(i,name,0);outcome(i,"ADD",labels[test],code);
  if(code==0) {
   Obj *plain=string(i,"v",1);INC(i,plain);info(i,"ORIGINAL",name);info(i,"PLAIN",plain);
   const char *words[]={"set"};Obj *value=string(i,"VALUE",5);INC(i,value);
   code=invoke(i,words,1,plain,value);outcome(i,"SET_PLAIN",labels[test],code);
   code=invoke(i,words,1,name,value);outcome(i,"SET_ORIGINAL",labels[test],code);
   Obj *other=string(i,alternate,sizeof(alternate));INC(i,other);info(i,"RAW_ALTERNATE",other);
   code=trace(i,other,1);outcome(i,"REMOVE_RAW_ALTERNATE",labels[test],code);info(i,"REMAINING",name);
   DEC(i,other);DEC(i,value);DEC(i,plain);
  }
  DEC(i,name);
#ifdef JIM_PROBE
  Jim_FreeInterp(i);
#else
  Tcl_DeleteInterp(i);
#endif
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
