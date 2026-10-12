#include <tcl.h>
#include <stdio.h>
#include <stdlib.h>
int main(int argc,char **argv) {
 if(argc!=2)return 2;FILE *f=fopen(argv[1],"rb");if(!f)return 3;
 fseek(f,0,SEEK_END);long n=ftell(f);rewind(f);char *s=malloc((size_t)n+1);if(!s)return 4;
 if(fread(s,1,(size_t)n,f)!=(size_t)n)return 5;fclose(f);s[n]=0;
 Tcl_Interp *i=Tcl_CreateInterp();int c=Tcl_EvalEx(i,s,(int)n,0);
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size len;
#else
 int len;
#endif
const unsigned char *r=(const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i),&len);
 printf("%d\t",c);for(int k=0;k<len;k++)printf("%02x",r[k]);puts("");free(s);Tcl_DeleteInterp(i);return 0;
}
