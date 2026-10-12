
#include "tclInt.h"
#include "tclStringRep.h"
#include <stdio.h>
#include <string.h>
static int ci, sharing;
static Tcl_Obj *make(int kind) {
 unsigned char binary[]={0xff,0};Tcl_UniChar unicode[]={0xe9,0x1f600};
 switch(kind){
 case 0:return Tcl_NewObj();
 case 1:return Tcl_NewStringObj("A",1);
 case 2:return Tcl_NewStringObj("\x80" "B",2);
 case 3:return Tcl_NewByteArrayObj(binary,2);
 case 4:{Tcl_Obj *o=Tcl_NewByteArrayObj(binary,2);Tcl_GetString(o);return o;}
 case 5:return Tcl_NewUnicodeObj(unicode,2);
 case 6:return Tcl_NewIntObj(7);
 case 7:return Tcl_NewListObj(0,NULL);
 case 8:{Tcl_Obj *x=Tcl_NewIntObj(7);return Tcl_NewListObj(1,&x);}
 case 9:return Tcl_NewDictObj();
 case 10:{Tcl_Obj *o=Tcl_NewStringObj("A",1);Tcl_GetUnicode(o);return o;}
 case 11:return Tcl_NewStringObj("A\0Z",3);
 case 12:return Tcl_NewStringObj("\xff",1);
 case 13:{Tcl_UniChar units[]={0xd800,0xdc00};return Tcl_NewUnicodeObj(units,2);}
 case 14:{Tcl_UniChar unit=0xdc00;return Tcl_NewUnicodeObj(&unit,1);}
 case 15:{Tcl_UniChar unit=0xd800;return Tcl_NewUnicodeObj(&unit,1);}
 default:return Tcl_NewStringObj("",0);
 }
}
static void snapshot(const char *when,int arg,Tcl_Obj *o) {
 printf("{\"case\":%d,\"sharing\":%d,\"window\":\"%s\",\"arg\":%d,\"type\":\"%s\",\"resident\":%d,\"refs\":%d,\"length\":%lld",ci,sharing,when,arg,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(int)o->refCount,(long long)(o->bytes?o->length:-1));
 if(o->typePtr && !strcmp(o->typePtr->name,"string")) {
   String *s=(String *)o->internalRep.twoPtrValue.ptr1;
   printf(",\"num_chars\":%lld,\"has_unicode\":%d",(long long)s->numChars,s->hasUnicode);
   if(s->hasUnicode){printf(",\"unicode\":[");for(Tcl_Size j=0;j<s->numChars;j++)printf("%s%u",j?",":"",(unsigned)s->unicode[j]);printf("]");}
 }
 puts("}");
}
int main(void) {
 setvbuf(stdout,NULL,_IONBF,0);
 int pairs[][3]={{11,1,-1},{1,11,-1},{12,1,-1},{1,12,-1},{12,2,-1},{11,2,-1},{13,1,-1},{15,14,-1},{14,15,-1},{13,2,-1},{15,2,-1},{0,11,0}};
 for(ci=0;ci<(int)(sizeof pairs/sizeof pairs[0]);ci++)for(sharing=0;sharing<2;sharing++){
  Tcl_Interp *i=Tcl_CreateInterp();int n=pairs[ci][2]<0?2:3;Tcl_Obj *v[3];
  for(int j=0;j<n;j++){v[j]=make(pairs[ci][j]);Tcl_IncrRefCount(v[j]);}
  if(sharing)Tcl_IncrRefCount(v[0]);
  for(int j=0;j<n;j++)snapshot("before",j,v[j]);
  Tcl_Obj *o=TclStringCat(i,n,v,TCL_STRING_IN_PLACE);
  for(int j=0;j<n;j++)snapshot("after-before-result-string",j,v[j]);
  if(o){snapshot("result-before-string",-1,o);printf("{\"case\":%d,\"sharing\":%d,\"result_identity\":[",ci,sharing);for(int j=0;j<n;j++)printf("%s%d",j?",":"",o==v[j]);printf("],\"result\":\"");Tcl_Size len;const unsigned char *s=(const unsigned char*)Tcl_GetStringFromObj(o,&len);for(Tcl_Size j=0;j<len;j++)printf("%02x",s[j]);puts("\"}");Tcl_IncrRefCount(o);Tcl_DecrRefCount(o);}
  else printf("{\"case\":%d,\"sharing\":%d,\"cat_failed\":true}\n",ci,sharing);
  if(sharing)Tcl_DecrRefCount(v[0]);for(int j=0;j<n;j++)Tcl_DecrRefCount(v[j]);Tcl_DeleteInterp(i);
 }
 return 0;
}
