#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Record what each core Tcl command's C implementation can touch.

For every release tree `tmp/tcl<release>/generic`, find each command's
procedure in the interpreter's command tables (`builtInCmds`, the ensemble
`EnsembleImplMap` tables, `mathOpCmds`, `BuiltinFuncTable`, and the
`switch`-dispatched subcommands of the 8.4 and 8.5 `ObjCmd`s), take the
transitive closure of the in-tree functions and macros it can reach, and name
the world-touching interfaces in that closure.

One file per release is written under
`rust/tcl-registry/tests/data/command_evidence/`, one line per command or
`command subcommand`:

    NAME<TAB>VERDICT<TAB>CLASSES<TAB>ROOTS

`VERDICT` is `closed` when the closure reaches no world-touching interface,
calls no script and produces no completion code beyond `TCL_OK` and
`TCL_ERROR`; otherwise `open`. `CLASSES` lists each reached class and the
first interface that showed it. The registry test
`command_evidence::complete_rows_match_the_c_evidence` holds every row the
registry marks `Complete` to a `closed` line for each release its window
covers.

The command procedure is the semantic reference: a command the bytecode
compiler inlines is held equivalent to its procedure by Tcl's own test suite.
The evidence is for a release build: every `#if` branch is read, except the
`TCL_MEM_DEBUG` allocator's memory log.

