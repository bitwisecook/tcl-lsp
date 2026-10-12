#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <errno.h>
#include "/tmp/2286-oracles/jimtcl/jim.c"
int main(void) {
 const char *inputs[]={"0","1","-1","9223372036854775807","-9223372036854775808","18446744073709551615","18446744073709551616","-18446744073709551616","0x7fffffffffffffff","-0x8000000000000000","0xffffffffffffffff","0b101","0o17","0d19","  +17  ","0x-5","0x","1.0","1e9999","1e-9999","nan","inf","","1x","A\0B","0x1.8p1"};
 for(int c=0;c<26;c++)for(int seed=0;seed<2;seed++)for(int kind=0;kind<2;kind++) {
  Jim_Interp *i=Jim_CreateInterp();Jim_Obj *o=Jim_NewStringObj(i,inputs[c],c==24?3:-1);Jim_IncrRefCount(o);
  errno=seed?ERANGE:0;int before=errno;int code;uint64_t bits=0;
  if(kind==0){jim_wide value=0;code=Jim_GetWide(i,o,&value);memcpy(&bits,&value,8);}
  else {double value=0;code=Jim_GetDouble(i,o,&value);memcpy(&bits,&value,8);}
  int after=errno;
  printf("%d\t%d\t%d\t%d\t%016llx\t%s\t%d\t%d\n",c,seed,kind,code,(unsigned long long)bits,o->typePtr?o->typePtr->name:"NULL",before==ERANGE,after==ERANGE);
  Jim_DecrRefCount(i,o);Jim_FreeInterp(i);
 }
 return 0;
}
