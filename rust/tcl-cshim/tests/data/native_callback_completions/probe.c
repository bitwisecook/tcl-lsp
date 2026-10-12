#include <tcl.h>
#include <stdio.h>
#include <stdint.h>
#if TCL_MAJOR_VERSION >= 9
#define SIZE Tcl_Size
#else
#define SIZE int
#endif
static int callback(ClientData data,Tcl_Interp *ip,int objc,Tcl_Obj *const objv[]) {
    const char result[]={'v',0,(char)255};
    int bits=(int)(intptr_t)data;
    Tcl_SetObjResult(ip,Tcl_NewStringObj(result,3));
    if(bits&256) Tcl_SetErrorCode(ip,"CUSTOM","E",NULL);
    return bits&255;
}
static void hex(Tcl_Obj *value) {
    if(!value) { fputs("null",stdout); return; }
    SIZE length; const char *bytes=Tcl_GetStringFromObj(value,&length);
    putchar('"'); for(SIZE i=0;i<length;i++) printf("%02x",(unsigned char)bytes[i]);putchar('"');
}
#if TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=5
static void field(Tcl_Obj *options,const char *name) {
    Tcl_Obj *key=Tcl_NewStringObj(name,-1),*value=NULL;Tcl_IncrRefCount(key);
    Tcl_DictObjGet(NULL,options,key,&value);hex(value);Tcl_DecrRefCount(key);
}
#endif
int main(int argc,char **argv) {
    Tcl_FindExecutable(argv[0]); Tcl_Interp *ip=Tcl_CreateInterp();
    int codes[]={0,1,2,3,4,7};
    for(int seeded=0;seeded<2;seeded++) for(int n=0;n<6;n++) {
        Tcl_ResetResult(ip);
        int code=callback((ClientData)(intptr_t)(codes[n]|(seeded?256:0)),ip,0,NULL);
        Tcl_Obj *result=Tcl_GetObjResult(ip);Tcl_IncrRefCount(result);
#if TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=5
        Tcl_Obj *options=Tcl_GetReturnOptions(ip,code);Tcl_IncrRefCount(options);
#endif
        printf("{\"code\":%d,\"seeded\":%d,\"result\":",code,seeded);hex(result);
#if TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=5
        fputs(",\"option_code\":",stdout);field(options,"-code");
        fputs(",\"option_level\":",stdout);field(options,"-level");
        fputs(",\"errorcode\":",stdout);field(options,"-errorcode");
        fputs(",\"errorinfo\":",stdout);field(options,"-errorinfo");
        fputs(",\"errorline\":",stdout);field(options,"-errorline");
        fputs(",\"errorstack\":",stdout);field(options,"-errorstack");
        Tcl_DecrRefCount(options);
#endif
        puts("}");Tcl_DecrRefCount(result);
    }
    Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;
}
