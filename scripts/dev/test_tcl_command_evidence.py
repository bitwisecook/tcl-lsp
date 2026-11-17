# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Tests for the pure parts of `tcl_command_evidence.py`."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import tcl_command_evidence as ev  # noqa: E402

ANSI_SOURCE = """\
int
Tcl_LlengthObjCmd(
    void *dummy,
    Tcl_Interp *interp)
{
    /* Tcl_SetVar2 in a comment is not a call. */
    Tcl_WrongNumArgs(interp, 1, objv, "list");
    return Helper(interp);
}

static int
Helper(
    Tcl_Interp *interp)
{
    return TCL_OK;
}
"""

KR_SOURCE = """\
int
Tcl_PutsObjCmd(dummy, interp, objc, objv)
    ClientData dummy;
    Tcl_Interp *interp;
{
    char *s = "Tcl_GetTime(";
    return Tcl_WriteObj(chan, objv[1]);
}
"""


def tree_of(**sources: str) -> ev.Tree:
    tree = ev.Tree()
    for name, raw in sources.items():
        tree.sources[name] = raw
        stripped = ev.strip_c(raw)
        tree.macros.update(ev.find_macros(stripped))
        tree.functions.update(ev.find_functions(stripped, name))
        tree.tables[name] = ev.find_tables(stripped)
    return tree


def test_strip_c_blanks_comments_and_literals_but_keeps_lines():
    text = 'a /* b\nc */ "d(e)" \'f\' // g\nh'
    stripped = ev.strip_c(text)
    assert stripped.count("\n") == text.count("\n")
    assert "b" not in stripped and "d(e)" not in stripped and "g" not in stripped
    assert stripped.startswith("a ") and stripped.endswith("h")


def test_functions_are_found_in_ansi_and_k_and_r_layout():
    tree = tree_of(**{"a.c": ANSI_SOURCE, "b.c": KR_SOURCE})
    assert {"Tcl_LlengthObjCmd", "Helper", "Tcl_PutsObjCmd"} <= tree.functions.keys()
    assert "Helper(interp)" in tree.functions["Tcl_LlengthObjCmd"].body


def test_the_closure_follows_calls_and_ignores_comments_and_strings():
    tree = tree_of(**{"a.c": ANSI_SOURCE, "b.c": KR_SOURCE})
    closed = ev.closure(tree, ["Tcl_LlengthObjCmd"])
    assert closed.closed, closed.classes
    assert "Helper" in closed.reached
    puts = ev.closure(tree, ["Tcl_PutsObjCmd"])
    assert puts.classes == {"channels": "Tcl_WriteObj"}


def test_a_macro_is_followed_like_a_function():
    tree = tree_of(**{"a.h": "#define TclGetVarish(i, n) \\\n    Tcl_GetVar2(i, n, NULL, 0)\n",
                      "a.c": "int\nF(\n    int x)\n{\n    return TclGetVarish(interp, x);\n}\n"})
    assert ev.closure(tree, ["F"]).classes == {"variables": "Tcl_GetVar2"}


def test_non_normal_completion_codes_open_the_closure():
    tree = tree_of(**{"a.c": "int\nBreakCmd(\n    int x)\n{\n    return TCL_BREAK;\n}\n"})
    result = ev.closure(tree, ["BreakCmd"])
    assert not result.closed and result.codes == {"TCL_BREAK"}


def test_error_reporting_is_not_entered():
    tree = tree_of(**{"a.c": (
        "void\nTcl_SetErrorCode(\n    int x)\n{\n    Tcl_SetVar2(x);\n}\n\n"
        "int\nF(\n    int x)\n{\n    Tcl_SetErrorCode(x);\n    return TCL_ERROR;\n}\n"
    )})
    assert ev.closure(tree, ["F"]).closed


def test_table_rows_tolerate_nested_braces():
    source = 'static const X t[] = {\n    { "+", AddCmd, Compile, {0}, NULL},\n    { "!", NotCmd, Compile, {1}, "boolean"},\n    {NULL, NULL}\n};'
    rows = ev.table_entries(source, 0)
    assert rows == [("+", ["AddCmd", "Compile", "NULL"]), ("!", ["NotCmd", "Compile"])]


SWITCH_SOURCE = """\
int
Tcl_StringObjCmd(
    int objc)
{
    static const char *options[] = {
\t"length", "map", NULL
    };
    enum options {
\tSTR_LENGTH, STR_MAP
    };
    Prologue();
    switch ((enum options) index) {
    case STR_LENGTH:
\tLengthOf();
\tbreak;
    case STR_MAP: {
\tif (x) {
\t    goto badMap;
\t}
\tbreak;
    }
    }
    return TCL_OK;
  badMap:
    Tcl_SetVar2(x);
    return TCL_ERROR;
}
"""


def test_switch_subcommands_slice_each_case_with_the_code_around_it():
    tree = tree_of(**{"a.c": SWITCH_SOURCE})
    subs = ev.switch_subcommands(tree, "string", "Tcl_StringObjCmd")
    assert set(subs) == {"string length", "string map"}
    _, length = subs["string length"]
    assert "LengthOf" in length and "Prologue" in length
    _, mapped = subs["string map"]
    assert "LengthOf" not in mapped
    assert "Tcl_SetVar2" in mapped, "a goto reaches the code after its label"


def test_a_reviewed_edge_is_not_followed():
    tree = tree_of(**{"a.c": (
        "int\nGetEndOffsetFromObj(\n    int x)\n{\n    Tcl_ExprObj(x);\n    return TCL_OK;\n}\n\n"
        "int\nOther(\n    int x)\n{\n    Tcl_ExprObj(x);\n    return TCL_OK;\n}\n"
    )})
    assert ev.closure(tree, ["GetEndOffsetFromObj"]).closed
    assert ev.closure(tree, ["Other"]).classes == {"scripts": "Tcl_ExprObj"}


