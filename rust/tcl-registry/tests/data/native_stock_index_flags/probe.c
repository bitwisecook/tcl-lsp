#include <tcl.h>
#include <stdio.h>
static const char *const words[]={"provide","present","",NULL};
struct Entry {const char *word; const char *pad;};
static const struct Entry entries[]={{"provide","padding"},{"present",NULL},{NULL,NULL}};
static void row(const char *stage,int code,int index,Tcl_Obj *o,char *before){
 printf("%s\t%d\t%d\t%s\t%d\t%d\n",stage,code,index,o&&o->typePtr?o->typePtr->name:"none",o&&o->bytes!=NULL,o?o->bytes==before:1);
}
int main(int argc,char **argv){
 Tcl_FindExecutable(argv[0]); Tcl_Interp*i=Tcl_CreateInterp();int index=-1,code;
 Tcl_Obj*o=Tcl_NewStringObj("present",7);Tcl_IncrRefCount(o);char*before=o->bytes;
 code=Tcl_GetIndexFromObjStruct(i,o,entries,sizeof(char*),"option",0,&index);row("pointer-stride",code,index,o,before);
 index=-1;code=Tcl_GetIndexFromObjStruct(i,o,entries,sizeof(struct Entry),"option",0,&index);row("struct-stride",code,index,o,before);
 Tcl_InvalidateStringRep(o);index=-1;code=Tcl_GetIndexFromObjStruct(i,o,entries,sizeof(char*),"option",0,&index);row("changed-stride-absent",code,index,o,NULL);Tcl_DecrRefCount(o);
#if TCL_MAJOR_VERSION >= 9
 o=Tcl_NewStringObj("pro",3);Tcl_IncrRefCount(o);before=o->bytes;index=-1;
 code=Tcl_GetIndexFromObj(i,o,words,"option",0,&index);row("ordinary",code,index,o,before);
 index=-1;code=Tcl_GetIndexFromObj(i,o,words,"option",TCL_EXACT|TCL_INDEX_TEMP_TABLE,&index);row("temporary-exact-miss",code,index,o,before);
 index=-1;code=Tcl_GetIndexFromObj(i,o,words,"option",TCL_EXACT,&index);row("cache-after-temporary-miss",code,index,o,before);Tcl_DecrRefCount(o);
 o=Tcl_NewStringObj("pro",3);Tcl_IncrRefCount(o);before=o->bytes;index=-1;code=Tcl_GetIndexFromObj(i,o,words,"option",TCL_INDEX_TEMP_TABLE,&index);row("temporary-fresh",code,index,o,before);Tcl_DecrRefCount(o);
 o=Tcl_NewStringObj("",0);Tcl_IncrRefCount(o);before=o->bytes;index=-2;code=Tcl_GetIndexFromObj(i,o,words,"option",TCL_NULL_OK,&index);row("null-empty",code,index,o,before);
 index=-1;code=Tcl_GetIndexFromObj(i,o,words,"option",0,&index);row("empty-exact-entry",code,index,o,before);
 Tcl_InvalidateStringRep(o);index=-1;code=Tcl_GetIndexFromObj(i,o,words,"option",TCL_NULL_OK,&index);row("cached-null-absent",code,index,o,NULL);Tcl_DecrRefCount(o);
 index=-2;code=Tcl_GetIndexFromObjStruct(i,NULL,words,sizeof(char*),"option",TCL_NULL_OK,&index);row("null-object",code,index,NULL,NULL);
#endif
 Tcl_DeleteInterp(i);Tcl_Finalize();return 0;
}
