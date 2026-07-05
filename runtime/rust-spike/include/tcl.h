// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/*
 * ============================================================================
 * SPIKE -- throwaway proof-of-concept. NOT the final design.
 * This is a hand-cut, widened subset that proves real extensions COMPILE
 * against an authored header; it is not exhaustive and the struct bodies for
 * channels / filesystems / obj-types carry only the fields the validation
 * tests touch. Do not treat this file's shape as the production target -- see
 * docs/design/runtime/c-extension-abi.md.
 * ============================================================================
 *
 * tcl.h -- authored, API-compatible subset for the Rust-runtime C-extension
 * spikes. Faithful to the real Tcl 9.0 tcl.h / tclDecls.h signatures so stock
 * extension source compiles unchanged, but the ABI behind the calls is ours
 * (direct C-ABI imports; no binary stubs table).
 *
 * Coverage is validated by runtime/rust-spike/compile-check against the real
 * Tcl dltest samples (pkga/pkgb/pkgt; pkgooa via tclOO.h) plus two synthetic
 * extensions exercising the channel / Tcl_ObjType / filesystem / threading /
 * NRE surface and (via tclTomMath.h) bignum.
 */
#ifndef SPIKE_TCL_H
#define SPIKE_TCL_H

/* Real tcl.h pulls in the C library; we do too (compiled with `zig cc`, which
 * supplies wasi-libc headers). Extensions such as pkgb.c use snprintf without
 * an explicit include and rely on this. */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- result codes & flags ---- */
#define TCL_OK 0
#define TCL_ERROR 1
#define TCL_RETURN 2
#define TCL_BREAK 3
#define TCL_CONTINUE 4

#define TCL_EVAL_GLOBAL 0x020000
#define TCL_EVAL_DIRECT 0x040000

/* ---- core scalar types ---- */
typedef ptrdiff_t Tcl_Size;
typedef int64_t Tcl_WideInt;
typedef uint64_t Tcl_WideUInt;
#define TCL_INDEX_NONE ((Tcl_Size) -1)
#define TCL_INTEGER_SPACE (3 * (int) sizeof(Tcl_WideInt))
#define TCL_STUB_MAGIC ((int) 0xFCA3BACF)

#define DLLEXPORT __attribute__((visibility("default")))

typedef void *ClientData;
typedef struct Tcl_Interp Tcl_Interp;
typedef struct Tcl_Command_ *Tcl_Command;
typedef struct Tcl_Trace_ *Tcl_Trace;
typedef struct Tcl_ObjType Tcl_ObjType;

/* ---- Tcl_Obj ---- */
typedef union Tcl_ObjInternalRep {
    long longValue;
    double doubleValue;
    void *otherValuePtr;
    Tcl_WideInt wideValue;
    struct {
        void *ptr1;
        void *ptr2;
    } twoPtrValue;
    struct {
        void *ptr;
        Tcl_Size size;
    } ptrAndSize;
} Tcl_ObjInternalRep;

typedef struct Tcl_Obj {
    Tcl_Size refCount;
    char *bytes;
    Tcl_Size length;
    const Tcl_ObjType *typePtr;
    Tcl_ObjInternalRep internalRep;
} Tcl_Obj;

/* Tcl_Obj internal-rep procs + custom obj type registration (survey surface). */
typedef void(Tcl_FreeInternalRepProc)(Tcl_Obj *objPtr);
typedef void(Tcl_DupInternalRepProc)(Tcl_Obj *srcPtr, Tcl_Obj *dupPtr);
typedef void(Tcl_UpdateStringProc)(Tcl_Obj *objPtr);
typedef int(Tcl_SetFromAnyProc)(Tcl_Interp *interp, Tcl_Obj *objPtr);

struct Tcl_ObjType {
    const char *name;
    Tcl_FreeInternalRepProc *freeIntRepProc;
    Tcl_DupInternalRepProc *dupIntRepProc;
    Tcl_UpdateStringProc *updateStringProc;
    Tcl_SetFromAnyProc *setFromAnyProc;
};

extern void Tcl_RegisterObjType(const Tcl_ObjType *typePtr);
extern const Tcl_ObjType *Tcl_GetObjType(const char *typeName);

/* ---- command procs (8.x int-arity + 9.x Tcl_Size-arity) ---- */
typedef int(Tcl_ObjCmdProc)(void *clientData, Tcl_Interp *interp, int objc,
                            Tcl_Obj *const *objv);
typedef int(Tcl_ObjCmdProc2)(void *clientData, Tcl_Interp *interp, Tcl_Size objc,
                             Tcl_Obj *const *objv);
