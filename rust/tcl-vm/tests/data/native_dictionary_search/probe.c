#include <stdio.h>
#ifdef USE_JIM
#include "jim.h"
int main(void){puts("unsupported-native-C-search");return 0;}
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
int main(void){puts("unsupported-native-C-search");return 0;}
#else
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static void window(int mode,const char*stage,Tcl_Obj*root,Tcl_Obj*key,Tcl_Obj*value,int selected){
 printf("%d\t%s\t%s\t%d\t%d\t%d\t%d\t%d\n",mode,stage,root?(root->typePtr?root->typePtr->name:"none"):"dropped",root?root->bytes!=NULL:0,root?root->refCount:0,key->refCount,value->refCount,selected);
}
int main(int argc,char**argv){(void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp*i=Tcl_CreateInterp();
 for(int mode=0;mode<5;mode++){
  Tcl_Obj*key=Tcl_NewStringObj("k\0x",3),*value=Tcl_NewStringObj("V\xff",2),*root=Tcl_NewDictObj();Tcl_IncrRefCount(key);Tcl_IncrRefCount(value);Tcl_IncrRefCount(root);
  if(Tcl_DictObjPut(i,root,key,value)!=TCL_OK)return 3;
  window(mode,"before-search",root,key,value,0);
  Tcl_DictSearch search;Tcl_Obj*k=NULL,*v=NULL;int done=0;
  if(Tcl_DictObjFirst(i,root,&search,&k,&v,&done)!=TCL_OK||done)return 4;
  window(mode,"after-search",root,key,value,k==key&&v==value);
  if(mode==1||mode==2){Size length;if(Tcl_ListObjLength(i,root,&length)!=TCL_OK)return 5;}
  if(mode==3){Tcl_Obj*copy=Tcl_DuplicateObj(root);Tcl_IncrRefCount(copy);if(Tcl_DictObjPut(i,copy,Tcl_NewStringObj("other",-1),Tcl_NewStringObj("OTHER",-1))!=TCL_OK)return 6;Tcl_DecrRefCount(copy);}
  if(mode==2||mode==4){Tcl_DictObjNext(&search,&k,&v,&done);if(!done)return 7;}
  window(mode,"after-action",root,key,value,(mode==2||mode==4)?0:1);
  Tcl_DictObjDone(&search);
  window(mode,"after-done",root,key,value,0);
  Tcl_DecrRefCount(root);window(mode,"after-root-drop",NULL,key,value,0);Tcl_DecrRefCount(key);Tcl_DecrRefCount(value);
 }
 Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
#endif
#endif
