#include <tcl.h>
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Tcl_Obj *first;
static Tcl_Obj *firstChild;
static const char *mode;
static int calls;
static int capture(ClientData unused,Tcl_Interp *i,int argc,Tcl_Obj *const argv[]) {
    Tcl_Obj *v; Count n, length; Tcl_Obj **members; const char *before,*after,*bytes; Tcl_WideInt value;
    int same, children=-1, childPrevious=-1, refcount;
    (void)unused;
    if (argc != 2) return TCL_ERROR;
    v=argv[1];refcount=v->refCount;before=v->typePtr?v->typePtr->name:"none";
    same=first?first==v:-1;
    if (!strcmp(mode,"number")) { if (Tcl_GetWideIntFromObj(NULL,v,&value)!=TCL_OK) return TCL_ERROR; }
    else if (!strcmp(mode,"static-list") || !strcmp(mode,"dynamic-list")) {
        if (Tcl_ListObjGetElements(i,v,&n,&members)!=TCL_OK || n!=2) return TCL_ERROR;
        children=members[0]==members[1];childPrevious=firstChild?firstChild==members[0]:-1;
        if (!firstChild) {firstChild=members[0];Tcl_IncrRefCount(firstChild);}
    }
    after=v->typePtr?v->typePtr->name:"none";
    bytes=Tcl_GetStringFromObj(v,&length);
    printf("%s\t%d\t%s\t%s\t%d\t%d\t%d\t%d\t",mode,calls++,before,after,same,children,childPrevious,refcount);
    for (n=0;n<length;n++) printf("%02x",(unsigned char)bytes[n]);puts("");
    if (!first) {first=v;Tcl_IncrRefCount(first);}
    Tcl_ResetResult(i);return TCL_OK;
}
static int run(Tcl_Interp*i,const char *source) {
    int code=Tcl_Eval(i,source);if(code!=TCL_OK){fprintf(stderr,"%s\n",Tcl_GetStringResult(i));}return code;
}
static void reset(const char *name) {
    if(first)Tcl_DecrRefCount(first);if(firstChild)Tcl_DecrRefCount(firstChild);
    first=firstChild=NULL;mode=name;calls=0;
}
int main(int argc,char **argv) {
    Tcl_Interp *i; (void)argc;Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();Tcl_CreateObjCommand(i,"capture",capture,NULL,NULL);
    reset("number");if(run(i,"proc p {} {capture 17}; proc q {} {capture 17}; p; p; q; rename p {}; rename q {}; proc r {} {capture 17}; r"))return 1;
    reset("static-list");if(run(i,"proc p {} {capture [list A A]}; p; p"))return 1;
    reset("dynamic-list");if(run(i,"proc p {x} {capture [list $x $x]}; set x Q; p $x; p $x"))return 1;
    reset("opaque");{
        const char source[]="proc p {} {capture {\xff\0tail}}; p; p";
        Tcl_Obj *script=Tcl_NewStringObj(source,sizeof(source)-1);int code;Tcl_IncrRefCount(script);code=Tcl_EvalObjEx(i,script,0);Tcl_DecrRefCount(script);
        if(code!=TCL_OK){fprintf(stderr,"opaque: %s\n",Tcl_GetStringResult(i));return 1;}
    }
    reset("end");Tcl_DeleteInterp(i);
    {
        const char *values[]={"17", "+17", "017", "-0", "0", "-17", "17\0suffix", "17\0", "9223372036854775807", "-9223372036854775808", "9223372036854775808"};
        const int lengths[]={2,3,3,2,1,3,9,3,19,20,19};
        unsigned k;for(k=0;k<sizeof(lengths)/sizeof(lengths[0]);k++) {
            char script[160];const char *prefix="proc p {} {capture {",*suffix="}}; p";int n=0;Tcl_Obj *obj;int code;
            i=Tcl_CreateInterp();Tcl_CreateObjCommand(i,"capture",capture,NULL,NULL);reset("initial");
            memcpy(script+n,prefix,strlen(prefix));n+=strlen(prefix);memcpy(script+n,values[k],lengths[k]);n+=lengths[k];memcpy(script+n,suffix,strlen(suffix));n+=strlen(suffix);
            obj=Tcl_NewStringObj(script,n);Tcl_IncrRefCount(obj);code=Tcl_EvalObjEx(i,obj,0);Tcl_DecrRefCount(obj);if(code!=TCL_OK)return 1;
            reset("end");Tcl_DeleteInterp(i);
        }
    }
    Tcl_Finalize();return 0;
}