typedef void(Tcl_CmdDeleteProc)(void *clientData);
typedef int(Tcl_CmdObjTraceProc2)(void *clientData, Tcl_Interp *interp,
                                  Tcl_Size level, const char *command,
                                  Tcl_Command commandInfo, Tcl_Size objc,
                                  Tcl_Obj *const *objv);
typedef void(Tcl_CmdObjTraceDeleteProc)(void *clientData);

/* ---- bootstrap / packages ---- */
extern const char *Tcl_InitStubs(Tcl_Interp *interp, const char *version,
                                 int exact);
extern int Tcl_PkgProvideEx(Tcl_Interp *interp, const char *name,
                            const char *version, const void *clientData);
#define Tcl_PkgProvide(interp, name, version)                                  \
    Tcl_PkgProvideEx(interp, name, version, NULL)

/* ---- command creation ---- */
extern Tcl_Command Tcl_CreateObjCommand(Tcl_Interp *interp, const char *cmdName,
                                        Tcl_ObjCmdProc *proc, void *clientData,
                                        Tcl_CmdDeleteProc *deleteProc);
extern Tcl_Command Tcl_CreateObjCommand2(Tcl_Interp *interp, const char *cmdName,
                                         Tcl_ObjCmdProc2 *proc, void *clientData,
                                         Tcl_CmdDeleteProc *deleteProc);
extern Tcl_Trace Tcl_CreateObjTrace2(Tcl_Interp *interp, Tcl_Size level,
                                     int flags, Tcl_CmdObjTraceProc2 *proc,
                                     void *clientData,
                                     Tcl_CmdObjTraceDeleteProc *delProc);

/* ---- value access / creation ---- */
extern char *Tcl_GetStringFromObj(Tcl_Obj *objPtr, Tcl_Size *lengthPtr);
extern char *Tcl_GetString(Tcl_Obj *objPtr);
extern Tcl_Size Tcl_NumUtfChars(const char *src, Tcl_Size length);
extern int Tcl_UtfNcmp(const char *s1, const char *s2, size_t n);
extern int Tcl_GetIntFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr, int *intPtr);
extern int Tcl_GetWideIntFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
                                 Tcl_WideInt *widePtr);
extern int Tcl_GetDoubleFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
                                double *doublePtr);
extern int Tcl_GetBooleanFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
                                 int *boolPtr);
extern Tcl_Obj *Tcl_NewWideIntObj(Tcl_WideInt value);
extern Tcl_Obj *Tcl_NewStringObj(const char *bytes, Tcl_Size length);
extern Tcl_Obj *Tcl_NewDoubleObj(double value);
extern Tcl_Obj *Tcl_NewBooleanObj(int value);
extern Tcl_Obj *Tcl_NewListObj(Tcl_Size objc, Tcl_Obj *const objv[]);
extern Tcl_Obj *Tcl_NewObj(void);
extern Tcl_Obj *Tcl_DuplicateObj(Tcl_Obj *objPtr);
#define Tcl_NewIntObj(value) Tcl_NewWideIntObj((Tcl_WideInt) (value))

extern int Tcl_ListObjAppendElement(Tcl_Interp *interp, Tcl_Obj *listPtr,
                                    Tcl_Obj *objPtr);
extern int Tcl_ListObjGetElements(Tcl_Interp *interp, Tcl_Obj *listPtr,
                                  Tcl_Size *objcPtr, Tcl_Obj ***objvPtr);

extern void Tcl_IncrRefCount(Tcl_Obj *objPtr);
extern void Tcl_DecrRefCount(Tcl_Obj *objPtr);

/* Bignum object conversion is public (tcl.h); raw mp_* arithmetic lives in the
 * sibling tclTomMath.h. `value` is an `mp_int *`. */
extern Tcl_Obj *Tcl_NewBignumObj(void *value);
extern int Tcl_GetBignumFromObj(Tcl_Interp *interp, Tcl_Obj *objPtr,
                                void *value);

/* ---- result / error ---- */
extern void Tcl_SetObjResult(Tcl_Interp *interp, Tcl_Obj *resultObjPtr);
extern Tcl_Obj *Tcl_GetObjResult(Tcl_Interp *interp);
extern void Tcl_WrongNumArgs(Tcl_Interp *interp, Tcl_Size objc,
                             Tcl_Obj *const *objv, const char *message);
extern void Tcl_AppendResult(Tcl_Interp *interp, ...);
extern int Tcl_GetErrorLine(Tcl_Interp *interp);

/* ---- eval ---- */
extern int Tcl_EvalEx(Tcl_Interp *interp, const char *script, Tcl_Size numBytes,
                      int flags);
