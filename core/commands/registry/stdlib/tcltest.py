"""tcltest -- Tcl stdlib test framework package (package require tcltest)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_PKG = "tcltest"
_SOURCE = "Tcl stdlib tcltest package"


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
            validation=ValidationSpec(arity=Arity(2)),
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
