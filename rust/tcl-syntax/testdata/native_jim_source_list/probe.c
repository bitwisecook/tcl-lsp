#include <stdio.h>
#include <string.h>
#include "jim.h"
static const char *type(Jim_Obj *o) { return o->typePtr ? o->typePtr->name : "none"; }
static void hex(const char *s,int n) { int k;for(k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);if(!n)printf("-"); }
int main(void) {
    static const char *inputs[]={"","a b","\n a\n b","{a\nb} c","\"a\nb\" c","a\\\nb c","{a\\\nb} c","\"a\\\nb\" c","{","\"","a\\","{a\\}x b","a\r\n b","a\\n b","{{a}\nb} c","a\0b c"};
    int mode,k;Jim_Interp *i=Jim_CreateInterp();
    for(mode=0;mode<4;mode++)for(k=0;k<16;k++) {
        Jim_Obj *r=Jim_NewStringObj(i,inputs[k],k==15?5:(int)strlen(inputs[k]));
        Jim_Obj *file=NULL,*dup=NULL;int line,n,j,base,after;
        Jim_IncrRefCount(r);
        if(mode) {
            file=mode==2?Jim_NewIntObj(i,17):Jim_NewStringObj(i,"f\0\xff",3);
            Jim_IncrRefCount(file);Jim_SetSourceInfo(i,r,file,10);
            if(mode==3) { dup=Jim_DuplicateObj(i,r);Jim_IncrRefCount(dup);Jim_DecrRefCount(i,r);r=dup; }
        } else file=Jim_GetSourceInfo(i,r,&line);
        base=file->refCount;
        printf("P\t%d\t%d\t%s\t%d\t%s\t%d\t%d\n",mode,k,type(r),r->bytes!=NULL,type(file),file->bytes!=NULL,base);
        n=Jim_ListLength(i,r);after=file->refCount;
        printf("L\t%d\t%d\t%s\t%d\t%d\t%s\t%d\t%d\n",mode,k,type(r),r->bytes!=NULL,n,type(file),file->bytes!=NULL,after);
        for(j=0;j<n;j++) {
            Jim_Obj *e=Jim_ListGetIndex(i,r,j),*found=Jim_GetSourceInfo(i,e,&line);int len;const char *s=Jim_GetString(e,&len);
            printf("E\t%d\t%d\t%d\t%s\t%d\t%d\t%d\t%d\t",mode,k,j,type(e),e->bytes!=NULL,e->refCount,found==file,line);hex(s,len);printf("\n");
        }
        Jim_DecrRefCount(i,r);
        printf("D\t%d\t%d\t%d\n",mode,k,file->refCount);
        if(mode)Jim_DecrRefCount(i,file);
    }
    Jim_FreeInterp(i);return 0;
}