extern int Tcl_EvalObjEx(Tcl_Interp *interp, Tcl_Obj *objPtr, int flags);
extern int Tcl_EvalObjv(Tcl_Interp *interp, Tcl_Size objc, Tcl_Obj *const objv[],
                        int flags);

/* ---- NRE (non-recursive engine) entry points (survey surface) ---- */
extern Tcl_Command Tcl_NRCreateCommand(Tcl_Interp *interp, const char *cmdName,
                                       Tcl_ObjCmdProc *proc,
                                       Tcl_ObjCmdProc *nreProc, void *clientData,
                                       Tcl_CmdDeleteProc *deleteProc);
extern int Tcl_NRCallObjProc(Tcl_Interp *interp, Tcl_ObjCmdProc *objProc,
                             void *clientData, int objc,
                             Tcl_Obj *const objv[]);

/* ---- channels (survey surface): representative fields only ---- */
typedef struct Tcl_Channel_ *Tcl_Channel;
typedef struct Tcl_ChannelType Tcl_ChannelType;
typedef int(Tcl_DriverInputProc)(void *instanceData, char *buf, int toRead,
                                 int *errorCodePtr);
typedef int(Tcl_DriverOutputProc)(void *instanceData, const char *buf,
                                  int toWrite, int *errorCodePtr);
typedef void(Tcl_DriverWatchProc)(void *instanceData, int mask);
typedef int(Tcl_DriverGetHandleProc)(void *instanceData, int direction,
                                     void **handlePtr);
typedef int(Tcl_DriverClose2Proc)(void *instanceData, Tcl_Interp *interp,
                                  int flags);

struct Tcl_ChannelType {
    const char *typeName;
    void *version;
    void *closeProc;
    Tcl_DriverInputProc *inputProc;
    Tcl_DriverOutputProc *outputProc;
    void *seekProc;
    void *setOptionProc;
    void *getOptionProc;
    Tcl_DriverWatchProc *watchProc;
    Tcl_DriverGetHandleProc *getHandleProc;
    Tcl_DriverClose2Proc *close2Proc;
    void *blockModeProc;
    void *flushProc;
    void *handlerProc;
    void *wideSeekProc;
    void *threadActionProc;
    void *truncateProc;
};

extern Tcl_Channel Tcl_CreateChannel(const Tcl_ChannelType *typePtr,
                                     const char *chanName, void *instanceData,
                                     int mask);
extern void Tcl_RegisterChannel(Tcl_Interp *interp, Tcl_Channel chan);
extern Tcl_Channel Tcl_GetChannel(Tcl_Interp *interp, const char *chanName,
                                  int *modePtr);
extern Tcl_Channel Tcl_StackChannel(Tcl_Interp *interp,
                                    const Tcl_ChannelType *typePtr,
                                    void *instanceData, int mask,
                                    Tcl_Channel prevChan);
extern void *Tcl_GetChannelInstanceData(Tcl_Channel chan);
extern const Tcl_ChannelType *Tcl_GetChannelType(Tcl_Channel chan);

/* ---- filesystem (survey surface): representative fields only ---- */
typedef struct Tcl_Filesystem Tcl_Filesystem;
typedef int(Tcl_FSPathInFilesystemProc)(Tcl_Obj *pathPtr, void **clientDataPtr);

struct Tcl_Filesystem {
    const char *typeName;
    Tcl_Size structureLength;
    int version;
    Tcl_FSPathInFilesystemProc *pathInFilesystemProc;
    void *dupInternalRepProc;
    void *freeInternalRepProc;
    void *internalToNormalizedProc;
    void *createInternalRepProc;
    void *normalizePathProc;
    void *filesystemPathTypeProc;
    void *filesystemSeparatorProc;
    void *statProc;
    void *accessProc;
    void *openFileChannelProc;
};

extern int Tcl_FSRegister(void *clientData, const Tcl_Filesystem *fsPtr);
extern int Tcl_FSUnregister(const Tcl_Filesystem *fsPtr);

/* ---- threading (survey surface; lives in tcl.h, not a separate header) ---- */
typedef struct Tcl_ThreadId_ *Tcl_ThreadId;
typedef void *Tcl_Mutex;
typedef void *Tcl_Condition;
typedef struct {
    void *data;
} Tcl_ThreadDataKey;
typedef struct Tcl_Time {
    long sec;
    long usec;
} Tcl_Time;
typedef void(Tcl_ThreadCreateProc)(void *clientData);

extern int Tcl_CreateThread(Tcl_ThreadId *idPtr, Tcl_ThreadCreateProc *proc,
                            void *clientData, Tcl_Size stackSize, int flags);
