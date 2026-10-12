/* tcl-lsp -- current original-header String replacement controls.
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
struct Case { const char *id; struct Input subject, first, last, replacement; int omit, alias_first, alias_last, alias_replacement; int binary_subject, binary_replacement, shared_subject, resident_subject, resident_replacement; };
#define B(s) { (const unsigned char *)(s), (int)(sizeof(s) - 1) }
static struct Case cases[] = {
    { "binary-partial-deletion", B("A\0\xffZ"), B("1"), B("2"), B(""), 1,0,0,0, 1,0,0,0,0 },
    { "binary-whole-deletion", B("A\0\xffZ"), B("0"), B("end"), B(""), 1,0,0,0, 1,0,0,0,0 },
    { "binary-whole-original-replacement", B("A\0\xffZ"), B("0"), B("end"), B("Q\0\xfe"), 0,0,0,0, 1,1,0,0,0 },
    { "binary-equal-unshared-splice", B("A\0\xffZ"), B("1"), B("2"), B("Q\xfe"), 0,0,0,0, 1,1,0,0,0 },
    { "binary-equal-shared-splice", B("A\0\xffZ"), B("1"), B("2"), B("Q\xfe"), 0,0,0,0, 1,1,1,0,0 },
    { "binary-unequal-splice", B("A\0\xffZ"), B("1"), B("2"), B("Q\0\xfe"), 0,0,0,0, 1,1,0,0,0 },
    { "binary-ordinary-string-replacement", B("A\0\xffZ"), B("1"), B("2"), B("\xce\xa9"), 0,0,0,0, 1,0,0,0,0 },
    { "binary-subject-with-resident-string", B("A\0\xffZ"), B("1"), B("2"), B("Q\xfe"), 0,0,0,0, 1,1,0,1,0 },
    { "binary-replacement-with-resident-string", B("A\0\xffZ"), B("1"), B("2"), B("Q\xfe"), 0,0,0,0, 1,1,0,0,1 },
    { "binary-inverted-replacement-unreached", B("A\0\xffZ"), B("2"), B("1"), B("Q\0\xfe"), 0,0,0,0, 1,1,0,0,0 },
    { "binary-empty-original", B(""), B("0"), B("end"), B("Q\0\xfe"), 0,0,0,0, 1,1,0,0,0 }
};
static void input_hex(const char *name, struct Input input) {
    int k; printf("%s\t", name); for (k = 0; k < input.length; k++) printf("%02x", input.bytes[k]); printf("\n");
}
int main(int argc, char **argv) {
    Interpreter *i; Object *objects[6], *owners[7], *result; int ownerc = 0, objc, code, n, k;
    if (argc != 2) return 64;
    n = atoi(argv[1]); if (n < 0 || n >= (int)(sizeof(cases) / sizeof(cases[0]))) return 65;
#ifdef PROBE_JIM
    i = Jim_CreateInterp(); Jim_RegisterCoreCommands(i);
    printf("provider\tJim\n");
#else
    Tcl_FindExecutable(argv[0]); i = Tcl_CreateInterp();
    printf("provider\tC-Tcl\n");
#endif
    printf("case\t%d\t%s\n", n, cases[n].id);
    input_hex("input-subject", cases[n].subject);
    input_hex("input-first", cases[n].first);
    input_hex("input-last", cases[n].last);
    input_hex("input-replacement", cases[n].replacement);
    printf("input-roles\tomit=%d\tfirst=subject:%d\tlast=first:%d\treplacement=subject:%d\n",
        cases[n].omit, cases[n].alias_first, cases[n].alias_last, cases[n].alias_replacement);
#ifdef PROBE_JIM
#define CONSTRUCT(slot, data) new_bytes(i, (data).bytes, (data).length)
#else
#define CONSTRUCT(slot, data) (((slot)==2 && cases[n].binary_subject) || ((slot)==5 && cases[n].binary_replacement) ? Tcl_NewByteArrayObj((data).bytes, (data).length) : new_bytes(i, (data).bytes, (data).length))
#endif
#define MAKE(slot, data) do { objects[slot] = CONSTRUCT(slot, data); hold(objects[slot]); owners[ownerc++] = objects[slot]; } while (0)
    { struct Input command = B("string"), selector = B("replace"); MAKE(0, command); MAKE(1, selector); }
    MAKE(2, cases[n].subject);
    if (cases[n].alias_first) objects[3] = objects[2]; else MAKE(3, cases[n].first);
    if (cases[n].alias_last) objects[4] = objects[3]; else MAKE(4, cases[n].last);
    if (cases[n].alias_replacement) objects[5] = objects[2]; else MAKE(5, cases[n].replacement);
    objc = cases[n].omit ? 5 : 6;
    printf("binary-roles\tsubject=%d\treplacement=%d\textra-subject-reference=%d\tresident-subject=%d\tresident-replacement=%d\n", cases[n].binary_subject, cases[n].binary_replacement, cases[n].shared_subject, cases[n].resident_subject, cases[n].resident_replacement);
    if (cases[n].shared_subject) { hold(objects[2]); owners[ownerc++] = objects[2]; }
    if (cases[n].resident_subject) bytes(i, "setup-subject-string", objects[2]);
    if (cases[n].resident_replacement) bytes(i, "setup-replacement-string", objects[5]);
    for (k = 2; k < 6; k++) { char label[32]; sprintf(label, "before-arg%d", k); snapshot(label, objects[k]); }
#ifdef PROBE_JIM
    Jim_SetResultString(i, "PRIOR", 5);
    code = Jim_EvalObjVector(i, objc, objects);
    result = i->result;
#else
    Tcl_SetObjResult(i, Tcl_NewStringObj("PRIOR", 5));
    code = Tcl_EvalObjv(i, objc, objects, TCL_EVAL_GLOBAL);
    result = Tcl_GetObjResult(i);
#endif
    printf("code\t%d\n", code);
    for (k = 2; k < 6; k++) { char label[32]; sprintf(label, "reached-arg%d", k); snapshot(label, objects[k]); }
    snapshot("reached-result", result);
    printf("result-alias\tsubject=%d\tfirst=%d\tlast=%d\treplacement=%d\n",
        result == objects[2], result == objects[3], result == objects[4], result == objects[5]);
    bytes(i, "result-bytes", result);
    snapshot("materialised-result", result);
    bytes(i, "subject-bytes", objects[2]);
    for (k = ownerc - 1; k >= 0; k--) release(i, owners[k]);
#ifdef PROBE_JIM
    Jim_FreeInterp(i);
#else
    Tcl_DeleteInterp(i); Tcl_Finalize();
#endif
    return 0;
}
