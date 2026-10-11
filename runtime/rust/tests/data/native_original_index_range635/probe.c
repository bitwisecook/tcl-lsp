/* tcl-lsp -- current original-header String index/range observations.
 * Copyright (C) 2026 James Deucker (bitwisecook).
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Every source below is a counted C byte array, not Tcl source text.
 * No index/subject/result getter occurs between the original EvalObjv and
 * the "reached" snapshots. Result bytes are materialised only afterward.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef PROBE_JIM
#include "jim.h"
typedef Jim_Obj Object;
typedef Jim_Interp Interpreter;
static Object *new_bytes(Interpreter *i, const unsigned char *s, int n) { return Jim_NewStringObj(i, (const char *)s, n); }
static void hold(Object *o) { Jim_IncrRefCount(o); }
static void release(Interpreter *i, Object *o) { Jim_DecrRefCount(i, o); }
static void snapshot(const char *name, Object *o) {
    printf("%s\t%s\t%d\t%d\t%lld", name, o->typePtr ? o->typePtr->name : "none", o->bytes != NULL, o->refCount, (long long)o->length);
    if (o->typePtr && strcmp(o->typePtr->name, "string") == 0) printf("\tchars=%d", o->internalRep.strValue.charLength);
    printf("\n");
}
static void bytes(Interpreter *i, const char *name, Object *o) {
    int n, k; const unsigned char *s = (const unsigned char *)Jim_GetString(o, &n);
    (void)i; printf("%s\t", name); for (k = 0; k < n; k++) printf("%02x", s[k]); printf("\n");
}
#else
#include "tcl.h"
typedef Tcl_Obj Object;
typedef Tcl_Interp Interpreter;
static Object *new_bytes(Interpreter *i, const unsigned char *s, int n) { (void)i; return Tcl_NewStringObj((const char *)s, n); }
static void hold(Object *o) { Tcl_IncrRefCount(o); }
static void release(Interpreter *i, Object *o) { (void)i; Tcl_DecrRefCount(o); }
static void snapshot(const char *name, Object *o) {
    printf("%s\t%s\t%d\t%d\t%lld\n", name, o->typePtr ? o->typePtr->name : "none", o->bytes != NULL, o->refCount, (long long)o->length);
}
static void bytes(Interpreter *i, const char *name, Object *o) {
#if TCL_MAJOR_VERSION >= 9
    Tcl_Size n, k;
#else
    int n, k;
#endif
    const unsigned char *s = (const unsigned char *)Tcl_GetStringFromObj(o, &n);
    (void)i; printf("%s\t", name); for (k = 0; k < n; k++) printf("%02x", s[k]); printf("\n");
}
#endif
struct Input { const unsigned char *bytes; int length; };
struct Case { const char *id, *operation; struct Input subject, first, last; int producer, materialise_subject, alias_first, alias_last, extra_subject; };
#define B(s) { (const unsigned char *)(s), (int)(sizeof(s) - 1) }
static struct Case cases[] = {
 { "ascii-index", "index", B("ABC"), B("1"), B(""), 0,0,0,0,0 },
 { "ascii-whole-range", "range", B("ABC"), B("0"), B("end"), 0,0,0,0,0 },
 { "raw-four-byte-index-first", "index", B("A\xf0\x9f\x98\x80Z"), B("1"), B(""), 0,0,0,0,0 },
 { "raw-four-byte-index-next", "index", B("A\xf0\x9f\x98\x80Z"), B("2"), B(""), 0,0,0,0,0 },
 { "raw-four-byte-range", "range", B("A\xf0\x9f\x98\x80Z"), B("1"), B("2"), 0,0,0,0,0 },
 { "cesu-surrogate-index-first", "index", B("A\xed\xa0\xbd\xed\xb8\x80Z"), B("1"), B(""), 0,0,0,0,0 },
 { "cesu-surrogate-index-next", "index", B("A\xed\xa0\xbd\xed\xb8\x80Z"), B("2"), B(""), 0,0,0,0,0 },
 { "cesu-surrogate-range", "range", B("A\xed\xa0\xbd\xed\xb8\x80Z"), B("1"), B("2"), 0,0,0,0,0 },
 { "counted-nul-index", "index", B("A\0B"), B("1"), B(""), 0,0,0,0,0 },
 { "counted-nul-whole-range", "range", B("A\0B"), B("0"), B("2"), 0,0,0,0,0 },
 { "raw-invalid-index", "index", B("\x80\xff" "B"), B("0"), B(""), 0,0,0,0,0 },
 { "raw-invalid-range", "range", B("\x80\xff" "B"), B("0"), B("1"), 0,0,0,0,0 },
 { "proper-binary-index", "index", B("\0\xff" "A"), B("1"), B(""), 1,0,0,0,0 },
 { "proper-binary-whole-range", "range", B("\0\xff" "A"), B("0"), B("2"), 1,0,0,0,0 },
 { "subject-is-original-index", "index", B("2*1"), B(""), B(""), 0,0,1,0,0 },
 { "same-first-last-original", "range", B("A\xc3\xa9\xe2\x82\xacZ"), B("1"), B(""), 0,0,0,1,0 },
 { "proper-binary-resident-shared-range", "range", B("\0\xff" "A"), B("0"), B("1"), 1,1,0,0,1 },
 { "original-unicode-surrogate-range", "range", B(""), B("1"), B("2"), 2,0,0,0,0 },
 { "counted-nul-original-index", "index", B("ABC"), B("1\0tail"), B(""), 0,0,0,0,0 },
 { "counted-nul-original-end", "range", B("ABC"), B("0"), B("end-1\0tail"), 0,0,0,0,0 }
};
static void input_hex(const char *name, struct Input input) {
 int k; printf("%s\t", name); for (k=0;k<input.length;k++) printf("%02x",input.bytes[k]); printf("\n");
}
int main(int argc, char **argv) {
 Interpreter *i; Object *objects[5], *owners[8], *result; int ownerc=0, objc, code, n, k;
 if (argc!=2) return 64;
 n=atoi(argv[1]); if(n<0 || n >= (int)(sizeof(cases)/sizeof(cases[0]))) return 65;
#ifdef PROBE_JIM
 i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); printf("provider\tJim\n");
#else
 Tcl_FindExecutable(argv[0]); i=Tcl_CreateInterp(); printf("provider\tC-Tcl\n");
#endif
 printf("case\t%d\t%s\n",n,cases[n].id); printf("operation\t%s\n",cases[n].operation);
 input_hex("input-subject",cases[n].subject); input_hex("input-first",cases[n].first); input_hex("input-last",cases[n].last);
 printf("input-roles\tproducer=%d\tmaterialise=%d\tfirst=subject:%d\tlast=first:%d\textra-subject=%d\n",cases[n].producer,cases[n].materialise_subject,cases[n].alias_first,cases[n].alias_last,cases[n].extra_subject);
#ifdef PROBE_JIM
 if(cases[n].producer!=0) { printf("skipped\tproper-binary-or-unicode-constructor-unavailable\n"); Jim_FreeInterp(i); return 0; }
#endif
#define MAKE(slot,data) do { objects[slot]=new_bytes(i,(data).bytes,(data).length); hold(objects[slot]); owners[ownerc++]=objects[slot]; } while(0)
 { struct Input command=B("string"); MAKE(0,command); }
 { struct Input selector={ (const unsigned char *)cases[n].operation, (int)strlen(cases[n].operation) }; MAKE(1,selector); }
#ifndef PROBE_JIM
 if(cases[n].producer==1) objects[2]=Tcl_NewByteArrayObj(cases[n].subject.bytes,cases[n].subject.length);
 else if(cases[n].producer==2) { const Tcl_UniChar units[]={0x41,0xd83d,0xde00,0x5a}; objects[2]=Tcl_NewUnicodeObj(units,4); printf("input-unicode-units\t0041,d83d,de00,005a\tunit-bytes=%d\n",(int)sizeof(Tcl_UniChar)); }
 else objects[2]=new_bytes(i,cases[n].subject.bytes,cases[n].subject.length);
#else
 objects[2]=new_bytes(i,cases[n].subject.bytes,cases[n].subject.length);
#endif
 hold(objects[2]); owners[ownerc++]=objects[2];
 if(cases[n].extra_subject) { hold(objects[2]); owners[ownerc++]=objects[2]; }
 if(cases[n].materialise_subject) { snapshot("before-preparation-subject",objects[2]); bytes(i,"preparation-subject-bytes",objects[2]); snapshot("after-preparation-subject",objects[2]); }
 if(cases[n].alias_first) objects[3]=objects[2]; else MAKE(3,cases[n].first);
 if(cases[n].alias_last) objects[4]=objects[3]; else MAKE(4,cases[n].last);
 objc=strcmp(cases[n].operation,"index")==0 ? 4 : 5;
 for(k=2;k<5;k++) { char label[32]; sprintf(label,"before-arg%d",k); snapshot(label,objects[k]); }
#ifdef PROBE_JIM
 Jim_SetResultString(i,"PRIOR",5); code=Jim_EvalObjVector(i,objc,objects); result=i->result;
#else
 Tcl_SetObjResult(i,Tcl_NewStringObj("PRIOR",5)); code=Tcl_EvalObjv(i,objc,objects,TCL_EVAL_GLOBAL); result=Tcl_GetObjResult(i);
#endif
 printf("code\t%d\n",code);
 for(k=2;k<5;k++) { char label[32]; sprintf(label,"reached-arg%d",k); snapshot(label,objects[k]); }
 snapshot("reached-result",result); printf("result-alias\tsubject=%d\tfirst=%d\tlast=%d\n",result==objects[2],result==objects[3],result==objects[4]);
 bytes(i,"result-bytes",result); snapshot("materialised-result",result); bytes(i,"subject-bytes",objects[2]); snapshot("materialised-subject",objects[2]);
 for(k=ownerc-1;k>=0;k--) release(i,owners[k]);
#ifdef PROBE_JIM
 Jim_FreeInterp(i);
#else
 Tcl_DeleteInterp(i); Tcl_Finalize();
#endif
 return 0;
}