extern int Tcl_JoinThread(Tcl_ThreadId id, int *result);
extern void Tcl_MutexLock(Tcl_Mutex *mutexPtr);
extern void Tcl_MutexUnlock(Tcl_Mutex *mutexPtr);
extern void Tcl_ConditionWait(Tcl_Condition *condPtr, Tcl_Mutex *mutexPtr,
                              const Tcl_Time *timePtr);
extern void Tcl_ConditionNotify(Tcl_Condition *condPtr);
extern void *Tcl_GetThreadData(Tcl_ThreadDataKey *keyPtr, Tcl_Size size);

/* ---- Tcl_DString ---- */
#define TCL_DSTRING_STATIC_SIZE 200
typedef struct Tcl_DString {
    char *string;
    Tcl_Size length;
    Tcl_Size spaceAvl;
    char staticSpace[TCL_DSTRING_STATIC_SIZE];
} Tcl_DString;
extern void Tcl_DStringInit(Tcl_DString *dsPtr);
extern char *Tcl_DStringAppend(Tcl_DString *dsPtr, const char *bytes,
                               Tcl_Size length);
extern void Tcl_DStringFree(Tcl_DString *dsPtr);

/* ---- alloc ---- */
extern void *Tcl_Alloc(Tcl_Size size);
extern void Tcl_Free(void *ptr);

/* ---- hash tables (survey surface; e.g. pkgua.c keeps per-interp state) ---- */
#define TCL_SMALL_HASH_TABLE 4
#define TCL_STRING_KEYS (0)
#define TCL_ONE_WORD_KEYS (1)
#define TCL_CUSTOM_TYPE_KEYS (-2)

struct Tcl_HashKeyType;
typedef struct Tcl_HashEntry Tcl_HashEntry;
typedef struct Tcl_HashTable Tcl_HashTable;

struct Tcl_HashEntry {
    Tcl_HashEntry *nextPtr;
    Tcl_HashTable *tablePtr;
    size_t hash;
    void *clientData;
    union {
        char *oneWordValue;
        Tcl_Obj *objPtr;
        int words[1];
        char string[1];
    } key;
};

struct Tcl_HashTable {
    Tcl_HashEntry **buckets;
    Tcl_HashEntry *staticBuckets[TCL_SMALL_HASH_TABLE];
    Tcl_Size numBuckets;
    Tcl_Size numEntries;
    Tcl_Size rebuildSize;
    int downShift;
    int mask;
    int keyType;
    Tcl_HashEntry *(*findProc)(Tcl_HashTable *tablePtr, const char *key);
    Tcl_HashEntry *(*createProc)(Tcl_HashTable *tablePtr, const char *key,
                                 int *newPtr);
    const struct Tcl_HashKeyType *typePtr;
};

typedef struct Tcl_HashSearch {
    Tcl_HashTable *tablePtr;
    Tcl_Size nextIndex;
    Tcl_HashEntry *nextEntryPtr;
} Tcl_HashSearch;

#define Tcl_GetHashValue(h) ((h)->clientData)
#define Tcl_SetHashValue(h, value) ((h)->clientData = (void *) (value))

extern void Tcl_InitHashTable(Tcl_HashTable *tablePtr, int keyType);
extern void Tcl_DeleteHashTable(Tcl_HashTable *tablePtr);
extern Tcl_HashEntry *Tcl_CreateHashEntry(Tcl_HashTable *tablePtr,
                                          const void *key, int *newPtr);
extern Tcl_HashEntry *Tcl_FindHashEntry(Tcl_HashTable *tablePtr,
                                        const void *key);
extern void Tcl_DeleteHashEntry(Tcl_HashEntry *entryPtr);
extern Tcl_HashEntry *Tcl_FirstHashEntry(Tcl_HashTable *tablePtr,
                                         Tcl_HashSearch *searchPtr);
extern Tcl_HashEntry *Tcl_NextHashEntry(Tcl_HashSearch *searchPtr);

/* ---- variables, command teardown, unload (pkgua.c) ---- */
#define TCL_GLOBAL_ONLY 1
#define TCL_APPEND_VALUE 4
#define TCL_UNLOAD_DETACH_FROM_PROCESS (1 << 1)
extern const char *Tcl_SetVar2(Tcl_Interp *interp, const char *part1,
                               const char *part2, const char *newValue,
                               int flags);
extern int Tcl_DeleteCommandFromToken(Tcl_Interp *interp, Tcl_Command command);

/* ---- stubs-table presence (some extensions, e.g. pkgooa, introspect it).
 * Under our direct-ABI model these are nominal: the pointers are non-NULL and,
 * in a production runtime, would be populated with our real implementations. */
struct TclStubs;
extern const struct TclStubs *tclStubsPtr;

#ifdef __cplusplus
}
#endif

#endif /* SPIKE_TCL_H */
