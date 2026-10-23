#include <stdio.h>
#include "/tmp/2286-oracles/jimtcl/jim.c"
int main(void) {
 setvbuf(stdout,NULL,_IONBF,0);
 for(int mode=0;mode<4;mode++) {
  Jim_Interp *i=Jim_CreateInterp();
  Jim_Obj *f=Jim_NewStringObj(i,"FILE",4); Jim_IncrRefCount(f);
  Jim_Obj *child=Jim_NewStringObj(i,mode==3?"4":"MEMBER",-1);
  Jim_SetSourceInfo(i,child,f,7);
  if(mode==1)Jim_IncrRefCount(child);
  Jim_Obj *parent=Jim_NewListObj(i,&child,1);Jim_IncrRefCount(parent);
  Jim_Obj *duplicate=NULL;
  if(mode==2){duplicate=Jim_DuplicateObj(i,parent);Jim_IncrRefCount(duplicate);}
  int len;Jim_Obj **borrowed;JimListGetElements(i,parent,&len,&borrowed);
  printf("BEFORE\t%d\t%d\t%d\t%d\t%d\n",mode,f->refCount,child->refCount,len,borrowed[0]==child);
  if(mode==3){jim_wide value;int result=Jim_GetWide(i,parent,&value);printf("SHIMMER\t%d\t%d\t%lld\t%d\n",mode,result,(long long)value,f->refCount);}
  else {Jim_DecrRefCount(i,parent);parent=NULL;printf("AFTER_FIRST\t%d\t%d\n",mode,f->refCount);}
  if(duplicate){Jim_DecrRefCount(i,duplicate);printf("AFTER_LAST\t%d\t%d\n",mode,f->refCount);}
  if(mode==1){printf("EXTERNAL_LIVE\t%d\t%d\t%s\n",mode,child->refCount,child->typePtr->name);Jim_DecrRefCount(i,child);printf("AFTER_EXTERNAL\t%d\t%d\n",mode,f->refCount);}
  if(parent)Jim_DecrRefCount(i,parent);
  Jim_DecrRefCount(i,f);Jim_FreeInterp(i);
 }
 return 0;
}
