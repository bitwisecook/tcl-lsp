#include <tcl.h>
#include <stdio.h>
#include <limits.h>
#include <string.h>
static Tcl_Obj *make(int k) {
 const char zero[]={ '1',0,'X'};
 switch(k) {
 case 0:return Tcl_NewIntObj(17);
 case 1:return Tcl_NewLongObj(4294967295L);
 case 2:return Tcl_NewWideIntObj(17);
 case 3:return Tcl_NewWideIntObj(4294967296LL);
 case 4:return Tcl_NewStringObj("17",-1);
 case 5:return Tcl_NewStringObj("4294967295",-1);
 case 6:return Tcl_NewStringObj("4294967296",-1);
 case 7:return Tcl_NewStringObj("18446744073709551615",-1);
 case 8:return Tcl_NewStringObj("-18446744073709551615",-1);
 case 9:return Tcl_NewStringObj("18446744073709551616",-1);
 case 10:return Tcl_NewStringObj(zero,3);
 case 11:return Tcl_NewDoubleObj(17.0);
 default:return Tcl_NewStringObj("08",-1);
 }
}
static const char *type(Tcl_Obj *o){return o->typePtr?o->typePtr->name:"NULL";}
static void hex(const char *s,int n){int j;for(j=0;j<n;j++)printf("%02x",(unsigned char)s[j]);}
int main(int argc,char **argv){int k,g;Tcl_Interp *i;Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();printf("META\t%zu\t%zu\n",sizeof(long),sizeof(Tcl_WideInt));
 for(k=0;k<13;k++)for(g=0;g<3;g++){Tcl_Obj *o=make(k);const char *before=type(o);char *bs=o->bytes;int iv=777,code,n;long lv=777;Tcl_WideInt wv=777;Tcl_Obj *m;Tcl_IncrRefCount(o);Tcl_ResetResult(i);Tcl_SetErrorCode(i,"SEEDED","CODE",NULL);
 if(g==0)code=Tcl_GetIntFromObj(i,o,&iv);else if(g==1)code=Tcl_GetLongFromObj(i,o,&lv);else code=Tcl_GetWideIntFromObj(i,o,&wv);
 printf("ROW\t%d\t%d\t%s\t%s\t%d\t%d\t%d\t%lld\t",k,g,before,type(o),bs!=NULL,o->bytes!=NULL,bs&&bs==o->bytes,(long long)(g==0?iv:g==1?lv:wv));
 m=Tcl_GetObjResult(i);{const char *s=Tcl_GetStringFromObj(m,&n);hex(s,n);}printf("\t%d\n",code);Tcl_DecrRefCount(o);
 } Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
