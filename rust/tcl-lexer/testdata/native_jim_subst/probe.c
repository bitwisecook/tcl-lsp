#include <stdio.h>
#include <string.h>
#include "/tmp/2286-oracles/jimtcl/jim.c"
static const char *type(Jim_Obj *o) { return o->typePtr ? o->typePtr->name : "NULL"; }
static void hex(Jim_Obj *o) { if (!o->bytes) { printf("-"); return; } for (int j=0;j<o->length;j++) printf("%02x",(unsigned char)o->bytes[j]); }
static void subst(int c,int flags,const char *bytes,int len) {
 Jim_Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i);
 Jim_Obj *f=Jim_NewStringObj(i,"FILE",4),*o=Jim_NewStringObj(i,bytes,len);
 Jim_IncrRefCount(f);Jim_IncrRefCount(o);Jim_SetSourceInfo(i,o,f,7);
 struct JimParserCtx parser;JimParserInit(&parser,o->bytes,o->length,1);int ordinal=0;while(1){const char *start=parser.p;JimParseSubst(&parser,flags);if(parser.eof)break;printf("RAW\t%d\t%d\t%d\t%d\t%d\t%ld\t%ld\t%ld\t%ld\t%d\n",c,flags,ordinal++,parser.tt,parser.tline,(long)(start-o->bytes),(long)(parser.p-o->bytes),(long)(parser.tstart-o->bytes),(long)(parser.tend+1-o->bytes),parser.linenr);}
 printf("BEFORE\t%d\t%d\t%s\t%d\t%d\n",c,flags,type(o),o->refCount,f->refCount);
 SetSubstFromAny(i,o,flags); ScriptObj *s=Jim_GetIntRepPtr(o);
 printf("SUBST\t%d\t%d\t%s\t%d\t%d\t%d\t%d\t%d\t%d\n",c,flags,type(o),s->len,s->substFlags,s->inUse,s->fileNameObj==i->emptyObj,f->refCount,o->refCount);
 for(int j=0;j<s->len;j++) { Jim_Obj *t=s->token[j].objPtr;
  printf("TOKEN\t%d\t%d\t%d\t%d\t%s\t%d\t%d\t",c,flags,j,s->token[j].type,type(t),t->refCount,t->bytes?t->length:-1);hex(t);printf("\n");
 }
 Jim_Obj *d=Jim_DuplicateObj(i,o);Jim_IncrRefCount(d);
 printf("DUP\t%d\t%d\t%s\t%d\t%d\n",c,flags,type(d),d->bytes!=NULL,f->refCount);
 Jim_DecrRefCount(i,d);Jim_DecrRefCount(i,o);printf("RETIRE\t%d\t%d\t%d\n",c,flags,f->refCount);
 Jim_DecrRefCount(i,f);Jim_FreeInterp(i);
}
static void children(const char *stage, Jim_Obj *o, Jim_Obj *f, Jim_Obj *originalname, Jim_Obj *originalindex) {
 Jim_Obj *n=o->internalRep.dictSubstValue.varNameObjPtr,*k=o->internalRep.dictSubstValue.indexObjPtr;
 printf("CHILDREN\t%s\t%s\t%d\t%d\t%d\t%d\t%d\t%s\t%s\t",stage,type(o),n==originalname,k==originalindex,n->refCount,k->refCount,f->refCount,type(n),type(k));hex(n);printf("\t");hex(k);printf("\n");
}
static void interpolated(void) {
 Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_Eval(i,"set k KEY; set d [dict create KEY VALUE]");
 Jim_Obj *f=Jim_NewStringObj(i,"FILE",4);Jim_IncrRefCount(f);
 const char *text[]={"d","(","k",")"};ScriptToken tok[4];
 for(int j=0;j<4;j++){tok[j].objPtr=Jim_NewStringObj(i,text[j],-1);Jim_IncrRefCount(tok[j].objPtr);tok[j].type=j==2?JIM_TT_VAR:JIM_TT_ESC;}
 Jim_SetSourceInfo(i,tok[0].objPtr,f,7);
 Jim_Obj *key=Jim_GetVariable(i,tok[2].objPtr,JIM_ERRMSG); /* Native VAR evaluation repeats the same cache hit. */
 printf("OPT_BEFORE\t%d\t%d\t%d\n",tok[0].objPtr->refCount,key->refCount,f->refCount);
 Jim_Obj *o=JimInterpolateTokens(i,tok,4,JIM_NONE);Jim_IncrRefCount(o);children("interpolated",o,f,tok[0].objPtr,key);
 Jim_Obj *d=Jim_DuplicateObj(i,o);Jim_IncrRefCount(d);children("interpolated-duplicate",d,f,tok[0].objPtr,key);Jim_DecrRefCount(i,d);
 SetDictSubstFromAny(i,o);children("dict-converted",o,f,tok[0].objPtr,key);
 Jim_Obj *r=JimExpandDictSugar(i,o);printf("EXPAND\t%d\t%s\t%d\t",r!=NULL,type(o),f->refCount);if(r)hex(r);else hex(Jim_GetResult(i));printf("\n");
 children("expanded",o,f,tok[0].objPtr,key);
 Jim_DecrRefCount(i,o);for(int j=0;j<4;j++)Jim_DecrRefCount(i,tok[j].objPtr);printf("OPT_RETIRE\t%d\n",f->refCount);Jim_DecrRefCount(i,f);Jim_FreeInterp(i);
}
static void reuse(int fatal) {
 Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_Obj *o=Jim_NewStringObj(i,"set x 1",-1);Jim_IncrRefCount(o);JimSetScriptFromAny(i,o);ScriptObj *a=Jim_GetIntRepPtr(o);ScriptObj *b=Jim_GetSubst(i,o,0);
 printf("REUSE\t%d\t%d\t%d\n",a==b,b->substFlags,b->len);fflush(stdout);
 if(fatal){Jim_Obj *r;int code=Jim_SubstObj(i,o,&r,0);printf("REUSE_OUTCOME\t%d\t%d\t",code,r!=NULL);hex(Jim_GetResult(i));printf("\n");}else {b=Jim_GetSubst(i,o,JIM_SUBST_NOESC);printf("REPARSE\t%d\t%d\t%d\n",a==b,b->substFlags,b->len);Jim_DecrRefCount(i,o);Jim_FreeInterp(i);}
}
int main(int argc,char **argv) {
 setvbuf(stdout,NULL,_IONBF,0);
 if(argc>1 && !strcmp(argv[1],"fatal-reuse")){reuse(1);return 0;}
 if(argc>1 && !strcmp(argv[1],"fatal-nul")){Jim_Interp *i=Jim_CreateInterp();Jim_Obj *o=Jim_NewStringObj(i,"d\0(k)",5);Jim_IncrRefCount(o);SetDictSubstFromAny(i,o);return 0;}
 const char *cases[]={"a\n$k[set x X]\\n{q};z", "A\0B$k[set x X]", "${x\ny}P\nQ", "d($k)"};int lens[]={-1,14,-1,-1};
 for(int c=0;c<4;c++)for(int flags=0;flags<8;flags++)subst(c,flags,cases[c],lens[c]);
 interpolated();reuse(0);return 0;
}