Usage: scripts/dev/tcl_command_evidence.py [tmp-dir] [out-dir]
"""

from __future__ import annotations

import pathlib
import re
import sys
from dataclasses import dataclass, field

RELEASES = ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]

KEYWORDS = frozenset(
    "if else while for do switch case return sizeof break continue goto default "
    "typedef struct union enum static const volatile register extern inline "
    "unsigned signed int long short char float double void defined endif ifdef ifndef elif include "
    "__asm__ __asm asm __volatile__ volatile".split()
)

# Interfaces whose reach makes a command touch something beyond its operands
# and result. Matched as exact names, or as prefixes where the entry ends in
# `*`. Each class is one reason the command is not closed.
WORLD_CLASSES: dict[str, tuple[str, ...]] = {
    "variables": (
        "Tcl_SetVar*", "Tcl_ObjSetVar2", "Tcl_GetVar*", "Tcl_ObjGetVar2",
        "Tcl_UnsetVar*", "Tcl_UpVar*", "TclPtrSetVar*", "TclPtrGetVar*",
        "TclPtrIncrObjVar*", "TclLookupVar", "TclObjLookupVar*", "TclPtrUnsetVar*",
        "TclLookupArrayElement", "Tcl_LinkVar", "TclVarHashCreateVar",
        "TclObjVarErrMsg", "Tcl_TraceVar*", "TclCallVarTraces",
        "TclObjCallVarTraces", "TclGetNamespaceFromObj",
    ),
    "scripts": (
        "Tcl_Eval*", "TclEval*", "Tcl_NREval*", "TclNREval*", "Tcl_GlobalEval*",
        "Tcl_ExprObj", "Tcl_ExprString", "Tcl_ExprLong*", "Tcl_ExprDouble*",
        "Tcl_ExprBoolean*", "TclExecuteByteCode", "TclNRExecuteByteCode",
        "TclObjInvoke*", "TclInvoke*", "Tcl_NRCallObjProc", "TclNRInvoke*",
        "TclCompileScript", "TclCompileExpr", "Tcl_NRExprObj", "TclNRRunCallbacks",
        "Tcl_NRAddCallback", "TclNRAddCallback", "TclSetTailcall",
        "TclNRTailcallEval", "TclNRCoroutine*",
    ),
    "commands": (
        "Tcl_CreateObjCommand*", "Tcl_CreateCommand", "TclCreateObjCommand*",
        "Tcl_DeleteCommand*", "TclRenameCommand", "Tcl_FindCommand",
        "Tcl_GetCommandFromObj", "TclGetOriginalCommand", "Tcl_GetCommandInfo*",
        "Tcl_SetCommandInfo*", "TclMakeEnsemble", "Tcl_CreateEnsemble",
        "TclCreateProc", "TclFindProc", "TclGetObjInterpProc",
    ),
    "namespaces": (
        "Tcl_GetCurrentNamespace", "Tcl_GetGlobalNamespace", "Tcl_FindNamespace",
        "Tcl_CreateNamespace", "Tcl_DeleteNamespace", "TclGetNamespaceForQualName",
        "Tcl_PushCallFrame", "TclPushStackFrame", "Tcl_PopCallFrame",
        "TclGetFrame", "TclObjGetFrame",
    ),
    "channels": (
        "Tcl_GetChannel*", "TclGetChannelFromObj", "Tcl_Write*", "Tcl_Read*",
        "Tcl_Gets*", "Tcl_Seek", "Tcl_Tell", "Tcl_Flush", "Tcl_Close*",
        "Tcl_OpenFileChannel", "Tcl_FSOpenFileChannel", "Tcl_Eof",
        "Tcl_InputBlocked", "Tcl_SetChannelOption", "Tcl_GetChannelOption",
        "Tcl_RegisterChannel", "Tcl_UnregisterChannel", "Tcl_CreateChannel",
        "Tcl_GetStdChannel", "TclGetOpenMode", "fprintf", "printf", "fputs", "fflush",
        "fwrite",
    ),
    "filesystem": (
        "Tcl_FS*", "TclFS*", "TclpObj*", "TclpOpen*", "TclpGetCwd", "TclpStat",
        "TclpAccess", "TclpMatchInDirectory", "Tcl_Stat", "Tcl_Access",
        "TclpTempFileName", "TclGetPathType", "TclpFindExecutable",
        "TclpGetNativeCwd", "TclGetProcessGlobalValue", "Tcl_GetNameOfExecutable",
        "TclpGetUserHome", "TclpLoadFile", "Tcl_LoadFile",
    ),
    "clock": (
        "Tcl_GetTime", "TclpGetClicks", "TclpGetWideClicks", "TclpGetMicroseconds",
        "TclpGetSeconds", "TclpGetDate", "TclpLocaltime", "TclpGmtime", "time",
        "gettimeofday", "clock_gettime", "TclpGetTimeZone", "TclpGetTZName",
        "TclScaleTime", "localtime", "gmtime", "mktime", "TclGetMonotonicTime",
        "_ftime", "ftime", "GetSystemTimeAsFileTime", "QueryPerformanceCounter",
    ),
    "entropy": ("rand", "random", "srand", "TclpGetRandom*"),
    "process": (
        "getpid", "TclpGetPid", "getenv", "TclGetEnv", "TclSetEnv", "TclUnsetEnv",
        "Tcl_GetHostName", "TclpGetHostName", "Tcl_Exit", "Tcl_Finalize",
        "TclpCreateProcess", "Tcl_ReapDetachedProcs", "TclpGetUserName",
        "Tcl_GetPid", "TclpGetPlatform", "_wgetenv", "GetEnvironmentVariable*",
    ),
    "interpreter": (
        "Tcl_GetAssocData", "Tcl_SetAssocData", "Tcl_DeleteAssocData",
        "Tcl_CreateInterp", "Tcl_DeleteInterp", "Tcl_GetSlave", "Tcl_GetChild",
        "Tcl_CreateAlias*", "Tcl_IsSafe", "Tcl_HideCommand", "Tcl_ExposeCommand",
        "Tcl_SetRecursionLimit", "Tcl_LimitCheck", "TclInterpReady",
        "Tcl_CreateTrace", "Tcl_CreateObjTrace", "Tcl_DeleteTrace",
        "Tcl_CreateTimerHandler", "Tcl_DeleteTimerHandler", "Tcl_DoWhenIdle",
        "Tcl_CancelIdleCall", "Tcl_DoOneEvent", "Tcl_ServiceAll",
        "Tcl_AsyncMark", "Tcl_AsyncInvoke", "Tcl_ThreadQueueEvent",
        "Tcl_SetMaxBlockTime", "Tcl_Sleep",
    ),
    "packages": (
        "Tcl_PkgRequire*", "Tcl_PkgProvide*", "Tcl_PkgPresent*", "TclPkgRequire*",
        "Tcl_StaticPackage", "Tcl_StaticLibrary", "TclGetLoadedPackages*",
        "TclGetLoadedLibraries",
    ),
    "encodings": (
        "Tcl_GetEncoding", "Tcl_GetEncodingFromObj", "Tcl_SetSystemEncoding",
        "Tcl_GetEncodingName", "Tcl_GetEncodingNames", "Tcl_GetEncodingSearchPath",
        "Tcl_SetEncodingSearchPath", "TclGetEncodingFromObj",
    ),
}

COMPLETION_CODES = ("TCL_BREAK", "TCL_CONTINUE", "TCL_RETURN")

# Interpreter fields a closed command may touch: its result, the error state
# an `Error` completion writes, scratch memory, and the ensemble rewrite used
# only to word an error message. Any other `iPtr->` / `interp->` field is
# interpreter state, and opens the closure.
RESULT_FIELDS = frozenset(
    "objResultPtr result resultSpace freeProc appendResult appendAvl appendUsed "
    "errorInfo errorCode errorLine errorStack resetErrorStack returnOpts returnCode "
    "returnLevel returnCodeKey returnErrorcodeKey returnErrorinfoKey returnErrorlineKey "
    "returnErrorstackKey returnLevelKey returnOptionsKey ecVar eiVar upLiteral "
    "callLiteral innerLiteral innerContext emptyObjPtr ensembleRewrite execEnvPtr "
    "stubTable flags".split()
)
TABLE_CALL = re.compile(r"\b(\w+)\s*\[[^\]]*\]\s*\(")
MEMBER = re.compile(r"(?:->|\.)\s*(\w+)")
FIELD = re.compile(r"\b(?:iPtr|interp)\s*->\s*(\w+)|\(\s*\(\s*Interp\s*\*\s*\)\s*\w+\s*\)\s*->\s*(\w+)")

# Procedures a closed command may call through a pointer: an object type's
# own value procedures (`Tcl_ObjType`), a hash table's key procedures
# (`Tcl_HashKeyType`) and its own `findProc` / `createProc`, which only
# find, compare, hash and free keys, and the
# interpreter result's `freeProc`, which releases a legacy string result.
VALUE_PROCS = frozenset(
    "setFromAnyProc updateStringProc dupIntRepProc freeIntRepProc "
    "hashKeyProc compareKeysProc allocEntryProc freeEntryProc findProc createProc "
    "freeProc lengthProc indexProc sliceProc reverseProc getElementsProc "
    "setElementProc replaceProc inOperProc".split()
)
INDIRECT = re.compile(
    r"\(\s*\*\s*([\w.\[\]>-]+)\s*\)\s*\(|(?:->|\.)\s*(\w+)\s*\("
)

# Interfaces the closure does not enter, with the reason. Each one only
# reports an error to its caller through the interpreter's result and its
# error information, which a successful completion never observes, or panics.
ERROR_REPORTING = frozenset(
    "Tcl_SetErrorCode Tcl_SetObjErrorCode Tcl_AddErrorInfo Tcl_AddObjErrorInfo "
    "Tcl_AppendObjToErrorInfo TclSetErrorCode Tcl_SetErrorCodeVA Tcl_Panic "
    "Tcl_PanicVA panic TclPanic Tcl_WrongNumArgs TclLogCommandInfo "
    # The panic path ends the process; it is not a completion.
    "abort DebugBreak __builtin_trap __debugbreak ExitProcess panicProc".split()
)


# Calls the closure does not follow, each read in the source with the reason
# the call cannot touch the world. Keep this short: every entry is a claim.
REVIEWED_EDGES: dict[tuple[str, str], str] = {
    # 8.7/9.x index arithmetic beyond the wide range: `GetEndOffsetFromObj`
    # has already parsed the word as `INT±INT`, so the expression it hands
    # `Tcl_ExprObj` (in the caller's or a fresh interpreter) has no
    # substitution and evaluates no command.
    ("GetEndOffsetFromObj", "Tcl_ExprObj"): "INT±INT already parsed",
    ("GetEndOffsetFromObj", "Tcl_CreateInterp"): "INT±INT already parsed",
    ("GetEndOffsetFromObj", "Tcl_DeleteInterp"): "INT±INT already parsed",
    # 8.5 `Tcl_ResetResult` copies a pending legacy error into `::errorCode`
    # and `::errorInfo` (`ERR_LEGACY_COPY`): the error-state variables the
    # interpreter writes on every error completion, which an `Error`
    # completion already admits.
    ("Tcl_ResetResult", "Tcl_ObjSetVar2"): "legacy ::errorCode / ::errorInfo copy",
    # The expression functions' wrappers call their `clientData`, which
    # `BuiltinFuncTable` registers as a C library maths function.
    ("ExprUnaryFunc", "func"): "a BuiltinFuncTable libm function",
    ("ExprBinaryFunc", "func"): "a BuiltinFuncTable libm function",
    ("ExprBinaryDIFunc", "func"): "a BuiltinFuncTable libm function",
    # 9.1 `TclFormatDouble` reads the locale only to replace `snprintf`'s
    # decimal separator with `.`, so its result is the same in every locale.
    ("TclFormatDouble", "localeconv"): "normalises the decimal separator",
}


# C library functions a closed command may call: they compute from their
# arguments and the memory they are handed. Any other call the tree does not
# define opens the closure.
LIBC_PURE = frozenset(
    "memcpy memmove memset memcmp memchr strlen strcmp strncmp strchr strrchr strstr "
    "strcpy strncpy strcat strncat strspn strcspn strpbrk sprintf snprintf vsnprintf "
    "isdigit isspace isalpha isalnum isupper islower isxdigit ispunct isprint iscntrl "
    "isgraph isascii tolower toupper abs labs llabs fabs floor ceil sqrt cbrt pow exp "
    "exp2 expm1 log log10 log2 log1p logb sin cos tan asin acos atan atan2 sinh cosh "
    "tanh asinh acosh atanh fmod frexp ldexp modf hypot isnan isinf isfinite signbit "
    "copysign nextafter remainder remquo trunc round lround rint nearbyint fma fmin "
    "fmax fdim erf erfc tgamma lgamma ilogb scalbn strtol strtoul strtoll strtoull "
    "strtod qsort bsearch va_start va_end va_arg va_copy assert offsetof alloca "
    "malloc calloc realloc free strerror bcopy GetLastError SetLastError "
    # The FPU rounding mode, saved and restored around a conversion.
    "ieee_flags fpsetmask fpgetround fpsetround fesetround fegetround _controlfp "
    "_control87 fegetenv fesetenv".split()
)
# Thread synchronisation and thread-local storage: they order and isolate
# threads, and touch no interpreter state.
THREAD_SYNC = frozenset(
    "GetCurrentThreadId EnterCriticalSection LeaveCriticalSection TlsGetValue TlsSetValue "
    "pthread_self pthread_mutex_lock pthread_mutex_unlock pthread_getspecific "
    "pthread_setspecific pthread_once pthread_equal Tcl_MutexLock Tcl_MutexUnlock "
    "Tcl_MutexTryLock Tcl_ConditionWait Tcl_ConditionNotify TclpMutexLock "
    "TclpMutexUnlock".split()
)


def local_pointer_reaches_only_named(fn: Function | None, ident: str, tree: "Tree") -> bool:
    """Whether a call through `ident` reaches only procedures the closure
    follows: a parameter (the caller names what it passes), or a local whose
    every assignment is a named function, `NULL`, or an object type's own
    value procedure."""
    if fn is None:
        return False
    if re.search(rf"\(\s*\*\s*{ident}\s*\)|\b{ident}\s*[,)]", fn.signature):
        return True
    assigned = re.findall(rf"\b{ident}\s*=(?!=)\s*([^;]+);", fn.body)
    if not assigned:
        return False
    for rhs in assigned:
        choices = rhs.split("?", 1)[1].split(":") if "?" in rhs else [rhs]
        for choice in choices:
            choice = choice.strip().strip("()").strip()
            member = re.search(r"(?:->|\.)\s*(\w+)\s*$", choice)
            if not (
                choice in ("NULL", "0")
                or choice in tree.functions
                or choice in LIBC_PURE
                or re.match(r"TclObjTypeHasProc\s*\(", choice)
                or (member and member.group(1) in VALUE_PROCS)
            ):
                return False
    return True


THREAD_FAMILY = re.compile(
    r"pthread_\w+|\w*CriticalSection|Tcl_Mutex\w*|Tcl_Condition\w*|TclpMutex\w*|"
    r"TclpThread\w*|Tls\w+|GetCurrentThread\w*|TclpFree\w*Mutex"
)
CALL = re.compile(r"(?<!->)(?<!\.)\b([A-Za-z_]\w*)\s*\(")


# The `TCL_MEM_DEBUG` allocator and its object variants: a debug build
# configuration that adds a memory log on stderr. The evidence is for builds
# without it, so the closure does not enter it.
DEBUG_ALLOCATOR = re.compile(r"Tcl_Db\w+|TclDb\w+|Tcl_ValidateAllMemory|TclDumpMemoryInfo")


@dataclass
class Function:
    name: str
    body: str
    path: str
    signature: str = ""


@dataclass
class Tree:
    functions: dict[str, Function] = field(default_factory=dict)
    macros: dict[str, str] = field(default_factory=dict)
    sources: dict[str, str] = field(default_factory=dict)
    # File-static tables of procedures, per file: a table is followed only
    # from a body in its own file, as only that file can name it.
    tables: dict[str, dict[str, str]] = field(default_factory=dict)
    # The `Interp` structure's own fields, from `tclInt.h`.
    interp_fields: frozenset[str] = frozenset()


def strip_c(text: str) -> str:
    """Blank comments, string and character literals, keeping line structure."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "*":
            end = text.find("*/", i + 2)
            end = n if end < 0 else end + 2
            out.append(re.sub(r"[^\n]", " ", text[i:end]))
            i = end
        elif c == "/" and nxt == "/":
            end = text.find("\n", i)
            end = n if end < 0 else end
            out.append(" " * (end - i))
            i = end
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            literal = text[i : j + 1]
            out.append(c + re.sub(r"[^\n]", " ", literal[1:-1]) + c)
            i = j + 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def string_literals(text: str) -> list[str]:
    """The double-quoted literals of `text`, unescaped only for `\\\\`."""
    return [m.group(1) for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', text)]


DEF_START = re.compile(r"^(?:[A-Za-z_][\w\s*]*?[\s*])?([A-Za-z_]\w*)\s*\(", re.MULTILINE)


def find_functions(stripped: str, path: str) -> dict[str, Function]:
    """Function definitions in Tcl's layout: name at column 0, `{` and `}`
    at column 0 around the body."""
    found: dict[str, Function] = {}
    lines = stripped.split("\n")
    i = 0
    while i < len(lines):
        m = DEF_START.match(lines[i])
        if not m or m.group(1) in KEYWORDS or lines[i].rstrip().endswith(";"):
            i += 1
            continue
        if lines[i].rstrip().endswith("{"):
            # The opening brace on the definition's own line.
            k = i + 1
            while k < len(lines) and not lines[k].startswith("}"):
                k += 1
            found[m.group(1)] = Function(m.group(1), "\n".join(lines[i : k + 1]), path, lines[i])
            i = k + 1
            continue
        j = i + 1
        while j < len(lines) and j < i + 40:
            line = lines[j]
            if line.startswith("{"):
                break
            if line[:1] not in ("", " ", "\t") or line.rstrip().endswith(";") and line[:1] not in (" ", "\t"):
                j = len(lines)
                break
            j += 1
        if j >= len(lines) or j >= i + 40 or not lines[j].startswith("{"):
            i += 1
            continue
        k = j + 1
        while k < len(lines) and not lines[k].startswith("}"):
            k += 1
        found[m.group(1)] = Function(
            m.group(1), "\n".join(lines[j : k + 1]), path, "\n".join(lines[i:j])
        )
        i = k + 1
    return found


MACRO = re.compile(r"^[ \t]*#[ \t]*define[ \t]+([A-Za-z_]\w*)((?:\\\n|[^\n])*)", re.MULTILINE)


def find_macros(stripped: str) -> dict[str, str]:
    return {m.group(1): m.group(2).replace("\\\n", " ") for m in MACRO.finditer(stripped)}


TABLE = re.compile(
    r"(?m)^(?:static\s+)?(?:const\s+|CONST\s+)?[\w\s\*]*?\b(\w+)\s*\[\s*\w*\s*\]\s*=\s*\{"
)


def find_tables(stripped: str) -> dict[str, str]:
    """File-level array initialisers: a table of procedures is followed like
    a body, so a call through one of its entries reaches every entry."""
    tables = {}
    for m in TABLE.finditer(stripped):
        end = stripped.find("\n};", m.end())
        if end > 0:
            tables[m.group(1)] = stripped[m.end() : end]
    return tables


# The directories of a release tree that define what `generic/` calls: the
# platform layers (both are read, so a call reaches either platform's
# body), bundled libtommath, and the compatibility replacements.
SOURCE_DIRS = ("generic", "unix", "win", "macosx", "libtommath", "compat", "compat/zlib", "utf8proc")


def load_tree(generic: pathlib.Path) -> Tree:
    tree = Tree()
    root = generic.parent
    paths = [p for d in SOURCE_DIRS for p in sorted((root / d).glob("*.[ch]"))]
    for path in paths:
        raw = path.read_text(encoding="latin-1")
        tree.sources[path.name] = raw
        stripped = strip_c(raw)
        for name, body in find_macros(stripped).items():
            tree.macros.setdefault(name, body)
        # Headers define `static inline` functions as well as macros.
        if True:
            for name, fn in find_functions(stripped, path.name).items():
                if name in tree.functions:
                    # One body per platform: a call may reach either.
                    tree.functions[name].body += "\n" + fn.body
                else:
                    tree.functions[name] = fn
            tree.tables[path.name] = find_tables(stripped)
    tree.interp_fields = interp_fields(tree.sources.get("tclInt.h", ""))
    return tree


def interp_fields(header: str) -> frozenset[str]:
    """The member names of `typedef struct Interp { … } Interp;`."""
    m = re.search(r"typedef\s+struct\s+Interp\s*\{", header)
    if not m:
        return frozenset()
    body = strip_c(header[m.end() : header.index("} Interp;", m.end())])
    names = set()
    depth = 0
    statement = ""
    for ch in body:
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == ";":
            found = re.findall(r"(\w+)\s*(?:\[[^\]]*\])?\s*$", statement)
            if found and depth == 0:
                names.add(found[0])
            statement = ""
            continue
        if depth == 0 or ch not in "{}":
            statement += ch
        if ch == "}" and depth == 0:
            statement = ""
    return frozenset(names)


IDENT = re.compile(r"\b([A-Za-z_]\w*)\b")


def matches(name: str, patterns: tuple[str, ...]) -> bool:
    return any(name.startswith(p[:-1]) if p.endswith("*") else name == p for p in patterns)


def world_class(name: str) -> str | None:
    for cls, patterns in WORLD_CLASSES.items():
        if matches(name, patterns):
            return cls
    return None


@dataclass
class Evidence:
    classes: dict[str, str]
    codes: set[str]
    reached: set[str]
    via: dict[str, str] = field(default_factory=dict)

    def path(self, name: str) -> list[str]:
        """The call chain from a root to `name`."""
        chain = [name]
        while chain[-1] in self.via:
            chain.append(self.via[chain[-1]])
        return chain[::-1]

    @property
    def closed(self) -> bool:
        return not self.classes and not self.codes


def closure(
    tree: Tree,
    roots: list[str],
    bodies: list[str] | None = None,
    context: str | None = None,
) -> Evidence:
    """Everything `roots` (and extra code `bodies`, a slice of the function
    `context`) can reach in the tree."""
    classes: dict[str, str] = {}
    codes: set[str] = set()
    seen: set[str] = set()
    via: dict[str, str] = {}
    pending: list[tuple[str | None, str | None, str | None]] = [
        (None, body, None) for body in bodies or []
    ]
    pending += [(root, None, None) for root in roots]
    while pending:
        name, body, caller = pending.pop()
        file = tree.functions[context].path if context in tree.functions else None
        if name is not None:
            if name in seen:
                continue
            seen.add(name)
            if caller is not None:
                via[name] = caller
            cls = world_class(name)
            if cls:
                classes.setdefault(cls, name)
                continue
            if name in ERROR_REPORTING or DEBUG_ALLOCATOR.fullmatch(name):
                continue
            if name in tree.functions:
                body = tree.functions[name].body
                file = tree.functions[name].path
            elif name in tree.macros:
                body = tree.macros[name]
            elif "#" in name:
                file, table = name.split("#", 1)
                body = tree.tables[file][table]
            else:
                continue
        for code in COMPLETION_CODES:
            if re.search(rf"\b{code}\b", body):
                codes.add(code)
        tables = tree.tables.get(file, {}) if file else {}
        code = re.sub(r"(?m)^[ \t]*#.*$", "", body)
        for called in CALL.findall(code):
            if (
                called in KEYWORDS
                or called in tree.functions
                or called in tree.macros
                or called in LIBC_PURE
                or called in ERROR_REPORTING
                or DEBUG_ALLOCATOR.fullmatch(called)
                or called in THREAD_SYNC
                or THREAD_FAMILY.match(called)
                or world_class(called)
                or local_pointer_reaches_only_named(tree.functions.get(name or context or ""), called, tree)
                or (name, called) in REVIEWED_EDGES
                or called in (tree.tables.get(file, {}) if file else {})
            ):
                continue
            classes.setdefault("unresolved", f"{name or 'body'}:{called}")
        for m in FIELD.finditer(body):
            member = m.group(1) or m.group(2)
            if member not in RESULT_FIELDS:
                classes.setdefault("interpreter", f"{name or 'body'}:{member}")
        # However the interpreter is reached (`((Interp *)(interp))->…`, a
        # copied pointer), one of its own fields is interpreter state.
        for member in MEMBER.findall(code):
            if member in tree.interp_fields and member not in RESULT_FIELDS:
                classes.setdefault("interpreter", f"{name or 'body'}:{member}")
        for table in TABLE_CALL.findall(code):
            if table not in tables:
                classes.setdefault("indirect", f"{name or 'body'}:{table}[]")
        for m in INDIRECT.finditer(body):
            pointer, member = m.group(1), m.group(2)
            # A call through a plain local pointer reaches only procedures
            # this closure names, and so already follows; one stored in a
            # structure or a table may reach anything.
            if pointer is not None and re.fullmatch(r"\w+", pointer):
                continue
            if pointer is not None:
                member = re.split(r"->|\.", pointer)[-1]
            if member in VALUE_PROCS:
                continue
            classes.setdefault("indirect", f"{name or 'body'}:{pointer or member}")
        for ident in IDENT.findall(body):
            if ident in seen or ident in KEYWORDS or (name, ident) in REVIEWED_EDGES:
                continue
            if ident in tables and f"{file}#{ident}" not in seen:
                pending.append((f"{file}#{ident}", None, name))
            elif ident in tree.functions or ident in tree.macros or world_class(ident):
                pending.append((ident, None, name))
    return Evidence(classes, codes, seen, via)


def table_entries(source: str, start: int) -> list[tuple[str, list[str]]]:
    """The `{"name", ident, …}` rows of the initialiser opening after `start`.

    A row may hold nested braces, as `mathOpCmds` does for its operand counts.
    """
    open_brace = source.index("{", start)
    rows: list[tuple[str, list[str]]] = []
    depth = 0
    row_start = None
    for k in range(open_brace, len(source)):
        ch = source[k]
        if ch == "{":
            depth += 1
            if depth == 2:
                row_start = k + 1
        elif ch == "}":
            if depth == 2 and row_start is not None:
                row = source[row_start:k]
                m = re.match(r'\s*"([^"]+)"\s*,(.*)', row, re.DOTALL)
                if m:
                    rows.append((m.group(1), IDENT.findall(strip_c(m.group(2)))))
                row_start = None
            depth -= 1
            if depth == 0:
                break
    return rows


def command_procs(idents: list[str], tree: Tree) -> list[str]:
    """The procedures of a table row: in-tree functions, compile procs excluded."""
    return [
        i for i in idents
        if (i in tree.functions or i in tree.macros)
        and not i.startswith("TclCompile")
        and not i.isupper()
    ]


def builtin_commands(tree: Tree) -> dict[str, list[str]]:
    source = tree.sources["tclBasic.c"]
    m = re.search(r"\bbuiltInCmds\[\]\s*=", source)
    return {name: command_procs(ids, tree) for name, ids in table_entries(source, m.end())}


def ensemble_commands(tree: Tree) -> dict[str, list[str]]:
    """`ensemble subcommand` → procedures, from each `EnsembleImplMap` named
    by a `TclMakeEnsemble(interp, "NAME", MAP)` call or, from 9.1, by a row of
    the `ensembleCommands` table."""
    maps: dict[str, tuple[str, int]] = {}
    for source in tree.sources.values():
        for m in re.finditer(r"\bEnsembleImplMap\s+(\w+)\[\]\s*=", source):
            maps.setdefault(m.group(1), (source, m.end()))
    named: list[tuple[str, str, str | None]] = []
    for source in tree.sources.values():
        for m in re.finditer(r'TclMakeEnsemble\(\s*\w+\s*,\s*"([^"]+)"\s*,\s*(\w+)\s*\)', source):
            named.append((m.group(1), m.group(2), source))
    basic = tree.sources["tclBasic.c"]
    setup = re.search(r"\bensembleCommands\[\]\s*=", basic)
    if setup:
        for name, ids in table_entries(basic, setup.end()):
            if ids:
                named.append((name, ids[0], None))
    found: dict[str, list[str]] = {}
    for ensemble, table, caller in named:
        local = caller and re.search(rf"\b{table}\[\]\s*=", caller)
        if local:
            source, start = caller, local.end()
        elif table in maps:
            source, start = maps[table]
        else:
            continue
        ensemble = ensemble.removeprefix("::")
        for sub, ids in table_entries(source, start):
            found[f"{ensemble} {sub}"] = command_procs(ids, tree)
    return found


def math_commands(tree: Tree) -> dict[str, list[str]]:
    found: dict[str, list[str]] = {}
    source = tree.sources["tclBasic.c"]
    for table, ns in (("BuiltinFuncTable", "tcl::mathfunc"), ("mathOpCmds", "tcl::mathop")):
        t = re.search(rf"\b{table}\[\]\s*=", source)
        if not t:
            continue
        for name, ids in table_entries(source, t.end()):
            procs = command_procs(ids, tree)
            if ns == "tcl::mathop" and not procs:
                procs = ["TclVariadicOpCmd", "TclNoIdentOpCmd", "TclSortingOpCmd", "TclSingleOpCmd"]
                procs = [p for p in procs if p in tree.functions]
            found[f"{ns}::{name}"] = procs
    return found


CASE = re.compile(r"\bcase\s+(\w+)\s*:")
SWITCH = re.compile(r"\bswitch\s*\(\s*\(\s*enum\s+\w+\s*\)\s*\w+\s*\)|\bswitch\s*\(\s*index\s*\)")


def switch_subcommands(tree: Tree, ensemble: str, proc: str) -> dict[str, tuple[list[str], str]]:
    """The `switch`-dispatched subcommands of one `ObjCmd`: each subcommand's
    code is its case, any case it falls into, and the code around the switch.

    Returns `ensemble sub` → (procedures, code)."""
    fn = tree.functions.get(proc)
    if fn is None:
        return {}
    raw = tree.sources[fn.path]
    at = re.search(rf"^{proc}\s*\(", raw, re.MULTILINE)
    if at is None:
        return {}
    raw_body = raw[at.start() :]
    arrays = re.search(r"char\s*\*\s*(?:CONST|const)?\s*\w+\[\]\s*=\s*\{([^}]*)\}", raw_body)
    enum = re.search(r"enum\s+\w*\s*\{([^}]*)\}", fn.body)
    switch = SWITCH.search(fn.body)
    if not (arrays and enum and switch):
        return {}
    names = [s for s in string_literals(arrays.group(1))]
    labels = [s.strip() for s in enum.group(1).split(",") if s.strip()]
    if len(labels) < len(names):
        return {}
    open_brace = fn.body.index("{", switch.end())
    depth = 0
    for k in range(open_brace, len(fn.body)):
        if fn.body[k] == "{":
            depth += 1
        elif fn.body[k] == "}":
            depth -= 1
            if depth == 0:
                break
    block = fn.body[open_brace + 1 : k]
    epilogue = fn.body[k + 1 :]
    # Code after a label that follows a `return` is reached only by `goto`,
    # which the cases that jump there add for themselves.
    label = re.search(r"\n\s*(?!default\b)(\w+)\s*:(?!:)", epilogue)
    if label and re.search(r"\breturn\b[^;]*;\s*$", epilogue[: label.start()]):
        epilogue = epilogue[: label.start()]
    around = fn.body[:open_brace] + epilogue
    # Cases at the switch's own depth.
    cases: list[tuple[str, int]] = []
    depth = 0
    for i, ch in enumerate(block):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif depth == 0:
            m = CASE.match(block, i)
            if m and (i == 0 or not block[i - 1].isalnum()):
                cases.append((m.group(1), i))
    slices: dict[str, str] = {}
    for idx, (label, start) in enumerate(cases):
        end = cases[idx + 1][1] if idx + 1 < len(cases) else len(block)
        slices[label] = block[start:end]
    found: dict[str, tuple[list[str], str]] = {}
    for name, label in zip(names, labels):
        # An option table (`lsearch -all`) is dispatched the same way, but
        # its words are not subcommands.
        if label not in slices or name.startswith("-"):
            continue
        order = [lbl for lbl, _ in cases]
        pos = order.index(label)
        code = slices[label]
        while pos + 1 < len(order) and not re.search(r"\b(break|return|goto)\b[^;]*;\s*\}?\s*$", code.rstrip()):
            pos += 1
            code += slices[order[pos]]
        # A `goto LABEL` reaches the code after `LABEL:` up to the next case
        # of the switch, or to the end of the function after it.
        followed: set[str] = set()
        pending_targets = re.findall(r"\bgoto\s+(\w+)\s*;", code)
        while pending_targets:
            target = pending_targets.pop()
            if target in followed:
                continue
            followed.add(target)
            label = re.compile(rf"(?m)^\s*{target}\s*:(?!:)")
            inside = label.search(block)
            if inside:
                ends = [start for _, start in cases if start > inside.start()]
                reached = block[inside.start() : ends[0] if ends else len(block)]
            else:
                outside = label.search(fn.body, k)
                reached = fn.body[outside.start() :] if outside else ""
            code += reached
            pending_targets += re.findall(r"\bgoto\s+(\w+)\s*;", reached)
        found[f"{ensemble} {name}"] = ([], around + code)
    return found


def release_commands(tree: Tree) -> dict[str, tuple[list[str], list[str], str | None]]:
    """Every command and subcommand → (root procedures, extra code, the
    function the extra code is a slice of)."""
    commands: dict[str, tuple[list[str], list[str]]] = {}
    for name, procs in builtin_commands(tree).items():
        commands[name] = (procs, [], None)
        for proc in procs:
            for sub, (roots, code) in switch_subcommands(tree, name, proc).items():
                commands.setdefault(sub, (roots, [code], proc))
    for name, procs in ensemble_commands(tree).items():
        commands[name] = (procs, [], None)
    for name, procs in math_commands(tree).items():
        commands[name] = (procs, [], None)
    # 8.5 to 9.0 register the C half of `clock` as `::tcl::clock::NAME`
    # from the `clockCommands` table, behind the library's ensemble.
    clock = tree.sources.get("tclClock.c", "")
    table = re.search(r"\bclockCommands\[\]\s*=", clock)
    if table:
        for name, ids in table_entries(clock, table.end()):
            commands.setdefault(f"clock {name}", (command_procs(ids, tree), [], None))
    return commands


def evidence_lines(tree: Tree) -> list[str]:
    lines = []
    for name, (roots, bodies, context) in sorted(release_commands(tree).items()):
        if not roots and not bodies:
            continue
        ev = closure(tree, roots, bodies, context)
        verdict = "closed" if ev.closed else "open"
        classes = [f"{cls}:{iface}" for cls, iface in sorted(ev.classes.items())]
        classes += [f"completion:{code}" for code in sorted(ev.codes)]
        lines.append(f"{name}\t{verdict}\t{','.join(classes) or '-'}\t{','.join(roots) or 'switch'}")
    return lines


def main(argv: list[str]) -> int:
    root = pathlib.Path(__file__).resolve().parents[2]
    tmp = pathlib.Path(argv[1]) if len(argv) > 1 else root / "tmp"
    out = pathlib.Path(argv[2]) if len(argv) > 2 else root / "rust/tcl-registry/tests/data/command_evidence"
    out.mkdir(parents=True, exist_ok=True)
    for release in RELEASES:
        generic = tmp / f"tcl{release}" / "generic"
        if not generic.is_dir():
            print(f"missing {generic}", file=sys.stderr)
            return 1
        tree = load_tree(generic)
        lines = evidence_lines(tree)
        major_minor = ".".join(release.split(".")[:2])
        header = (
            f"# tcl{release} generic/*.c, by scripts/dev/tcl_command_evidence.py\n"
            "# NAME\tVERDICT\tCLASSES\tROOTS\n"
        )
        (out / f"tcl{major_minor}.txt").write_text(header + "\n".join(lines) + "\n")
        print(f"tcl{release}: {len(lines)} commands, {sum('	closed	' in l for l in lines)} closed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
