#include <stdio.h>
#include "tcl.h"
static Tcl_Obj *windows;
static int capture(ClientData unused,Tcl_Interp*i,int n,Tcl_Obj*const v[]){(void)unused;if(n!=3)return TCL_ERROR;Tcl_Obj*pair[2]={v[1],Tcl_NewIntObj(v[2]->refCount)};Tcl_ListObjAppendElement(i,windows,Tcl_NewListObj(2,pair));Tcl_ResetResult(i);return TCL_OK;}
static void hex(const char*s,int n){for(int j=0;j<n;j++)printf("%02x",(unsigned char)s[j]);}
int main(int argc,char**argv){(void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp*i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK){fputs(Tcl_GetStringResult(i),stderr);return 2;}
Tcl_CreateObjCommand(i,"capture",capture,NULL,NULL);
const char*actions[]={"set ignored OK","error BODY","break","continue","return BODY"};const char*names[]={"normal","error","break","continue","return"};
for(int family=0;family<2;family++)for(int mode=0;mode<5;mode++){
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION<5
continue;
#elif TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION<6
if(family)continue;
#endif
windows=Tcl_NewListObj(0,NULL);Tcl_IncrRefCount(windows);char source[1024];snprintf(source,sizeof(source),"proc p {} {set d {k V}; capture before $d;set c [catch {dict %s {k v} $d {capture during $d;%s}} r o];capture after $d;list $c $r [dict get $o -code] [dict get $o -level] $o};p",family?"map":"for",actions[mode]);
int code=Tcl_Eval(i,source);Tcl_Obj*result=Tcl_GetObjResult(i);Tcl_IncrRefCount(result);printf("%s-%s\t%d\t",family?"map":"for",names[mode],code);
#if TCL_MAJOR_VERSION>=9
Tcl_Size length;
#else
int length;
#endif
const char*text=Tcl_GetStringFromObj(result,&length);hex(text,(int)length);putchar('\t');text=Tcl_GetStringFromObj(windows,&length);hex(text,(int)length);putchar('\n');Tcl_DecrRefCount(result);Tcl_DecrRefCount(windows);
}
Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
