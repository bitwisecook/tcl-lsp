#include <stdio.h>
#include <stdint.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION>=9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
#endif
int main(int ac,char**av){uint64_t bits[]={0x7ff8000000000000ULL,0xfff8000000000000ULL,0x7ff0000000000000ULL,0xfff0000000000000ULL,0,0x8000000000000000ULL};int k;(void)ac;
#ifdef USE_JIM
Jim_Interp*i=Jim_CreateInterp();(void)av;
#else
Tcl_FindExecutable(av[0]);Tcl_Interp*i=Tcl_CreateInterp();
#endif
for(k=0;k<6;k++){double d;int j;const char*s;memcpy(&d,&bits[k],8);
#ifdef USE_JIM
Jim_Obj*o=Jim_NewDoubleObj(i,d);int n;Jim_IncrRefCount(o);s=Jim_GetString(o,&n);
#else
Tcl_Obj*o=Tcl_NewDoubleObj(d);Count n;Tcl_IncrRefCount(o);s=Tcl_GetStringFromObj(o,&n);
#endif
printf("%d\t",k);for(j=0;j<n;j++)printf("%02x",(unsigned char)s[j]);printf("\n");
#ifdef USE_JIM
Jim_DecrRefCount(i,o);
#else
Tcl_DecrRefCount(o);
#endif
}
#ifdef USE_JIM
Jim_FreeInterp(i);
#else
Tcl_DeleteInterp(i);
#endif
return 0;}
