#include <stdio.h>
#include "/tmp/2286-oracles/jimtcl/jim.c"
static void snapshot(int k, Jim_Interp *i, Jim_Obj *o, Jim_Obj *f) {
    ScriptObj *s=Jim_GetIntRepPtr(o);
    printf("SCRIPT\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\n",k,s->len,s->firstline,s->linenr,s->missing,s->inUse,s->fileNameObj==f,f->refCount,o->bytes!=NULL);
    for(int j=0;j<s->len;j++) {
        Jim_Obj *t=s->token[j].objPtr;
        printf("TOKEN\t%d\t%d\t%d\t%s\t%d\t%d\t%d",k,j,s->token[j].type,t->typePtr?t->typePtr->name:"NULL",t->bytes!=NULL,t->bytes?t->length:-1,t->refCount);
        if(t->typePtr==&scriptLineObjType)printf("\t%d\t%d",t->internalRep.scriptLineValue.argc,t->internalRep.scriptLineValue.line);
        else if(t->typePtr==&intObjType)printf("\t%lld\t-",(long long)JimWideValue(t));
        else if(t->typePtr==&sourceObjType)printf("\t%d\t%d",t->internalRep.sourceValue.fileNameObj==f,t->internalRep.sourceValue.lineNumber);
        printf("\n");
    }
}
int main(void) {
    const char *cases[]={"", " \nset a \"x$y\\n[z]\"; set b {one\ntwo}", "set x {bad", "set x \"bad\n$y", "set x [echo \"bad", "set x ${bad", "set x $a(b", "set x a($y)", "{*}$x", "{expand}$x", "{bad}tail", "# A\0B\nset x X", "set x \\", "set x $($y)", "set x {a}b\n{z}q"};
    int lens[]={0,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,13,-1,-1,-1};
    for(int k=0;k<15;k++) {
        Jim_Interp *i=Jim_CreateInterp();
        Jim_Obj *f=Jim_NewStringObj(i,"FILE",4), *o=Jim_NewStringObj(i,cases[k],lens[k]);
        Jim_IncrRefCount(f); Jim_IncrRefCount(o); Jim_SetSourceInfo(i,o,f,7);
        printf("SOURCE\t%d\t%d\t%d\n",k,f->refCount,o->refCount);
        JimSetScriptFromAny(i,o); snapshot(k,i,o,f);
        Jim_Obj *d=Jim_DuplicateObj(i,o); Jim_IncrRefCount(d);
        printf("DUP\t%d\t%s\t%d\t%d\t%d\n",k,d->typePtr?d->typePtr->name:"NULL",d->bytes!=NULL,d->length,f->refCount);
        ScriptObj *s=Jim_GetIntRepPtr(o);
        for(int j=0;j<s->len;j++)if(s->token[j].type==JIM_TT_LINE) {
            Jim_Obj *ld=Jim_DuplicateObj(i,s->token[j].objPtr); Jim_IncrRefCount(ld);
            printf("LINE_DUP\t%d\t%d\t%s\t%d\t%d\n",k,j,ld->typePtr?ld->typePtr->name:"NULL",ld->bytes!=NULL,ld->length);
            Jim_DecrRefCount(i,ld);
        }
        Jim_DecrRefCount(i,d); Jim_DecrRefCount(i,o);
        printf("RETIRED\t%d\t%d\n",k,f->refCount);
        Jim_DecrRefCount(i,f); Jim_FreeInterp(i);
    }
    Jim_Interp *i=Jim_CreateInterp();
    ScriptObj *s=JimGetScript(i,i->emptyObj);
    printf("TRUE_EMPTY\t%s\t%s\t%d\t%d\n",i->emptyObj->typePtr?i->emptyObj->typePtr->name:"NULL",i->nullScriptObj->typePtr?i->nullScriptObj->typePtr->name:"NULL",s->len,s->fileNameObj==i->emptyObj);
    Jim_Obj *other=Jim_NewEmptyStringObj(i); Jim_IncrRefCount(other); JimGetScript(i,other);
    printf("EQUAL_EMPTY\t%s\t%d\n",other->typePtr?other->typePtr->name:"NULL",other==i->nullScriptObj);
    Jim_DecrRefCount(i,other); Jim_FreeInterp(i); return 0;
}