GOTO_CASE_SOURCE = """\
int
Tcl_StringObjCmd(
    int objc)
{
    static const char *options[] = {
\t"compare", "equal", "is", NULL
    };
    enum options {
\tSTR_COMPARE, STR_EQUAL, STR_IS
    };
    switch ((enum options) index) {
    case STR_EQUAL:
\tgoto shared;
    case STR_COMPARE:
    shared:
\tCompareThem();
\tbreak;
    case STR_IS:
\tTcl_ObjSetVar2(x);
\tbreak;
    }
    return TCL_OK;
}
"""


def test_a_goto_into_another_case_takes_only_that_case():
    tree = tree_of(**{"a.c": GOTO_CASE_SOURCE})
    subs = ev.switch_subcommands(tree, "string", "Tcl_StringObjCmd")
    _, equal = subs["string equal"]
    assert "CompareThem" in equal
    assert "Tcl_ObjSetVar2" not in equal
    _, is_ = subs["string is"]
    assert "Tcl_ObjSetVar2" in is_


def test_interpreter_fields_and_stored_pointers_open_the_closure():
    tree = tree_of(**{"a.c": (
        "int\nReadsTable(\n    int x)\n{\n    return iPtr->packageTable;\n}\n\n"
        "int\nSetsResult(\n    int x)\n{\n    iPtr->objResultPtr = x;\n    return TCL_OK;\n}\n\n"
        "int\nCallsStored(\n    int x)\n{\n    cmdPtr->objProc(x);\n    return TCL_OK;\n}\n\n"
        "int\nCallsLocal(\n    int x)\n{\n    fn = Helper;\n    (*fn)(x);\n    return TCL_OK;\n}\n\n"
        "int\nHelper(\n    int x)\n{\n    return TCL_OK;\n}\n"
    )})
    assert ev.closure(tree, ["ReadsTable"]).classes == {"interpreter": "ReadsTable:packageTable"}
    assert ev.closure(tree, ["SetsResult"]).closed
    assert ev.closure(tree, ["CallsStored"]).classes == {"indirect": "CallsStored:objProc"}
    local = ev.closure(tree, ["CallsLocal"])
    assert local.closed and "Helper" in local.reached


def test_a_file_static_table_is_followed_only_from_its_own_file():
    tree = tree_of(**{
        "a.c": "static const T table[] = {\n    {\"x\", Writer},\n};\n\nint\nUses(\n    int x)\n{\n    return table[x].proc(x);\n}\n\nint\nWriter(\n    int x)\n{\n    return Tcl_SetVar2(x);\n}\n",
        "b.c": "int\nOther(\n    int table)\n{\n    return table;\n}\n",
    })
    assert ev.closure(tree, ["Uses"]).classes.get("variables") == "Tcl_SetVar2"
    assert ev.closure(tree, ["Other"]).closed


def test_a_dereferenced_result_free_procedure_is_a_value_procedure():
    tree = tree_of(**{"a.c": (
        "int\nResets(\n    int x)\n{\n    (*iPtr->freeProc)(iPtr->result);\n    return TCL_OK;\n}\n"
    )})
    assert ev.closure(tree, ["Resets"]).closed


def test_a_call_nothing_defines_opens_the_closure_unless_it_is_pure_libc():
    tree = tree_of(**{"a.c": (
        "int\nF(\n    int x)\n{\n    memcpy(a, b, 1);\n    return TclpGetDayTime(x);\n}\n"
    )})
    assert ev.closure(tree, ["F"]).classes == {"unresolved": "F:TclpGetDayTime"}


def test_a_call_through_a_local_pointer_is_followed_only_when_every_value_is_named():
    tree = tree_of(**{"a.c": (
        "int\nNamed(\n    int x)\n{\n    cmp = x ? Helper : strcmp;\n    return cmp(x);\n}\n\n"
        "int\nStored(\n    int x)\n{\n    proc = refPtr->callback;\n    return proc(x);\n}\n\n"
        "int\nParameter(\n    int (*func)(int),\n    int x)\n{\n    return func(x);\n}\n\n"
        "int\nHelper(\n    int x)\n{\n    return TCL_OK;\n}\n"
    )})
    assert ev.closure(tree, ["Named"]).closed
    assert ev.closure(tree, ["Stored"]).classes == {"unresolved": "Stored:proc"}
    assert ev.closure(tree, ["Parameter"]).closed


def test_a_switch_slice_follows_its_files_tables_and_a_call_through_an_unknown_table_opens():
    tree = tree_of(**{"a.c": """\
static const Proc subCmds[] = {
    InfoWriter,
};

int
Tcl_TraceObjCmd(
    int objc)
{
    static const char *options[] = {
\t"info", "other", NULL
    };
    enum options {
\tTRACE_INFO, TRACE_OTHER
    };
    switch ((enum options) index) {
    case TRACE_INFO:
\treturn subCmds[objc](objc);
    case TRACE_OTHER:
\treturn unknownTable[objc](objc);
    }
    return TCL_OK;
}

int
InfoWriter(
    int x)
{
    return Tcl_SetVar2(x);
}
"""})
    subs = ev.switch_subcommands(tree, "trace", "Tcl_TraceObjCmd")
    roots, code = subs["trace info"]
    assert ev.closure(tree, roots, [code], "Tcl_TraceObjCmd").classes == {"variables": "Tcl_SetVar2"}
    roots, code = subs["trace other"]
    assert "indirect" in ev.closure(tree, roots, [code], "Tcl_TraceObjCmd").classes
