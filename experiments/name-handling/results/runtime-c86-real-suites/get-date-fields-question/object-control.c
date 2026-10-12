#include "tclInt.h"
#include <stdio.h>
#include <string.h>
static const char *type(Tcl_Obj *v) {return v->typePtr?v->typePtr->name:"NULL";}
static Tcl_Obj *key(Tcl_Interp *i,Tcl_Obj *dict,const char *name) {
    Tcl_DictSearch search;Tcl_Obj *k,*v;int done;
    Tcl_DictObjFirst(i,dict,&search,&k,&v,&done);
    while(!done) {if(!strcmp(Tcl_GetString(k),name)){Tcl_DictObjDone(&search);return k;}Tcl_DictObjNext(&search,&k,&v,&done);}
    Tcl_DictObjDone(&search);return NULL;
}
int main(int argc,char **argv) {
    Tcl_FindExecutable(argv[0]);Tcl_Interp *i=Tcl_CreateInterp();
    if(Tcl_Init(i)!=TCL_OK){fprintf(stderr,"init: %s\n",Tcl_GetStringResult(i));return 1;}
    Tcl_Obj *name=Tcl_NewStringObj("N\0\xff",3);Tcl_IncrRefCount(name);
    Tcl_Obj *cells[4]={Tcl_NewStringObj("-9223372036854775808",-1),Tcl_NewStringObj("0",-1),Tcl_NewStringObj("ignored",-1),name};
    Tcl_Obj *row=Tcl_NewListObj(4,cells),*table=Tcl_NewListObj(1,&row);
    Tcl_Obj *words[4]={Tcl_NewStringObj("::tcl::clock::GetDateFields",-1),Tcl_NewStringObj("0",-1),table,Tcl_NewStringObj("2361222",-1)};
    for(int n=0;n<4;n++)Tcl_IncrRefCount(words[n]);
    printf("BEFORE seconds %s change %s table %s transition %s offset %s name %s\n",type(words[1]),type(words[3]),type(table),type(cells[0]),type(cells[1]),type(name));
    int code=Tcl_EvalObjv(i,4,words,TCL_EVAL_GLOBAL);
    printf("CALL code %d result %s seconds %s change %s table %s transition %s offset %s name %s\n",code,type(Tcl_GetObjResult(i)),type(words[1]),type(words[3]),type(table),type(cells[0]),type(cells[1]),type(name));
    if(code==TCL_OK){
        Tcl_Obj *result=Tcl_GetObjResult(i),*value,*k=Tcl_NewStringObj("tzName",-1);Tcl_IncrRefCount(result);Tcl_IncrRefCount(k);
        Tcl_DictObjGet(i,result,k,&value);printf("NAME same_input %d refs %d\n",value==name,name->refCount);
        Tcl_Obj *secondsKey=key(i,result,"seconds"),*eraKey=key(i,result,"era"),*result2;
        Tcl_EvalObjv(i,4,words,TCL_EVAL_GLOBAL);result2=Tcl_GetObjResult(i);
        printf("KEYS shared_seconds %d shared_era %d distinct_result %d\n",secondsKey==key(i,result2,"seconds"),eraKey==key(i,result2,"era"),result!=result2);
        Tcl_ResetResult(i);Tcl_DecrRefCount(result);Tcl_DecrRefCount(k);
    }
    for(int n=0;n<4;n++)Tcl_DecrRefCount(words[n]);
    int len;const unsigned char *bytes=(const unsigned char*)Tcl_GetStringFromObj(name,&len);
    printf("RETIRED name_refs %d raw ",name->refCount);for(int n=0;n<len;n++)printf("%02x",bytes[n]);puts("");
    Tcl_DecrRefCount(name);Tcl_DeleteInterp(i);Tcl_Finalize();return code==TCL_OK?0:1;
}
