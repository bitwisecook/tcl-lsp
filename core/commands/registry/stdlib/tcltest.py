"""tcltest -- Tcl stdlib test framework package (package require tcltest).

Covers all public (namespace-exported) commands from the tcltest package
as shipped in Tcl 8.4–9.0, including deprecated configuration accessors
and tcltest-1 compatibility commands.
"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import ArgRole, Arity, BodyKind
from ._base import register

_PKG = "tcltest"
_SOURCE = "Tcl stdlib tcltest package"
_SOURCE_DEPR = "Tcl stdlib tcltest package (deprecated)"
_SOURCE_V1 = "Tcl stdlib tcltest package (v1 compat)"


# Primary functional commands

_TCLTEST_BODY_OPTIONS = frozenset({"-setup", "-body", "-cleanup"})


def _test_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve argument roles for ``tcltest::test``.

    Syntax:
      - test name description ?option value ...?
      - test name description ?constraints? body result

    Options whose values are Tcl script bodies: -setup, -body, -cleanup.
    """
    roles: dict[int, frozenset[ArgRole]] = {}
    body = frozenset({ArgRole.BODY})
    # Skip name (0) and description (1), then scan option-value pairs.
    has_body_option = False
    i = 2
    while i < len(args) - 1:
        if args[i] in _TCLTEST_BODY_OPTIONS:
            roles[i + 1] = body
            has_body_option = True
        i += 2
    # Legacy positional form: test name description ?constraints? body result
    # Only apply when no body-related options were used.
    if not has_body_option and len(args) >= 4:
        # In the positional form, the body is always the penultimate argument.
        body_index = len(args) - 2
        roles[body_index] = body
    return roles


