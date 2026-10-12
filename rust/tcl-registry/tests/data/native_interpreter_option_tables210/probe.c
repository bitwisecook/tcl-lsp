#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL; return i; }
static int evaluate(Interp *i,const char *s,size_t n) { Jim_Obj *o=Jim_NewStringObj(i,s,(int)n); Jim_IncrRefCount(o); int code=Jim_EvalObj(i,o); Jim_DecrRefCount(i,o); return code; }
static void report(Interp *i,const char *tag,int code) { int n; const unsigned char *s=(const unsigned char *)Jim_GetString(Jim_GetResult(i),&n); printf("%s|%d|",tag,code); for(int k=0;k<n;k++)printf("%02x",s[k]); puts(""); fflush(stdout); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return NULL; return i; }
static int evaluate(Interp *i,const char *s,size_t n) { return Tcl_EvalEx(i,s,(Count)n,0); }
static void report(Interp *i,const char *tag,int code) { Count n; const unsigned char *s=(const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n); printf("%s|%d|",tag,code); for(Count k=0;k<n;k++)printf("%02x",s[k]); puts(""); fflush(stdout); }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif
static int file(Interp *i,const char *path,const char *tag) {
 if(!strcmp(path,"-"))return 0;
 FILE *fp=fopen(path,"rb"); if(!fp)return 2;
 if(fseek(fp,0,SEEK_END))return 2; long length=ftell(fp); if(length<0 || length>1048576)return 2; rewind(fp);
 char *source=(char *)malloc((size_t)length+1); if(!source)return 2;
 if(fread(source,1,(size_t)length,fp)!=(size_t)length)return 2; fclose(fp); source[length]=0;
 int code=evaluate(i,source,(size_t)length); report(i,tag,code); free(source); return 0;
}
int main(int argc,char **argv) {
 if(argc!=4)return 2;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#endif
 Interp *i=fresh(); if(!i)return 3; report(i,"VERSION",evaluate(i,"info patchlevel",15));
 if(file(i,argv[1],"PRELUDE") || file(i,argv[2],"ORIGINAL") || file(i,argv[3],"QUERY"))return 2;
 destroy(i); return 0;
}
