"""tcltest C binary commands -- built-in test commands from tclTest.c.

These commands are compiled into the ``tcltest`` binary (not the tcltest
package).  They are available in the Tcl test suite interpreter and are
used extensively in the core Tcl test files (``tests/*.test``).

Extracted from ``generic/tclTest.c``, ``generic/tclTestObj.c``, and
``generic/tclTestProcBodyObj.c`` across Tcl 8.4–9.0.
"""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl test binary (tclTest.c)"

# (command_name, one-line summary) for all C test commands.
_C_TEST_COMMANDS: tuple[tuple[str, str], ...] = (
    ("testapplylambda", "Test apply with lambda expressions."),
    ("testappverifierpresent", "Check whether the app verifier is present (9.0+)."),
    ("testasync", "Test asynchronous event handlers."),
    ("testbigdata", "Create large data objects for testing (9.0+)."),
    ("testbignumobj", "Test bignum Tcl_Obj operations."),
    ("testbooleanobj", "Test boolean Tcl_Obj operations."),
    ("testbumpinterpepoch", "Bump the interpreter compilation epoch."),
    ("testbytestring", "Create a bytestring Tcl_Obj from a string."),
    ("testchannel", "Test channel introspection and manipulation."),
    ("testchannelevent", "Test channel event handling."),
    ("testcmdinfo", "Test Tcl_GetCommandInfo / Tcl_SetCommandInfo."),
    ("testcmdtoken", "Test command token operations."),
    ("testcmdtrace", "Test command tracing."),
    ("testconcatobj", "Test Tcl_ConcatObj."),
    ("testcpuid", "Test CPUID instruction."),
    ("testcreatecommand", "Test Tcl_CreateCommand."),
    ("testdcall", "Test Tcl_CallWhenDeleted."),
    ("testdel", "Test command deletion callbacks."),
    ("testdelassocdata", "Test Tcl_DeleteAssocData."),
    ("testdoubledigits", "Test double-to-string digit conversion."),
    ("testdoubleobj", "Test double Tcl_Obj operations."),
    ("testdstring", "Test Tcl_DString operations."),
    ("testencoding", "Test encoding operations."),
    ("testevalex", "Test Tcl_EvalEx."),
    ("testevalobjv", "Test Tcl_EvalObjv."),
    ("testevent", "Test Tcl_QueueEvent and event processing."),
    ("testexithandler", "Test Tcl_CreateExitHandler."),
    ("testexitmainloop", "Test Tcl_SetMainLoop exit."),
    ("testexprdouble", "Test Tcl_ExprDouble."),
    ("testexprdoubleobj", "Test Tcl_ExprDoubleObj."),
    ("testexprlong", "Test Tcl_ExprLong."),
    ("testexprlongobj", "Test Tcl_ExprLongObj."),
    ("testexprparser", "Test the expression parser."),
    ("testexprstring", "Test Tcl_ExprString."),
    ("testfevent", "Test file event handling."),
    ("testfile", "Test file system operations."),
    ("testfilelink", "Test file link operations."),
    ("testfilesystem", "Test the virtual file system."),
    ("testfindfirst", "Test Tcl_FindFirst / hash iteration."),
    ("testfindlast", "Test Tcl_FindLast / hash iteration."),
    ("testfstildeexpand", "Test tilde expansion in file paths (9.0+)."),
    ("testgetassocdata", "Test Tcl_GetAssocData."),
    ("testgetindexfromobjstruct", "Test Tcl_GetIndexFromObjStruct."),
    ("testgetint", "Test Tcl_GetInt."),
    ("testgetintforindex", "Test Tcl_GetIntForIndex (9.0+)."),
    ("testgetplatform", "Test Tcl_GetPlatform."),
    ("testgetunichar", "Test Tcl_GetUniChar."),
    ("testgetvarfullname", "Test Tcl_GetVariableFullName."),
    ("testhandlecount", "Test handle count tracking (9.0+)."),
    ("testhashsystemhash", "Test system hash implementation."),
    ("testindexobj", "Test Tcl_GetIndexFromObj."),
    ("testinterpdelete", "Test Tcl_DeleteInterp."),
    ("testinterpresolver", "Test the namespace / command resolver."),
    ("testintobj", "Test integer Tcl_Obj operations."),
    ("testlink", "Test Tcl_LinkVar."),
    ("testlinkarray", "Test linked array variables (9.0+)."),
    ("testlistobj", "Test list Tcl_Obj operations."),
    ("testlistrep", "Test list internal representation (9.0+)."),
    ("testlocale", "Test locale operations."),
    ("testlongsize", "Report sizeof(long) (9.0+)."),
    ("testlutil", "Test list utility functions (9.0+)."),
    ("testmainthread", "Return the ID of the main thread."),
    ("testmsb", "Test most-significant-bit computation (9.0+)."),
    ("testnrelevels", "Test NRE evaluation levels."),
    ("testnreunwind", "Test NRE stack unwinding."),
    ("testnumutfchars", "Test Tcl_NumUtfChars."),
    ("testobj", "Test Tcl_Obj type operations."),
    ("testpanic", "Test Tcl_Panic."),
    ("testparseargs", "Test argument parsing."),
    ("testparser", "Test the Tcl script parser."),
    ("testparsevar", "Test Tcl_ParseVar."),
    ("testparsevarname", "Test variable name parsing."),
    ("testpreferstable", "Test TIP 268 stable-vs-latest preference (9.0+)."),
    ("testprint", "Test formatted output."),
    ("testpurebytesobj", "Test pure-bytes Tcl_Obj operations."),
    ("testregexp", "Test regular expression engine."),
    ("testreturn", "Test Tcl_SetReturnOptions."),
    ("testsaveresult", "Test Tcl_SaveResult / Tcl_RestoreResult."),
    ("testservicemode", "Test Tcl_SetServiceMode."),
    ("testset2", "Test Tcl_SetVar2."),
    ("testsetassocdata", "Test Tcl_SetAssocData."),
    ("testsetbytearraylength", "Test Tcl_SetByteArrayLength."),
    ("testseterr", "Test Tcl_SetErrno."),
    ("testseterrorcode", "Test Tcl_SetErrorCode."),
    ("testsetmainloop", "Test Tcl_SetMainLoop."),
    ("testsetnoerr", "Test clearing errno."),
    ("testsetobjerrorcode", "Test Tcl_SetObjErrorCode."),
    ("testsetplatform", "Test Tcl_SetPlatform."),
    ("testsimplefilesystem", "Test a simple virtual file system."),
    ("testsize", "Report sizeof various types."),
    ("testsocket", "Test socket operations (9.0+)."),
    ("teststaticlibrary", "Test Tcl_StaticLibrary (9.0+)."),
    ("teststaticpkg", "Test Tcl_StaticPackage."),
    ("teststringbytes", "Test Tcl_GetStringFromObj byte length."),
    ("teststringobj", "Test string Tcl_Obj operations."),
    ("testtranslatefilename", "Test Tcl_TranslateFileName."),
    ("testuniclass", "Test Unicode character classification (9.0+)."),
    ("testupvar", "Test Tcl_UpVar / Tcl_UpVar2."),
    ("testutfnext", "Test Tcl_UtfNext."),
    ("testutfprev", "Test Tcl_UtfPrev."),
    ("testwrongnumargs", "Test Tcl_WrongNumArgs."),
    # Non-test-prefixed commands from the test binary
    ("gettimes", "Get timing information for performance testing."),
    ("noop", "Do nothing (used for timing baselines)."),
    ("lgen", "Lazy list generator command (9.0+)."),
    ("lstring", "String-backed list command for testing (9.0+)."),
)


def _make_spec(cmd_name: str, summary: str) -> CommandSpec:
    return CommandSpec(
        name=cmd_name,
        warn_missing_import=False,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(cmd_name,),
            source=_SOURCE,
        ),
        validation=ValidationSpec(arity=Arity(0)),
    )


def _make_cls(cmd_name: str, summary: str) -> type[CommandDef]:
    """Build a CommandDef subclass for a single C test command."""

    class _Cmd(CommandDef):
        name = cmd_name

        @classmethod
        def spec(cls) -> CommandSpec:
            return _make_spec(cmd_name, summary)

    _Cmd.__name__ = _Cmd.__qualname__ = cmd_name.replace("-", "_")
    return _Cmd


for _name, _summary in _C_TEST_COMMANDS:
    register(_make_cls(_name, _summary))
