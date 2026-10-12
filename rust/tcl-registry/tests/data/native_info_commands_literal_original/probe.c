#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static void hex(const char *s, size_t n) { for(size_t k=0;k<n;k++)printf("%02x",(unsigned char)s[k]); }
static char *load(const char *p,size_t *n) { FILE *f=fopen(p,"rb"); if(!f)return NULL; if(fseek(f,0,SEEK_END)){fclose(f);return NULL;} long z=ftell(f); if(z<0||z>1048576){fclose(f);return NULL;} rewind(f); char *s=malloc((size_t)z+1);if(!s){fclose(f);return NULL;}if(fread(s,1,(size_t)z,f)!=(size_t)z){free(s);fclose(f);return NULL;}fclose(f);s[z]=0;*n=(size_t)z;return s;}
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp ProbeInterp;
typedef Jim_Obj Obj;
typedef int Count;
/* Exact selected jim.c ScriptToken/ScriptObj layout; only a retained live actual procedure body is accessed. */
typedef struct { Jim_Obj *objPtr; int type; } SelectedScriptToken;
typedef struct { SelectedScriptToken *token; Jim_Obj *fileNameObj; int len; int substFlags; int inUse; int firstline; int linenr; int missing; } SelectedScriptObj;
static ProbeInterp *fresh(void) { ProbeInterp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK){Jim_FreeInterp(i);return NULL;}return i; }
static Obj *object(ProbeInterp *i,const char *s,size_t n) { Obj *o=Jim_NewStringObj(i,s,(int)n);Jim_IncrRefCount(o);return o; }
static void hold(Obj *o) { Jim_IncrRefCount(o); }
static void drop(ProbeInterp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int eval(ProbeInterp *i,const char *s,size_t n) { Obj *o=object(i,s,n);int c=Jim_EvalObj(i,o);drop(i,o);return c; }
static int vector(ProbeInterp *i,int n,Obj **o) { return Jim_EvalObjVector(i,n,o); }
static Obj *result(ProbeInterp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,Count *n) { return Jim_GetString(o,n); }
static void header(const char *kind,long index,Obj *o) { printf("HEADER|%s|%ld|%s|%d|%d\n",kind,index,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->length);if(o->bytes){printf("RESIDENT_BYTES|%s|%ld|",kind,index);hex(o->bytes,o->length);puts("");} }
static Obj *body(ProbeInterp *i,Obj *name) { Jim_Cmd *c=Jim_GetCommand(i,name,0);return c&&(c->flags&JIM_CMD_ISPROC)?c->u.proc.bodyObjPtr:NULL; }
static void code(ProbeInterp *i,Obj *o) { (void)i;header("BODY",0,o);if(o->typePtr&&strcmp(o->typePtr->name,"script")==0){SelectedScriptObj *s=o->internalRep.ptr;printf("SCRIPT|%d|%d|%d\n",s->len,s->substFlags,s->inUse);for(int k=0;k<s->len;k++){printf("TOKEN|%d|%d\n",k,s->token[k].type);header("TOKEN",k,s->token[k].objPtr);Count n;const char *p=bytes(s->token[k].objPtr,&n);printf("TOKEN_BYTES|%d|",k);hex(p,n);puts("");}}puts("ORIGINAL_DISASSEMBLY|not_applicable_Jim_script_tokens_are_not_C_Tcl_opcodes"); }
static void children(ProbeInterp *i,Obj *o) { if(o->typePtr&&strcmp(o->typePtr->name,"list")==0){int n=o->internalRep.listValue.len;printf("LIST_LENGTH|%d\n",n);for(int k=0;k<n;k++){Obj *e=Jim_ListGetIndex(i,o,k);header("CHILD",k,e);Count z;const char *s=bytes(e,&z);printf("CHILD_BYTES|%d|",k);hex(s,z);puts("");}} }
static void birth_children(ProbeInterp *i,Obj *o) { (void)i;if(o->typePtr&&strcmp(o->typePtr->name,"list")==0){for(int k=0;k<o->internalRep.listValue.len;k++)header("CHILD_BIRTH",k,o->internalRep.listValue.ele[k]);} }
static void original_disassembly(ProbeInterp *i,Obj *name,Obj *o) { (void)i;(void)name;(void)o; }
static void destroy(ProbeInterp *i) { Jim_FreeInterp(i); }
#else
#include "tclInt.h"
#include "tclCompile.h"
typedef Tcl_Interp ProbeInterp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static ProbeInterp *fresh(void) { ProbeInterp *i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK){Tcl_DeleteInterp(i);return NULL;}return i; }
static Obj *object(ProbeInterp *i,const char *s,size_t n) { (void)i;Obj *o=Tcl_NewStringObj(s,(Count)n);Tcl_IncrRefCount(o);return o; }
static void hold(Obj *o) { Tcl_IncrRefCount(o); }
static void drop(ProbeInterp *i,Obj *o) { (void)i;Tcl_DecrRefCount(o); }
static int eval(ProbeInterp *i,const char *s,size_t n) { return Tcl_EvalEx(i,s,(Count)n,0); }
static int vector(ProbeInterp *i,int n,Obj **o) { return Tcl_EvalObjv(i,n,o,0); }
static Obj *result(ProbeInterp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,Count *n) { return Tcl_GetStringFromObj(o,n); }
static void header(const char *kind,long index,Obj *o) { printf("HEADER|%s|%ld|%s|%d|%lld\n",kind,index,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(long long)o->length);if(o->bytes){printf("RESIDENT_BYTES|%s|%ld|",kind,index);hex(o->bytes,o->length);puts("");} }
static Obj *body(ProbeInterp *i,Obj *name) { Count n;const char *s=bytes(name,&n);Proc *p=TclFindProc((Interp *)i,s);return p?p->bodyPtr:NULL; }
static void code(ProbeInterp *i,Obj *o) { (void)i;header("BODY",0,o);if(!o->typePtr||strcmp(o->typePtr->name,"bytecode")!=0){puts("BYTECODE|absent");return;}ByteCode *b=o->internalRep.twoPtrValue.ptr1;printf("BYTECODE|%lld|%lld|",(long long)b->numCodeBytes,(long long)b->numLitObjects);hex((const char *)b->codeStart,b->numCodeBytes);puts("");for(Count offset=0;offset<b->numCodeBytes;){unsigned int op=b->codeStart[offset];const InstructionDesc *d=&tclInstructionTable[op];if(d->numBytes<1||offset+d->numBytes>b->numCodeBytes){puts("INVALID_OPCODE");break;}printf("INSTRUCTION|%lld|%u|%s|",(long long)offset,op,d->name);hex((const char *)b->codeStart+offset,d->numBytes);puts("");offset+=d->numBytes;}for(Count k=0;k<b->numLitObjects;k++){Obj *lit=b->objArrayPtr[k];header("LITERAL",(long)k,lit);Count n;const char *s=bytes(lit,&n);printf("LITERAL_BYTES|%lld|",(long long)k);hex(s,n);puts("");}}
static void children(ProbeInterp *i,Obj *o) { if(o->typePtr&&strcmp(o->typePtr->name,"list")==0){Count n;Obj **v;if(Tcl_ListObjGetElements(i,o,&n,&v)==TCL_OK){printf("LIST_LENGTH|%lld\n",(long long)n);for(Count k=0;k<n;k++){header("CHILD",(long)k,v[k]);Count z;const char *s=bytes(v[k],&z);printf("CHILD_BYTES|%lld|",(long long)k);hex(s,z);puts("");}}} }
static void birth_children(ProbeInterp *i,Obj *o) { if(o->typePtr&&strcmp(o->typePtr->name,"list")==0){Count n;Obj **v;if(Tcl_ListObjGetElements(i,o,&n,&v)==TCL_OK){for(Count k=0;k<n;k++)header("CHILD_BIRTH",(long)k,v[k]);}} }
static void original_disassembly(ProbeInterp *i,Obj *name,Obj *o) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION < 6
 (void)i;(void)name;(void)o;puts("ORIGINAL_DISASSEMBLY|not_available_non_debug_build_original_opcode_bytes_and_instruction_table_retained");