@register
class TcltestTest(CommandDef):
    name = "tcltest::test"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::test",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Define and run a single test case.",
                synopsis=(
                    "tcltest::test name description ?option value ...?",
                    "tcltest::test name description ?constraints? body result",
                ),
                snippet=(
                    "The primary command for defining tests.  Options include "
                    "``-body``, ``-result``, ``-output``, ``-errorOutput``, "
                    "``-returnCodes``, ``-match``, ``-setup``, ``-cleanup``, "
                    "and ``-constraints``."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="test name description ?option value ...?",
                    options=(
                        OptionSpec(name="-body", takes_value=True, value_hint="script"),
                        OptionSpec(name="-result", takes_value=True),
                        OptionSpec(name="-output", takes_value=True),
                        OptionSpec(name="-errorOutput", takes_value=True),
                        OptionSpec(name="-returnCodes", takes_value=True),
                        OptionSpec(name="-errorCode", takes_value=True),
                        OptionSpec(name="-match", takes_value=True),
                        OptionSpec(name="-setup", takes_value=True, value_hint="script"),
                        OptionSpec(name="-cleanup", takes_value=True, value_hint="script"),
                        OptionSpec(name="-constraints", takes_value=True),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(2)),
            arg_role_resolver=_test_arg_roles,
            body_kind=BodyKind.STRUCTURAL,
            is_namespace_exported=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class TcltestCleanupTests(CommandDef):
    name = "tcltest::cleanupTests"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::cleanupTests",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Print statistics and clean up after a test file.",
                synopsis=("tcltest::cleanupTests",),
                snippet=(
                    "Call at the end of each test file.  Prints a summary "
                    "of passed/failed/skipped tests and performs clean-up."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


# Configuration commands


@register
class TcltestConfigure(CommandDef):
    name = "tcltest::configure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::configure",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set tcltest configuration options.",
                synopsis=("tcltest::configure ?option? ?value option value ...?",),
                snippet=(
                    "Options include ``-verbose``, ``-debug``, "
                    "``-outfile``, ``-errfile``, ``-tmpdir``, "
                    "``-testdir``, ``-file``, ``-notfile``, ``-match``, "
                    "``-skip``, ``-constraints``, ``-limitconstraints``, "
                    "``-singleproc``, ``-preservecore``, ``-load``, "
                    "``-loadfile``."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )


@register
class TcltestCustomMatch(CommandDef):
    name = "tcltest::customMatch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::customMatch",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Register a custom matching command for test results.",
                synopsis=("tcltest::customMatch mode command",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class TcltestTestConstraint(CommandDef):
    name = "tcltest::testConstraint"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::testConstraint",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set a named test constraint boolean.",
                synopsis=("tcltest::testConstraint constraint ?value?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class TcltestOutputChannel(CommandDef):
    name = "tcltest::outputChannel"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::outputChannel",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set the channel for test output.",
                synopsis=("tcltest::outputChannel ?channelID?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )


@register
class TcltestErrorChannel(CommandDef):
    name = "tcltest::errorChannel"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::errorChannel",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set the channel for test error output.",
                synopsis=("tcltest::errorChannel ?channelID?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )


@register
class TcltestInterpreter(CommandDef):
    name = "tcltest::interpreter"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::interpreter",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set the path of the interpreter for subprocess tests.",
                synopsis=("tcltest::interpreter ?interp?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )


@register
class TcltestRunAllTests(CommandDef):
    name = "tcltest::runAllTests"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::runAllTests",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Source all test files matching the configured patterns.",
                synopsis=("tcltest::runAllTests",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class TcltestLoadTestedCommands(CommandDef):
    name = "tcltest::loadTestedCommands"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::loadTestedCommands",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Evaluate the ``-load`` or ``-loadfile`` script to load commands under test.",
                synopsis=("tcltest::loadTestedCommands",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class TcltestMakeFile(CommandDef):
    name = "tcltest::makeFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::makeFile",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Create a temporary test file with the given contents.",
                synopsis=("tcltest::makeFile contents name ?directory?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
        )


@register
class TcltestRemoveFile(CommandDef):
    name = "tcltest::removeFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::removeFile",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Remove a temporary test file.",
                synopsis=("tcltest::removeFile name ?directory?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class TcltestMakeDirectory(CommandDef):
    name = "tcltest::makeDirectory"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::makeDirectory",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Create a temporary test directory.",
                synopsis=("tcltest::makeDirectory name ?directory?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class TcltestRemoveDirectory(CommandDef):
    name = "tcltest::removeDirectory"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::removeDirectory",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Remove a temporary test directory.",
                synopsis=("tcltest::removeDirectory name ?directory?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class TcltestViewFile(CommandDef):
    name = "tcltest::viewFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::viewFile",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the contents of a file as a string.",
                synopsis=("tcltest::viewFile name ?directory?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


# Deprecated configuration accessors (candidates for deprecation; each
# duplicates a ``tcltest::configure -option`` call).


def _depr_accessor(
    name: str,
    summary: str,
    synopsis: str,
    *,
    arity: Arity = Arity(0, 1),
) -> CommandSpec:
    """Helper for deprecated get/set configuration accessor commands."""
    return CommandSpec(
        name=f"tcltest::{name}",
        required_package=_PKG,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(synopsis,),
            source=_SOURCE_DEPR,
        ),
        validation=ValidationSpec(arity=arity),
        deprecated_replacement="tcltest::configure",
    )


@register
class TcltestVerbose(CommandDef):
    name = "tcltest::verbose"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "verbose",
            "Get or set verbosity level.  Deprecated: use ``configure -verbose``.",
            "tcltest::verbose ?level?",
        )


@register
class TcltestDebug(CommandDef):
    name = "tcltest::debug"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "debug",
            "Get or set debug level.  Deprecated: use ``configure -debug``.",
            "tcltest::debug ?level?",
        )


@register
class TcltestErrorFile(CommandDef):
    name = "tcltest::errorFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "errorFile",
            "Get or set the error output file.  Deprecated: use ``configure -errfile``.",
            "tcltest::errorFile ?filename?",
        )


@register
class TcltestOutputFile(CommandDef):
    name = "tcltest::outputFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "outputFile",
            "Get or set the output file.  Deprecated: use ``configure -outfile``.",
            "tcltest::outputFile ?filename?",
        )


@register
class TcltestLimitConstraints(CommandDef):
    name = "tcltest::limitConstraints"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "limitConstraints",
            "Get or set constraint limiting.  Deprecated: use ``configure -limitconstraints``.",
            "tcltest::limitConstraints ?boolean?",
        )


@register
class TcltestLoadFile(CommandDef):
    name = "tcltest::loadFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "loadFile",
            "Get or set the load file path.  Deprecated: use ``configure -loadfile``.",
            "tcltest::loadFile ?filename?",
        )


@register
class TcltestLoadScript(CommandDef):
    name = "tcltest::loadScript"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "loadScript",
            "Get or set the load script.  Deprecated: use ``configure -load``.",
            "tcltest::loadScript ?script?",
        )


@register
class TcltestMatch(CommandDef):
    name = "tcltest::match"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "match",
            "Get or set test match patterns.  Deprecated: use ``configure -match``.",
            "tcltest::match ?patternList?",
        )


@register
class TcltestSkip(CommandDef):
    name = "tcltest::skip"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "skip",
            "Get or set test skip patterns.  Deprecated: use ``configure -skip``.",
            "tcltest::skip ?patternList?",
        )


@register
class TcltestMatchFiles(CommandDef):
    name = "tcltest::matchFiles"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "matchFiles",
            "Get or set matching file patterns.  Deprecated: use ``configure -file``.",
            "tcltest::matchFiles ?patternList?",
        )


@register
class TcltestMatchDirectories(CommandDef):
    name = "tcltest::matchDirectories"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "matchDirectories",
            "Get or set matching directory patterns.  Deprecated: use ``configure -relateddir``.",
            "tcltest::matchDirectories ?patternList?",
        )


@register
class TcltestSkipFiles(CommandDef):
    name = "tcltest::skipFiles"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "skipFiles",
            "Get or set file skip patterns.  Deprecated: use ``configure -notfile``.",
            "tcltest::skipFiles ?patternList?",
        )


