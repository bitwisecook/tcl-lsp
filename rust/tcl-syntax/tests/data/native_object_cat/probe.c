
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
 int pairs[][3]={{1,1,-1},{3,3,-1},{3,1,-1},{4,3,-1},{5,1,-1},{1,5,-1},{1,2,-1},{6,2,-1},{5,3,-1},{7,1,-1},{9,1,-1},{8,1,-1},{0,5,0},{5,0,0},{0,0,6},{0,0,0},{10,1,-1},{1,10,-1},{6,0,0},{8,0,0},{9,0,0},{0,1,0},{1,0,1}};
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