#else
 (void)o;Obj *v[3]={object(i,"::tcl::unsupported::disassemble",31),object(i,"proc",4),name};int c=vector(i,3,v);printf("ORIGINAL_DISASSEMBLY|%d|",c);Count n;const char *s=bytes(result(i),&n);hex(s,n);puts("");drop(i,v[0]);drop(i,v[1]);
#endif
}
static void destroy(ProbeInterp *i) { Tcl_DeleteInterp(i); }
#endif
static void completion(ProbeInterp *i,const char *tag,int c) { Obj *o=result(i);hold(o);printf("COMPLETION|%s|%d\n",tag,c);header(tag,0,o);children(i,o);Count n;const char *s=bytes(o,&n);printf("RESULT_BYTES|%s|",tag);hex(s,n);puts("");drop(i,o); }
int main(int argc,char **argv) { if(argc!=7)return 2;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#endif
 ProbeInterp *i=fresh();if(!i)return 3;completion(i,"VERSION",eval(i,"info patchlevel",15));size_t z;char *s=load(argv[1],&z);if(!s)return 4;int c=eval(i,s,z);free(s);completion(i,"PRELUDE",c);if(c){destroy(i);return 0;}
 s=load(argv[2],&z);if(!s)return 4;Obj *b=object(i,s,z);free(s);Obj *name=object(i,argv[3],strlen(argv[3]));Obj *args=object(i,argv[4],strlen(argv[4]));Obj *v[4]={object(i,"proc",4),name,args,b};c=vector(i,4,v);completion(i,"DEFINE",c);drop(i,v[0]);drop(i,args);drop(i,b);if(c){drop(i,name);destroy(i);return 0;}
 Obj *actual=body(i,name);if(!actual)return 5;hold(actual);Obj *inv[2]={name,NULL};int n=1;if(strcmp(argv[5],"-")!=0){s=load(argv[5],&z);if(!s)return 4;inv[1]=object(i,s,z);free(s);n=2;}c=vector(i,n,inv);Obj *answer=result(i);hold(answer);printf("COMPLETION|ORIGINAL|%d\n",c);header("ORIGINAL",0,answer);birth_children(i,answer);if(n==2)header("ORIGINAL_ARGUMENT",0,inv[1]);code(i,actual);children(i,answer);Count count;const char *p=bytes(answer,&count);printf("RESULT_BYTES|ORIGINAL|");hex(p,count);puts("");drop(i,answer);if(n==2)drop(i,inv[1]);original_disassembly(i,name,actual);drop(i,actual);
 if(strcmp(argv[6],"-")!=0){s=load(argv[6],&z);if(!s)return 4;c=eval(i,s,z);free(s);completion(i,"AFTER_QUERY",c);}drop(i,name);destroy(i);return 0; }