@register
class TcltestSkipDirectories(CommandDef):
    name = "tcltest::skipDirectories"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "skipDirectories",
            "Get or set directory skip patterns.  Deprecated: use ``configure -asidefromdir``.",
            "tcltest::skipDirectories ?patternList?",
        )


@register
class TcltestPreserveCore(CommandDef):
    name = "tcltest::preserveCore"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "preserveCore",
            "Get or set core file preservation.  Deprecated: use ``configure -preservecore``.",
            "tcltest::preserveCore ?level?",
        )


@register
class TcltestSingleProcess(CommandDef):
    name = "tcltest::singleProcess"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "singleProcess",
            "Get or set single-process mode.  Deprecated: use ``configure -singleproc``.",
            "tcltest::singleProcess ?boolean?",
        )


@register
class TcltestTemporaryDirectory(CommandDef):
    name = "tcltest::temporaryDirectory"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "temporaryDirectory",
            "Get or set the temporary directory.  Deprecated: use ``configure -tmpdir``.",
            "tcltest::temporaryDirectory ?path?",
        )


@register
class TcltestTestsDirectory(CommandDef):
    name = "tcltest::testsDirectory"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "testsDirectory",
            "Get or set the tests directory.  Deprecated: use ``configure -testdir``.",
            "tcltest::testsDirectory ?path?",
        )


@register
class TcltestWorkingDirectory(CommandDef):
    name = "tcltest::workingDirectory"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _depr_accessor(
            "workingDirectory",
            "Get or set the working directory for tests.",
            "tcltest::workingDirectory ?path?",
        )


@register
class TcltestNormalizeMsg(CommandDef):
    name = "tcltest::normalizeMsg"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::normalizeMsg",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Normalise an error message for comparison (lowercase, strip trailing newline).",
                synopsis=("tcltest::normalizeMsg msg",),
                source=_SOURCE_DEPR,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            deprecated_replacement="tcltest::customMatch",
        )


@register
class TcltestNormalizePath(CommandDef):
    name = "tcltest::normalizePath"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::normalizePath",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Normalise a file path (8.4 compatibility shim for ``file normalize``).",
                synopsis=("tcltest::normalizePath pathVar",),
                source=_SOURCE_DEPR,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            arg_roles={0: ArgRole.VAR_WRITE},
            deprecated_replacement="file normalize",
        )


@register
class TcltestBytestring(CommandDef):
    name = "tcltest::bytestring"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::bytestring",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Convert a string to its byte representation (Tcl < 9.0).",
                synopsis=("tcltest::bytestring string",),
                snippet="Equivalent to ``encoding convertfrom identity``.  Not exported in Tcl 9.0+.",
                source=_SOURCE_DEPR,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            deprecated_replacement="encoding convertfrom identity",
        )


# tcltest v1 compatibility commands


@register
class TcltestSaveState(CommandDef):
    name = "tcltest::saveState"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::saveState",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Save current interpreter state (procs and vars) for later restoration.",
                synopsis=("tcltest::saveState",),
                source=_SOURCE_V1,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class TcltestRestoreState(CommandDef):
    name = "tcltest::restoreState"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::restoreState",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Restore interpreter state saved by ``saveState``.",
                synopsis=("tcltest::restoreState",),
                source=_SOURCE_V1,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class TcltestMainThread(CommandDef):
    name = "tcltest::mainThread"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::mainThread",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set the main thread ID.",
                synopsis=("tcltest::mainThread ?id?",),
                source=_SOURCE_V1,
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )


@register
class TcltestGetMatchingFiles(CommandDef):
    name = "tcltest::getMatchingFiles"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::getMatchingFiles",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return list of test files matching configured patterns.",
                synopsis=("tcltest::getMatchingFiles ?directory ...?",),
                source=_SOURCE_V1,
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )


@register
class TcltestThreadReap(CommandDef):
    name = "tcltest::threadReap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcltest::threadReap",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Terminate all threads except the main thread and return the thread count.",
                synopsis=("tcltest::threadReap",),
                source=_SOURCE_V1,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )
